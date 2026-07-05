use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

/// Hashes both inputs with SHA-256 and compares the resulting digests
/// in constant time to reduce timing side channels during equality checks.

/// Note: hashing itself is not constant-time with respect to input length.
pub fn secure_compare(provided: &str, expected: &str) -> bool {
    let provided_hash = Sha256::digest(provided.as_bytes());
    let expected_hash = Sha256::digest(expected.as_bytes());

    bool::from(provided_hash.ct_eq(&expected_hash))
}