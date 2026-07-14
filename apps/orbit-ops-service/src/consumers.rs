use crate::ServiceState;
use crate::kafka_guard;
use orbitlogs_models::TelemetryRecord;
use rdkafka::consumer::{CommitMode, Consumer};
use rdkafka::message::Message;
use rdkafka::producer::{FutureRecord, FutureProducer};
use uuid::Uuid;
use chrono::Utc;
use std::sync::Arc;
use std::time::Duration;
use tokio_util::sync::CancellationToken;
use tracing::{error, info};
use orbitlogs_models::kafka::KafkaConfigBuilder;
use sqlx::Row;

pub async fn run_telemetry_loop(
    state: Arc<ServiceState>,
    brokers: String,
    topic: String,
    group_id: String,
    cancel_token: CancellationToken,
) -> Result<(), anyhow::Error> {
    let mut config = KafkaConfigBuilder::from_env(brokers).build();
    config
        .set("group.id", &group_id)
        .set("enable.partition.eof", "false")
        .set("session.timeout.ms", "6000")
        .set("enable.auto.commit", "false")
        .set("auto.offset.reset", "latest");

    let consumer = kafka_guard::KafkaConsumerGuard {
        consumer: config.create()?,
        topic: topic.clone(),
    };
    consumer.subscribe(&[&topic])?;

    info!(
        "Operations background listener subscribed to topic {}",
        topic
    );

    loop {
        tokio::select! {
            _ = cancel_token.cancelled() => break,
            msg_res = consumer.recv() => {
                match msg_res {
                    Ok(msg) => {
                        if let Some(Ok(payload)) = msg.payload_view::<str>() {
                            if let Ok(rec) = serde_json::from_str::<TelemetryRecord>(payload) {
                                let sat_id = rec.satellite_id.clone();

                                let rec_twin = state.twin.update_twin(&sat_id, rec);

                                for processor in &state.processors {
                                    let _ = processor.process(&sat_id, &rec_twin).await;
                                }

                                verify_pending_commands(&state, &rec_twin).await;
                            }
                        }
                        let _ = consumer.commit_message(&msg, CommitMode::Async);
                    }
                    Err(e) => {
                        error!("Kafka processed telemetry consumer error: {}", e);
                        tokio::time::sleep(Duration::from_millis(500)).await;
                    }
                }
            }
        }
    }
    Ok(())
}

pub async fn verify_pending_commands(state: &ServiceState, rec: &TelemetryRecord) {
    let rows = match sqlx::query("SELECT id, command_type, status, parameters FROM commands WHERE satellite_id = $1 AND status IN ('TRANSMITTED', 'EXECUTING', 'QUEUED', 'UPLINKED') ORDER BY created_at ASC")
        .bind(&rec.satellite_id)
        .fetch_all(&state.db_pool)
        .await
    {
        Ok(r) => r,
        Err(_) => return,
    };

    for row in rows {
        let cmd_id: Uuid = row.get("id");
        let cmd_type: String = row.get("command_type");

        let mut matched = false;
        let expected_field;
        let expected_val;

        match cmd_type.as_str() {
            "CMD_SHED_LOAD_SECONDARY" | "CMD_PAYLOAD_DEACTIVATE" | "CMD_ENTER_SAFE_MODE" => {
                expected_field = "payload_active".to_string();
                expected_val = "false".to_string();
                if let Some(active) = rec.payload_active {
                    if !active {
                        matched = true;
                    }
                }
            }
            "CMD_TTC_POWER_BOOST" => {
                expected_field = "signal_strength".to_string();
                expected_val = ">-75".to_string();
                if let Some(sig) = rec.signal_strength {
                    if sig > -75.0 {
                        matched = true;
                    }
                }
            }
            "CMD_PROP_THRUST" => {
                expected_field = "velocity".to_string();
                expected_val = ">7.6".to_string();
                if let Some(vel) = rec.velocity {
                    if vel > 7.6 {
                        matched = true;
                    }
                }
            }
            _ => {
                matched = true;
                expected_field = "any".to_string();
                expected_val = "any".to_string();
            }
        }

        if matched {
            state.twin.remove_command(rec.satellite_id.as_str(), &cmd_type);

            if let Err(e) = sqlx::query(
                "UPDATE commands SET status = 'CONFIRMED', executed_at = $1 WHERE id = $2",
            )
            .bind(Utc::now())
            .bind(cmd_id)
            .execute(&state.db_pool)
            .await
            {
                error!(
                    "Failed to update command status to CONFIRMED for command {}: {}",
                    cmd_id, e
                );
            }

            let v_id = Uuid::new_v4();
            if let Err(e) = sqlx::query("INSERT INTO command_verifications (id, command_id, satellite_id, command_type, expected_telemetry_field, expected_telemetry_value, status) VALUES ($1, $2, $3, $4, $5, $6, $7)")
                .bind(v_id)
                .bind(cmd_id)
                .bind(&rec.satellite_id)
                .bind(&cmd_type)
                .bind(expected_field)
                .bind(expected_val)
                .bind("CONFIRMED")
                .execute(&state.db_pool)
                .await
            {
                error!("Failed to insert command verification for command {}: {}", cmd_id, e);
            }

            let verify_payload = serde_json::json!({
                "timestamp": Utc::now(),
                "verification_id": v_id,
                "command_id": cmd_id,
                "satellite_id": rec.satellite_id,
                "command_type": cmd_type,
                "status": "CONFIRMED",
                "message": format!("Command {} of type {} successfully executed and verified via telemetry feedback.", cmd_id, cmd_type),
            });
            if let Ok(payload) = serde_json::to_string(&verify_payload) {
                let record = FutureRecord::to("command.verification.events")
                    .key(rec.satellite_id.as_str())
                    .payload(&payload);
                let _ = state
                    .kafka_producer
                    .send(record, Duration::from_secs(5))
                    .await;
            }

            let timeline_payload = serde_json::json!({
                "timestamp": Utc::now(),
                "event_id": Uuid::new_v4(),
                "source": "COMMAND",
                "satellite_id": rec.satellite_id,
                "event_type": "COMMAND_VERIFIED",
                "severity": "INFO",
                "message": format!("Command verified: '{}' is CONFIRMED", cmd_type),
                "details": {
                    "command_id": cmd_id,
                    "verification_id": v_id,
                    "status": "CONFIRMED"
                }
            });
            if let Ok(payload) = serde_json::to_string(&timeline_payload) {
                let record = FutureRecord::to(&state.timeline_topic)
                    .key(rec.satellite_id.as_str())
                    .payload(&payload);
                let _ = state
                    .kafka_producer
                    .send(record, Duration::from_secs(5))
                    .await;
            }
        }
    }
}
