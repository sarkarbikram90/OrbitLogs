# OrbitLogs Astrodynamics Crate 🛰️📐

The **OrbitLogs Astrodynamics** shared library implements LEO orbital propagation and geodetic coordinate transformations used by contact predictors and Space Traffic Management daemons.

## Core Features

- **ECI to ECEF Rotation**: Rotates Cartesian coordinates from the Earth-Centered Inertial (TEME) frame to the Earth-Centered Earth-Fixed (ECEF) frame by calculating Greenwich Mean Sidereal Time (GMST).
- **Coriolis Velocity Corrections**: Corrects velocity vectors during frame rotation using Earth's angular rotation rate.
- **WGS84 Geodetic Transforms**: Converts between Cartesian ECEF and geodetic coordinates (Latitude, Longitude, Altitude).
- **Local Look Angles (AER)**: Rotates coordinates to local East-North-Up (ENU) antenna reference frames to calculate Azimuth, Elevation, and Range metrics.
- **SGP4 Propagation**: Predicts upcoming satellite paths and validates contact visibility windows from Two-Line Element (TLE) datasets.

## Usage

Include in `Cargo.toml`:
```toml
orbitlogs-astrodynamics = { workspace = true }
```

Sample code:
```rust
use orbitlogs_astrodynamics::{GeodeticCoords, geodetic_to_ecef};

let sf_station = GeodeticCoords {
    lat_deg: 37.7749,
    lon_deg: -122.4194,
    alt_m: 0.0,
};
let ecef_vector = geodetic_to_ecef(sf_station);
```
