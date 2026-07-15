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
#![allow(clippy::manual_clamp)]
use chrono::Utc;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use std::time::SystemTime;
use uuid::Uuid;

use crate::scenario::Scenario;

// Import TelemetryRecord from the shared orbitlogs_models crate
pub use orbitlogs_models::TelemetryRecord;

// Import GroundStationHealth as GroundStationState from the shared orbitlogs_models crate
pub use orbitlogs_models::GroundStationHealth as GroundStationState;

pub struct SatelliteSimulator {
    pub id: orbitlogs_models::SatelliteId,
    pub scenario: Scenario,
    pub ticks: u64,

    // Internal base state variables
    battery_voltage: f64,
    fuel_remaining: f32,
    temperature: f64,
    altitude: f64,
    velocity: f64,

    // RNG implementation
    rng: ChaCha8Rng,
}

impl SatelliteSimulator {
    pub fn new(id: orbitlogs_models::SatelliteId, scenario: Scenario, seed: Option<u64>) -> Self {
        let base_seed = seed.unwrap_or_else(|| {
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .expect("Time went backwards")
                .as_nanos() as u64
        });

        // Derive satellite-specific seed
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        use std::hash::{Hash, Hasher};
        id.hash(&mut hasher);
        let sat_seed = base_seed ^ hasher.finish();
        let rng = ChaCha8Rng::seed_from_u64(sat_seed);

        Self {
            id,
            scenario,
            ticks: 0,
            battery_voltage: 13.8,
            fuel_remaining: 95.0,
            temperature: 25.0,
            altitude: 520.0,
            velocity: 7.6,
            rng,
        }
    }

    #[allow(dead_code)]
    pub fn set_scenario(&mut self, scenario: Scenario) {
        self.scenario = scenario;
    }

    #[allow(dead_code)]
    pub fn next_record(&mut self) -> TelemetryRecord {
        // Legacy support wrapper: generates a unified record representing "all" subsystems
        self.next_subsystem_record(
            orbitlogs_models::Subsystem::Unified,
            orbitlogs_models::MissionId::new("unknown"),
            None,
        )
    }

    // Modular Subsystem Generators to avoid massive monolith function

    fn update_power_and_thermal(
        &mut self,
        orbital_phase: f64,
        volt_noise: f64,
        temp_noise: f64,
    ) -> (f64, f64) {
        let mut solar_panel = if orbital_phase > 0.0 {
            120.0 * orbital_phase + self.rng.gen_range(-5.0..5.0)
        } else {
            0.0
        };

        if self.temperature > 100.0 {
            solar_panel *= 0.5;
        }

        let mut bat_current = if solar_panel > 10.0 {
            (solar_panel / 12.0) - 3.0 + self.rng.gen_range(-0.5..0.5)
        } else {
            -2.5 + self.rng.gen_range(-0.2..0.2)
        };

        match self.scenario {
            Scenario::Normal => {
                self.temperature = 25.0 + 5.0 * orbital_phase + temp_noise;
                self.battery_voltage = 13.6 + 0.6 * orbital_phase + volt_noise;
                self.fuel_remaining -= 0.0001;

                if self.battery_voltage < 11.5 {
                    self.battery_voltage = 11.5;
                }
            }
            Scenario::ThermalSpike => {
                self.temperature += 2.5 + self.rng.gen_range(0.0..1.5);
                if self.temperature > 130.0 {
                    self.temperature = 130.0 + temp_noise;
                }
                self.battery_voltage = 13.0 + 0.4 * orbital_phase + volt_noise;
                self.fuel_remaining -= 0.0001;
            }
            Scenario::BatteryDegradation => {
                self.battery_voltage -= 0.15 + self.rng.gen_range(0.0..0.05);
                if self.battery_voltage < 8.0 {
                    self.battery_voltage = 8.0;
                }
                bat_current = -5.0 + self.rng.gen_range(-1.0..1.0);
                solar_panel *= 0.5;
                self.temperature = 25.0 + 3.0 * orbital_phase + temp_noise;
                self.fuel_remaining -= 0.0001;
            }
            Scenario::CommunicationDegradation => {
                self.temperature = 25.0 + 5.0 * orbital_phase + temp_noise;
                self.battery_voltage = 13.6 + 0.6 * orbital_phase + volt_noise;
                self.fuel_remaining -= 0.0001;
            }
            Scenario::FuelLeakage => {
                self.fuel_remaining -= 1.2 + self.rng.gen_range(0.0..0.5);
                if self.fuel_remaining < 0.0 {
                    self.fuel_remaining = 0.0;
                }
                self.temperature = 25.0 + 5.0 * orbital_phase + temp_noise;
                self.battery_voltage = 13.6 + 0.6 * orbital_phase + volt_noise;
            }
        }

        (solar_panel, bat_current)
    }

    fn update_propulsion(&mut self, alt_noise: f64) {
        if self.fuel_remaining <= 0.0 {
            self.altitude -= 0.5 + self.rng.gen_range(0.0..0.1);
            if self.altitude < 100.0 {
                self.altitude = 100.0;
            }
            self.velocity = 7.0 - (self.altitude - 100.0) * 0.005 + self.rng.gen_range(-0.01..0.01);
        } else {
            self.altitude += alt_noise;
            self.velocity = 7.67 - (self.altitude - 520.0) * 0.01 + self.rng.gen_range(-0.01..0.01);
        }
    }

    fn update_adcs(&mut self) -> (f32, f32, f32) {
        let mut pitch = (self.ticks as f32 * 0.02).cos() * 5.0 + self.rng.gen_range(-0.5..0.5);
        let mut roll = (self.ticks as f32 * 0.03).sin() * 3.0 + self.rng.gen_range(-0.3..0.3);
        let mut yaw = (self.ticks as f32 * 0.01 + 1.2).sin() * 4.0 + self.rng.gen_range(-0.4..0.4);

        if self.id.as_str() == "SAT-0001" {
            let ts = chrono::Utc::now().timestamp() as f64;
            let phase = ts * 0.001;
            self.altitude = 421.5;
            self.velocity = 7.66;
            pitch = (phase.sin() * 51.64) as f32;
            roll = ((phase * 1.5).cos() * 180.0) as f32;
        } else if self.id.as_str() == "SAT-0002" {
            let tle_l1 = "1 25544U 98067A   26166.57790509  .00015525  00000-0  28286-3 0  9997";
            let tle_l2 = "2 25544  51.6416 195.1432 0005211  92.5188  64.2188 15.49896541571216";
            let (lat, lon, alt) = crate::adapters::OrbitAdapters::propagate_tle(
                tle_l1,
                tle_l2,
                self.ticks as f64 * 5.0,
            );
            self.altitude = alt;
            pitch = lat as f32;
            roll = lon as f32;
        }

        if self.scenario == Scenario::FuelLeakage && self.fuel_remaining > 0.0 {
            pitch += self.rng.gen_range(-20.0..20.0);
            roll += self.rng.gen_range(-20.0..20.0);
            yaw += self.rng.gen_range(-20.0..20.0);
        }

        if self.battery_voltage < 11.0 {
            pitch += self.rng.gen_range(-15.0..15.0);
            roll += self.rng.gen_range(-15.0..15.0);
            yaw += self.rng.gen_range(-15.0..15.0);
        }

        (pitch, roll, yaw)
    }

    fn update_comms(&mut self, gs_state: Option<&GroundStationState>) -> (f32, f32, f32) {
        let mut latency = 45.0f32 + self.rng.gen_range(-5.0f32..5.0f32);
        let mut loss = self.rng.gen_range(0.0f32..0.5f32);
        let mut signal = -70.0f32 + self.rng.gen_range(-2.0f32..2.0f32);

        if self.scenario == Scenario::CommunicationDegradation {
            latency += (self.ticks as f32 * 50.0f32).min(3000.0f32);
            loss = (0.5f32 + (self.ticks as f32 * 1.5f32)).min(100.0f32);
            signal = (-70.0f32 - (self.ticks as f32 * 1.2f32)).max(-125.0f32);
        }

        let thermal_stress = self.temperature > 90.0;
        if thermal_stress || self.battery_voltage < 10.5 {
            latency += 500.0f32;
            loss += 10.0f32;
            signal -= 15.0f32;
        }

        if let Some(gs) = gs_state {
            if gs.status == orbitlogs_models::StationStatus::Outage {
                latency = 3000.0f32;
                loss = 100.0f32;
                signal = -125.0f32;
            } else if gs.status == orbitlogs_models::StationStatus::Degraded {
                if gs.weather_state == orbitlogs_models::WeatherState::SevereStorm
                    || gs.weather_state == orbitlogs_models::WeatherState::HeavySnow
                {
                    loss += 15.0f32;
                    signal -= 20.0f32;
                    latency += 150.0f32;
                } else {
                    loss += 10.0f32;
                    signal -= 10.0f32;
                    latency += 100.0f32;
                }
            }
        }

        loss = loss.clamp(0.0f32, 100.0f32);
        signal = signal.max(-130.0f32);

        (latency, loss, signal)
    }

    fn update_flight_computer(&mut self, orbital_phase: f64) -> (f32, f32) {
        let is_throttled = self.temperature > 80.0;
        let cpu = if is_throttled {
            92.0f32 + self.rng.gen_range(-2.0f32..2.0f32)
        } else {
            match self.scenario {
                Scenario::ThermalSpike => 90.0f32 + self.rng.gen_range(-5.0f32..5.0f32),
                _ => 15.0f32 + 10.0f32 * orbital_phase as f32 + self.rng.gen_range(-2.0f32..2.0f32),
            }
        }
        .clamp(1.0f32, 100.0f32);

        let mem = match self.scenario {
            Scenario::ThermalSpike => 85.0f32 + self.rng.gen_range(-2.0f32..2.0f32),
            _ => 22.0f32 + self.rng.gen_range(-1.0f32..1.0f32),
        }
        .clamp(1.0f32, 100.0f32);

        (cpu, mem)
    }

    fn update_payload(&mut self) -> (bool, f64) {
        let payload_active = self.battery_voltage > 11.5;
        let payload_power_draw: f64 = if payload_active {
            15.0 + self.rng.gen_range(-1.0..1.0)
        } else {
            0.0
        };

        (payload_active, payload_power_draw)
    }

    pub fn next_subsystem_record(
        &mut self,
        subsystem: orbitlogs_models::Subsystem,
        mission_id: orbitlogs_models::MissionId,
        gs_state: Option<&GroundStationState>,
    ) -> TelemetryRecord {
        self.ticks += 1;

        let orbital_phase = (self.ticks as f64 * 0.05).sin();
        let volt_noise = self.rng.gen_range(-0.1..0.1);
        let temp_noise = self.rng.gen_range(-0.5..0.5);
        let alt_noise = self.rng.gen_range(-0.2..0.2);

        // Core updates
        let (solar_panel, bat_current) =
            self.update_power_and_thermal(orbital_phase, volt_noise, temp_noise);
        self.update_propulsion(alt_noise);
        let (pitch, roll, yaw) = self.update_adcs();
        let (latency, loss, signal) = self.update_comms(gs_state);
        let (cpu, mem) = self.update_flight_computer(orbital_phase);
        let (payload_active, payload_power_draw) = self.update_payload();

        // Construct Telemetry Record with fields matching the requested subsystem
        let mut record = TelemetryRecord {
            satellite_id: self.id.clone(),
            subsystem_id: subsystem,
            mission_id,
            timestamp: Utc::now(),
            battery_voltage: None,
            battery_current: None,
            solar_panel_output: None,
            cpu_usage: None,
            memory_usage: None,
            temperature: None,
            orientation_pitch: None,
            orientation_roll: None,
            orientation_yaw: None,
            velocity: None,
            altitude: None,
            fuel_remaining: None,
            communication_latency: None,
            packet_loss: None,
            signal_strength: None,
            payload_power_draw: None,
            payload_active: None,
            event_id: Uuid::new_v4(),
        };

        match subsystem {
            orbitlogs_models::Subsystem::Power => {
                record.battery_voltage = Some((self.battery_voltage * 100.0).round() / 100.0);
                record.battery_current = Some((bat_current * 100.0).round() / 100.0);
                record.solar_panel_output = Some((solar_panel * 100.0).round() / 100.0);
            }
            orbitlogs_models::Subsystem::Thermal => {
                record.temperature = Some((self.temperature * 100.0).round() / 100.0);
            }
            orbitlogs_models::Subsystem::Communications => {
                record.communication_latency = Some((latency * 10.0).round() / 10.0);
                record.packet_loss = Some((loss * 10.0).round() / 10.0);
                record.signal_strength = Some((signal * 10.0).round() / 10.0);
            }
            orbitlogs_models::Subsystem::Propulsion => {
                record.velocity = Some((self.velocity * 10000.0).round() / 10000.0);
                record.altitude = Some((self.altitude * 100.0).round() / 100.0);
                record.fuel_remaining = Some((self.fuel_remaining * 10.0).round() / 10.0);
            }
            orbitlogs_models::Subsystem::Adcs => {
                record.orientation_pitch = Some((pitch * 100.0).round() / 100.0);
                record.orientation_roll = Some((roll * 100.0).round() / 100.0);
                record.orientation_yaw = Some((yaw * 100.0).round() / 100.0);
            }
            orbitlogs_models::Subsystem::FlightComputer => {
                record.cpu_usage = Some((cpu * 10.0).round() / 10.0);
                record.memory_usage = Some((mem * 10.0).round() / 10.0);
            }
            orbitlogs_models::Subsystem::Payload => {
                record.payload_power_draw = Some((payload_power_draw * 100.0).round() / 100.0);
                record.payload_active = Some(payload_active);
            }
            _ => {
                // Populate all fields for backward compatibility (subsystem "all" or unified fallback)
                record.battery_voltage = Some((self.battery_voltage * 100.0).round() / 100.0);
                record.battery_current = Some((bat_current * 100.0).round() / 100.0);
                record.solar_panel_output = Some((solar_panel * 100.0).round() / 100.0);
                record.temperature = Some((self.temperature * 100.0).round() / 100.0);
                record.communication_latency = Some((latency * 10.0).round() / 10.0);
                record.packet_loss = Some((loss * 10.0).round() / 10.0);
                record.signal_strength = Some((signal * 10.0).round() / 10.0);
                record.velocity = Some((self.velocity * 10000.0).round() / 10000.0);
                record.altitude = Some((self.altitude * 100.0).round() / 100.0);
                record.fuel_remaining = Some((self.fuel_remaining * 10.0).round() / 10.0);
                record.orientation_pitch = Some((pitch * 100.0).round() / 100.0);
                record.orientation_roll = Some((roll * 100.0).round() / 100.0);
                record.orientation_yaw = Some((yaw * 100.0).round() / 100.0);
                record.cpu_usage = Some((cpu * 10.0).round() / 10.0);
                record.memory_usage = Some((mem * 10.0).round() / 10.0);
                record.payload_power_draw = Some((payload_power_draw * 100.0).round() / 100.0);
                record.payload_active = Some(payload_active);
            }
        }

        record
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simulator_creation() {
        let sim = SatelliteSimulator::new(
            orbitlogs_models::SatelliteId::new("SAT-9999"),
            Scenario::Normal,
            Some(12345),
        );
        assert_eq!(sim.id.as_str(), "SAT-9999");
        assert_eq!(sim.scenario, Scenario::Normal);
        assert_eq!(sim.ticks, 0);
    }

    #[test]
    fn test_deterministic_generation() {
        let mut sim1 = SatelliteSimulator::new(
            orbitlogs_models::SatelliteId::new("SAT-0001"),
            Scenario::Normal,
            Some(42),
        );
        let mut sim2 = SatelliteSimulator::new(
            orbitlogs_models::SatelliteId::new("SAT-0001"),
            Scenario::Normal,
            Some(42),
        );

        let rec1 = sim1.next_record();
        let rec2 = sim2.next_record();

        assert_eq!(rec1.battery_voltage, rec2.battery_voltage);
        assert_eq!(rec1.solar_panel_output, rec2.solar_panel_output);
        assert_eq!(rec1.temperature, rec2.temperature);
    }

    #[test]
    fn test_thermal_spike_scenario() {
        let mut sim = SatelliteSimulator::new(
            orbitlogs_models::SatelliteId::new("SAT-0002"),
            Scenario::ThermalSpike,
            Some(101),
        );
        let base_rec = sim.next_subsystem_record(
            orbitlogs_models::Subsystem::Thermal,
            orbitlogs_models::MissionId::new("test-mission"),
            None,
        );

        let mut prev_temp = base_rec.temperature.unwrap();
        for _ in 0..5 {
            let rec = sim.next_subsystem_record(
                orbitlogs_models::Subsystem::Thermal,
                orbitlogs_models::MissionId::new("test-mission"),
                None,
            );
            let temp = rec.temperature.unwrap();
            assert!(temp > prev_temp || temp >= 129.0);
            prev_temp = temp;
        }
    }

    #[test]
    fn test_fuel_leakage_scenario() {
        let mut sim = SatelliteSimulator::new(
            orbitlogs_models::SatelliteId::new("SAT-0003"),
            Scenario::FuelLeakage,
            Some(202),
        );
        let rec = sim.next_subsystem_record(
            orbitlogs_models::Subsystem::Propulsion,
            orbitlogs_models::MissionId::new("test-mission"),
            None,
        );
        assert!(rec.fuel_remaining.unwrap() < 95.0);
    }

    #[test]
    fn test_thermal_side_effects() {
        let mut sim = SatelliteSimulator::new(
            orbitlogs_models::SatelliteId::new("SAT-0004"),
            Scenario::ThermalSpike,
            Some(303),
        );

        // Advance ticks to trigger high temperature
        for _ in 0..30 {
            let _ = sim.next_subsystem_record(
                orbitlogs_models::Subsystem::Thermal,
                orbitlogs_models::MissionId::new("test-mission"),
                None,
            );
        }

        // Verify side effect: CPU Throttling
        let rec_cpu = sim.next_subsystem_record(
            orbitlogs_models::Subsystem::FlightComputer,
            orbitlogs_models::MissionId::new("test-mission"),
            None,
        );
        assert!(
            rec_cpu.cpu_usage.unwrap() > 90.0,
            "CPU should be throttled (>90%) due to thermal spike"
        );
    }

    #[test]
    fn test_battery_depletion_side_effects() {
        let mut sim = SatelliteSimulator::new(
            orbitlogs_models::SatelliteId::new("SAT-0005"),
            Scenario::BatteryDegradation,
            Some(404),
        );

        // Advance ticks to drain battery
        for _ in 0..20 {
            let _ = sim.next_subsystem_record(
                orbitlogs_models::Subsystem::Power,
                orbitlogs_models::MissionId::new("test-mission"),
                None,
            );
        }

        // Verify side effect: Payload shutdown
        let rec_payload = sim.next_subsystem_record(
            orbitlogs_models::Subsystem::Payload,
            orbitlogs_models::MissionId::new("test-mission"),
            None,
        );
        assert!(
            !rec_payload.payload_active.unwrap(),
            "Payload should shut down on low battery"
        );
        assert_eq!(
            rec_payload.payload_power_draw.unwrap(),
            0.0,
            "Payload power draw should be 0 on low battery"
        );
    }

    #[test]
    fn test_ground_station_outage_modulation() {
        let mut sim = SatelliteSimulator::new(
            orbitlogs_models::SatelliteId::new("SAT-0001"),
            Scenario::Normal,
            Some(42),
        );
        let gs = GroundStationState {
            timestamp: Utc::now(),
            station_id: orbitlogs_models::StationId::new("GS-0001"),
            station_name: "Svalbard".to_string(),
            status: orbitlogs_models::StationStatus::Outage,
            antenna_health: 100.0,
            weather_state: orbitlogs_models::WeatherState::Clear,
            congestion: 0.0,
            latency: 42.0,
            bandwidth: 1000.0,
            explanation: "Outage".to_string(),
            backhaul_health: None,
            packet_loss: None,
            signal_quality: None,
            maintenance: None,
        };

        let rec = sim.next_subsystem_record(
            orbitlogs_models::Subsystem::Communications,
            orbitlogs_models::MissionId::new("test-mission"),
            Some(&gs),
        );
        assert_eq!(rec.packet_loss.unwrap(), 100.0);
        assert_eq!(rec.communication_latency.unwrap(), 3000.0);
        assert_eq!(rec.signal_strength.unwrap(), -125.0);
    }

    #[test]
    fn test_ground_station_degraded_weather_modulation() {
        let mut sim = SatelliteSimulator::new(
            orbitlogs_models::SatelliteId::new("SAT-0001"),
            Scenario::Normal,
            Some(42),
        );
        // Get normal record first to establish baseline
        let normal_rec = sim.next_subsystem_record(
            orbitlogs_models::Subsystem::Communications,
            orbitlogs_models::MissionId::new("test-mission"),
            None,
        );
        let normal_loss = normal_rec.packet_loss.unwrap();
        let normal_signal = normal_rec.signal_strength.unwrap();
        let normal_latency = normal_rec.communication_latency.unwrap();

        // Now with Degraded Weather
        let gs = GroundStationState {
            timestamp: Utc::now(),
            station_id: orbitlogs_models::StationId::new("GS-0001"),
            station_name: "Svalbard".to_string(),
            status: orbitlogs_models::StationStatus::Degraded,
            antenna_health: 100.0,
            weather_state: orbitlogs_models::WeatherState::SevereStorm,
            congestion: 0.0,
            latency: 42.0,
            bandwidth: 1000.0,
            explanation: "Severe Storm".to_string(),
            backhaul_health: None,
            packet_loss: None,
            signal_quality: None,
            maintenance: None,
        };

        // Reset sim/ticks to keep random values aligned
        let mut sim2 = SatelliteSimulator::new(
            orbitlogs_models::SatelliteId::new("SAT-0001"),
            Scenario::Normal,
            Some(42),
        );
        let rec = sim2.next_subsystem_record(
            orbitlogs_models::Subsystem::Communications,
            orbitlogs_models::MissionId::new("test-mission"),
            Some(&gs),
        );

        assert_eq!(rec.packet_loss.unwrap(), (normal_loss + 15.0).min(100.0));
        assert_eq!(
            rec.signal_strength.unwrap(),
            (normal_signal - 20.0).max(-130.0)
        );
        assert_eq!(rec.communication_latency.unwrap(), normal_latency + 150.0);
    }
}
