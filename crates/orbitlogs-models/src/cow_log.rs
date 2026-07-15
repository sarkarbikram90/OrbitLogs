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
use crate::SatelliteId;
use std::borrow::Cow;

// =========================================================================
// Cow<'_, str> — Zero-Copy When Possible, Owned When Necessary (P1 #4)
// =========================================================================
//
// `Cow` defers the allocation decision: static messages stay borrowed
// (zero allocation), while dynamic messages are owned.

/// A log entry that uses `Cow<'a, str>` to avoid heap allocations
/// for static messages while still supporting dynamic content.
///
/// # Lifetime
/// `'a` ties the log entry to the lifetime of any borrowed data.
/// Static messages have `'static` lifetime and never allocate.
#[derive(Debug, Clone)]
pub struct LogEntry<'a> {
    /// The satellite this log entry pertains to.
    pub satellite_id: &'a SatelliteId,

    /// The log message. Borrowed if static, owned if dynamic.
    pub message: Cow<'a, str>,

    /// Optional detailed information.
    pub details: Option<Cow<'a, str>>,
}

impl<'a> LogEntry<'a> {
    /// Create a log entry with a static message — zero heap allocation.
    ///
    /// # Examples
    /// ```ignore
    /// let sat_id = SatelliteId::from_trusted("SAT-0001".to_string());
    /// let entry = LogEntry::static_msg(&sat_id, "Health check passed");
    /// ```
    pub fn static_msg(satellite_id: &'a SatelliteId, msg: &'static str) -> Self {
        Self {
            satellite_id,
            message: Cow::Borrowed(msg),
            details: None,
        }
    }

    /// Create a log entry with a dynamic (owned) message.
    ///
    /// Use this when the message is constructed at runtime.
    pub fn dynamic_msg(satellite_id: &'a SatelliteId, msg: String) -> Self {
        Self {
            satellite_id,
            message: Cow::Owned(msg),
            details: None,
        }
    }

    /// Add details to the log entry.
    pub fn with_details(mut self, details: impl Into<Cow<'a, str>>) -> Self {
        self.details = Some(details.into());
        self
    }

    /// Returns true if the message is borrowed (zero-copy).
    pub fn is_borrowed(&self) -> bool {
        matches!(&self.message, Cow::Borrowed(_))
    }
}

impl std::fmt::Display for LogEntry<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.satellite_id, self.message)?;
        if let Some(ref details) = self.details {
            write!(f, " — {}", details)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn static_msg_is_borrowed() {
        let sat_id = SatelliteId::from_trusted("SAT-0001".to_string());
        let entry = LogEntry::static_msg(&sat_id, "Health check passed");
        assert!(entry.is_borrowed());
    }

    #[test]
    fn dynamic_msg_is_owned() {
        let sat_id = SatelliteId::from_trusted("SAT-0001".to_string());
        let msg = format!("Temperature spike: {:.1}°C", 82.5);
        let entry = LogEntry::dynamic_msg(&sat_id, msg);
        assert!(!entry.is_borrowed());
    }

    #[test]
    fn display_without_details() {
        let sat_id = SatelliteId::from_trusted("SAT-0001".to_string());
        let entry = LogEntry::static_msg(&sat_id, "Nominal");
        assert_eq!(entry.to_string(), "[SAT-0001] Nominal");
    }

    #[test]
    fn display_with_details() {
        let sat_id = SatelliteId::from_trusted("SAT-0002".to_string());
        let entry = LogEntry::static_msg(&sat_id, "Warning")
            .with_details("Battery voltage below threshold");
        assert_eq!(
            entry.to_string(),
            "[SAT-0002] Warning — Battery voltage below threshold"
        );
    }
}
