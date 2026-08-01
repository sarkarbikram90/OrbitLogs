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

use crate::domain_error;
use serde::{Deserialize, Serialize};

/// Macro that generates validated newtype ID structs with full trait impls.
macro_rules! define_newtype_id {
    ($name:ident, $prefix:literal, $error_name:ident, $default_val:expr) => {
        #[derive(
            Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, sqlx::Type,
        )]
        #[sqlx(transparent)]
        pub struct $name(pub(crate) String);

        impl $name {
            /// Validated constructor: ensures the ID starts with the expected prefix.
            pub fn parse(raw: impl AsRef<str>) -> Result<Self, domain_error::$error_name> {
                let s = raw.as_ref();
                if !s.starts_with($prefix) {
                    return Err(domain_error::$error_name::InvalidPrefix(s.to_string()));
                }
                Ok(Self(s.to_string()))
            }

            /// Unvalidated constructor for backward compatibility.
            pub fn new(id: impl Into<String>) -> Self {
                Self(id.into())
            }

            /// Internal constructor for trusted sources (serde, DB rows).
            #[allow(dead_code)]
            pub(crate) fn from_trusted(s: String) -> Self {
                Self(s)
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl std::ops::Deref for $name {
            type Target = str;
            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl From<String> for $name {
            fn from(s: String) -> Self {
                Self(s)
            }
        }

        impl From<&str> for $name {
            fn from(s: &str) -> Self {
                Self(s.to_string())
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self($default_val.to_string())
            }
        }
    };
}

define_newtype_id!(SatelliteId, "SAT-", SatelliteIdError, "");
define_newtype_id!(StationId, "GS-", StationIdError, "");
define_newtype_id!(MissionId, "MISSION-", MissionIdError, "unknown");

impl SatelliteId {
    /// Derives the linked ground station ID deterministically from the satellite ID.
    pub fn linked_ground_station(&self) -> StationId {
        if let Some(num_str) = self.0.strip_prefix("SAT-") {
            if let Ok(num) = num_str.parse::<usize>() {
                let gs_num = ((num - 1) % 4) + 1;
                return StationId::from_trusted(format!("GS-{:04}", gs_num));
            }
        }
        StationId::from_trusted("GS-0001".to_string())
    }
}
