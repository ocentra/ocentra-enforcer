//! Raw hash input converted into canonical domain digests.

use sha2::Digest as _;

use crate::hashes::Sha256;

// BOUNDARY-INVARIANT: raw borrowed bytes are converted immediately into a
// Sha256 domain value and never stored as domain state.
// boundaryOwnerNote: enforcer-domain owns the shared hash-input boundary.
// Negative invalid-input coverage is not applicable: every byte sequence,
// including empty input, is valid SHA-256 material and is covered by tests.

/// Hash raw boundary bytes into a canonical digest.
#[must_use]
#[doc = "Convert arbitrary boundary bytes into a canonical SHA-256 domain value."]
pub fn validate(bytes: &[u8]) -> Sha256 {
    let mut hasher = sha2::Sha256::new();
    hasher.update(bytes);
    Sha256::from_digest(hasher.finalize())
}
