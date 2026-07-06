//! Branded digest newtypes. Matches the `enforcer-core` hash-chain wire
//! form: `sha256:` prefix + 64 lowercase hex characters.

use enforcer_core::error::DecodeError;
use sha2::Digest as _;

/// Branded SHA-256 digest (`sha256:<64 lowercase hex>`).
#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    serde::Serialize,
    serde::Deserialize,
    ts_rs::TS,
)]
#[serde(try_from = "String", into = "String")]
#[ts(type = "string")]
pub struct Sha256(String);

impl Sha256 {
    /// View the full prefixed digest string.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The 64-char hex payload without the `sha256:` prefix.
    pub fn hex(&self) -> &str {
        self.0
            .get(enforcer_core::hash_chain::DIGEST_PREFIX.len()..)
            .unwrap_or_default()
    }

    /// Hash `bytes` and mint the branded digest directly (`sha256:<64 lowercase
    /// hex>`). This is the constructor for callers that hold raw content and
    /// need its digest, as opposed to [`Sha256::try_from`]/[`FromStr::from_str`]
    /// which parse an already-computed digest string. Infallible: SHA-256
    /// always produces exactly 64 lowercase hex chars, so the output always
    /// satisfies [`Sha256::try_from`]'s own validation.
    pub fn of(bytes: &[u8]) -> Self {
        let mut hasher = sha2::Sha256::new();
        hasher.update(bytes);
        let raw = hasher.finalize();
        let prefix_len = enforcer_core::hash_chain::DIGEST_PREFIX.len();
        let mut hex = String::with_capacity(prefix_len + raw.len() * 2);
        hex.push_str(enforcer_core::hash_chain::DIGEST_PREFIX);
        for byte in raw {
            use std::fmt::Write as _;
            // Writing to a String cannot fail; ignore the Infallible result.
            let _ = write!(hex, "{byte:02x}");
        }
        Self(hex)
    }
}

impl TryFrom<String> for Sha256 {
    type Error = DecodeError;

    fn try_from(raw: String) -> Result<Self, DecodeError> {
        let Some(hex) = raw.strip_prefix(enforcer_core::hash_chain::DIGEST_PREFIX) else {
            return Err(DecodeError::new("sha256", "missing `sha256:` prefix"));
        };
        let ok = hex.len() == 64
            && hex
                .chars()
                .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c));
        if ok {
            Ok(Self(raw))
        } else {
            Err(DecodeError::new(
                "sha256",
                "expected 64 lowercase hex chars after `sha256:`",
            ))
        }
    }
}

impl std::str::FromStr for Sha256 {
    type Err = DecodeError;

    fn from_str(raw: &str) -> Result<Self, DecodeError> {
        Self::try_from(raw.to_owned())
    }
}

impl From<Sha256> for String {
    fn from(value: Sha256) -> String {
        value.0
    }
}

impl std::fmt::Display for Sha256 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::Sha256;
    use enforcer_core::error::DecodeError;

    #[test]
    fn accepts_core_hash_chain_output() -> Result<(), DecodeError> {
        // Real digest produced by the core primitive this brand wraps.
        let digest = enforcer_core::hash_chain::link_digest(None, b"payload");
        let branded: Sha256 = digest.parse()?;
        assert_eq!(branded.as_str(), digest);
        assert_eq!(branded.hex().len(), 64);
        Ok(())
    }

    #[test]
    fn rejects_malformed_digests() {
        let bad_cases = [
            "",
            "sha256:",
            "sha256:short",
            "deadbeef",
            // uppercase hex
            &format!("sha256:{}", "A".repeat(64)),
            // non-hex chars
            &format!("sha256:{}", "g".repeat(64)),
            // wrong prefix
            &format!("md5:{}", "a".repeat(64)),
        ];
        for bad in bad_cases {
            let outcome: Result<Sha256, _> = bad.parse();
            assert!(outcome.is_err(), "should reject {bad:?}");
        }
    }

    #[test]
    fn serde_round_trip_preserves_wire_form() -> Result<(), serde_json::Error> {
        let wire = format!("\"sha256:{}\"", "ab".repeat(32));
        let parsed: Sha256 = serde_json::from_str(&wire)?;
        assert_eq!(serde_json::to_string(&parsed)?, wire);
        Ok(())
    }

    /// Named oracle for `proof/schema/a05-sha256.txt`: `Sha256::of` hashes
    /// content and mints a value that is itself accepted by `Sha256::parse`
    /// (`of()`+`parse()` round-trip), and length/case/charset violations are
    /// rejected both via `FromStr`/`TryFrom` and via serde at the boundary.
    #[test]
    fn sha256_brand_decode() -> Result<(), DecodeError> {
        // `of()` mints a digest that round-trips through `parse()`.
        let minted = Sha256::of(b"hello world");
        let reparsed: Sha256 = minted.as_str().parse()?;
        assert_eq!(minted, reparsed);
        assert_eq!(minted.hex().len(), 64);
        assert!(minted.hex().chars().all(|c| c.is_ascii_hexdigit()));
        assert!(minted.hex().chars().all(|c| !c.is_ascii_uppercase()));

        // Deterministic: same bytes -> same digest; different bytes -> different digest.
        assert_eq!(Sha256::of(b"hello world"), Sha256::of(b"hello world"));
        assert_ne!(Sha256::of(b"hello world"), Sha256::of(b"hello world!"));

        // Empty input is a valid (well-known) SHA-256 preimage, not a special case.
        let empty = Sha256::of(b"");
        assert_eq!(
            empty.hex(),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );

        // Wrong length / case / charset all fail closed via FromStr/TryFrom.
        for bad in [
            "sha256:short",
            &format!("sha256:{}", "A".repeat(64)), // uppercase
            &format!("sha256:{}", "g".repeat(64)), // non-hex
            &format!("sha256:{}", "a".repeat(63)), // too short
            &format!("sha256:{}", "a".repeat(65)), // too long
        ] {
            let outcome: Result<Sha256, DecodeError> = bad.parse();
            assert!(outcome.is_err(), "should reject {bad:?}");
        }

        // Same violations fail closed through the serde boundary too.
        for bad_json in [
            "\"sha256:short\"",
            &format!("\"sha256:{}\"", "A".repeat(64)),
            &format!("\"sha256:{}\"", "g".repeat(64)),
        ] {
            let outcome: Result<Sha256, _> = serde_json::from_str(bad_json);
            assert!(outcome.is_err(), "serde should reject {bad_json:?}");
        }

        Ok(())
    }

    // Compile-reject fixture (acceptance criterion: "the private field makes
    // a bare `String` populating a `Sha256` field a compile error"). This is
    // proved at review/proof time, not by a runtime test -- Sha256's only
    // field is private (`Sha256(String)` with no public tuple-index/ctor), so
    // the snippet below does not compile if uncommented:
    //
    //     struct Holder {
    //         digest: Sha256,
    //     }
    //     fn bad(raw: String) -> Holder {
    //         Holder { digest: raw } // expected `Sha256`, found `String`
    //     }
    //
    // fails with E0308 (mismatched types): there is no `From<String> for
    // Sha256`/`Into<Sha256> for String` impl (only the reverse, fallible
    // `TryFrom<String>`), so a bare `String` never satisfies a `Sha256`-typed
    // field or parameter.
}
