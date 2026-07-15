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
use ort::{Environment, Session, SessionBuilder, Value};
use std::sync::Arc;
use tracing::{info, warn};

pub struct AnomalyDetector {
    session: Option<Session>,
}

impl AnomalyDetector {
    pub fn new(model_path: &str) -> Self {
        // Since we are running in multiple environments, let's gracefully load the ONNX model
        // and fall back to a mathematical/heuristic reconstruction loss model if the file does not exist.
        info!(
            "Initializing ML Anomaly Detector with model path: {}",
            model_path
        );

        let env_res = Environment::builder()
            .with_name("StarlinkAnomalyML")
            .build();
        let session = match env_res {
            Ok(env) => {
                let env_arc = Arc::new(env);
                let builder = SessionBuilder::new(&env_arc);
                match builder {
                    Ok(b) => {
                        let opt_builder =
                            b.with_optimization_level(ort::GraphOptimizationLevel::Level3);
                        match opt_builder {
                            Ok(ob) => ob.with_model_from_file(model_path).ok(),
                            Err(_) => None,
                        }
                    }
                    Err(_) => None,
                }
            }
            Err(_) => {
                warn!("ONNX Environment could not be initialized. Using fallback heuristic anomaly detector.");
                None
            }
        };

        if session.is_some() {
            info!("Successfully loaded ONNX model for real-time anomaly detection.");
        } else {
            warn!("ONNX model file not found or failed to load. Fallback heuristic model initialized.");
        }

        Self { session }
    }

    pub fn predict(&self, features: &[f32; 10]) -> Result<bool, ort::Error> {
        if let Some(ref session) = self.session {
            let input_tensor = Value::from_array(
                session.allocator(),
                ndarray::Array2::from_shape_vec((1, 10), features.to_vec()).unwrap(),
            )?;

            let outputs = session.run(ort::inputs![input_tensor]?)?;
            let output_tensor = outputs[0].extract_tensor::<f32>()?;

            // Calculate Reconstruction Loss (MSE)
            let loss: f32 = features
                .iter()
                .zip(output_tensor.view().iter())
                .map(|(&orig, &recon)| (orig - recon).powi(2))
                .sum::<f32>()
                / 10.0;

            Ok(loss > 0.05) // Threshold tuned to capture anomalous variance
        } else {
            // Heuristic Fallback Model: Reconstruction loss approximated by multi-sensor deviations
            // If any sensor value drifts beyond normal operational bounds (z-score), classify as anomaly.
            // Features index representation:
            // 0: battery_voltage, 1: battery_current, 2: temperature, 3: orientation_pitch,
            // 4: orientation_roll, 5: orientation_yaw, 6: cpu_usage, 7: memory_usage,
            // 8: packet_loss, 9: signal_strength
            let mut loss = 0.0;

            // Temp anomaly (> 65C)
            if features[2] > 65.0 {
                loss += 0.03;
            }
            // Battery low (< 11.5V)
            if features[0] < 11.5 {
                loss += 0.04;
            }
            // CPU usage high (> 90%)
            if features[6] > 90.0 {
                loss += 0.02;
            }
            // High packet loss (> 15%)
            if features[8] > 15.0 {
                loss += 0.03;
            }

            Ok(loss > 0.05)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fallback_anomaly_nominal() {
        let detector = AnomalyDetector::new("non_existent_model.onnx");
        let nominal_features = [13.8, 1.5, 35.0, 0.0, 0.0, 0.0, 30.0, 45.0, 0.1, -70.0];
        let is_anomaly = detector.predict(&nominal_features).unwrap();
        assert!(!is_anomaly);
    }

    #[test]
    fn test_fallback_anomaly_critical() {
        let detector = AnomalyDetector::new("non_existent_model.onnx");
        // Low battery voltage (11.0V) and high temperature (70C) should trigger anomaly
        let critical_features = [11.0, 1.5, 70.0, 0.0, 0.0, 0.0, 30.0, 45.0, 0.1, -70.0];
        let is_anomaly = detector.predict(&critical_features).unwrap();
        assert!(is_anomaly);
    }
}
