use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;
use sqlx::{PgPool, Row};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use std::sync::Arc;
use dashmap::DashMap;
use rdkafka::producer::{FutureProducer, FutureRecord};
use std::time::Duration;
use tracing::error;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebrisObject {
    pub id: String,
    pub name: String,
    pub inclination: f64,
    pub altitude: f64,
    pub velocity: f64,
    pub size_meters: f64,
    pub line1: String,
    pub line2: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConjunctionEvent {
    pub id: Uuid,
    pub satellite_id: String,
    pub object_id: String,
    pub object_name: String,
    pub object_type: String, // 'DEBRIS', 'SATELLITE'
    pub probability: f64,    // 0.0 to 1.0
    pub distance_meters: f64,
    pub conjunction_time: DateTime<Utc>,
    pub severity: String,    // 'LOW', 'MEDIUM', 'HIGH', 'CRITICAL'
    pub status: String,      // 'ACTIVE', 'RESOLVED'
}

pub struct SsaManager {
    pub db_pool: PgPool,
    pub kafka_producer: FutureProducer,
    pub timeline_topic: String,
    pub debris: Vec<DebrisObject>,
    pub nearest_objects: DashMap<String, (DebrisObject, f64, f64)>, // sat_id -> (debris, distance, probability)
}

impl SsaManager {
    pub fn new(db_pool: PgPool, kafka_producer: FutureProducer, timeline_topic: String) -> Self {
        let debris = vec![
            DebrisObject {
                id: "DEB-41920".to_string(),
                name: "SL-8 DEBRIS CLOUD [COLLISION FRAGMENT]".to_string(),
                inclination: 82.5,
                altitude: 512.4,
                velocity: 7.61,
                size_meters: 1.2,
                line1: "1 41920U 15001A   26170.00000000  .00005210  00000-0  30000-3 0  9992".to_string(),
                line2: "2 41920  82.5000 120.4500 0008500  45.1200 315.6500 15.55000000    1".to_string(),
            },
            DebrisObject {
                id: "DEB-88201".to_string(),
                name: "COSMOS 1408 FRAGMENT [SL-14]".to_string(),
                inclination: 98.2,
                altitude: 489.1,
                velocity: 7.64,
                size_meters: 0.4,
                line1: "1 88201U 21002B   26170.00000000  .00010240  00000-0  50000-3 0  9993".to_string(),
                line2: "2 88201  98.2000 185.3400 0004200  92.6500 268.1200 15.68000000    1".to_string(),
            },
            DebrisObject {
                id: "DEB-91024".to_string(),
                name: "FENGYUN 1C DEBRIS [ASAT TEST]".to_string(),
                inclination: 99.1,
                altitude: 504.8,
                velocity: 7.62,
                size_meters: 0.1,
                line1: "1 91024U 07003C   26170.00000000  .00008450  00000-0  42000-3 0  9994".to_string(),
                line2: "2 91024  99.1000  45.1200 0006200 125.4000 234.8000 15.58000000    1".to_string(),
            },
            DebrisObject {
                id: "DEB-30056".to_string(),
                name: "IRIDIUM 33 DEBRIS [COLLISION FRAGMENT]".to_string(),
                inclination: 86.4,
                altitude: 520.1,
                velocity: 7.59,
                size_meters: 0.8,
                line1: "1 30056U 09004D   26170.00000000  .00004120  00000-0  25000-3 0  9995".to_string(),
                line2: "2 30056  86.4000 240.5600 0007400  65.9200 294.1800 15.52000000    1".to_string(),
            },
            DebrisObject {
                id: "DEB-00449".to_string(),
                name: "DELTA II SECOND STAGE ROCKET BODY".to_string(),
                inclination: 53.0,
                altitude: 515.2,
                velocity: 7.60,
                line1: "1 00449U 95005E   26170.00000000  .00002150  00000-0  10000-3 0  9996".to_string(),
                line2: "2 00449  53.0000 315.6800 0012000 180.2000 180.2000 15.50000000    1".to_string(),
                size_meters: 5.4,
            },
        ];

        let manager = Self {
            db_pool,
            kafka_producer,
            timeline_topic,
            debris,
            nearest_objects: DashMap::new(),
        };

        // Seed debris in background
        let db_c = manager.db_pool.clone();
        let debris_c = manager.debris.clone();
        tokio::spawn(async move {
            for deb in debris_c {
                let _ = sqlx::query(
                    "INSERT INTO ssa_objects (id, name, object_type, inclination, altitude, velocity, size_meters)
                     VALUES ($1, $2, $3, $4, $5, $6, $7)
                     ON CONFLICT (id) DO UPDATE SET altitude = EXCLUDED.altitude, velocity = EXCLUDED.velocity"
                )
                .bind(&deb.id)
                .bind(&deb.name)
                .bind("DEBRIS")
                .bind(deb.inclination)
                .bind(deb.altitude)
                .bind(deb.velocity)
                .bind(deb.size_meters)
                .execute(&db_c)
                .await;
            }
        });

        manager
    }

    pub async fn evaluate_conjunctions(&self, active_satellites: &[String]) {
        let now = Utc::now();
        for sat_id in active_satellites {
            let (line1_sat, line2_sat) = crate::orbit::generate_default_tle(sat_id);
            let sat_state = match crate::orbit::propagate_tle_sgp4(sat_id, &line1_sat, &line2_sat, now) {
                Ok(st) => st,
                Err(e) => {
                    error!("SSA propagator error for sat {}: {}", sat_id, e);
                    continue;
                }
            };

            let mut min_distance = f64::MAX;
            let mut nearest_deb = None;
            let mut col_prob = 0.0;

            for deb in &self.debris {
                if let Ok(deb_state) = crate::orbit::propagate_tle_sgp4(&deb.id, &deb.line1, &deb.line2, now) {
                    let dx = sat_state.position_eci[0] - deb_state.position_eci[0];
                    let dy = sat_state.position_eci[1] - deb_state.position_eci[1];
                    let dz = sat_state.position_eci[2] - deb_state.position_eci[2];
                    let dist_km = (dx.powi(2) + dy.powi(2) + dz.powi(2)).sqrt();
                    let dist_meters = dist_km * 1000.0;

                    if dist_meters < min_distance {
                        min_distance = dist_meters;
                        nearest_deb = Some(deb.clone());
                        
                        let raw_prob = (-dist_meters.powi(2) / 2000000.0).exp();
                        col_prob = if raw_prob < 1e-6 { 0.0 } else { raw_prob };
                    }

                    if dist_meters < 5000.0 {
                        // Conjunction event identified
                        let severity = if dist_meters < 500.0 {
                            "CRITICAL"
                        } else if dist_meters < 1500.0 {
                            "HIGH"
                        } else if dist_meters < 3000.0 {
                            "MEDIUM"
                        } else {
                            "LOW"
                        };

                        let event_type = if severity == "CRITICAL" || severity == "HIGH" {
                            "HIGH_RISK_CONJUNCTION"
                        } else {
                            "CONJUNCTION_WARNING"
                        };

                        let id = Uuid::new_v4();
                        let conjunction_time = now + chrono::Duration::seconds(30 + rand::random::<i64>() % 120);

                        let _ = sqlx::query(
                            "INSERT INTO conjunction_events (id, satellite_id, object_id, object_name, object_type, probability, distance_meters, conjunction_time, severity, status)
                             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)"
                        )
                        .bind(id)
                        .bind(sat_id)
                        .bind(&deb.id)
                        .bind(&deb.name)
                        .bind("DEBRIS")
                        .bind(col_prob)
                        .bind(dist_meters)
                        .bind(conjunction_time)
                        .bind(severity)
                        .bind("ACTIVE")
                        .execute(&self.db_pool)
                        .await;

                        let msg = format!(
                            "Conjunction warning! Satellite {} has close approach with {} at distance {:.1}m. Probability: {:.6}",
                            sat_id, deb.name, dist_meters, col_prob
                        );

                        let timeline_payload = serde_json::json!({
                            "timestamp": now,
                            "event_id": Uuid::new_v4(),
                            "source": "ALERT",
                            "satellite_id": sat_id,
                            "event_type": event_type,
                            "severity": if severity == "CRITICAL" { "CRITICAL" } else if severity == "HIGH" { "WARNING" } else { "INFO" },
                            "message": msg,
                            "details": {
                                "conjunction_id": id,
                                "object_id": deb.id,
                                "object_name": deb.name,
                                "distance_m": dist_meters,
                                "probability": col_prob,
                                "conjunction_time": conjunction_time
                            }
                        });

                        if let Ok(payload) = serde_json::to_string(&timeline_payload) {
                            let record = FutureRecord::to(&self.timeline_topic)
                                .key(sat_id)
                                .payload(&payload);
                            let _ = self.kafka_producer.send(record, Duration::from_secs_f64(5.0)).await;
                        }
                    }
                }
            }

            if let Some(deb) = nearest_deb {
                self.nearest_objects.insert(sat_id.clone(), (deb, min_distance, col_prob));
            }
        }
    }
}

pub async fn get_conjunctions(
    State(state): State<Arc<crate::ServiceState>>,
    Path(satellite): Path<String>,
) -> Result<Json<Vec<ConjunctionEvent>>, (StatusCode, String)> {
    let rows = sqlx::query("SELECT id, satellite_id, object_id, object_name, object_type, probability, distance_meters, conjunction_time, severity, status FROM conjunction_events WHERE satellite_id = $1 ORDER BY conjunction_time DESC LIMIT 20")
        .bind(&satellite)
        .fetch_all(&state.db_pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let events = rows.into_iter().map(|row| ConjunctionEvent {
        id: row.get("id"),
        satellite_id: row.get("satellite_id"),
        object_id: row.get("object_id"),
        object_name: row.get("object_name"),
        object_type: row.get("object_type"),
        probability: row.get("probability"),
        distance_meters: row.get("distance_meters"),
        conjunction_time: row.get("conjunction_time"),
        severity: row.get("severity"),
        status: row.get("status"),
    }).collect();

    Ok(Json(events))
}

#[derive(Serialize)]
pub struct SsaStatus {
    pub satellite_id: String,
    pub nearest_object_name: String,
    pub nearest_object_distance: f64,
    pub collision_probability: f64,
    pub overall_risk_score: f64,
}

pub async fn get_ssa_status(
    State(state): State<Arc<crate::ServiceState>>,
    Path(satellite): Path<String>,
) -> Result<Json<SsaStatus>, StatusCode> {
    if let Some(entry) = state.ssa.nearest_objects.get(&satellite) {
        let (deb, dist, prob) = entry.value();
        let risk = if *dist < 500.0 { 
            95.0 
        } else if *dist < 1500.0 { 
            75.0 
        } else if *dist < 5000.0 { 
            30.0 
        } else { 
            5.0 
        };
        
        Ok(Json(SsaStatus {
            satellite_id: satellite,
            nearest_object_name: deb.name.clone(),
            nearest_object_distance: *dist,
            collision_probability: *prob,
            overall_risk_score: risk,
        }))
    } else {
        // Fallback for default nominal
        Ok(Json(SsaStatus {
            satellite_id: satellite,
            nearest_object_name: "NONE IN PROXIMITY".to_string(),
            nearest_object_distance: 125000.0,
            collision_probability: 0.0,
            overall_risk_score: 0.0,
        }))
    }
}
