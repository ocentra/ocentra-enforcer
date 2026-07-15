//! Pure SHA-256 hash-chain primitive.
//!
//! RECONCILED 2026-07-05: this module was originally specified as an
//! OcentraParent `logging-core` borrow, but the real upstream source
//! (reachable at vendor time; unreachable when this module was first
//! written, per lesson L12) contains NO hash-chain logic anywhere —
//! `logging-core` has no such primitive to port. This is Enforcer-native
//! code, inspired by the same tamper-evident-chain idea used elsewhere in
//! the OcentraParent ecosystem, not a vendored module. No reconciliation
//! is pending; there is nothing upstream to reconcile against.
//!
//! Side-effect free: every function here is pure compute over byte slices.
//! Each link's digest covers its payload AND the previous link's digest, so
//! any tampered/removed/reordered entry breaks every digest after it.
//! `enforcer-proof` (arc-17) reuses this primitive for its tamper-evident
//! journal envelope; core owns only the primitive.

use enforcer_domain::hashes::SHA256_PREFIX;
use sha2::Digest;

/// Compute the digest for one link: SHA-256 over the previous digest (or
/// nothing, for the genesis link) followed by the payload bytes.
pub fn link_digest(prev_digest: Option<&str>, payload: &[u8]) -> String {
    let mut hasher = sha2::Sha256::new();
    if let Some(prev) = prev_digest {
        hasher.update(prev.as_bytes());
    }
    hasher.update(payload);
    let raw = hasher.finalize();
    let mut hex = String::with_capacity(SHA256_PREFIX.len() + raw.len() * 2);
    hex.push_str(SHA256_PREFIX);
    for byte in raw {
        use std::fmt::Write as _;
        // Writing to a String cannot fail; ignore the Infallible result.
        let _ = write!(hex, "{byte:02x}");
    }
    hex
}

/// Verification failure describing the first broken link.
#[derive(Debug, PartialEq, Eq)]
pub struct ChainBreak {
    /// Zero-based index of the first link whose digest did not verify.
    pub index: usize,
    /// The digest recorded on the broken link.
    pub recorded: String,
    /// The digest recomputed from payload + previous digest.
    pub expected: String,
}

/// Verify a whole chain of `(payload, recorded_digest)` links.
///
/// Returns `Ok(count)` with the number of verified links, or the first
/// [`ChainBreak`] encountered.
pub fn verify_chain<'a, I>(links: I) -> std::result::Result<usize, ChainBreak>
where
    I: IntoIterator<Item = (&'a [u8], &'a str)>,
{
    let mut prev: Option<String> = None;
    let mut count = 0usize;
    for (index, (payload, recorded)) in links.into_iter().enumerate() {
        let expected = link_digest(prev.as_deref(), payload);
        if expected != recorded {
            return Err(ChainBreak {
                index,
                recorded: recorded.to_owned(),
                expected,
            });
        }
        prev = Some(expected);
        count += 1;
    }
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::{link_digest, verify_chain, ChainBreak};
    use enforcer_domain::hashes::SHA256_PREFIX;

    fn build_chain(payloads: &[&[u8]]) -> Vec<String> {
        let mut digests = Vec::new();
        let mut prev: Option<String> = None;
        for payload in payloads {
            let digest = link_digest(prev.as_deref(), payload);
            prev = Some(digest.clone());
            digests.push(digest);
        }
        digests
    }

    #[test]
    fn digests_are_prefixed_and_deterministic() {
        let a = link_digest(None, b"payload");
        let b = link_digest(None, b"payload");
        assert_eq!(a, b);
        assert!(a.starts_with(SHA256_PREFIX));
        assert_eq!(a.len(), SHA256_PREFIX.len() + 64);
    }

    #[test]
    fn same_payload_different_prev_yields_different_digest() {
        let genesis = link_digest(None, b"payload");
        let chained = link_digest(Some(&genesis), b"payload");
        assert_ne!(genesis, chained);
    }

    #[test]
    fn intact_chain_verifies() {
        let payloads: Vec<&[u8]> = vec![b"one", b"two", b"three"];
        let digests = build_chain(&payloads);
        let links = payloads
            .iter()
            .copied()
            .zip(digests.iter().map(String::as_str));
        assert_eq!(verify_chain(links), Ok(3));
    }

    #[test]
    fn tampered_payload_is_detected_at_its_index() {
        let payloads: Vec<&[u8]> = vec![b"one", b"two", b"three"];
        let digests = build_chain(&payloads);
        // Tamper with the middle payload but keep its recorded digest.
        let tampered: Vec<&[u8]> = vec![b"one", b"TWO", b"three"];
        let links = tampered
            .iter()
            .copied()
            .zip(digests.iter().map(String::as_str));
        let outcome = verify_chain(links);
        match outcome {
            Err(ChainBreak { index, .. }) => assert_eq!(index, 1),
            Ok(_) => unreachable!("tampered chain must not verify"),
        }
    }

    #[test]
    fn truncated_prefix_still_verifies_but_reordered_links_fail() {
        let payloads: Vec<&[u8]> = vec![b"one", b"two", b"three"];
        let digests = build_chain(&payloads);
        // A prefix of the chain is itself a valid chain.
        let prefix = payloads[..2]
            .iter()
            .copied()
            .zip(digests[..2].iter().map(String::as_str));
        assert_eq!(verify_chain(prefix), Ok(2));
        // Swapping two links breaks verification.
        let swapped_payloads: Vec<&[u8]> = vec![b"two", b"one", b"three"];
        let links = swapped_payloads
            .iter()
            .copied()
            .zip(digests.iter().map(String::as_str));
        assert!(verify_chain(links).is_err());
    }
}
