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
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

/// Hashes both inputs with SHA-256 and compares the resulting digests
/// in constant time to reduce timing side channels during equality checks.
///
/// Note: hashing itself is not constant-time with respect to input length.
pub fn secure_compare(provided: &str, expected: &str) -> bool {
    let provided_hash = Sha256::digest(provided.as_bytes());
    let expected_hash = Sha256::digest(expected.as_bytes());

    bool::from(provided_hash.ct_eq(&expected_hash))
}
