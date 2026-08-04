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
use ort::session::Session;
use std::sync::Mutex;
use tracing::{info, warn};

#[allow(dead_code)]
pub struct AnomalyDetector {
    session: Option<Mutex<Session>>,
}

#[allow(dead_code)]
impl AnomalyDetector {
    pub fn new(model_path: &str) -> Self {
        // Since we are running in multiple environments, let's gracefully load the ONNX model
        // and fall back to a mathematical/heuristic reconstruction loss model if the file does not exist.
        info!("Initializing ML Anomaly Detector: {}", model_path);

        let session = (|| -> Result<Session, ort::Error> {
            let builder = Session::builder()?;
            let mut builder = builder
                .with_optimization_level(ort::session::builder::GraphOptimizationLevel::Level3)?;
            let session = builder.commit_from_file(model_path)?;
            Ok(session)
        })()
        .map(Mutex::new)
        .ok();

        if session.is_some() {
            info!("Successfully loaded ONNX model for real-time anomaly detection.");
        } else {
            warn!("ONNX model file not found or failed to load. Fallback heuristic model initialized.");
        }

        Self { session }
    }

    pub fn predict(&self, features: &[f32; 10]) -> Result<bool, ort::Error> {
        if let Some(ref session_mutex) = self.session {
            let mut session = session_mutex.lock().unwrap();
            let input_tensor = ndarray::Array2::from_shape_vec((1, 10), features.to_vec()).unwrap();
            let input_value = ort::value::Tensor::from_array(input_tensor)?;

            let outputs = session.run(ort::inputs![input_value])?;
            let output_tensor = outputs[0].try_extract_array::<f32>()?;

            // Calculate Reconstruction Loss (MSE)
            let loss: f32 = features
                .iter()
                .zip(output_tensor.iter())
                .map(|(&orig, &recon)| (orig - recon).powi(2))
                .sum::<f32>()
                / 10.0;

            Ok(loss > 0.05) // Threshold tuned to capture anomalous variance
        } else {
            // Multi-variate Statistical Model: Mahalanobis Distance & Z-Score Normalized Sensor Deviations
            // Features vector indices:
            // 0: battery_voltage (mean 13.8V, std 0.5V)
            // 1: battery_current (mean 1.5A, std 0.3A)
            // 2: temperature (mean 35.0C, std 5.0C)
            // 3: pitch (mean 0.0, std 2.0)
            // 4: roll (mean 0.0, std 2.0)
            // 5: yaw (mean 0.0, std 2.0)
            // 6: cpu_usage (mean 30.0%, std 10.0%)
            // 7: memory_usage (mean 45.0%, std 10.0%)
            // 8: packet_loss (mean 0.1%, std 0.5%)
            // 9: signal_strength (mean -70.0 dBm, std 5.0 dBm)
            let nominal_means: [f32; 10] = [13.8, 1.5, 35.0, 0.0, 0.0, 0.0, 30.0, 45.0, 0.1, -70.0];
            let nominal_stds: [f32; 10] = [0.5, 0.3, 5.0, 2.0, 2.0, 2.0, 10.0, 10.0, 0.5, 5.0];

            let mut squared_z_sum = 0.0;
            let mut max_z = 0.0f32;

            for i in 0..10 {
                let z = ((features[i] - nominal_means[i]) / nominal_stds[i]).abs();
                squared_z_sum += z.powi(2);
                if z > max_z {
                    max_z = z;
                }
            }

            let mahalanobis_distance = (squared_z_sum / 10.0).sqrt();

            // Flag as anomaly if overall normalized distance > 1.8 OR any single sensor exceeds 3.5 sigma
            Ok(mahalanobis_distance > 1.8 || max_z > 3.5)
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
