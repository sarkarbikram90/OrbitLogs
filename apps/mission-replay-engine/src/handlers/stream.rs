// Copyright 2026 OrbitLogs Authors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::sse::{Event, KeepAlive, Sse},
};
use futures_util::stream::Stream;
use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tracing::{error, info};

use crate::db;
use crate::engine::query_and_send_events;
use crate::metrics;
use crate::session::api_error;
use crate::AppState;

pub async fn stream_replay(
    State(state): State<Arc<AppState>>,
    Path(id_param): Path<String>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, axum::response::Response> {
    let session_id = match uuid::Uuid::parse_str(&id_param) {
        Ok(uid) => uid,
        Err(_) => return Err(api_error(StatusCode::BAD_REQUEST, "Invalid session ID")),
    };

    let session = {
        if let Some(s) = state.sessions.get(&session_id) {
            s.clone()
        } else {
            match db::get_replay_session(&state.db_pool, session_id).await {
                Ok(Some(s)) => {
                    state.sessions.insert(session_id, s.clone());
                    s
                }
                Ok(None) => {
                    return Err(api_error(StatusCode::NOT_FOUND, "Replay session not found"))
                }
                Err(e) => {
                    error!("Database error fetching session: {}", e);
                    return Err(api_error(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Database error",
                    ));
                }
            }
        }
    };

    let (tx, rx) = mpsc::channel::<Result<Event, Infallible>>(100);
    let state_clone = Arc::clone(&state);

    tokio::spawn(async move {
        let satellite_id = session.satellite_id.clone();
        let start_time = session.start_time;
        let end_time = session.end_time;
        let mut local_position = session.current_position;
        let mut last_known_position = session.current_position;
        let mut last_known_status = session.status.clone();
        let mut last_known_speed = session.speed;

        let mut last_tick = tokio::time::Instant::now();
        let mut last_db_write = tokio::time::Instant::now();
        let tick_duration = Duration::from_millis(200);

        info!("Starting SSE Replay stream for session: {}", session_id);

        loop {
            tokio::time::sleep(tick_duration).await;

            let current_session = match state_clone.sessions.get(&session_id) {
                Some(s) => s.clone(),
                None => match db::get_replay_session(&state_clone.db_pool, session_id).await {
                    Ok(Some(s)) => {
                        state_clone.sessions.insert(session_id, s.clone());
                        s
                    }
                    Ok(None) => {
                        info!(
                            "Session {} not found in DB or cache, exiting stream loop",
                            session_id
                        );
                        break;
                    }
                    Err(e) => {
                        error!(
                            "Database error fetching session {} state: {}",
                            session_id, e
                        );
                        continue;
                    }
                },
            };

            let current_status = current_session.status.clone();
            let current_speed = current_session.speed;
            let db_position = current_session.current_position;

            let mut seek_to = None;
            let mut play_next = None;
            let mut stop_at = None;

            if db_position != last_known_position {
                seek_to = Some(db_position);
                local_position = db_position;
                last_known_position = db_position;
            } else if current_status != last_known_status || current_speed != last_known_speed {
                last_known_status = current_status.clone();
                last_known_speed = current_speed;

                let sse_session = db::ReplaySession {
                    session_id,
                    satellite_id: satellite_id.clone(),
                    start_time,
                    end_time,
                    current_position: local_position,
                    speed: current_speed,
                    status: current_status.clone(),
                    created_at: None,
                    updated_at: None,
                };
                let state_json = serde_json::to_string(&sse_session).unwrap_or_default();
                if tx
                    .send(Ok(Event::default().event("state").data(state_json)))
                    .await
                    .is_err()
                {
                    break;
                }
            }

            if current_status == "STOPPED" {
                stop_at = Some(local_position);
            } else if seek_to.is_none() && current_status == "PLAYING" {
                let now = tokio::time::Instant::now();
                let elapsed_real = now.duration_since(last_tick);
                last_tick = now;

                let elapsed_replay_secs = elapsed_real.as_secs_f32() * current_speed;
                let start_query = local_position;
                let candidate_end = local_position
                    + chrono::Duration::milliseconds((elapsed_replay_secs * 1000.0) as i64);

                if candidate_end >= end_time {
                    stop_at = Some(end_time);
                } else {
                    play_next = Some((start_query, candidate_end));
                }
            } else {
                last_tick = tokio::time::Instant::now();
            }

            if let Some(seek_pos) = seek_to {
                info!("Seek detected to {} in session {}", seek_pos, session_id);
                let sse_session = db::ReplaySession {
                    session_id,
                    satellite_id: satellite_id.clone(),
                    start_time,
                    end_time,
                    current_position: seek_pos,
                    speed: current_speed,
                    status: current_status.clone(),
                    created_at: None,
                    updated_at: None,
                };
                let state_json = serde_json::to_string(&sse_session).unwrap_or_default();
                if tx
                    .send(Ok(Event::default().event("state").data(state_json)))
                    .await
                    .is_err()
                {
                    break;
                }

                let start_query = seek_pos - chrono::Duration::seconds(1);
                let end_query = seek_pos;
                let _ = query_and_send_events(
                    &state_clone.db_pool,
                    satellite_id.as_deref(),
                    start_query,
                    end_query,
                    session_id,
                    &tx,
                )
                .await;

                last_tick = tokio::time::Instant::now();
                continue;
            }

            if let Some(stop_pos) = stop_at {
                info!(
                    "Session {} status is STOPPED, exiting stream loop",
                    session_id
                );

                let _ = db::update_replay_session(
                    &state_clone.db_pool,
                    session_id,
                    stop_pos,
                    "STOPPED",
                    current_speed,
                )
                .await;
                if let Some(mut s) = state_clone.sessions.get_mut(&session_id) {
                    s.current_position = stop_pos;
                    s.status = "STOPPED".to_string();
                }

                let sse_session = db::ReplaySession {
                    session_id,
                    satellite_id: satellite_id.clone(),
                    start_time,
                    end_time,
                    current_position: stop_pos,
                    speed: current_speed,
                    status: "STOPPED".to_string(),
                    created_at: None,
                    updated_at: None,
                };
                let state_json = serde_json::to_string(&sse_session).unwrap_or_default();
                let _ = tx
                    .send(Ok(Event::default().event("state").data(state_json)))
                    .await;

                let playing_count: i64 = sqlx::query_scalar(
                    "SELECT COUNT(*) FROM replay_sessions WHERE status = 'PLAYING'",
                )
                .fetch_one(&state_clone.db_pool)
                .await
                .unwrap_or_default();
                metrics::ACTIVE_REPLAYS.set(playing_count as f64);

                if local_position < stop_pos {
                    let _ = query_and_send_events(
                        &state_clone.db_pool,
                        satellite_id.as_deref(),
                        local_position,
                        stop_pos,
                        session_id,
                        &tx,
                    )
                    .await;
                }
                break;
            }

            if let Some((start_query, end_query)) = play_next {
                if let Some(mut s) = state_clone.sessions.get_mut(&session_id) {
                    s.current_position = end_query;
                }

                let now_inst = tokio::time::Instant::now();
                if now_inst.duration_since(last_db_write) >= Duration::from_secs(5) {
                    let _ = db::update_replay_session(
                        &state_clone.db_pool,
                        session_id,
                        end_query,
                        &current_status,
                        current_speed,
                    )
                    .await;
                    last_db_write = now_inst;
                }

                let sse_session = db::ReplaySession {
                    session_id,
                    satellite_id: satellite_id.clone(),
                    start_time,
                    end_time,
                    current_position: end_query,
                    speed: current_speed,
                    status: current_status.clone(),
                    created_at: None,
                    updated_at: None,
                };
                let state_json = serde_json::to_string(&sse_session).unwrap_or_default();
                if tx
                    .send(Ok(Event::default().event("state").data(state_json)))
                    .await
                    .is_err()
                {
                    break;
                }

                if let Err(e) = query_and_send_events(
                    &state_clone.db_pool,
                    satellite_id.as_deref(),
                    start_query,
                    end_query,
                    session_id,
                    &tx,
                )
                .await
                {
                    error!("Error querying or sending replay events: {}", e);
                    break;
                }

                local_position = end_query;
                last_known_position = end_query;
            } else {
                let sse_session = db::ReplaySession {
                    session_id,
                    satellite_id: satellite_id.clone(),
                    start_time,
                    end_time,
                    current_position: local_position,
                    speed: current_speed,
                    status: current_status.clone(),
                    created_at: None,
                    updated_at: None,
                };
                let state_json = serde_json::to_string(&sse_session).unwrap_or_default();
                if tx
                    .send(Ok(Event::default().event("state").data(state_json)))
                    .await
                    .is_err()
                {
                    break;
                }
            }
        }

        let final_pos = {
            if let Some(s) = state_clone.sessions.get(&session_id) {
                Some((s.current_position, s.status.clone(), s.speed))
            } else {
                None
            }
        };
        if let Some((pos, status, speed)) = final_pos {
            info!(
                "Persisting final replay session state to DB on exit: position={}, status={}",
                pos, status
            );
            let _ =
                db::update_replay_session(&state_clone.db_pool, session_id, pos, &status, speed)
                    .await;
        }
    });

    let stream = ReceiverStream::new(rx);
    Ok(Sse::new(stream).keep_alive(KeepAlive::default()))
}
