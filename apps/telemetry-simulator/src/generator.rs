use chrono::Utc;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use std::time::SystemTime;
use uuid::Uuid;

use crate::scenario::Scenario;

// TelemetryRecord is imported from the shared domain crate
pub use spaceops_models::TelemetryRecord;

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

    pub fn next_record(&mut self) -> TelemetryRecord {
        self.ticks += 1;

        // Add random walk or cyclic components depending on ticks
        let orbital_phase = (self.ticks as f64 * 0.05).sin(); // Simulated day/night cycle

        // Base fluctuations
        let volt_noise = self.rng.gen_range(-0.1..0.1);
        let temp_noise = self.rng.gen_range(-0.5..0.5);
        let alt_noise = self.rng.gen_range(-0.2..0.2);

        // Base normal variables
        let mut solar_panel = if orbital_phase > 0.0 {
            120.0 * orbital_phase + self.rng.gen_range(-5.0..5.0)
        } else {
            0.0 // In Shadow
        };

        let mut bat_current = if solar_panel > 10.0 {
            // Charging battery
            (solar_panel / 12.0) - 3.0 + self.rng.gen_range(-0.5..0.5)
        } else {
            // Discharging battery (system load)
            -2.5 + self.rng.gen_range(-0.2..0.2)
        };

        // Scenario state updates
        match self.scenario {
            Scenario::Normal => {
                // Temperature stays steady within bounds
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
                solar_panel *= 0.8; // Thermal stress degrades solar efficiency
            }
            Scenario::BatteryDegradation => {
                // Rapid loss of voltage and charging failure
                self.battery_voltage -= 0.15 + self.rng.gen_range(0.0..0.05);
                if self.battery_voltage < 8.0 {
                    self.battery_voltage = 8.0;
                }
                bat_current = -5.0 + self.rng.gen_range(-1.0..1.0); // Severe discharge
                solar_panel *= 0.5; // Battery issues could affect electrical distribution
            }
            Scenario::CommunicationDegradation => {
                // Affects communication parameters directly below
            }
            Scenario::FuelLeakage => {
                // Severe drop in fuel
                self.fuel_remaining -= 1.2 + self.rng.gen_range(0.0..0.5);
                if self.fuel_remaining < 0.0 {
                    self.fuel_remaining = 0.0;
                }
                // Attitude becomes erratic due to escaping gas pushing satellite
            }
        }

        // Orbital variables with minor deviations
        self.altitude += alt_noise;
        self.velocity = 7.67 - (self.altitude - 520.0) * 0.01 + self.rng.gen_range(-0.01..0.01);

        // Attitude (Pitch, Roll, Yaw) - degrees
        let mut pitch = (self.ticks as f32 * 0.02).cos() * 5.0 + self.rng.gen_range(-0.5..0.5);
        let mut roll = (self.ticks as f32 * 0.03).sin() * 3.0 + self.rng.gen_range(-0.3..0.3);
        let mut yaw = (self.ticks as f32 * 0.01 + 1.2).sin() * 4.0 + self.rng.gen_range(-0.4..0.4);

        if self.scenario == Scenario::FuelLeakage && self.fuel_remaining > 0.0 {
            // Chaotic spinning from thrust asymmetry
            pitch += self.rng.gen_range(-20.0..20.0);
            roll += self.rng.gen_range(-20.0..20.0);
            yaw += self.rng.gen_range(-20.0..20.0);
        }

        // Communication Metrics
        let mut latency = 45.0 + self.rng.gen_range(-5.0..5.0);
        let mut loss = self.rng.gen_range(0.0..0.5);
        let mut signal = -70.0 + self.rng.gen_range(-2.0..2.0);

        if self.scenario == Scenario::CommunicationDegradation {
            // Extreme noise, fading signal
            latency += (self.ticks as f32 * 50.0).min(3000.0);
            loss = (0.5 + (self.ticks as f32 * 1.5)).min(100.0);
            signal = (-70.0 - (self.ticks as f32 * 1.2)).max(-125.0);
        }

        // Resource usage
        let cpu = match self.scenario {
            Scenario::ThermalSpike => 90.0f32 + self.rng.gen_range(-5.0f32..5.0f32), // High CPU causing or caused by thermal
            _ => 15.0f32 + 10.0f32 * orbital_phase as f32 + self.rng.gen_range(-2.0f32..2.0f32),
        }
        .clamp(1.0f32, 100.0f32);

        let mem = match self.scenario {
            Scenario::ThermalSpike => 85.0f32 + self.rng.gen_range(-2.0f32..2.0f32),
            _ => 22.0f32 + self.rng.gen_range(-1.0f32..1.0f32),
        }
        .clamp(1.0f32, 100.0f32);

        TelemetryRecord {
            satellite_id: self.id.clone(),
            timestamp: Utc::now(),
            battery_voltage: (self.battery_voltage * 100.0).round() / 100.0,
            battery_current: (bat_current * 100.0).round() / 100.0,
            solar_panel_output: (solar_panel * 100.0).round() / 100.0,
            cpu_usage: (cpu * 10.0).round() / 10.0,
            memory_usage: (mem * 10.0).round() / 10.0,
            temperature: (self.temperature * 100.0).round() / 100.0,
            orientation_pitch: (pitch * 100.0).round() / 100.0,
            orientation_roll: (roll * 100.0).round() / 100.0,
            orientation_yaw: (yaw * 100.0).round() / 100.0,
            velocity: (self.velocity * 10000.0).round() / 10000.0,
            altitude: (self.altitude * 100.0).round() / 100.0,
            fuel_remaining: (self.fuel_remaining * 10.0).round() / 10.0,
            communication_latency: (latency * 10.0).round() / 10.0,
            packet_loss: (loss * 10.0).round() / 10.0,
            signal_strength: (signal * 10.0).round() / 10.0,
            event_id: Uuid::new_v4(),
        }
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
        let base_rec = sim.next_record();

        let mut prev_temp = base_rec.temperature;
        for _ in 0..5 {
            let rec = sim.next_record();
            assert!(rec.temperature > prev_temp || rec.temperature >= 129.0);
            prev_temp = rec.temperature;
        }
    }

    #[test]
    fn test_fuel_leakage_scenario() {
        let mut sim =
            SatelliteSimulator::new("SAT-0003".to_string(), Scenario::FuelLeakage, Some(202));
        let rec = sim.next_record();
        assert!(rec.fuel_remaining < 95.0);
    }
}
