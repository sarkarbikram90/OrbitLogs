use chrono::{DateTime, Datelike, Timelike, Utc};
use nalgebra::Vector3;

pub const EARTH_ROTATION_RATE: f64 = 7.2921151467e-5; // Earth's angular velocity in rad/s

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GeodeticCoords {
    pub lat_deg: f64,
    pub lon_deg: f64,
    pub alt_m: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EnuCoords {
    pub east: f64,
    pub north: f64,
    pub up: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LookAngles {
    pub azimuth_deg: f64,
    pub elevation_deg: f64,
    pub range_m: f64,
}

/// Computes the Julian Date at a given DateTime.
pub fn datetime_to_jd(dt: DateTime<Utc>) -> f64 {
    let year = dt.year() as f64;
    let month = dt.month() as f64;
    let day = dt.day() as f64;
    let hour = dt.hour() as f64;
    let min = dt.minute() as f64;
    let sec = dt.second() as f64;
    let ms = dt.timestamp_subsec_millis() as f64;

    let (y, m) = if month <= 2.0 {
        (year - 1.0, month + 12.0)
    } else {
        (year, month)
    };

    let a = (y / 100.0).floor();
    let b = 2.0 - a + (a / 4.0).floor();

    let jd = (365.25 * (y + 4716.0)).floor()
        + (30.6001 * (m + 1.0)).floor()
        + day
        + b
        - 1524.5;

    let day_fraction = (hour + min / 60.0 + (sec + ms / 1000.0) / 3600.0) / 24.0;
    jd + day_fraction
}

/// Calculates the Greenwich Mean Sidereal Time (GMST) in radians.
pub fn calculate_gmst_rad(dt: DateTime<Utc>) -> f64 {
    let jd = datetime_to_jd(dt);
    let t = (jd - 2451545.0) / 36525.0;
    
    // GMST in seconds of time (IAU 1982 model)
    let gmst_sec = 24110.54841 + 8640184.812866 * t + 0.093104 * t.powi(2) - 6.2e-6 * t.powi(3);
    
    // Convert to hours, then to radians
    let gmst_hours = (gmst_sec / 3600.0) % 24.0;
    let gmst_rad = gmst_hours * (2.0 * std::f64::consts::PI / 24.0);
    
    // Normalize to [0, 2pi]
    let two_pi = 2.0 * std::f64::consts::PI;
    (gmst_rad % two_pi + two_pi) % two_pi
}

/// Rotates a position vector from Earth-Centered Inertial (ECI / TEME) to Earth-Centered, Earth-Fixed (ECEF) frame.
/// Input coordinates should be in km or meters; output matches input units.
pub fn eci_to_ecef(pos_eci: Vector3<f64>, dt: DateTime<Utc>) -> Vector3<f64> {
    let gmst = calculate_gmst_rad(dt);
    let cos_g = gmst.cos();
    let sin_g = gmst.sin();

    Vector3::new(
        cos_g * pos_eci.x + sin_g * pos_eci.y,
        -sin_g * pos_eci.x + cos_g * pos_eci.y,
        pos_eci.z,
    )
}

/// Rotates velocity from ECI to ECEF, accounting for the Earth's rotation (Coriolis acceleration).
/// Input velocity in km/s or m/s; output matches input units.
pub fn eci_velocity_to_ecef(
    pos_ecef: Vector3<f64>,
    vel_eci: Vector3<f64>,
    dt: DateTime<Utc>,
) -> Vector3<f64> {
    let gmst = calculate_gmst_rad(dt);
    let cos_g = gmst.cos();
    let sin_g = gmst.sin();

    let vel_rot = Vector3::new(
        cos_g * vel_eci.x + sin_g * vel_eci.y,
        -sin_g * vel_eci.x + cos_g * vel_eci.y,
        vel_eci.z,
    );

    // omega x r correction
    Vector3::new(
        vel_rot.x + EARTH_ROTATION_RATE * pos_ecef.y,
        vel_rot.y - EARTH_ROTATION_RATE * pos_ecef.x,
        vel_rot.z,
    )
}

/// Converts ECEF coordinates (meters) to Geodetic (Latitude, Longitude, Altitude) on WGS84 ellipsoid.
pub fn ecef_to_geodetic(pos: Vector3<f64>) -> GeodeticCoords {
    let a: f64 = 6378137.0; // WGS84 semi-major axis (meters)
    let f: f64 = 1.0 / 298.257223563; // Flattening
    let b: f64 = a * (1.0 - f); // Semi-minor axis
    let e2 = (a.powi(2) - b.powi(2)) / a.powi(2); // First eccentricity squared
    let ep2 = (a.powi(2) - b.powi(2)) / b.powi(2); // Second eccentricity squared

    let p = (pos.x.powi(2) + pos.y.powi(2)).sqrt();
    let theta = (pos.z * a).atan2(p * b);

    let lat = (pos.z + ep2 * b * theta.sin().powi(3)).atan2(p - e2 * a * theta.cos().powi(3));
    let lon = pos.y.atan2(pos.x);

    let n = a / (1.0 - e2 * lat.sin().powi(2)).sqrt();
    let alt = p / lat.cos() - n;

    GeodeticCoords {
        lat_deg: lat.to_degrees(),
        lon_deg: lon.to_degrees(),
        alt_m: alt,
    }
}

/// Converts Geodetic (Lat/Lon in degrees, Alt in meters) to ECEF coordinates (meters).
pub fn geodetic_to_ecef(lat_deg: f64, lon_deg: f64, alt_m: f64) -> Vector3<f64> {
    let lat = lat_deg.to_radians();
    let lon = lon_deg.to_radians();
    let a: f64 = 6378137.0;
    let f: f64 = 1.0 / 298.257223563;
    let e2: f64 = f * (2.0 - f);

    let n = a / (1.0 - e2 * lat.sin().powi(2)).sqrt();
    
    let x = (n + alt_m) * lat.cos() * lon.cos();
    let y = (n + alt_m) * lat.cos() * lon.sin();
    let z = (n * (1.0 - e2) + alt_m) * lat.sin();

    Vector3::new(x, y, z)
}

/// Translates an ECEF position to East-North-Up (ENU) coordinates relative to a reference ECEF point.
pub fn ecef_to_enu(
    pos_ecef: Vector3<f64>,
    ref_lat_deg: f64,
    ref_lon_deg: f64,
    ref_ecef: Vector3<f64>,
) -> EnuCoords {
    let lat = ref_lat_deg.to_radians();
    let lon = ref_lon_deg.to_radians();

    let d = pos_ecef - ref_ecef;

    let cos_lat = lat.cos();
    let sin_lat = lat.sin();
    let cos_lon = lon.cos();
    let sin_lon = lon.sin();

    let east = -sin_lon * d.x + cos_lon * d.y;
    let north = -sin_lat * cos_lon * d.x - sin_lat * sin_lon * d.y + cos_lat * d.z;
    let up = cos_lat * cos_lon * d.x + cos_lat * sin_lon * d.y + sin_lat * d.z;

    EnuCoords { east, north, up }
}

/// Computes look angles (Azimuth, Elevation, Range) from local ENU coordinates.
pub fn calculate_look_angles(enu: &EnuCoords) -> LookAngles {
    let range = (enu.east.powi(2) + enu.north.powi(2) + enu.up.powi(2)).sqrt();
    let elevation = enu.up.atan2((enu.east.powi(2) + enu.north.powi(2)).sqrt());
    let mut azimuth = enu.east.atan2(enu.north);

    if azimuth < 0.0 {
        azimuth += 2.0 * std::f64::consts::PI;
    }

    LookAngles {
        azimuth_deg: azimuth.to_degrees(),
        elevation_deg: elevation.to_degrees(),
        range_m: range,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ecef_geodetic_roundtrip() {
        // Test near SpaceX HQ (lat: 33.92, lon: -118.32, alt: 31 meters)
        let lat = 33.9205;
        let lon = -118.3273;
        let alt = 31.0;

        let ecef = geodetic_to_ecef(lat, lon, alt);
        let geodetic = ecef_to_geodetic(ecef);

        assert!((geodetic.lat_deg - lat).abs() < 1e-5);
        assert!((geodetic.lon_deg - lon).abs() < 1e-5);
        assert!((geodetic.alt_m - alt).abs() < 1.0); // Within 1 meter
    }

    #[test]
    fn test_look_angles() {
        // Simple case: satellite straight above ground station
        let enu = EnuCoords {
            east: 0.0,
            north: 0.0,
            up: 500_000.0, // 500 km altitude
        };
        let look = calculate_look_angles(&enu);
        assert_eq!(look.elevation_deg, 90.0);
        assert_eq!(look.range_m, 500_000.0);
    }
}
