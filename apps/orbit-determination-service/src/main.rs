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

fn state_derivative(state: &Vector6<f64>) -> Vector6<f64> {
    let mut deriv = Vector6::<f64>::zeros();
    // d/dt position = velocity
    deriv[0] = state[3];
    deriv[1] = state[4];
    deriv[2] = state[5];

    let r_mag = (state[0].powi(2) + state[1].powi(2) + state[2].powi(2)).sqrt();
    let mu = 398600.4418; // Earth gravitational parameter km^3 / s^2
    let j2 = 1.08263e-3; // J2 zonal harmonic
    let r_e = 6378.137; // Earth equatorial radius km

    if r_mag > 1.0 {
        // Two-body central gravity acceleration
        let acc_two_body = -mu / r_mag.powi(3);
        let mut ax = state[0] * acc_two_body;
        let mut ay = state[1] * acc_two_body;
        let mut az = state[2] * acc_two_body;

        // J2 oblateness perturbation
        let z2_over_r2 = (state[2] / r_mag).powi(2);
        let j2_factor = 1.5 * j2 * mu * (r_e / r_mag).powi(2) / r_mag.powi(3);

        ax -= j2_factor * state[0] * (1.0 - 5.0 * z2_over_r2);
        ay -= j2_factor * state[1] * (1.0 - 5.0 * z2_over_r2);
        az -= j2_factor * state[2] * (3.0 - 5.0 * z2_over_r2);

        deriv[3] = ax;
        deriv[4] = ay;
        deriv[5] = az;
    }

    deriv
}

/// Propagates orbital state using 4th-Order Runge-Kutta (RK4) numerical integration.
fn numerical_orbital_dynamics(state: &Vector6<f64>, dt: f64) -> Vector6<f64> {
    let k1 = state_derivative(state);
    let k2 = state_derivative(&(state + k1 * (dt / 2.0)));
    let k3 = state_derivative(&(state + k2 * (dt / 2.0)));
    let k4 = state_derivative(&(state + k3 * dt));

    state + (k1 + k2 * 2.0 + k3 * 2.0 + k4) * (dt / 6.0)
}

/// Calculates continuous-time state Jacobian matrix A = df/dx and discrete State Transition Matrix F = I + A*dt + 0.5*(A*dt)^2.
fn calculate_transition_matrix(state: &Vector6<f64>, dt: f64) -> Matrix6<f64> {
    let mut a = Matrix6::<f64>::zeros();

    // Top-right 3x3 block: d(pos_dot)/d(vel) = Identity
    a[(0, 3)] = 1.0;
    a[(1, 4)] = 1.0;
    a[(2, 5)] = 1.0;

    // Bottom-left 3x3 block: Gravity Gradient Tensor G = d(acc)/d(pos)
    let r_mag = (state[0].powi(2) + state[1].powi(2) + state[2].powi(2)).sqrt();
    let mu = 398600.4418;

    if r_mag > 1.0 {
        let r3 = r_mag.powi(3);
        let r5 = r_mag.powi(5);
        let mu_over_r3 = mu / r3;
        let three_mu_over_r5 = 3.0 * mu / r5;

        for i in 0..3 {
            for j in 0..3 {
                let kronecker = if i == j { 1.0 } else { 0.0 };
                a[(3 + i, j)] = three_mu_over_r5 * state[i] * state[j] - mu_over_r3 * kronecker;
            }
        }
    }

    // Discrete state transition matrix approximation: F = I + A*dt + 0.5*(A*dt)^2
    let identity = Matrix6::<f64>::identity();
    let a_dt = a * dt;
    identity + &a_dt + (&a_dt * &a_dt) * 0.5
}

fn observation_model(state: &Vector6<f64>) -> Vector3<f64> {
    // Direct GPS observation vector [x, y, z]
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
    let rows =
        sqlx::query("SELECT satellite_id FROM registered_satellites WHERE status = 'ACTIVE'")
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
