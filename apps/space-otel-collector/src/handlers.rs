use std::sync::Arc;

use opentelemetry::Context;
use orbitlogs_models::TelemetryRecord;
use tracing_opentelemetry::OpenTelemetrySpanExt;

use crate::metrics::{
    GroundStationGauges, SatelliteGauges, GS_GAUGES_CACHE, SATELLITE_GAUGES_CACHE,
};

/// Process a telemetry record: update satellite Prometheus gauges and emit
/// OTel events for thermal / CPU anomalies.
pub fn handle_telemetry(payload: &str, parent_context: Context) {
    if let Ok(rec) = serde_json::from_str::<TelemetryRecord>(payload) {
        let span = tracing::info_span!("otel_process_telemetry", satellite_id = %rec.satellite_id);
        span.set_parent(parent_context);
        let _enter = span.enter();

        // 1. Record metrics
        let gauges = if let Some(g) = SATELLITE_GAUGES_CACHE.get(rec.satellite_id.as_str()) {
            g.clone()
        } else {
            SATELLITE_GAUGES_CACHE
                .entry(rec.satellite_id.to_string())
                .or_insert_with(|| Arc::new(SatelliteGauges::new(rec.satellite_id.as_str())))
                .clone()
        };

        if let Some(val) = rec.battery_voltage {
            gauges.battery_voltage.set(val);
        }
        if let Some(val) = rec.battery_current {
            gauges.battery_current.set(val);
        }
        if let Some(val) = rec.solar_panel_output {
            gauges.solar_output.set(val);
        }
        if let Some(val) = rec.cpu_usage {
            gauges.cpu_usage.set(val as f64);
        }
        if let Some(val) = rec.memory_usage {
            gauges.memory_usage.set(val as f64);
        }
        if let Some(val) = rec.fuel_remaining {
            gauges.fuel_remaining.set(val as f64);
        }
        if let Some(val) = rec.signal_strength {
            gauges.signal_strength.set(val as f64);
        }
        if let Some(val) = rec.packet_loss {
            gauges.packet_loss.set(val as f64);
        }
        if let Some(val) = rec.communication_latency {
            gauges.link_latency.set(val as f64);
        }

        // 2. Emit OTel logs / thermal events if critical anomalies
        if let Some(temp) = rec.temperature {
            if temp > 80.0 {
                tracing::event!(
                    tracing::Level::WARN,
                    event_type = "thermal_events",
                    satellite_id = %rec.satellite_id,
                    temperature = temp,
                    "Thermal warning: Core temperature is elevated"
                );
            }
        }
        if let Some(fc_cpu) = rec.cpu_usage {
            if fc_cpu > 95.0 {
                tracing::event!(
                    tracing::Level::ERROR,
                    event_type = "subsystem_failures",
                    subsystem = "flight_computer",
                    satellite_id = %rec.satellite_id,
                    "Flight computer CPU usage is critically high"
                );
            }
        }
    }
}

/// Process a command lifecycle event: emit OTel traces for command state
/// transitions.
pub fn handle_command_event(payload: &str, parent_context: Context) {
    if let Ok(cmd) = serde_json::from_str::<serde_json::Value>(payload) {
        let cmd_id = cmd["id"].as_str().unwrap_or("unknown");
        let sat_id = cmd["satellite_id"].as_str().unwrap_or("unknown");
        let ctype = cmd["command_type"].as_str().unwrap_or("unknown");
        let status = cmd["status"].as_str().unwrap_or("unknown");

        // Output Command Traces
        let span = tracing::info_span!(
            "command_lifecycle",
            command_id = %cmd_id,
            satellite_id = %sat_id,
            command_type = %ctype,
            status = %status
        );
        span.set_parent(parent_context);
        let _enter = span.enter();

        tracing::event!(
            tracing::Level::INFO,
            event_type = "command_lifecycle",
            command_id = %cmd_id,
            satellite_id = %sat_id,
            status = %status,
            "Command lifecycle state transition"
        );
    }
}

/// Process a ground station health message: update GS Prometheus gauges and
/// emit OTel events for outage / degradation.
pub fn handle_gs_health(payload: &str, parent_context: Context) {
    if let Ok(gs) = serde_json::from_str::<orbitlogs_models::GroundStationHealth>(payload) {
        let span = tracing::info_span!("groundstation_telemetry", station_id = %gs.station_id);
        span.set_parent(parent_context);
        let _enter = span.enter();

        let gs_gauges = if let Some(g) = GS_GAUGES_CACHE.get(gs.station_id.as_str()) {
            g.clone()
        } else {
            GS_GAUGES_CACHE
                .entry(gs.station_id.to_string())
                .or_insert_with(|| Arc::new(GroundStationGauges::new(gs.station_id.as_str())))
                .clone()
        };

        gs_gauges.link_latency.set(gs.latency as f64);
        gs_gauges.packet_loss.set(gs.congestion as f64);

        if gs.status == orbitlogs_models::StationStatus::Outage
            || gs.status == orbitlogs_models::StationStatus::Degraded
        {
            tracing::event!(
                tracing::Level::ERROR,
                event_type = "communication_events",
                station_id = %gs.station_id,
                weather = %gs.weather_state,
                explanation = %gs.explanation,
                "Ground station outage / degradation detected"
            );
        }
    }
}

/// Process a satellite health check message: emit OTel warnings when the
/// overall health state is not GREEN.
pub fn handle_satellite_health(payload: &str, parent_context: Context) {
    if let Ok(health) = serde_json::from_str::<serde_json::Value>(payload) {
        let sat_id = health["satellite_id"].as_str().unwrap_or("unknown");
        let ostate = health["overall_state"].as_str().unwrap_or("GREEN");
        let oscore = health["overall_score"].as_i64().unwrap_or(100);

        let span = tracing::info_span!("satellite_health_check", satellite_id = %sat_id);
        span.set_parent(parent_context);
        let _enter = span.enter();

        if ostate != "GREEN" {
            tracing::event!(
                tracing::Level::WARN,
                event_type = "subsystem_failures",
                satellite_id = %sat_id,
                health_score = oscore,
                "Satellite health degradation event generated"
            );
        }
    }
}
