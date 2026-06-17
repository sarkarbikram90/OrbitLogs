use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc, TimeZone};
use std::f64::consts::PI;

const EARTH_RADIUS_KM: f64 = 6378.137;
const GM_KM3_S2: f64 = 398600.4418; // G * Earth Mass
const EARTH_ROTATION_RAD_S: f64 = 7.29211510e-5;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrbitState {
    pub satellite_id: String,
    pub timestamp: DateTime<Utc>,
    pub position_eci: [f64; 3],  // km
    pub velocity_eci: [f64; 3],  // km/s
    pub latitude: f64,           // degrees
    pub longitude: f64,          // degrees
    pub altitude: f64,           // km
    pub semi_major_axis: f64,    // km
    pub eccentricity: f64,
    pub inclination: f64,        // degrees
    pub raan: f64,               // degrees
    pub arg_of_perigee: f64,     // degrees
    pub mean_anomaly: f64,       // degrees
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PassPrediction {
    pub station: String,
    pub aos: DateTime<Utc>,
    pub los: DateTime<Utc>,
    pub duration: f64,          // minutes
    pub max_elevation: f64,     // degrees
}

#[derive(Debug, Clone)]
pub struct GroundStationInfo {
    pub name: &'static str,
    pub latitude: f64,
    pub longitude: f64,
    pub altitude_km: f64,
}

pub fn get_ground_stations() -> Vec<GroundStationInfo> {
    vec![
        GroundStationInfo { name: "Svalbard", latitude: 78.2297, longitude: 15.6469, altitude_km: 0.5 },
        GroundStationInfo { name: "Troll", latitude: -72.0119, longitude: 2.5330, altitude_km: 1.3 },
        GroundStationInfo { name: "Kiruna", latitude: 67.8558, longitude: 20.9629, altitude_km: 0.4 },
        GroundStationInfo { name: "McMurdo", latitude: -77.8463, longitude: 166.6682, altitude_km: 0.1 },
        GroundStationInfo { name: "Bangalore", latitude: 13.0300, longitude: 77.5600, altitude_km: 0.9 },
        GroundStationInfo { name: "Hyderabad", latitude: 17.3850, longitude: 78.4867, altitude_km: 0.5 },
        GroundStationInfo { name: "Delhi", latitude: 28.6139, longitude: 77.2090, altitude_km: 0.20 },
        GroundStationInfo { name: "Siliguri", latitude: 26.7271, longitude: 88.3953, altitude_km: 0.11 },
    ]
}

// Custom simple TLE parser & Keplerian analytical propagator
#[derive(Debug, Clone)]
pub struct KeplerianElements {
    pub epoch_year: i32,
    pub epoch_day: f64,
    pub inclination: f64,        // radians
    pub raan: f64,               // radians
    pub eccentricity: f64,
    pub arg_of_perigee: f64,     // radians
    pub mean_anomaly: f64,       // radians
    pub mean_motion: f64,        // rad/s (revs/day converted)
    pub semi_major_axis: f64,    // km
}

impl KeplerianElements {
    // Generate deterministic Keplerian orbital elements for a satellite index
    pub fn for_satellite(sat_id: &str) -> Self {
        let idx = sat_id.strip_prefix("SAT-")
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(1);
        
        let inc_deg = 53.0 + (idx as f64 * 3.7) % 45.0; // standard Starlink-like inclination
        let raan_deg = (idx as f64 * 36.0) % 360.0;
        let ecc = 0.0001 + (idx as f64 * 0.00005) % 0.01;
        let perigee_deg = (idx as f64 * 45.0) % 360.0;
        let mean_anom_deg = (idx as f64 * 72.0) % 360.0;
        let revs_per_day = 15.0 + (idx as f64 * 0.1) % 1.5; // ~90-96 min period

        let mean_motion_rad_s = revs_per_day * 2.0 * PI / 86400.0;
        // a = (GM / n^2)^(1/3)
        let semi_major_axis = (GM_KM3_S2 / mean_motion_rad_s.powi(2)).cbrt();

        Self {
            epoch_year: 2026,
            epoch_day: 170.0, // June 19, 2026 approx
            inclination: inc_deg * PI / 180.0,
            raan: raan_deg * PI / 180.0,
            eccentricity: ecc,
            arg_of_perigee: perigee_deg * PI / 180.0,
            mean_anomaly: mean_anom_deg * PI / 180.0,
            mean_motion: mean_motion_rad_s,
            semi_major_axis,
        }
    }

    pub fn propagate(&self, time: DateTime<Utc>) -> OrbitState {
        // Calculate seconds since epoch
        // Reference epoch: epoch_year 2026, epoch_day 170.0
        let epoch = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        let epoch = epoch + chrono::Duration::milliseconds((self.epoch_day * 86400000.0) as i64);
        
        let dt_seconds = (time - epoch).num_milliseconds() as f64 / 1000.0;

        // Mean anomaly at time t: M = M0 + n * dt
        let mut m = self.mean_anomaly + self.mean_motion * dt_seconds;
        m = m % (2.0 * PI);
        if m < 0.0 {
            m += 2.0 * PI;
        }

        // Solve Kepler's equation: E - e * sin(E) = M
        let mut e_anom = m;
        for _ in 0..10 {
            let diff = e_anom - self.eccentricity * e_anom.sin() - m;
            let deriv = 1.0 - self.eccentricity * e_anom.cos();
            e_anom -= diff / deriv;
        }

        // Position and velocity in orbital plane
        let cos_e = e_anom.cos();
        let sin_e = e_anom.sin();
        let x_plane = self.semi_major_axis * (cos_e - self.eccentricity);
        let y_plane = self.semi_major_axis * (1.0 - self.eccentricity.powi(2)).sqrt() * sin_e;

        // Derivative of E: dE/dt = n / (1 - e * cos(E))
        let de_dt = self.mean_motion / (1.0 - self.eccentricity * cos_e);
        let vx_plane = -self.semi_major_axis * sin_e * de_dt;
        let vy_plane = self.semi_major_axis * (1.0 - self.eccentricity.powi(2)).sqrt() * cos_e * de_dt;

        // Rotate to ECI Frame
        let cos_w = self.arg_of_perigee.cos();
        let sin_w = self.arg_of_perigee.sin();
        let cos_o = self.raan.cos();
        let sin_o = self.raan.sin();
        let cos_i = self.inclination.cos();
        let sin_i = self.inclination.sin();

        let r11 = cos_o * cos_w - sin_o * sin_w * cos_i;
        let r12 = -cos_o * sin_w - sin_o * cos_w * cos_i;
        let r21 = sin_o * cos_w + cos_o * sin_w * cos_i;
        let r22 = -sin_o * sin_w + cos_o * cos_w * cos_i;
        let r31 = sin_w * sin_i;
        let r32 = cos_w * sin_i;

        let x_eci = r11 * x_plane + r12 * y_plane;
        let y_eci = r21 * x_plane + r22 * y_plane;
        let z_eci = r31 * x_plane + r32 * y_plane;

        let vx_eci = r11 * vx_plane + r12 * vy_plane;
        let vy_eci = r21 * vx_plane + r22 * vy_plane;
        let vz_eci = r31 * vx_plane + r32 * vy_plane;

        // Convert ECI to Geodetic (Latitude, Longitude, Altitude)
        // Earth rotation angle
        let theta = (EARTH_ROTATION_RAD_S * dt_seconds) % (2.0 * PI);
        let mut lon = y_eci.atan2(x_eci) - theta;
        lon = lon % (2.0 * PI);
        if lon < -PI {
            lon += 2.0 * PI;
        } else if lon > PI {
            lon -= 2.0 * PI;
        }

        let r_xy = (x_eci.powi(2) + y_eci.powi(2)).sqrt();
        let lat = z_eci.atan2(r_xy);
        let alt = (x_eci.powi(2) + y_eci.powi(2) + z_eci.powi(2)).sqrt() - EARTH_RADIUS_KM;

        OrbitState {
            satellite_id: "".to_string(), // Filled by caller
            timestamp: time,
            position_eci: [x_eci, y_eci, z_eci],
            velocity_eci: [vx_eci, vy_eci, vz_eci],
            latitude: lat * 180.0 / PI,
            longitude: lon * 180.0 / PI,
            altitude: alt,
            semi_major_axis: self.semi_major_axis,
            eccentricity: self.eccentricity,
            inclination: self.inclination * 180.0 / PI,
            raan: self.raan * 180.0 / PI,
            arg_of_perigee: self.arg_of_perigee * 180.0 / PI,
            mean_anomaly: e_anom * 180.0 / PI,
        }
    }
}

// Calculate slant range, elevation from Ground Station coordinates
pub fn calculate_elevation(
    sat_lat: f64,
    sat_lon: f64,
    sat_alt_km: f64,
    gs: &GroundStationInfo
) -> f64 {
    // Basic Spherical Trigonometry for Elevation calculation
    let lat_s = sat_lat * PI / 180.0;
    let lon_s = sat_lon * PI / 180.0;
    let lat_g = gs.latitude * PI / 180.0;
    let lon_g = gs.longitude * PI / 180.0;

    let r_s = EARTH_RADIUS_KM + sat_alt_km;
    let r_g = EARTH_RADIUS_KM + gs.altitude_km;

    // Direct angular distance (theta) between GS and Satellite subpoint
    let cos_theta = lat_g.sin() * lat_s.sin() + lat_g.cos() * lat_s.cos() * (lon_s - lon_g).cos();
    let theta = cos_theta.acos();

    // Slant range
    let d2 = r_g.powi(2) + r_s.powi(2) - 2.0 * r_g * r_s * cos_theta;
    let d = d2.sqrt();

    if d < 1e-3 {
        return 90.0;
    }

    // Elevation angle
    let sin_el = (r_s * theta.sin()) / d;
    let cos_el = (r_s * cos_theta - r_g) / d;
    let el = sin_el.atan2(cos_el);
    el * 180.0 / PI
}

// Compute upcoming passes for the next 24 hours
pub fn predict_passes(
    sat_id: &str,
    start_time: DateTime<Utc>,
    duration_hours: i64
) -> Vec<PassPrediction> {
    let elements = KeplerianElements::for_satellite(sat_id);
    let stations = get_ground_stations();
    let mut passes = Vec::new();

    // Sample every 30 seconds for passes
    let total_steps = duration_hours * 120; // 120 * 30 seconds = 1 hour
    
    for gs in &stations {
        let mut in_pass = false;
        let mut aos_time = start_time;
        let mut max_el = 0.0;

        for step in 0..total_steps {
            let t = start_time + chrono::Duration::seconds(step * 30);
            let state = elements.propagate(t);
            let el = calculate_elevation(state.latitude, state.longitude, state.altitude, gs);

            if el > 5.0 {
                if !in_pass {
                    in_pass = true;
                    aos_time = t;
                    max_el = el;
                } else if el > max_el {
                    max_el = el;
                }
            } else {
                if in_pass {
                    in_pass = false;
                    let los_time = t;
                    let duration_min = (los_time - aos_time).num_seconds() as f64 / 60.0;
                    if duration_min > 1.0 {
                        passes.push(PassPrediction {
                            station: gs.name.to_string(),
                            aos: aos_time,
                            los: los_time,
                            duration: duration_min,
                            max_elevation: max_el,
                        });
                    }
                    max_el = 0.0;
                }
            }
        }
    }

    // Sort by AOS
    passes.sort_by_key(|p| p.aos);
    passes
}
