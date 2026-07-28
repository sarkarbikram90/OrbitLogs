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

//! Database and Kafka persistence service operations for orbit calculations and TLE management.

use chrono::{DateTime, Utc};
use rdkafka::producer::{FutureProducer, FutureRecord};
use serde::Deserialize;
use sqlx::{PgPool, Row};
use std::f64::consts::PI;
use std::time::Duration;

use crate::orbit::{generate_default_tle, predict_passes, PassPrediction, GM_KM3_S2};

pub async fn get_latest_tle_or_default(db_pool: &PgPool, sat_id: &str) -> (String, String) {
    let row = sqlx::query(
        "SELECT line1, line2 FROM tle_records WHERE satellite_id = $1 ORDER BY epoch DESC LIMIT 1",
    )
    .bind(sat_id)
    .fetch_optional(db_pool)
    .await;
    match row {
        Ok(Some(r)) => {
            let l1: String = r.get("line1");
            let l2: String = r.get("line2");
            (l1, l2)
        }
        _ => generate_default_tle(sat_id),
    }
}

pub async fn calculate_and_persist_passes(
    db_pool: &PgPool,
    kafka_producer: &FutureProducer,
    timeline_topic: &str,
    sat_id: &str,
    start_time: DateTime<Utc>,
    duration_hours: i64,
) -> Result<Vec<PassPrediction>, String> {
    let (line1, line2) = get_latest_tle_or_default(db_pool, sat_id).await;
    let passes = predict_passes(sat_id, &line1, &line2, start_time, duration_hours);

    for pass in &passes {
        let id = uuid::Uuid::new_v4();
        let _ = sqlx::query(
            "INSERT INTO pass_predictions (id, satellite_id, station_id, aos, los, duration, max_elevation)
             VALUES ($1, $2, $3, $4, $5, $6, $7)"
        )
        .bind(id)
        .bind(sat_id)
        .bind(&pass.station)
        .bind(pass.aos)
        .bind(pass.los)
        .bind(pass.duration)
        .bind(pass.max_elevation)
        .execute(db_pool)
        .await;

        let timeline_payload = serde_json::json!({
            "timestamp": Utc::now(),
            "event_id": uuid::Uuid::new_v4(),
            "source": "ORBIT_PROPAGATOR",
            "satellite_id": sat_id,
            "event_type": "PASS_PREDICTED",
            "severity": "INFO",
            "message": format!("Upcoming pass predicted for satellite {} at station {}. AOS: {}, LOS: {}", sat_id, pass.station, pass.aos, pass.los),
            "details": {
                "prediction_id": id,
                "station_id": pass.station,
                "aos": pass.aos,
                "los": pass.los,
                "duration_min": pass.duration,
                "max_elevation_deg": pass.max_elevation
            }
        });

        if let Ok(payload) = serde_json::to_string(&timeline_payload) {
            let record = FutureRecord::to(timeline_topic)
                .key(sat_id)
                .payload(&payload);
            let _ = kafka_producer.send(record, Duration::from_secs(5)).await;
        }
    }

    Ok(passes)
}

#[derive(Debug, Deserialize)]
pub struct ImportTleRequest {
    pub satellite_id: String,
    pub line1: String,
    pub line2: String,
}

pub async fn import_tle_internal(
    db_pool: &PgPool,
    kafka_producer: &FutureProducer,
    timeline_topic: &str,
    req: ImportTleRequest,
) -> Result<uuid::Uuid, String> {
    let elements = match sgp4::Elements::from_tle(
        Some(req.satellite_id.clone()),
        req.line1.as_bytes(),
        req.line2.as_bytes(),
    ) {
        Ok(el) => el,
        Err(e) => {
            let error_msg = format!("TLE validation failed: {:?}", e);
            let timeline_payload = serde_json::json!({
                "timestamp": Utc::now(),
                "event_id": uuid::Uuid::new_v4(),
                "source": "TLE_MANAGER",
                "satellite_id": req.satellite_id,
                "event_type": "TLE_VALIDATION_FAILED",
                "severity": "ERROR",
                "message": error_msg,
                "details": {
                    "line1": req.line1,
                    "line2": req.line2,
                    "error": format!("{:?}", e)
                }
            });
            if let Ok(payload) = serde_json::to_string(&timeline_payload) {
                let record = FutureRecord::to(timeline_topic)
                    .key(&req.satellite_id)
                    .payload(&payload);
                let _ = kafka_producer.send(record, Duration::from_secs(5)).await;
            }
            return Err(error_msg);
        }
    };

    let epoch_time = DateTime::<Utc>::from_naive_utc_and_offset(elements.datetime, Utc);
    let inc = req.line2[8..16].trim().parse::<f64>().unwrap_or(53.0);
    let raan = req.line2[17..25].trim().parse::<f64>().unwrap_or(0.0);
    let ecc_str = req.line2[26..33].trim();
    let ecc = format!("0.{}", ecc_str).parse::<f64>().unwrap_or(0.0001);
    let arg_pe = req.line2[34..42].trim().parse::<f64>().unwrap_or(0.0);
    let mean_anom = req.line2[43..51].trim().parse::<f64>().unwrap_or(0.0);
    let mean_motion = req.line2[52..63].trim().parse::<f64>().unwrap_or(15.0);

    let mean_motion_rad_s = mean_motion * 2.0 * PI / 86400.0;
    let semi_major_axis = (GM_KM3_S2 / mean_motion_rad_s.powi(2)).cbrt();

    let id = uuid::Uuid::new_v4();

    sqlx::query(
        "INSERT INTO tle_records (id, satellite_id, epoch, line1, line2, inclination, raan, eccentricity, arg_of_perigee, mean_anomaly, mean_motion, semi_major_axis)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)"
    )
    .bind(id)
    .bind(&req.satellite_id)
    .bind(epoch_time)
    .bind(&req.line1)
    .bind(&req.line2)
    .bind(inc)
    .bind(raan)
    .bind(ecc)
    .bind(arg_pe)
    .bind(mean_anom)
    .bind(mean_motion)
    .bind(semi_major_axis)
    .execute(db_pool)
    .await
    .map_err(|e| format!("DB Insert Error: {}", e))?;

    let timeline_payload = serde_json::json!({
        "timestamp": Utc::now(),
        "event_id": uuid::Uuid::new_v4(),
        "source": "TLE_MANAGER",
        "satellite_id": req.satellite_id,
        "event_type": "TLE_UPDATED",
        "severity": "INFO",
        "message": format!("TLE updated successfully for satellite {}. Epoch: {}", req.satellite_id, epoch_time),
        "details": {
            "tle_id": id,
            "epoch": epoch_time,
            "inclination": inc,
            "raan": raan,
            "eccentricity": ecc,
            "mean_motion": mean_motion,
        }
    });

    if let Ok(payload) = serde_json::to_string(&timeline_payload) {
        let record = FutureRecord::to(timeline_topic)
            .key(&req.satellite_id)
            .payload(&payload);
        let _ = kafka_producer.send(record, Duration::from_secs(5)).await;
    }

    let _ = calculate_and_persist_passes(
        db_pool,
        kafka_producer,
        timeline_topic,
        &req.satellite_id,
        Utc::now(),
        24,
    )
    .await;

    Ok(id)
}

pub async fn check_tle_expiration(
    db_pool: &PgPool,
    kafka_producer: &FutureProducer,
    timeline_topic: &str,
    active_satellites: &[String],
) {
    for sat_id in active_satellites {
        let row = sqlx::query(
            "SELECT epoch FROM tle_records WHERE satellite_id = $1 ORDER BY epoch DESC LIMIT 1",
        )
        .bind(sat_id)
        .fetch_optional(db_pool)
        .await;

        if let Ok(Some(r)) = row {
            let epoch: DateTime<Utc> = r.get("epoch");
            if Utc::now().signed_duration_since(epoch).num_days() > 14 {
                let msg = format!(
                    "TLE has expired for satellite {}. Latest epoch: {} is older than 14 days.",
                    sat_id, epoch
                );
                let timeline_payload = serde_json::json!({
                    "timestamp": Utc::now(),
                    "event_id": uuid::Uuid::new_v4(),
                    "source": "TLE_MANAGER",
                    "satellite_id": sat_id,
                    "event_type": "TLE_EXPIRED",
                    "severity": "WARNING",
                    "message": msg,
                    "details": {
                        "epoch": epoch,
                        "days_old": Utc::now().signed_duration_since(epoch).num_days()
                    }
                });

                if let Ok(payload) = serde_json::to_string(&timeline_payload) {
                    let record = FutureRecord::to(timeline_topic)
                        .key(sat_id)
                        .payload(&payload);
                    let _ = kafka_producer.send(record, Duration::from_secs(5)).await;
                }
            }
        }
    }
}

pub async fn scrape_celestrak_tles(
    db_pool: &PgPool,
    kafka_producer: &FutureProducer,
    timeline_topic: &str,
) -> Result<(), anyhow::Error> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()?;

    let url = "https://celestrak.org/NORAD/elements/gp.php?GROUP=starlink&FORMAT=tle";
    let body = client.get(url).send().await?.text().await?;

    let lines: Vec<&str> = body.lines().map(|l| l.trim()).collect();
    let mut mapped_count = 0;

    for chunk in lines.chunks_exact(3) {
        if mapped_count >= 10 {
            break;
        }

        let name = chunk[0];
        let line1 = chunk[1];
        let line2 = chunk[2];

        if line1.starts_with('1') && line2.starts_with('2') {
            mapped_count += 1;
            let sat_id = format!("SAT-{:04}", mapped_count);

            tracing::info!("Scraped TLE for {} (NORAD name: {})", sat_id, name);

            let req = ImportTleRequest {
                satellite_id: sat_id.clone(),
                line1: line1.to_string(),
                line2: line2.to_string(),
            };

            if let Err(e) = import_tle_internal(db_pool, kafka_producer, timeline_topic, req).await
            {
                tracing::error!("Failed to import scraped TLE for {}: {}", sat_id, e);
            }
        }
    }

    if mapped_count > 0 {
        tracing::info!(
            "Successfully scraped and updated TLEs for {} satellites from CelesTrak",
            mapped_count
        );
    } else {
        anyhow::bail!("No valid TLEs parsed from CelesTrak response");
    }

    Ok(())
}
