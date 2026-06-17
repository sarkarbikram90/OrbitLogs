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
use tracing::{info, error};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebrisObject {
    pub id: String,
    pub name: String,
    pub inclination: f64,
    pub altitude: f64,
    pub velocity: f64,
    pub size_meters: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConjunctionEvent {
    pub id: Uuid,
    pub satellite_id: String,
    pub object_name: String,
    pub object_type: String, // 'DEBRIS', 'SATELLITE'
    pub probability: f64,    // 0.0 to 1.0
    pub distance_meters: f64,
    pub conjunction_time: DateTime<Utc>,
    pub severity: String, // 'INFO', 'WARNING', 'CRITICAL'
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
        // Initialize some standard cataloged orbital debris items
        let debris = vec![
            DebrisObject { id: "DEB-41920".to_string(), name: "SL-8 DEBRIS CLOUD [COLLISION FRAGMENT]".to_string(), inclination: 82.5, altitude: 512.4, velocity: 7.61, size_meters: 1.2 },
            DebrisObject { id: "DEB-88201".to_string(), name: "COSMOS 1408 FRAGMENT [SL-14]".to_string(), inclination: 98.2, altitude: 489.1, velocity: 7.64, size_meters: 0.4 },
            DebrisObject { id: "DEB-91024".to_string(), name: "FENGYUN 1C DEBRIS [ASAT TEST]".to_string(), inclination: 99.1, altitude: 504.8, velocity: 7.62, size_meters: 0.1 },
            DebrisObject { id: "DEB-30056".to_string(), name: "IRIDIUM 33 DEBRIS [COLLISION FRAGMENT]".to_string(), inclination: 86.4, altitude: 520.1, velocity: 7.59, size_meters: 0.8 },
            DebrisObject { id: "DEB-00449".to_string(), name: "DELTA II SECOND STAGE ROCKET BODY".to_string(), inclination: 53.0, altitude: 515.2, velocity: 7.60, size_meters: 5.4 },
        ];

        Self {
            db_pool,
            kafka_producer,
            timeline_topic,
            debris,
            nearest_objects: DashMap::new(),
        }
    }

    pub async fn evaluate_conjunctions(&self, active_satellites: &[String]) {
        for sat_id in active_satellites {
            // Pick a random debris object and generate a dynamic closest approach
            let idx = rand::random::<usize>() % self.debris.len();
            let deb = &self.debris[idx];
            
            // Random distance between 200m and 12,000m
            let distance = 200.0 + rand::random::<f64>() * 12000.0;
            
            // Compute collision probability based on size and distance (1D spatial normal approximation)
            let raw_prob = (-distance.powi(2) / 2000000.0).exp(); // scale variance to 1km approx
            let probability = if raw_prob < 1e-6 { 0.0 } else { raw_prob };

            // Update nearest object mapping
            self.nearest_objects.insert(sat_id.clone(), (deb.clone(), distance, probability));

            // Generate alarms if proximity breaches collision thresholds
            if distance < 5000.0 {
                let severity = if distance < 1200.0 { "CRITICAL" } else { "WARNING" };
                let event_type = if distance < 1200.0 { "HIGH_RISK_CONJUNCTION" } else { "CONJUNCTION_ALERT" };
                let id = Uuid::new_v4();
                let conjunction_time = Utc::now() + chrono::Duration::seconds(30 + rand::random::<i64>() % 300);

                // Insert into DB
                let _ = sqlx::query(
                    "INSERT INTO ssa_conjunctions (id, satellite_id, object_name, object_type, probability, distance_meters, conjunction_time, severity)
                     VALUES ($1, $2, $3, $4, $5, $6, $7, $8)"
                )
                .bind(id)
                .bind(sat_id)
                .bind(&deb.name)
                .bind("DEBRIS")
                .bind(probability)
                .bind(distance)
                .bind(conjunction_time)
                .bind(severity)
                .execute(&self.db_pool)
                .await;

                // Send notification on Event Bus
                let msg = format!(
                    "Conjunction warning! Satellite {} has close approach with {} at distance {:.1}m. Est. probability: {:.6}",
                    sat_id, deb.name, distance, probability
                );

                let timeline_payload = serde_json::json!({
                    "timestamp": Utc::now(),
                    "event_id": Uuid::new_v4(),
                    "source": "ALERT",
                    "satellite_id": sat_id,
                    "event_type": event_type,
                    "severity": severity,
                    "message": msg,
                    "details": {
                        "conjunction_id": id,
                        "object_name": deb.name,
                        "distance_m": distance,
                        "probability": probability,
                        "conjunction_time": conjunction_time
                    }
                });

                if let Ok(payload) = serde_json::to_string(&timeline_payload) {
                    let record = FutureRecord::to(&self.timeline_topic)
                        .key(sat_id)
                        .payload(&payload);
                    let _ = self.kafka_producer.send(record, Duration::from_secs(5)).await;
                }
            }
        }
    }
}

pub async fn get_conjunctions(
    State(state): State<Arc<crate::ServiceState>>,
    Path(satellite): Path<String>,
) -> Result<Json<Vec<ConjunctionEvent>>, (StatusCode, String)> {
    let rows = sqlx::query("SELECT * FROM ssa_conjunctions WHERE satellite_id = $1 ORDER BY conjunction_time DESC LIMIT 20")
        .bind(&satellite)
        .fetch_all(&state.db_pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let events = rows.into_iter().map(|row| ConjunctionEvent {
        id: row.get("id"),
        satellite_id: row.get("satellite_id"),
        object_name: row.get("object_name"),
        object_type: row.get("object_type"),
        probability: row.get("probability"),
        distance_meters: row.get("distance_meters"),
        conjunction_time: row.get("conjunction_time"),
        severity: row.get("severity"),
    }).collect();

    Ok(Json(events))
}

#[derive(Serialize)]
pub struct SsaStatus {
    pub satellite_id: String,
    pub nearest_object_name: String,
    pub nearest_object_distance: f64,
    pub collision_probability: f64,
    pub overall_risk_score: f64, // 0 to 100 composite
}

pub async fn get_ssa_status(
    State(state): State<Arc<crate::ServiceState>>,
    Path(satellite): Path<String>,
) -> Result<Json<SsaStatus>, StatusCode> {
    if let Some(entry) = state.ssa.nearest_objects.get(&satellite) {
        let (deb, dist, prob) = entry.value();
        let risk = if *dist < 1000.0 { 95.0 } else if *dist < 5000.0 { 50.0 } else { 5.0 };
        
        Ok(Json(SsaStatus {
            satellite_id: satellite,
            nearest_object_name: deb.name.clone(),
            nearest_object_distance: *dist,
            collision_probability: *prob,
            overall_risk_score: risk,
        }))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}
