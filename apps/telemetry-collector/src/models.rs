/// Re-export of TelemetryRecord from the shared orbitlogs-models crate.
///
/// This module previously contained a local definition of TelemetryRecord.
/// It has been refactored to use the canonical shared crate to ensure a
/// single source of truth across the simulator and collector services.
pub use orbitlogs_models::TelemetryRecord;
