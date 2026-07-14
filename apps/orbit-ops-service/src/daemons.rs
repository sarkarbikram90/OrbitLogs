use crate::ServiceState;
use crate::orbit;
use chrono::Utc;
use rdkafka::producer::FutureRecord;
use std::sync::Arc;
use std::time::Duration;
use tokio_util::sync::CancellationToken;
use tracing::{error, info};

pub fn spawn_orbit_propagation_daemon(state: Arc<ServiceState>, cancel_token: CancellationToken) {
    let state_o = Arc::clone(&state);
    let token_o = cancel_token.clone();
    tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = token_o.cancelled() => break,
                _ = tokio::time::sleep(Duration::from_secs(5)) => {
                    let now = Utc::now();
                    for sat_id in orbitlogs_models::constellation_iter::ConstellationIter::new(10) {
                        let (line1, line2) = orbit::get_latest_tle_or_default(&state_o.db_pool, &sat_id).await;
                        if let Ok(orbit_state) = orbit::propagate_tle_sgp4(&sat_id, &line1, &line2, now) {

                            if let Err(e) = sqlx::query(
                                "INSERT INTO orbit_positions (timestamp, satellite_id, latitude, longitude, altitude, velocity, orbital_period, inclination, eccentricity)
                                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)"
                            )
                            .bind(orbit_state.timestamp)
                            .bind(&orbit_state.satellite_id)
                            .bind(orbit_state.latitude)
                            .bind(orbit_state.longitude)
                            .bind(orbit_state.altitude)
                            .bind(orbit_state.velocity)
                            .bind(orbit_state.orbital_period)
                            .bind(orbit_state.inclination)
                            .bind(orbit_state.eccentricity)
                            .execute(&state_o.db_pool)
                            .await
                            {
                                error!("Failed to insert orbit position for {}: {}", orbit_state.satellite_id, e);
                            }

                            let orbit_event = serde_json::json!({
                                "event_type": "orbit.position.updated",
                                "satellite_id": orbit_state.satellite_id,
                                "timestamp": orbit_state.timestamp,
                                "latitude": orbit_state.latitude,
                                "longitude": orbit_state.longitude,
                                "altitude": orbit_state.altitude,
                                "velocity": orbit_state.velocity,
                            });
                            if let Ok(payload) = serde_json::to_string(&orbit_event) {
                                let record = FutureRecord::to("orbit.state")
                                    .key(&orbit_state.satellite_id)
                                    .payload(&payload);
                                let _ = state_o.kafka_producer.send(record, Duration::from_secs(5)).await;
                            }
                        }
                    }
                }
            }
        }
    });
}

pub fn spawn_ground_ops_ticks_daemon(state: Arc<ServiceState>, cancel_token: CancellationToken) {
    let state_g = Arc::clone(&state);
    let token_g = cancel_token.clone();
    tokio::spawn(async move {
        let active_sats: Vec<String> =
            orbitlogs_models::constellation_iter::ConstellationIter::new(10)
                .map(|id| id.as_str().to_string())
                .collect();
        loop {
            tokio::select! {
                _ = token_g.cancelled() => break,
                _ = tokio::time::sleep(Duration::from_secs(5)) => {
                    state_g.ground_ops.update_ticks(&active_sats, Utc::now()).await;
                }
            }
        }
    });
}

pub fn spawn_ssa_conjunctions_daemon(state: Arc<ServiceState>, cancel_token: CancellationToken) {
    let state_s = Arc::clone(&state);
    let token_s = cancel_token.clone();
    tokio::spawn(async move {
        let active_sats: Vec<String> =
            orbitlogs_models::constellation_iter::ConstellationIter::new(10)
                .map(|id| id.as_str().to_string())
                .collect();
        loop {
            tokio::select! {
                _ = token_s.cancelled() => break,
                _ = tokio::time::sleep(Duration::from_secs(10)) => {
                    state_s.ssa.evaluate_conjunctions(&active_sats).await;
                }
            }
        }
    });
}

pub fn spawn_tle_expiration_daemon(state: Arc<ServiceState>, cancel_token: CancellationToken) {
    let state_t = Arc::clone(&state);
    let token_t = cancel_token.clone();
    tokio::spawn(async move {
        let active_sats: Vec<String> =
            orbitlogs_models::constellation_iter::ConstellationIter::new(10)
                .map(|id| id.as_str().to_string())
                .collect();
        loop {
            tokio::select! {
                _ = token_t.cancelled() => break,
                _ = tokio::time::sleep(Duration::from_secs(60)) => {
                    orbit::check_tle_expiration(&state_t.db_pool, &state_t.kafka_producer, &state_t.timeline_topic, &active_sats).await;
                }
            }
        }
    });
}

pub fn spawn_tle_scraping_daemon(state: Arc<ServiceState>, cancel_token: CancellationToken) {
    let state_scrape = Arc::clone(&state);
    let token_scrape = cancel_token.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(10)).await;
        loop {
            tokio::select! {
                _ = token_scrape.cancelled() => break,
                _ = async {
                    info!("Automated TLE Scraping Daemon: fetching TLE data from CelesTrak...");
                    if let Err(e) = orbit::scrape_celestrak_tles(&state_scrape.db_pool, &state_scrape.kafka_producer, &state_scrape.timeline_topic).await {
                        error!("Automated TLE Scraping failed: {:?}", e);
                    }
                    tokio::time::sleep(Duration::from_secs(12 * 3600)).await;
                } => {}
            }
        }
    });
}
