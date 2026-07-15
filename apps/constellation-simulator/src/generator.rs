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
use uuid::Uuid;

pub struct SatState {
    pub id: String,
    pub battery_capacity: f64, // aging factor
    pub thermal_phase: f64,
    pub orbit_phase: f64,
    pub altitude: f64,
    pub fuel: f32,
    pub faulty: bool,
}

pub fn sim_tick_satellite(sat: &mut SatState) -> TelemetryRecord {
    // Simulate Battery Aging
    sat.battery_capacity -= 0.0001; // aging over time

    // Orbit Changes (orbital decay & phase shift)
    sat.orbit_phase += 0.05;
    sat.altitude -= 0.005; // slowly decay

    // Thermal Drift based on orbital phase (sun pointing vs eclipse)
    sat.thermal_phase += 0.08;
    let sun_exposure = sat.thermal_phase.sin();
    let temp = if sun_exposure > 0.0 {
        20.0 + (sun_exposure * 40.0) // sunny side: heats up
    } else {
        -10.0 + (sun_exposure * 15.0) // eclipse: cools down
    };

    // Ground Station Link changes
    let gs_lat = sat.orbit_phase.sin() * 50.0;
    let signal = if gs_lat > 20.0 {
        -50.0 + (rand::random::<f32>() * 10.0) // good signal
    } else if gs_lat > -20.0 {
        -80.0 + (rand::random::<f32>() * 15.0) // marginal
    } else {
        -110.0 // weak
    };

    // Subsystem metrics simulation
    let battery_v = 12.0 + (sun_exposure * 1.8) * (sat.battery_capacity / 100.0);
    let packet_loss = if signal < -95.0 { 15.0 } else { 0.1 };

    TelemetryRecord {
        satellite_id: orbitlogs_models::SatelliteId::new(sat.id.clone()),
        subsystem_id: orbitlogs_models::Subsystem::Unified,
        mission_id: if sat.faulty {
            orbitlogs_models::MissionId::new("MISSION-CRITICAL")
        } else {
            orbitlogs_models::MissionId::new("MISSION-CONSTELLATION")
        },
        timestamp: chrono::Utc::now(),
        battery_voltage: Some(battery_v),
        battery_current: Some(if sun_exposure > 0.0 { 1.5 } else { -0.5 }),
        solar_panel_output: Some(if sun_exposure > 0.0 { 120.0 } else { 0.0 }),
        cpu_usage: Some(15.0 + (rand::random::<f32>() * 10.0)),
        memory_usage: Some(30.0 + (rand::random::<f32>() * 5.0)),
        temperature: Some(if sat.faulty { temp + 45.0 } else { temp }),
        orientation_pitch: Some((sat.orbit_phase.cos() * 5.0) as f32),
        orientation_roll: Some(0.0),
        orientation_yaw: Some(0.0),
        velocity: Some(7.6),
        altitude: Some(sat.altitude),
        fuel_remaining: Some(sat.fuel),
        communication_latency: Some(50.0 + (gs_lat.abs() * 5.0) as f32),
        packet_loss: Some(packet_loss),
        signal_strength: Some(signal),
        payload_power_draw: Some(15.0),
        payload_active: Some(true),
        event_id: Uuid::new_v4(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sat_state_tick() {
        let mut sat = SatState {
            id: "SAT-0001".to_string(),
            battery_capacity: 100.0,
            thermal_phase: 0.0,
            orbit_phase: 0.0,
            altitude: 500.0,
            fuel: 95.0,
            faulty: false,
        };

        let rec = sim_tick_satellite(&mut sat);

        assert_eq!(rec.satellite_id.as_str(), "SAT-0001");
        assert_eq!(rec.mission_id.as_str(), "MISSION-CONSTELLATION");
        assert!(rec.validate().is_ok());
        assert!(sat.altitude < 500.0); // Orbit decay
        assert!(sat.battery_capacity < 100.0); // Battery aging
    }

    #[test]
    fn test_faulty_sat_state_tick() {
        let mut sat = SatState {
            id: "SAT-0123".to_string(),
            battery_capacity: 100.0,
            thermal_phase: 0.0,
            orbit_phase: 0.0,
            altitude: 500.0,
            fuel: 95.0,
            faulty: true,
        };

        let rec = sim_tick_satellite(&mut sat);

        assert_eq!(rec.satellite_id.as_str(), "SAT-0123");
        assert_eq!(rec.mission_id.as_str(), "MISSION-CRITICAL");
        assert!(rec.temperature.unwrap() > 40.0); // Elevated temp for faulty sat
    }
}
