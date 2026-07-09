use serde::{Deserialize, Serialize};
use std::time::Duration;

#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct IssPosition {
    pub latitude: f64,
    pub longitude: f64,
    pub altitude: f64,
    pub velocity: f64,
}

pub struct OrbitAdapters;

impl OrbitAdapters {
    /// ISS Adapter: Fetches live ISS position from public API, falls back to simulation if offline
    #[allow(dead_code)]
    pub async fn fetch_iss_position() -> IssPosition {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(2))
            .build();

        if let Ok(c) = client {
            if let Ok(resp) = c
                .get("http://api.open-notify.org/iss-now.json")
                .send()
                .await
            {
                if let Ok(val) = resp.json::<serde_json::Value>().await {
                    if let (Some(lat_s), Some(lon_s)) = (
                        val["iss_position"]["latitude"].as_str(),
                        val["iss_position"]["longitude"].as_str(),
                    ) {
                        if let (Ok(lat), Ok(lon)) = (lat_s.parse::<f64>(), lon_s.parse::<f64>()) {
                            return IssPosition {
                                latitude: lat,
                                longitude: lon,
                                altitude: 420.0, // ISS altitude (approx km)
                                velocity: 7.66,  // ISS orbital velocity (approx km/s)
                            };
                        }
                    }
                }
            }
        }

        // Fallback to simulated ISS orbit
        let ticks = chrono::Utc::now().timestamp() as f64;
        let phase = ticks * 0.001;
        IssPosition {
            latitude: (phase.sin() * 51.64), // ISS inclination is 51.64 degrees
            longitude: ((phase * 1.5).cos() * 180.0),
            altitude: 421.5,
            velocity: 7.66,
        }
    }

    /// TLE (Two-Line Element) Adapter: Simulates orbital propagation based on TLE parameters
    pub fn propagate_tle(
        _tle_line1: &str,
        tle_line2: &str,
        epoch_offset_sec: f64,
    ) -> (f64, f64, f64) {
        let elements = orbitlogs_models::TleElements::parse(tle_line2);
        let mean_motion = elements.mean_motion;
        let inclination = elements.inclination;

        // Standard orbital period in seconds: 86400 / mean_motion
        let period = 86400.0 / mean_motion;
        let angle = (2.0 * std::f64::consts::PI * epoch_offset_sec) / period;

        let lat = (angle.sin() * inclination).clamp(-90.0, 90.0);
        let lon = ((angle * 1.05).cos() * 180.0).clamp(-180.0, 180.0);
        let alt = 500.0 + 15.0 * angle.sin(); // elliptical height variance

        (lat, lon, alt)
    }

    /// SatNOGS Adapter: Mocks querying SatNOGS DB for transmitter details and frequency allocations
    #[allow(dead_code)]
    pub fn query_satnogs_metadata(satellite_id: &str) -> serde_json::Value {
        serde_json::json!({
            "satellite_id": satellite_id,
            "satnogs_id": 25544,
            "status": "alive",
            "transmitters": [
                {
                    "frequency": 437800000, // UHF band
                    "mode": "FM",
                    "type": "transponder"
                },
                {
                    "frequency": 145825000, // VHF APRS
                    "mode": "AFSK",
                    "type": "beacon"
                }
            ]
        })
    }

    /// N2YO Adapter: Calculates ground station passes (azimuth/elevation) for satellite tracking
    #[allow(dead_code)]
    pub fn calculate_passes(sat_lat: f64, sat_lon: f64, gs_lat: f64, gs_lon: f64) -> (f64, f64) {
        // Compute difference vectors on simplified spherical projection
        let d_lat = gs_lat - sat_lat;
        let d_lon = gs_lon - sat_lon;
        let dist = (d_lat.powi(2) + d_lon.powi(2)).sqrt();

        // Calculate simulated azimuth (0-360 degrees) and elevation (-90 to 90 degrees)
        let azimuth = (d_lon.atan2(d_lat).to_degrees() + 360.0) % 360.0;
        let elevation = (90.0 - dist * 1.5).clamp(-90.0, 90.0);

        (azimuth, elevation)
    }
}
