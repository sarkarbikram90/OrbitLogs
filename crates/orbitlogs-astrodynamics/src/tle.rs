use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sgp4::{Constants, Elements};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AstrodynamicsError {
    #[error("SGP4 propagation error: {0}")]
    Sgp4Error(String),
}

impl From<sgp4::Error> for AstrodynamicsError {
    fn from(err: sgp4::Error) -> Self {
        AstrodynamicsError::Sgp4Error(format!("{:?}", err))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TleRecord {
    pub satellite_id: String,
    pub epoch: DateTime<Utc>,
    pub line1: String,
    pub line2: String,
}

impl TleRecord {
    /// Validates the lines and parses Keplerian components using the SGP4 crate.
    pub fn to_elements(&self) -> Result<Elements, AstrodynamicsError> {
        let elements = Elements::from_tle(
            Some(self.satellite_id.clone()),
            self.line1.as_bytes(),
            self.line2.as_bytes(),
        )?;
        Ok(elements)
    }

    /// Propagates the orbit to a target time, returning position and velocity in ECI (TEME) frame in km and km/s.
    pub fn propagate_to(&self, target_time: DateTime<Utc>) -> Result<([f64; 3], [f64; 3]), AstrodynamicsError> {
        let elements = self.to_elements()?;
        let constants = Constants::from_elements(&elements)?;
        
        // Calculate minutes since epoch
        let delta_seconds = target_time.signed_duration_since(self.epoch).num_seconds() as f64;
        let delta_minutes = delta_seconds / 60.0;
        
        let prediction = constants.propagate(delta_minutes)?;
        
        Ok((prediction.position, prediction.velocity))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn test_iss_tle_propagation() {
        // ISS TLE from epoch 2020-351.52044816
        let record = TleRecord {
            satellite_id: "ISS".to_string(),
            epoch: Utc.with_ymd_and_hms(2020, 12, 16, 12, 29, 27).unwrap(),
            line1: "1 25544U 98067A   20351.52044816  .00001099  00000-0  28236-4 0  9997".to_string(),
            line2: "2 25544  51.6453 293.7432 0001229  93.1819  69.7547 15.49168434259798".to_string(),
        };

        let elements_res = record.to_elements();
        assert!(elements_res.is_ok(), "Failed to parse elements: {:?}", elements_res.err());

        // Propagate 10 minutes forward
        let target = record.epoch + chrono::Duration::minutes(10);
        let prop_res = record.propagate_to(target);
        assert!(prop_res.is_ok(), "Failed to propagate: {:?}", prop_res.err());

        let (pos, vel) = prop_res.unwrap();
        // Position coordinates in km should be non-zero
        assert!(pos[0] != 0.0);
        assert!(pos[1] != 0.0);
        assert!(pos[2] != 0.0);
        
        // Velocity coordinates in km/s should be non-zero
        assert!(vel[0] != 0.0);
        assert!(vel[1] != 0.0);
        assert!(vel[2] != 0.0);
    }
}
