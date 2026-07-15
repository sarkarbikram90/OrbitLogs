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
/// Re-export of TelemetryRecord from the shared orbitlogs-models crate.
///
/// This module previously contained a local definition of TelemetryRecord.
/// It has been refactored to use the canonical shared crate to ensure a
/// single source of truth across the simulator and collector services.
pub use orbitlogs_models::TelemetryRecord;
