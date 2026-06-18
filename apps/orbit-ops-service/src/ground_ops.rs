use serde::{Serialize, Deserialize};
use dashmap::DashMap;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use rdkafka::producer::{FutureProducer, FutureRecord};
use std::time::Duration;
use tracing::error;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroundStationOps {
    pub station_id: String,
    pub name: String,
    pub antenna_available: bool,
    pub rf_utilization: f64,        // 0.0 to 100.0 %
    pub active_satellite: Option<String>,
    pub contact_window_start: Option<DateTime<Utc>>,
    pub contact_window_end: Option<DateTime<Utc>>,
    pub total_uplinks: u32,
    pub total_downlinks: u32,
    pub failed_uplinks: u32,
    pub failed_downlinks: u32,
    pub missed_contacts: u32,
}

pub struct GroundOpsManager {
    pub states: DashMap<String, GroundStationOps>,
    kafka_producer: FutureProducer,
    timeline_topic: String,
    contacts_topic: String,
    db_pool: PgPool,
}

impl GroundOpsManager {
    pub fn new(
        db_pool: PgPool,
        kafka_producer: FutureProducer,
        timeline_topic: String,
        contacts_topic: String,
    ) -> Self {
        let manager = Self {
            states: DashMap::new(),
            kafka_producer,
            timeline_topic,
            contacts_topic,
            db_pool,
        };

        let stations = vec![
            ("GS-0001", "Svalbard"),
            ("GS-0002", "Troll"),
            ("GS-0003", "Kiruna"),
            ("GS-0004", "McMurdo"),
            ("GS-0005", "Bangalore"),
            ("GS-0006", "Hyderabad"),
            ("GS-0007", "Delhi"),
            ("GS-0008", "Siliguri"),
        ];

        for (id, name) in stations {
            manager.states.insert(
                id.to_string(),
                GroundStationOps {
                    station_id: id.to_string(),
                    name: name.to_string(),
                    antenna_available: true,
                    rf_utilization: 0.0,
                    active_satellite: None,
                    contact_window_start: None,
                    contact_window_end: None,
                    total_uplinks: 25 + rand::random::<u32>() % 100,
                    total_downlinks: 120 + rand::random::<u32>() % 200,
                    failed_uplinks: rand::random::<u32>() % 5,
                    failed_downlinks: rand::random::<u32>() % 10,
                    missed_contacts: rand::random::<u32>() % 3,
                },
            );
        }

        manager
    }

    pub async fn update_ticks(&self, active_satellites: &[String], time: DateTime<Utc>) {
        let stations = crate::orbit::get_ground_stations();
        
        for mut entry in self.states.iter_mut() {
            let ops = entry.value_mut();
            
            if rand::random::<f64>() < 0.005 {
                ops.antenna_available = !ops.antenna_available;
                if !ops.antenna_available {
                    self.publish_event(
                        &ops.station_id,
                        "GROUND_STATION_MAINTENANCE",
                        "CRITICAL",
                        &format!("Ground Station {} is entering scheduled antenna maintenance.", ops.name),
                        None
                    ).await;
                } else {
                    self.publish_event(
                        &ops.station_id,
                        "GROUND_STATION_ONLINE",
                        "INFO",
                        &format!("Ground Station {} maintenance completed, antenna is back online.", ops.name),
                        None
                    ).await;
                }
            }

            if !ops.antenna_available {
                ops.rf_utilization = 0.0;
                if let Some(sat) = ops.active_satellite.take() {
                    ops.missed_contacts += 1;
                    self.publish_event(
                        &ops.station_id,
                        "CONTACT_MISSED",
                        "CRITICAL",
                        &format!("Contact missed for satellite {} due to station {} antenna outage.", sat, ops.name),
                        Some(&sat)
                    ).await;
                }
                continue;
            }

            // Find all satellites currently visible (elevation > 5.0)
            let mut visible_sats = Vec::new();
            let gs_info = stations.iter().find(|s| s.name == ops.name);
            if let Some(gs) = gs_info {
                for sat_id in active_satellites {
                    let (line1, line2) = crate::orbit::generate_default_tle(sat_id);
                    if let Ok(state) = crate::orbit::propagate_tle_sgp4(sat_id, &line1, &line2, time) {
                        let el = crate::orbit::calculate_elevation(state.latitude, state.longitude, state.altitude, gs);
                        if el > 5.0 {
                            visible_sats.push((sat_id.clone(), el));
                        }
                    }
                }
            }

            let current_active = ops.active_satellite.clone();
            
            if visible_sats.is_empty() {
                if let Some(sat) = current_active {
                    ops.active_satellite = None;
                    ops.rf_utilization = 0.0;
                    ops.contact_window_start = None;
                    ops.contact_window_end = None;
                    self.publish_event(
                        &ops.station_id,
                        "GROUND_CONTACT_ENDED",
                        "INFO",
                        &format!("Pass ended for satellite {} with station {}.", sat, ops.name),
                        Some(&sat)
                    ).await;
                }
            } else {
                let mut best_sat = None;
                if let Some(ref cur) = current_active {
                    if visible_sats.iter().any(|(s, _)| s == cur) {
                        best_sat = Some(cur.clone());
                    }
                }

                if best_sat.is_none() {
                    visible_sats.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
                    best_sat = Some(visible_sats[0].0.clone());
                }

                if let Some(sat) = best_sat {
                    if Some(sat.clone()) != current_active {
                        ops.active_satellite = Some(sat.clone());
                        ops.contact_window_start = Some(time);
                        
                        let mut los_est = time + chrono::Duration::minutes(10);
                        if let Some(gs) = gs_info {
                            let (line1, line2) = crate::orbit::generate_default_tle(&sat);
                            for min in 1..25 {
                                let t_check = time + chrono::Duration::minutes(min);
                                if let Ok(state) = crate::orbit::propagate_tle_sgp4(&sat, &line1, &line2, t_check) {
                                    let el = crate::orbit::calculate_elevation(state.latitude, state.longitude, state.altitude, gs);
                                    if el <= 5.0 {
                                        los_est = t_check;
                                        break;
                                    }
                                }
                            }
                        }
                        ops.contact_window_end = Some(los_est);
                        ops.total_uplinks += 1;
                        self.publish_event(
                            &ops.station_id,
                            "GROUND_CONTACT_STARTED",
                            "INFO",
                            &format!("AOS (Acquisition of Signal) for {} on station {}.", sat, ops.name),
                            Some(&sat)
                        ).await;

                        let reservation_id = uuid::Uuid::new_v4();
                        let _ = sqlx::query(
                            "INSERT INTO contact_schedules (id, satellite_id, station_id, window_type, start_time, end_time, priority, status)
                             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)"
                        )
                        .bind(reservation_id)
                        .bind(&sat)
                        .bind(&ops.station_id)
                        .bind("TELEMETRY")
                        .bind(time)
                        .bind(los_est)
                        .bind("ROUTINE")
                        .bind("EXECUTED")
                        .execute(&self.db_pool)
                        .await;
                    }

                    ops.rf_utilization = 65.0 + rand::random::<f64>() * 20.0;
                    ops.total_downlinks += 1;
                    
                    if rand::random::<f64>() < 0.01 {
                        ops.failed_downlinks += 1;
                        self.publish_event(
                            &ops.station_id,
                            "DOWNLINK_FAILURE",
                            "WARNING",
                            &format!("Brief signal fade. Downlink packet drop on station {}.", ops.name),
                            Some(&sat)
                        ).await;
                    }
                }
            }
        }
    }

    async fn publish_event(&self, station_id: &str, event_type: &str, severity: &str, msg: &str, sat_id: Option<&str>) {
        let details = serde_json::json!({
            "station_id": station_id,
            "satellite_id": sat_id,
            "timestamp": Utc::now(),
        });

        let timeline_payload = serde_json::json!({
            "timestamp": Utc::now(),
            "event_id": uuid::Uuid::new_v4(),
            "source": "GROUND_STATION",
            "satellite_id": sat_id,
            "event_type": event_type,
            "severity": severity,
            "message": msg,
            "details": details
        });

        if let Ok(payload) = serde_json::to_string(&timeline_payload) {
            let key = sat_id.unwrap_or(station_id);
            let record = FutureRecord::to(&self.timeline_topic)
                .key(key)
                .payload(&payload);
            if let Err(e) = self.kafka_producer.send(record, Duration::from_secs(5)).await {
                error!("Failed to publish timeline event in ground station manager: {:?}", e);
            }

            let contact_record = FutureRecord::to(&self.contacts_topic)
                .key(station_id)
                .payload(&payload);
            if let Err(e) = self.kafka_producer.send(contact_record, Duration::from_secs(5)).await {
                error!("Failed to publish contact event in ground station manager: {:?}", e);
            }
        }
    }
}
