#![allow(clippy::manual_clamp, clippy::bool_assert_comparison)]

use chrono::Utc;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use std::time::SystemTime;
use uuid::Uuid;

use crate::scenario::Scenario;

// TelemetryRecord is imported from the shared domain crate
pub use spaceops_models::TelemetryRecord;

pub use spaceops_models::GroundStationHealth as GroundStationState;

pub struct SatelliteSimulator {
    pub id: String,
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
    pub fn new(id: String, scenario: Scenario, seed: Option<u64>) -> Self {
        let base_seed = seed.unwrap_or_else(|| {
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
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
        self.next_subsystem_record("all", "unknown", None)
    }

    pub fn next_subsystem_record(
        &mut self,
        subsystem: &str,
        mission_id: &str,
        gs_state: Option<&GroundStationState>,
    ) -> TelemetryRecord {
        self.ticks += 1;

        // Base fluctuations
        let orbital_phase = (self.ticks as f64 * 0.05).sin(); // Simulated day/night cycle
        let volt_noise = self.rng.gen_range(-0.1..0.1);
        let temp_noise = self.rng.gen_range(-0.5..0.5);
        let alt_noise = self.rng.gen_range(-0.2..0.2);

        // --- Core Subsystem State Logic ---

        // 1. Power Subsystem Base Variables
        let mut solar_panel = if orbital_phase > 0.0 {
            120.0 * orbital_phase + self.rng.gen_range(-5.0..5.0)
        } else {
            0.0 // In Shadow
        };

        // Side effect: Solar Panel Degradation due to high temperature (>100°C)
        if self.temperature > 100.0 {
            solar_panel *= 0.5;
        }

        let mut bat_current = if solar_panel > 10.0 {
            // Charging battery
            (solar_panel / 12.0) - 3.0 + self.rng.gen_range(-0.5..0.5)
        } else {
            // Discharging battery (system load)
            -2.5 + self.rng.gen_range(-0.2..0.2)
        };

        // 2. Thermal Subsystem Base updates
        match self.scenario {
            Scenario::Normal => {
                self.temperature = 25.0 + 5.0 * orbital_phase + temp_noise;
                self.battery_voltage = 13.6 + 0.6 * orbital_phase + volt_noise;
                self.fuel_remaining -= 0.0001; // Normal micro-usage

                if self.battery_voltage < 11.5 {
                    self.battery_voltage = 11.5;
                }
            }
            Scenario::ThermalSpike => {
                // Rapid continuous heat spike
                self.temperature += 2.5 + self.rng.gen_range(0.0..1.5);
                if self.temperature > 130.0 {
                    self.temperature = 130.0 + temp_noise; // Max thermal threshold
                }
                self.battery_voltage = 13.0 + 0.4 * orbital_phase + volt_noise;
                self.fuel_remaining -= 0.0001;
            }
            Scenario::BatteryDegradation => {
                // Rapid loss of voltage and charging failure
                self.battery_voltage -= 0.15 + self.rng.gen_range(0.0..0.05);
                if self.battery_voltage < 8.0 {
                    self.battery_voltage = 8.0;
                }
                bat_current = -5.0 + self.rng.gen_range(-1.0..1.0); // Severe discharge
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
                // Severe drop in fuel
                self.fuel_remaining -= 1.2 + self.rng.gen_range(0.0..0.5);
                if self.fuel_remaining < 0.0 {
                    self.fuel_remaining = 0.0;
                }
                self.temperature = 25.0 + 5.0 * orbital_phase + temp_noise;
                self.battery_voltage = 13.6 + 0.6 * orbital_phase + volt_noise;
            }
        }

        // 3. Propulsion Subsystem updates
        // Side effect: Orbit decay if fuel is completely empty
        if self.fuel_remaining <= 0.0 {
            self.altitude -= 0.5 + self.rng.gen_range(0.0..0.1);
            if self.altitude < 100.0 {
                self.altitude = 100.0; // Re-entry bound
            }
            self.velocity = 7.0 - (self.altitude - 100.0) * 0.005 + self.rng.gen_range(-0.01..0.01);
        } else {
            self.altitude += alt_noise;
            self.velocity = 7.67 - (self.altitude - 520.0) * 0.01 + self.rng.gen_range(-0.01..0.01);
        }

        // 4. ADCS Subsystem (Pitch, Roll, Yaw) - degrees
        let mut pitch = (self.ticks as f32 * 0.02).cos() * 5.0 + self.rng.gen_range(-0.5..0.5);
        let mut roll = (self.ticks as f32 * 0.03).sin() * 3.0 + self.rng.gen_range(-0.3..0.3);
        let mut yaw = (self.ticks as f32 * 0.01 + 1.2).sin() * 4.0 + self.rng.gen_range(-0.4..0.4);

        // Side effect: Chaotic spinning from gas propulsion leakage
        if self.scenario == Scenario::FuelLeakage && self.fuel_remaining > 0.0 {
            pitch += self.rng.gen_range(-20.0..20.0);
            roll += self.rng.gen_range(-20.0..20.0);
            yaw += self.rng.gen_range(-20.0..20.0);
        }

        // Side effect: ADCS drifting due to low battery voltage (<11.0V)
        if self.battery_voltage < 11.0 {
            pitch += self.rng.gen_range(-15.0..15.0);
            roll += self.rng.gen_range(-15.0..15.0);
            yaw += self.rng.gen_range(-15.0..15.0);
        }

        // 5. Communications Subsystem
        let mut latency = 45.0 + self.rng.gen_range(-5.0..5.0);
        let mut loss = self.rng.gen_range(0.0..0.5);
        let mut signal = -70.0 + self.rng.gen_range(-2.0..2.0);

        if self.scenario == Scenario::CommunicationDegradation {
            latency += (self.ticks as f32 * 50.0).min(3000.0);
            loss = (0.5 + (self.ticks as f32 * 1.5)).min(100.0);
            signal = (-70.0 - (self.ticks as f32 * 1.2)).max(-125.0);
        }

        // Side effect: Comms degraded due to extreme CPU usage (>85%) or extreme temperature (>90°C)
        let thermal_stress = self.temperature > 90.0;
        if thermal_stress || self.battery_voltage < 10.5 {
            latency += 500.0;
            loss += 10.0;
            if loss > 100.0 {
                loss = 100.0;
            }
            signal -= 15.0;
            if signal < -130.0 {
                signal = -130.0;
            }
        }

        // Ground station modulation
        if let Some(gs) = gs_state {
            if gs.status == "OUTAGE" {
                latency = 3000.0;
                loss = 100.0;
                signal = -125.0;
            } else if gs.status == "DEGRADED" {
                if gs.weather_state == "SevereStorm" || gs.weather_state == "HeavySnow" {
                    loss += 15.0;
                    signal -= 20.0;
                    latency += 150.0;
                } else {
                    loss += 10.0;
                    signal -= 10.0;
                    latency += 100.0;
                }
            }
        }

        // Ensure values are physically clamped/within bounds
        if loss > 100.0 {
            loss = 100.0;
        }
        if loss < 0.0 {
            loss = 0.0;
        }
        if signal < -130.0 {
            signal = -130.0;
        }

        // 6. Flight Computer Subsystem (Resource usage)
        // Side effect: CPU throttling/high-load during ThermalSpike Scenario (temperature >80.0°C)
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

        // 7. Payload Subsystem
        // Side effect: Payload shuts down automatically on critical battery (<11.5V)
        let payload_active = self.battery_voltage > 11.5;
        let payload_power_draw: f64 = if payload_active {
            15.0 + self.rng.gen_range(-1.0..1.0)
        } else {
            0.0
        };

        // Construct Telemetry Record with fields matching the requested subsystem
        let mut record = TelemetryRecord {
            satellite_id: self.id.clone(),
            subsystem_id: subsystem.to_string(),
            mission_id: mission_id.to_string(),
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

        match subsystem.to_lowercase().as_str() {
            "power" => {
                record.battery_voltage = Some((self.battery_voltage * 100.0).round() / 100.0);
                record.battery_current = Some((bat_current * 100.0).round() / 100.0);
                record.solar_panel_output = Some((solar_panel * 100.0).round() / 100.0);
            }
            "thermal" => {
                record.temperature = Some((self.temperature * 100.0).round() / 100.0);
            }
            "communications" => {
                record.communication_latency = Some((latency * 10.0).round() / 10.0);
                record.packet_loss = Some((loss * 10.0).round() / 10.0);
                record.signal_strength = Some((signal * 10.0).round() / 10.0);
            }
            "propulsion" => {
                record.velocity = Some((self.velocity * 10000.0).round() / 10000.0);
                record.altitude = Some((self.altitude * 100.0).round() / 100.0);
                record.fuel_remaining = Some((self.fuel_remaining * 10.0).round() / 10.0);
            }
            "adcs" => {
                record.orientation_pitch = Some((pitch * 100.0).round() / 100.0);
                record.orientation_roll = Some((roll * 100.0).round() / 100.0);
                record.orientation_yaw = Some((yaw * 100.0).round() / 100.0);
            }
            "flight_computer" | "flightcomputer" | "flight computer" => {
                record.cpu_usage = Some((cpu * 10.0).round() / 10.0);
                record.memory_usage = Some((mem * 10.0).round() / 10.0);
            }
            "payload" => {
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
        let sim = SatelliteSimulator::new("SAT-9999".to_string(), Scenario::Normal, Some(12345));
        assert_eq!(sim.id, "SAT-9999");
        assert_eq!(sim.scenario, Scenario::Normal);
        assert_eq!(sim.ticks, 0);
    }

    #[test]
    fn test_deterministic_generation() {
        let mut sim1 = SatelliteSimulator::new("SAT-0001".to_string(), Scenario::Normal, Some(42));
        let mut sim2 = SatelliteSimulator::new("SAT-0001".to_string(), Scenario::Normal, Some(42));

        let rec1 = sim1.next_record();
        let rec2 = sim2.next_record();

        assert_eq!(rec1.battery_voltage, rec2.battery_voltage);
        assert_eq!(rec1.solar_panel_output, rec2.solar_panel_output);
        assert_eq!(rec1.temperature, rec2.temperature);
    }

    #[test]
    fn test_thermal_spike_scenario() {
        let mut sim =
            SatelliteSimulator::new("SAT-0002".to_string(), Scenario::ThermalSpike, Some(101));
        let base_rec = sim.next_subsystem_record("Thermal", "test-mission", None);

        let mut prev_temp = base_rec.temperature.unwrap();
        for _ in 0..5 {
            let rec = sim.next_subsystem_record("Thermal", "test-mission", None);
            let temp = rec.temperature.unwrap();
            assert!(temp > prev_temp || temp >= 129.0);
            prev_temp = temp;
        }
    }

    #[test]
    fn test_fuel_leakage_scenario() {
        let mut sim =
            SatelliteSimulator::new("SAT-0003".to_string(), Scenario::FuelLeakage, Some(202));
        let rec = sim.next_subsystem_record("Propulsion", "test-mission", None);
        assert!(rec.fuel_remaining.unwrap() < 95.0);
    }

    #[test]
    fn test_thermal_side_effects() {
        let mut sim =
            SatelliteSimulator::new("SAT-0004".to_string(), Scenario::ThermalSpike, Some(303));

        // Advance ticks to trigger high temperature
        for _ in 0..30 {
            let _ = sim.next_subsystem_record("Thermal", "test-mission", None);
        }

        // Verify side effect: CPU Throttling
        let rec_cpu = sim.next_subsystem_record("Flight Computer", "test-mission", None);
        assert!(
            rec_cpu.cpu_usage.unwrap() > 90.0,
            "CPU should be throttled (>90%) due to thermal spike"
        );
    }

    #[test]
    fn test_battery_depletion_side_effects() {
        let mut sim = SatelliteSimulator::new(
            "SAT-0005".to_string(),
            Scenario::BatteryDegradation,
            Some(404),
        );

        // Advance ticks to drain battery
        for _ in 0..20 {
            let _ = sim.next_subsystem_record("Power", "test-mission", None);
        }

        // Verify side effect: Payload shutdown
        let rec_payload = sim.next_subsystem_record("Payload", "test-mission", None);
        assert_eq!(
            rec_payload.payload_active.unwrap(),
            false,
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
        let mut sim = SatelliteSimulator::new("SAT-0001".to_string(), Scenario::Normal, Some(42));
        let gs = GroundStationState {
            timestamp: Utc::now(),
            station_id: "GS-0001".to_string(),
            station_name: "Svalbard".to_string(),
            status: "OUTAGE".to_string(),
            antenna_health: 100.0,
            weather_state: "Clear".to_string(),
            congestion: 0.0,
            latency: 42.0,
            bandwidth: 1000.0,
            explanation: "Outage".to_string(),
        };

        let rec = sim.next_subsystem_record("Communications", "test-mission", Some(&gs));
        assert_eq!(rec.packet_loss.unwrap(), 100.0);
        assert_eq!(rec.communication_latency.unwrap(), 3000.0);
        assert_eq!(rec.signal_strength.unwrap(), -125.0);
    }

    #[test]
    fn test_ground_station_degraded_weather_modulation() {
        let mut sim = SatelliteSimulator::new("SAT-0001".to_string(), Scenario::Normal, Some(42));
        // Get normal record first to establish baseline
        let normal_rec = sim.next_subsystem_record("Communications", "test-mission", None);
        let normal_loss = normal_rec.packet_loss.unwrap();
        let normal_signal = normal_rec.signal_strength.unwrap();
        let normal_latency = normal_rec.communication_latency.unwrap();

        // Now with Degraded Weather
        let gs = GroundStationState {
            timestamp: Utc::now(),
            station_id: "GS-0001".to_string(),
            station_name: "Svalbard".to_string(),
            status: "DEGRADED".to_string(),
            antenna_health: 100.0,
            weather_state: "SevereStorm".to_string(),
            congestion: 0.0,
            latency: 42.0,
            bandwidth: 1000.0,
            explanation: "Severe Storm".to_string(),
        };

        // Reset sim/ticks to keep random values aligned
        let mut sim2 = SatelliteSimulator::new("SAT-0001".to_string(), Scenario::Normal, Some(42));
        let rec = sim2.next_subsystem_record("Communications", "test-mission", Some(&gs));

        assert_eq!(rec.packet_loss.unwrap(), (normal_loss + 15.0).min(100.0));
        assert_eq!(
            rec.signal_strength.unwrap(),
            (normal_signal - 20.0).max(-130.0)
        );
        assert_eq!(rec.communication_latency.unwrap(), normal_latency + 150.0);
    }
}
