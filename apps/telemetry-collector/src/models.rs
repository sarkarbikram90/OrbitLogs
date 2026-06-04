/// Re-export of TelemetryRecord from the shared spaceops-models crate.
///
/// This module previously contained a local definition of TelemetryRecord.
/// It has been refactored to use the canonical shared crate to ensure a
/// single source of truth across the simulator and collector services.
pub use spaceops_models::TelemetryRecord;
