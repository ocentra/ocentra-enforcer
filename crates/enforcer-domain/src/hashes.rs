//! Branded digest newtypes. Owns the shared hash-chain wire
//! form: `sha256:` prefix + 64 lowercase hex characters.

use crate::boundary::decode_error::DecodeError;

/// Digest string prefix identifying the SHA-256 wire representation.
pub const SHA256_PREFIX: &str = "sha256:";

/// Branded SHA-256 digest (`sha256:<64 lowercase hex>`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, ts_rs::TS)]
#[ts(type = "string")]
#[doc = "BRAND-INVARIANT: validated SHA-256 wire value with lowercase hexadecimal payload."]
pub struct Sha256(String);

impl Sha256 {
    /// Try to validate a prefixed SHA-256 digest from its wire form.
    pub fn try_new(raw: &str) -> Result<Self, DecodeError> {
        <Self as std::str::FromStr>::from_str(raw)
    }

    /// View the full prefixed digest string.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The 64-char hex payload without the `sha256:` prefix.
    pub fn hex(&self) -> &str {
        let Some(hex) = self.0.strip_prefix(SHA256_PREFIX) else {
            return "";
        };
        hex
    }

    /// Mint the canonical wire representation from a completed SHA-256 digest.
    pub fn from_digest(raw: sha2::digest::Output<sha2::Sha256>) -> Self {
        let prefix_len = SHA256_PREFIX.len();
        let mut hex = String::with_capacity(prefix_len + raw.len() * 2);
        hex.push_str(SHA256_PREFIX);
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
        let Some(hex) = raw.strip_prefix(SHA256_PREFIX) else {
            return Err(DecodeError::new(
                "sha256",
                "invalid SHA-256 digest: missing `sha256:` prefix",
            ));
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
                "invalid SHA-256 digest: expected 64 lowercase hex chars after `sha256:`",
            ))
        }
    }
}

impl std::str::FromStr for Sha256 {
    type Err = DecodeError;

    fn from_str(raw: &str) -> Result<Self, DecodeError> {
        // ALLOC-JUSTIFICATION: Sha256 owns canonical digest text after parsing a borrowed value.
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

// `records.rs` is a durable NDJSON wire contract. Keep the canonical digest
// private and validated while that contract serializes it as its established
// string representation; arbitrary strings still enter only through TryFrom.
impl serde::Serialize for Sha256 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> serde::Deserialize<'de> for Sha256 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = String::deserialize(deserializer)?;
        Self::try_from(raw).map_err(serde::de::Error::custom)
    }
}
