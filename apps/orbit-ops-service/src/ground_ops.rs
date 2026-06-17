use serde::{Serialize, Deserialize};
use dashmap::DashMap;
use chrono::{DateTime, Utc};
use std::sync::Arc;
use uuid::Uuid;
use rdkafka::producer::{FutureProducer, FutureRecord};
use std::time::Duration;
use tracing::{error, info};

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
}

impl GroundOpsManager {
    pub fn new(kafka_producer: FutureProducer, timeline_topic: String, contacts_topic: String) -> Self {
        let manager = Self {
            states: DashMap::new(),
            kafka_producer,
            timeline_topic,
            contacts_topic,
        };

        // Initialize standard ground stations
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

    pub async fn update_ticks(&self) {
        // Randomly simulate RF utilization shifts and antenna state checks
        for mut entry in self.states.iter_mut() {
            let ops = entry.value_mut();
            
            // Randomly toggle antenna availability (e.g. maintenance window)
            if rand::random::<f64>() < 0.01 {
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

            // Simulate satellite passes and RF shifts
            if ops.antenna_available {
                if ops.active_satellite.is_some() {
                    // 90% chance to stay connected, 10% chance to lose contact
                    if rand::random::<f64>() < 0.10 {
                        let sat = ops.active_satellite.take().unwrap();
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
                    } else {
                        // Dynamic RF utilization during contact
                        ops.rf_utilization = 65.0 + rand::random::<f64>() * 20.0;
                        ops.total_downlinks += 1;
                        if rand::random::<f64>() < 0.01 {
                            ops.failed_downlinks += 1;
                            self.publish_event(
                                &ops.station_id,
                                "DOWNLINK_FAILURE",
                                "WARNING",
                                &format!("Brief signal fade. Downlink packet drop on station {}.", ops.name),
                                ops.active_satellite.as_deref()
                            ).await;
                        }
                    }
                } else {
                    // Chance to acquire new satellite pass
                    if rand::random::<f64>() < 0.15 {
                        let sat_id = format!("SAT-{:04}", 1 + rand::random::<usize>() % 10);
                        ops.active_satellite = Some(sat_id.clone());
                        ops.rf_utilization = 80.0;
                        ops.contact_window_start = Some(Utc::now());
                        ops.contact_window_end = Some(Utc::now() + chrono::Duration::minutes(10));
                        ops.total_uplinks += 1;

                        self.publish_event(
                            &ops.station_id,
                            "GROUND_CONTACT_STARTED",
                            "INFO",
                            &format!("AOS (Acquisition of Signal) for {} on station {}.", sat_id, ops.name),
                            Some(&sat_id)
                        ).await;
                    }
                }
            } else {
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
            "event_id": Uuid::new_v4(),
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
            let _ = self.kafka_producer.send(record, Duration::from_secs(5)).await;

            let contact_record = FutureRecord::to(&self.contacts_topic)
                .key(station_id)
                .payload(&payload);
            let _ = self.kafka_producer.send(contact_record, Duration::from_secs(5)).await;
        }
    }
}
