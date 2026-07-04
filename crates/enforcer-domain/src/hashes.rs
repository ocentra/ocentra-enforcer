//! Branded digest newtypes. Matches the `enforcer-core` hash-chain wire
//! form: `sha256:` prefix + 64 lowercase hex characters.

use enforcer_core::error::DecodeError;

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
}
