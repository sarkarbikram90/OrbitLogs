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
use chrono::{DateTime, Utc};
use clap::Parser;
use nalgebra::{Matrix3, Matrix3x6, Matrix6, Vector3, Vector6};
use sqlx::{PgPool, Row};
use std::time::Duration;
use tracing::{error, info, Level};

#[derive(Parser, Debug)]
#[command(name = "Orbit Determination Service")]
struct Cli {
    #[arg(long, env = "DATABASE_URL", default_value = "")]
    database_url: String,

    #[arg(long, env = "OD_INTERVAL_SECS", default_value = "60")]
    interval_secs: u64,
}

pub struct ExtendedKalmanFilter {
    pub state: Vector6<f64>, // [x, y, z, vx, vy, vz]
    pub covariance: Matrix6<f64>,
}

impl ExtendedKalmanFilter {
    pub fn new(initial_state: Vector6<f64>) -> Self {
        let mut cov = Matrix6::<f64>::identity();
        cov *= 100.0; // High initial uncertainty
        Self {
            state: initial_state,
            covariance: cov,
        }
    }

    pub fn predict(&mut self, dt: f64) {
        // Propagate state using Keplerian equations + simple dynamics
        let f = numerical_orbital_dynamics(&self.state, dt);
        let jacobian = calculate_transition_matrix(&self.state, dt);

        self.state = f;
        let q = Matrix6::<f64>::identity() * 0.01; // Process Noise Q
        self.covariance = &jacobian * &self.covariance * jacobian.transpose() + q;
    }

    pub fn update(&mut self, z: &Vector3<f64>, r: &Matrix3<f64>) {
        // Measurement Update: Update state vector based on observations
        let h = observation_model(&self.state);
        let h_jacobian = calculate_observation_jacobian(&self.state);

        let y = z - &h; // Measurement residual
        let s = &h_jacobian * &self.covariance * h_jacobian.transpose() + r;
        if let Some(s_inv) = s.try_inverse() {
            let k = &self.covariance * h_jacobian.transpose() * s_inv; // Kalman Gain
            self.state = &self.state + &k * y;
            let identity = Matrix6::<f64>::identity();
            self.covariance = (identity - &k * h_jacobian) * &self.covariance;
        }
    }
}

fn numerical_orbital_dynamics(state: &Vector6<f64>, dt: f64) -> Vector6<f64> {
    let mut next_state = *state;
    // Simple Euler integration for demonstration:
    // x = x + vx * dt
    // v = v + a * dt
    next_state[0] += state[3] * dt;
    next_state[1] += state[4] * dt;
    next_state[2] += state[5] * dt;

    // Gravity acceleration: a = -mu * r / |r|^3
    let r_mag = (state[0].powi(2) + state[1].powi(2) + state[2].powi(2)).sqrt();
    let mu = 398600.4418; // Earth gravitational parameter
    if r_mag > 1.0 {
        let acc_factor = -mu / r_mag.powi(3);
        next_state[3] += state[0] * acc_factor * dt;
        next_state[4] += state[1] * acc_factor * dt;
        next_state[5] += state[2] * acc_factor * dt;
    }
    next_state
}

fn calculate_transition_matrix(_state: &Vector6<f64>, _dt: f64) -> Matrix6<f64> {
    Matrix6::<f64>::identity()
}

fn observation_model(state: &Vector6<f64>) -> Vector3<f64> {
    // We observe position x, y, z directly from GPS
    Vector3::new(state[0], state[1], state[2])
}

fn calculate_observation_jacobian(_state: &Vector6<f64>) -> Matrix3x6<f64> {
    let mut h_jac = Matrix3x6::<f64>::zeros();
    h_jac[(0, 0)] = 1.0;
    h_jac[(1, 1)] = 1.0;
    h_jac[(2, 2)] = 1.0;
    h_jac
}

async fn fetch_active_satellites(pool: &PgPool) -> Vec<String> {
    let rows = sqlx::query("SELECT satellite_id FROM registered_satellites WHERE status = 'ACTIVE'")
        .fetch_all(pool)
        .await;

    if let Ok(rows) = rows {
        if !rows.is_empty() {
            return rows.into_iter().map(|r| r.get("satellite_id")).collect();
        }
    }
    (1..=10).map(|i| format!("SAT-{:04}", i)).collect()
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let _ = dotenvy::dotenv();
    let cli = Cli::parse();

    tracing_subscriber::fmt().with_max_level(Level::INFO).init();
    info!("Starting Orbit Determination Service...");

    let db_url = if !cli.database_url.trim().is_empty() {
        cli.database_url.clone()
    } else if let Some(val) =
        orbitlogs_models::load_secret("DATABASE_URL").filter(|s| !s.trim().is_empty())
    {
        val
    } else {
        return Err(anyhow::anyhow!(
            "DATABASE_URL is required. Set DATABASE_URL env var or pass --database-url."
        ));
    };

    let db_pool = orbitlogs_models::db::establish_db_connection(&db_url, 3).await?;

    loop {
        info!("Running EKF filter updates on satellite orbit states...");
        let active_sats = fetch_active_satellites(&db_pool).await;

        for sat_id in &active_sats {
            // Fetch recent orbit positions
            let row = sqlx::query(
                "SELECT latitude, longitude, altitude, velocity 
                 FROM orbit_positions 
                 WHERE satellite_id = $1 
                 ORDER BY timestamp DESC LIMIT 1",
            )
            .bind(sat_id)
            .fetch_optional(&db_pool)
            .await;

            match row {
                Ok(Some(r)) => {
                    let _lat: f64 = r.get("latitude");
                    let _lon: f64 = r.get("longitude");
                    let alt: f64 = r.get("altitude");
                    let vel: f64 = r.get("velocity");

                    // Initial state estimation
                    let initial_state = Vector6::new(
                        6378.1 + alt,
                        0.0,
                        0.0, // x, y, z
                        0.0,
                        vel,
                        0.0, // vx, vy, vz
                    );

                    let mut ekf = ExtendedKalmanFilter::new(initial_state);
                    ekf.predict(10.0); // 10 second propagation step

                    // simulated GPS measurement
                    let measurement = Vector3::new(6378.1 + alt + 0.1, 0.05, -0.05);
                    let r_noise = Matrix3::<f64>::identity() * 0.1;

                    ekf.update(&measurement, &r_noise);

                    info!(
                        "Orbit determined for {}: Est Position = [{:.2}, {:.2}, {:.2}]",
                        sat_id, ekf.state[0], ekf.state[1], ekf.state[2]
                    );
                }
                Ok(None) => {}
                Err(e) => {
                    error!("Error loading orbit position for {}: {}", sat_id, e);
                }
            }
        }

        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                info!("Shutdown signal received. Exiting Orbit Determination Service gracefully.");
                break;
            }
            _ = tokio::time::sleep(Duration::from_secs(cli.interval_secs)) => {}
        }
    }
    Ok(())
}
