use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::ops::{Add, Neg, Sub};

// =========================================================================
// Bounded Numeric Newtypes — Parse, Don't Validate (P1 #13)
// =========================================================================
//
// These types make illegal values unrepresentable. Once constructed,
// consumers can *assume* the value is valid without re-checking.
//
// Note: TelemetryRecord fields are NOT changed to use these yet.
// These types are available for new code to opt into.

/// A percentage value guaranteed to be in [0.0, 100.0].
///
/// # Examples
/// ```
/// use orbitlogs_models::bounded::Percentage;
/// let p = Percentage::new(42.5).unwrap();
/// assert_eq!(p.get(), 42.5);
/// assert!(Percentage::new(105.0).is_err());
/// assert!(Percentage::new(-1.0).is_err());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Percentage(f32);

impl Percentage {
    pub fn new(value: f32) -> Result<Self, &'static str> {
        if (0.0..=100.0).contains(&value) {
            Ok(Self(value))
        } else {
            Err("Percentage must be between 0.0 and 100.0")
        }
    }

    pub fn get(self) -> f32 {
        self.0
    }
}

impl TryFrom<f32> for Percentage {
    type Error = &'static str;
    fn try_from(value: f32) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl Serialize for Percentage {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_f32(self.0)
    }
}

impl<'de> Deserialize<'de> for Percentage {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = f32::deserialize(deserializer)?;
        Self::new(value).map_err(serde::de::Error::custom)
    }
}

impl std::fmt::Display for Percentage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.1}%", self.0)
    }
}

/// A voltage value guaranteed to be in [0.0, 100.0] Volts.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Voltage(f64);

impl Voltage {
    pub fn new(value: f64) -> Result<Self, &'static str> {
        if (0.0..=100.0).contains(&value) {
            Ok(Self(value))
        } else {
            Err("Voltage must be between 0.0 and 100.0")
        }
    }

    pub fn get(self) -> f64 {
        self.0
    }
}

impl TryFrom<f64> for Voltage {
    type Error = &'static str;
    fn try_from(value: f64) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl Serialize for Voltage {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_f64(self.0)
    }
}

impl<'de> Deserialize<'de> for Voltage {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = f64::deserialize(deserializer)?;
        Self::new(value).map_err(serde::de::Error::custom)
    }
}

impl std::fmt::Display for Voltage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.2}V", self.0)
    }
}

/// A temperature value guaranteed to be in [-273.15, 500.0] °C.
/// Lower bound is absolute zero.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Temperature(f64);

impl Temperature {
    pub fn new(value: f64) -> Result<Self, &'static str> {
        if (-273.15..=500.0).contains(&value) {
            Ok(Self(value))
        } else {
            Err("Temperature must be between -273.15 and 500.0 °C")
        }
    }

    pub fn get(self) -> f64 {
        self.0
    }
}

impl TryFrom<f64> for Temperature {
    type Error = &'static str;
    fn try_from(value: f64) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl Serialize for Temperature {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_f64(self.0)
    }
}

impl<'de> Deserialize<'de> for Temperature {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = f64::deserialize(deserializer)?;
        Self::new(value).map_err(serde::de::Error::custom)
    }
}

impl std::fmt::Display for Temperature {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.1}°C", self.0)
    }
}

// =========================================================================
// Orbital Physics Types — Operator Overloading (P1 #14)
// =========================================================================
//
// Dimensional analysis via types: prevents nonsensical operations
// like adding two velocities or subtracting a DeltaV from a DeltaV
// when you meant to add it to a velocity.

/// Orbital velocity in km/s. Cannot be added to another OrbitalVelocity directly.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct OrbitalVelocity(pub f64);

/// Change in velocity (delta-V) in km/s.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct DeltaV(pub f64);

impl Add<DeltaV> for OrbitalVelocity {
    type Output = OrbitalVelocity;
    /// Apply a delta-V burn to the current velocity.
    fn add(self, rhs: DeltaV) -> Self::Output {
        OrbitalVelocity(self.0 + rhs.0)
    }
}

impl Sub<DeltaV> for OrbitalVelocity {
    type Output = OrbitalVelocity;
    /// Apply a retrograde delta-V burn.
    fn sub(self, rhs: DeltaV) -> Self::Output {
        OrbitalVelocity(self.0 - rhs.0)
    }
}

impl Sub for OrbitalVelocity {
    type Output = DeltaV;
    /// Compute the delta-V required to go from one velocity to another.
    fn sub(self, rhs: OrbitalVelocity) -> Self::Output {
        DeltaV(self.0 - rhs.0)
    }
}

impl Add for DeltaV {
    type Output = DeltaV;
    /// Combine two delta-V maneuvers.
    fn add(self, rhs: DeltaV) -> Self::Output {
        DeltaV(self.0 + rhs.0)
    }
}

impl Neg for DeltaV {
    type Output = DeltaV;
    /// Reverse the direction of a delta-V burn.
    fn neg(self) -> Self::Output {
        DeltaV(-self.0)
    }
}

impl std::fmt::Display for OrbitalVelocity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.4} km/s", self.0)
    }
}

impl std::fmt::Display for DeltaV {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Δv {:.4} km/s", self.0)
    }
}

// =========================================================================
// Tests
// =========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // --- Percentage ---

    #[test]
    fn percentage_valid_range() {
        assert!(Percentage::new(0.0).is_ok());
        assert!(Percentage::new(50.0).is_ok());
        assert!(Percentage::new(100.0).is_ok());
    }

    #[test]
    fn percentage_rejects_out_of_range() {
        assert!(Percentage::new(-0.1).is_err());
        assert!(Percentage::new(100.1).is_err());
        assert!(Percentage::new(f32::NAN).is_err());
    }

    #[test]
    fn percentage_serde_roundtrip() {
        let p = Percentage::new(42.5).unwrap();
        let json = serde_json::to_string(&p).unwrap();
        assert_eq!(json, "42.5");
        let deserialized: Percentage = serde_json::from_str(&json).unwrap();
        assert_eq!(p, deserialized);
    }

    #[test]
    fn percentage_serde_rejects_invalid() {
        let result: Result<Percentage, _> = serde_json::from_str("105.0");
        assert!(result.is_err());
    }

    #[test]
    fn percentage_try_from() {
        assert!(Percentage::try_from(50.0f32).is_ok());
        assert!(Percentage::try_from(200.0f32).is_err());
    }

    // --- Voltage ---

    #[test]
    fn voltage_valid_range() {
        assert!(Voltage::new(0.0).is_ok());
        assert!(Voltage::new(13.8).is_ok());
        assert!(Voltage::new(100.0).is_ok());
    }

    #[test]
    fn voltage_rejects_out_of_range() {
        assert!(Voltage::new(-1.0).is_err());
        assert!(Voltage::new(100.1).is_err());
    }

    // --- Temperature ---

    #[test]
    fn temperature_valid_range() {
        assert!(Temperature::new(-273.15).is_ok());
        assert!(Temperature::new(0.0).is_ok());
        assert!(Temperature::new(500.0).is_ok());
    }

    #[test]
    fn temperature_rejects_out_of_range() {
        assert!(Temperature::new(-274.0).is_err());
        assert!(Temperature::new(501.0).is_err());
    }

    // --- Orbital Physics ---

    #[test]
    fn orbital_velocity_add_delta_v() {
        let v = OrbitalVelocity(7.67);
        let dv = DeltaV(0.05);
        let result = v + dv;
        assert!((result.0 - 7.72).abs() < 1e-10);
    }

    #[test]
    fn orbital_velocity_sub_delta_v() {
        let v = OrbitalVelocity(7.67);
        let dv = DeltaV(0.05);
        let result = v - dv;
        assert!((result.0 - 7.62).abs() < 1e-10);
    }

    #[test]
    fn orbital_velocity_difference_yields_delta_v() {
        let v1 = OrbitalVelocity(7.72);
        let v2 = OrbitalVelocity(7.67);
        let dv: DeltaV = v1 - v2;
        assert!((dv.0 - 0.05).abs() < 1e-10);
    }

    #[test]
    fn delta_v_combine() {
        let dv1 = DeltaV(0.03);
        let dv2 = DeltaV(0.02);
        let combined = dv1 + dv2;
        assert!((combined.0 - 0.05).abs() < 1e-10);
    }

    #[test]
    fn delta_v_negate() {
        let dv = DeltaV(0.05);
        let reversed = -dv;
        assert!((reversed.0 - (-0.05)).abs() < 1e-10);
    }
}
