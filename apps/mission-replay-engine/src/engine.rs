use axum::response::sse::Event;
use std::convert::Infallible;
use tokio::sync::mpsc;

use crate::db;
use crate::metrics;
use crate::session::{ReplayData, TimedReplayData};

pub async fn query_and_send_events(
    pool: &sqlx::PgPool,
    satellite_id: Option<&str>,
    start: chrono::DateTime<chrono::Utc>,
    end: chrono::DateTime<chrono::Utc>,
    session_id: uuid::Uuid,
    tx: &mpsc::Sender<Result<Event, Infallible>>,
) -> Result<(), anyhow::Error> {
    let telemetry_fut = db::query_telemetry_range(pool, satellite_id, start, end);
    let events_fut = db::query_events_range(pool, satellite_id, start, end);
    let health_fut = db::query_satellite_health_range(pool, satellite_id, start, end);
    let gs_health_fut = db::query_ground_station_health_range(pool, start, end);

    let (telemetry, events, health, gs_health) =
        tokio::try_join!(telemetry_fut, events_fut, health_fut, gs_health_fut)?;

    let mut all_data = Vec::new();

    for t in telemetry {
        let rec: orbitlogs_models::TelemetryRecord = t.into();
        all_data.push(TimedReplayData {
            timestamp: rec.timestamp,
            data: ReplayData::Telemetry(rec),
        });
    }
    for e in events {
        all_data.push(TimedReplayData {
            timestamp: e.timestamp,
            data: ReplayData::Event(e),
        });
    }
    for h in health {
        let mapped: db::ReplaySatelliteHealth = h.into();
        all_data.push(TimedReplayData {
            timestamp: mapped.timestamp,
            data: ReplayData::SatelliteHealth(mapped),
        });
    }
    for gs in gs_health {
        all_data.push(TimedReplayData {
            timestamp: gs.timestamp,
            data: ReplayData::GroundStationHealth(gs),
        });
    }

    all_data.sort_by_key(|item| item.timestamp);

    for item in all_data {
        let (event_name, data_type) = match &item.data {
            ReplayData::Telemetry(_) => ("telemetry", "telemetry"),
            ReplayData::Event(_) => ("event", "event"),
            ReplayData::SatelliteHealth(_) => ("satellite_health", "satellite_health"),
            ReplayData::GroundStationHealth(_) => {
                ("ground_station_health", "ground_station_health")
            }
        };

        let json_data = serde_json::to_string(&item.data)?;
        let event = Event::default().event(event_name).data(json_data);

        if tx.send(Ok(event)).await.is_err() {
            return Err(anyhow::anyhow!("channel closed"));
        }

        metrics::REPLAY_EVENTS_STREAMED
            .with_label_values(&[&session_id.to_string(), data_type])
            .inc();
    }

    Ok(())
}
