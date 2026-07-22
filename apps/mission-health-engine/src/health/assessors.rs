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

use orbitlogs_models::TelemetryRecord;
use super::types::*;

pub fn assess_power(record: &TelemetryRecord) -> SubsystemHealth {
    let v = record.battery_voltage.unwrap_or(13.8);
    if !(10.0..=16.0).contains(&v) {
        let score = if v < 10.0 {
            lerp(v, 8.0, 10.0, 0.0, 24.0) as i32
        } else {
            lerp(v, 16.0, 18.0, 24.0, 0.0) as i32
        };
        SubsystemHealth {
            state: HealthState::Red,
            score,
            explanation: format!("Critical battery voltage: {:.2}V", v),
        }
    } else if v < 11.5 {
        let score = lerp(v, 10.0, 11.5, 25.0, 74.0) as i32;
        SubsystemHealth {
            state: HealthState::Yellow,
            score,
            explanation: format!("Low battery voltage: {:.2}V", v),
        }
    } else {
        let score = if v < 13.8 {
            lerp(v, 11.5, 12.5, 75.0, 100.0) as i32
        } else {
            lerp(v, 15.0, 16.0, 100.0, 75.0) as i32
        };
        SubsystemHealth {
            state: HealthState::Green,
            score,
            explanation: format!("Power system normal ({:.2}V)", v),
        }
    }
}

pub fn assess_thermal(record: &TelemetryRecord) -> SubsystemHealth {
    let t = record.temperature.unwrap_or(25.0);
    if !(-20.0..=85.0).contains(&t) {
        let score = if t < -20.0 {
            lerp(t, -40.0, -20.0, 0.0, 24.0) as i32
        } else {
            lerp(t, 85.0, 110.0, 24.0, 0.0) as i32
        };
        SubsystemHealth {
            state: HealthState::Red,
            score,
            explanation: format!("Critical temperature: {:.2}°C", t),
        }
    } else if !(0.0..=60.0).contains(&t) {
        let score = if t < 0.0 {
            lerp(t, -20.0, 0.0, 25.0, 74.0) as i32
        } else {
            lerp(t, 60.0, 85.0, 74.0, 25.0) as i32
        };
        SubsystemHealth {
            state: HealthState::Yellow,
            score,
            explanation: format!("Elevated temperature: {:.2}°C", t),
        }
    } else {
        let score = if t < 25.0 {
            lerp(t, 0.0, 15.0, 75.0, 100.0) as i32
        } else {
            lerp(t, 35.0, 60.0, 100.0, 75.0) as i32
        };
        SubsystemHealth {
            state: HealthState::Green,
            score,
            explanation: format!("Thermal system normal ({:.2}°C)", t),
        }
    }
}

pub fn assess_communications(
    record: &TelemetryRecord,
    gs_states: &std::collections::HashMap<String, GroundStationState>,
) -> SubsystemHealth {
    let loss = record.packet_loss.unwrap_or(0.0);
    let latency = record.communication_latency.unwrap_or(45.0);
    let signal = record.signal_strength.unwrap_or(-70.0);

    let linked_gs_id = record.satellite_id.linked_ground_station();
    let gs_state = gs_states.get(linked_gs_id.as_str());

    let is_degraded = loss > 5.0 || latency > 800.0 || signal < -95.0;
    let is_critical = loss > 15.0 || latency > 2000.0 || signal < -110.0;

    let loss_score = if loss > 15.0 {
        lerp(loss as f64, 15.0, 30.0, 24.0, 0.0) as i32
    } else if loss > 5.0 {
        lerp(loss as f64, 5.0, 15.0, 74.0, 25.0) as i32
    } else {
        lerp(loss as f64, 1.0, 5.0, 100.0, 75.0) as i32
    };

    let latency_score = if latency > 2000.0 {
        lerp(latency as f64, 2000.0, 5000.0, 24.0, 0.0) as i32
    } else if latency > 800.0 {
        lerp(latency as f64, 800.0, 2000.0, 74.0, 25.0) as i32
    } else {
        lerp(latency as f64, 100.0, 800.0, 100.0, 75.0) as i32
    };

    let signal_score = if signal < -110.0 {
        lerp(signal as f64, -120.0, -110.0, 0.0, 24.0) as i32
    } else if signal < -95.0 {
        lerp(signal as f64, -110.0, -95.0, 25.0, 74.0) as i32
    } else {
        lerp(signal as f64, -95.0, -75.0, 75.0, 100.0) as i32
    };

    let mut score = loss_score.min(latency_score).min(signal_score);
    let mut state = if is_critical {
        HealthState::Red
    } else if is_degraded {
        HealthState::Yellow
    } else {
        HealthState::Green
    };

    let explanation = if let Some(gs) = gs_state {
        if gs.status == orbitlogs_models::StationStatus::Outage
            || gs.status == orbitlogs_models::StationStatus::Degraded
        {
            if gs.status == orbitlogs_models::StationStatus::Outage {
                state = HealthState::Red;
                score = score.min(10);
            } else {
                state = HealthState::Yellow;
                score = score.min(45);
            }
            let prefix = if state == HealthState::Red {
                "Critical communications degradation"
            } else {
                "Communications degraded"
            };
            format!(
                "{} due to ground station {} {} (Weather: {})",
                prefix,
                gs.station_id,
                gs.status.to_string().to_lowercase(),
                gs.weather_state
            )
        } else if is_critical {
            format!("Critical comm degradation due to onboard transponder anomaly (Loss: {:.1}%, Latency: {:.1}ms, Signal: {:.1}dBm)", loss, latency, signal)
        } else if is_degraded {
            format!("Comm degradation due to onboard transponder anomaly (Loss: {:.1}%, Latency: {:.1}ms, Signal: {:.1}dBm)", loss, latency, signal)
        } else {
            format!(
                "Communications normal (Loss: {:.1}%, Latency: {:.1}ms, Signal: {:.1}dBm)",
                loss, latency, signal
            )
        }
    } else if is_critical {
        format!("Critical comm degradation due to onboard transponder anomaly (Loss: {:.1}%, Latency: {:.1}ms, Signal: {:.1}dBm)", loss, latency, signal)
    } else if is_degraded {
        format!("Comm degradation due to onboard transponder anomaly (Loss: {:.1}%, Latency: {:.1}ms, Signal: {:.1}dBm)", loss, latency, signal)
    } else {
        format!(
            "Communications normal (Loss: {:.1}%, Latency: {:.1}ms, Signal: {:.1}dBm)",
            loss, latency, signal
        )
    };

    SubsystemHealth {
        state,
        score,
        explanation,
    }
}

pub fn assess_propulsion(record: &TelemetryRecord) -> SubsystemHealth {
    let fuel = record.fuel_remaining.unwrap_or(95.0);
    let alt = record.altitude.unwrap_or(520.0);

    let is_critical = fuel < 5.0 || alt < 150.0;
    let is_degraded = fuel < 20.0 || alt < 300.0;

    let fuel_score = if fuel < 5.0 {
        lerp(fuel as f64, 0.0, 5.0, 0.0, 24.0) as i32
    } else if fuel < 20.0 {
        lerp(fuel as f64, 5.0, 20.0, 25.0, 74.0) as i32
    } else {
        lerp(fuel as f64, 20.0, 50.0, 75.0, 100.0) as i32
    };

    let alt_score = if alt < 150.0 {
        lerp(alt, 100.0, 150.0, 0.0, 24.0) as i32
    } else if alt < 300.0 {
        lerp(alt, 150.0, 300.0, 25.0, 74.0) as i32
    } else {
        lerp(alt, 300.0, 400.0, 75.0, 100.0) as i32
    };

    let score = fuel_score.min(alt_score);

    if is_critical {
        SubsystemHealth {
            state: HealthState::Red,
            score,
            explanation: format!(
                "Critical propulsion/orbital decay (Fuel: {:.1}%, Altitude: {:.1}km)",
                fuel, alt
            ),
        }
    } else if is_degraded {
        SubsystemHealth {
            state: HealthState::Yellow,
            score,
            explanation: format!(
                "Propulsion degraded/low orbit (Fuel: {:.1}%, Altitude: {:.1}km)",
                fuel, alt
            ),
        }
    } else {
        SubsystemHealth {
            state: HealthState::Green,
            score,
            explanation: format!(
                "Propulsion normal (Fuel: {:.1}%, Altitude: {:.1}km)",
                fuel, alt
            ),
        }
    }
}

pub fn assess_adcs(record: &TelemetryRecord) -> SubsystemHealth {
    let p = record.orientation_pitch.unwrap_or(0.0).abs();
    let r = record.orientation_roll.unwrap_or(0.0).abs();
    let y = record.orientation_yaw.unwrap_or(0.0).abs();
    let max_drift = p.max(r).max(y);

    if max_drift > 45.0 {
        let score = lerp(max_drift as f64, 45.0, 90.0, 24.0, 0.0) as i32;
        SubsystemHealth {
            state: HealthState::Red,
            score,
            explanation: format!(
                "Critical attitude drift (Pitch: {:.1}°, Roll: {:.1}°, Yaw: {:.1}°)",
                p, r, y
            ),
        }
    } else if max_drift > 15.0 {
        let score = lerp(max_drift as f64, 15.0, 45.0, 74.0, 25.0) as i32;
        SubsystemHealth {
            state: HealthState::Yellow,
            score,
            explanation: format!(
                "Attitude drift (Pitch: {:.1}°, Roll: {:.1}°, Yaw: {:.1}°)",
                p, r, y
            ),
        }
    } else {
        let score = lerp(max_drift as f64, 5.0, 15.0, 100.0, 75.0) as i32;
        SubsystemHealth {
            state: HealthState::Green,
            score,
            explanation: "ADCS control loop nominal".to_string(),
        }
    }
}

pub fn assess_payload(record: &TelemetryRecord) -> SubsystemHealth {
    let active = record.payload_active.unwrap_or(true);
    let draw = record.payload_power_draw.unwrap_or(15.0);
    let v = record.battery_voltage.unwrap_or(13.8);

    if active && draw > 30.0 {
        let score = lerp(draw, 30.0, 50.0, 24.0, 0.0) as i32;
        SubsystemHealth {
            state: HealthState::Red,
            score,
            explanation: format!("Critical payload power draw: {:.1}W", draw),
        }
    } else if !active && v > 11.5 {
        SubsystemHealth {
            state: HealthState::Red,
            score: 15,
            explanation: "Payload is inactive despite healthy battery".to_string(),
        }
    } else if !active && v <= 11.5 {
        let score = lerp(v, 10.0, 11.5, 25.0, 74.0) as i32;
        SubsystemHealth {
            state: HealthState::Yellow,
            score,
            explanation: "Payload auto-shutdown to protect battery".to_string(),
        }
    } else if active && draw > 20.0 {
        let score = lerp(draw, 20.0, 30.0, 74.0, 25.0) as i32;
        SubsystemHealth {
            state: HealthState::Yellow,
            score,
            explanation: format!("Elevated payload power draw: {:.1}W", draw),
        }
    } else {
        let score = lerp(draw, 18.0, 20.0, 100.0, 75.0) as i32;
        SubsystemHealth {
            state: HealthState::Green,
            score,
            explanation: format!("Payload operating normally ({:.1}W)", draw),
        }
    }
}

pub fn assess_flight_computer(record: &TelemetryRecord) -> SubsystemHealth {
    let cpu = record.cpu_usage.unwrap_or(25.0);
    let mem = record.memory_usage.unwrap_or(45.0);
    let max_load = cpu.max(mem);

    if max_load > 95.0 {
        let score = lerp(max_load as f64, 95.0, 100.0, 24.0, 0.0) as i32;
        SubsystemHealth {
            state: HealthState::Red,
            score,
            explanation: format!(
                "Critical flight computer load (CPU: {:.1}%, Mem: {:.1}%)",
                cpu, mem
            ),
        }
    } else if max_load > 80.0 {
        let score = lerp(max_load as f64, 80.0, 95.0, 74.0, 25.0) as i32;
        SubsystemHealth {
            state: HealthState::Yellow,
            score,
            explanation: format!(
                "High flight computer load (CPU: {:.1}%, Mem: {:.1}%)",
                cpu, mem
            ),
        }
    } else {
        let score = lerp(max_load as f64, 60.0, 80.0, 100.0, 75.0) as i32;
        SubsystemHealth {
            state: HealthState::Green,
            score,
            explanation: format!(
                "Flight computer operational (CPU: {:.1}%, Mem: {:.1}%)",
                cpu, mem
            ),
        }
    }
}

pub trait SubsystemAssessor: Send + Sync {
    fn assess(
        &self,
        record: &TelemetryRecord,
        gs_states: &std::collections::HashMap<String, GroundStationState>,
    ) -> SubsystemHealth;
}

pub struct PowerAssessor;
impl SubsystemAssessor for PowerAssessor {
    fn assess(
        &self,
        record: &TelemetryRecord,
        _gs_states: &std::collections::HashMap<String, GroundStationState>,
    ) -> SubsystemHealth {
        assess_power(record)
    }
}

pub struct ThermalAssessor;
impl SubsystemAssessor for ThermalAssessor {
    fn assess(
        &self,
        record: &TelemetryRecord,
        _gs_states: &std::collections::HashMap<String, GroundStationState>,
    ) -> SubsystemHealth {
        assess_thermal(record)
    }
}

pub struct CommunicationsAssessor;
impl SubsystemAssessor for CommunicationsAssessor {
    fn assess(
        &self,
        record: &TelemetryRecord,
        gs_states: &std::collections::HashMap<String, GroundStationState>,
    ) -> SubsystemHealth {
        assess_communications(record, gs_states)
    }
}

pub struct PropulsionAssessor;
impl SubsystemAssessor for PropulsionAssessor {
    fn assess(
        &self,
        record: &TelemetryRecord,
        _gs_states: &std::collections::HashMap<String, GroundStationState>,
    ) -> SubsystemHealth {
        assess_propulsion(record)
    }
}

pub struct AdcsAssessor;
impl SubsystemAssessor for AdcsAssessor {
    fn assess(
        &self,
        record: &TelemetryRecord,
        _gs_states: &std::collections::HashMap<String, GroundStationState>,
    ) -> SubsystemHealth {
        assess_adcs(record)
    }
}

pub struct PayloadAssessor;
impl SubsystemAssessor for PayloadAssessor {
    fn assess(
        &self,
        record: &TelemetryRecord,
        _gs_states: &std::collections::HashMap<String, GroundStationState>,
    ) -> SubsystemHealth {
        assess_payload(record)
    }
}

pub struct FlightComputerAssessor;
impl SubsystemAssessor for FlightComputerAssessor {
    fn assess(
        &self,
        record: &TelemetryRecord,
        _gs_states: &std::collections::HashMap<String, GroundStationState>,
    ) -> SubsystemHealth {
        assess_flight_computer(record)
    }
}
