use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

/// Performs a constant-time comparison of two strings by first hashing them
/// with SHA-256 and then comparing the hashes. This prevents timing attacks
/// that leak the length of the inputs.
pub fn secure_compare(provided: &str, expected: &str) -> bool {
    let provided_hash = Sha256::digest(provided.as_bytes());
    let expected_hash = Sha256::digest(expected.as_bytes());
    provided_hash.ct_eq(&expected_hash).unwrap_u8() == 1
}
