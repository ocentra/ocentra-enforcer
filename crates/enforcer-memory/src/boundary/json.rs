//! Typed JSON decoding at Memory's external transport boundary.

use serde::de::DeserializeOwned;

/// Decode external JSON into an explicit boundary DTO or serde value.
pub(crate) fn decode<T>(raw: &str) -> serde_json::Result<T>
where
    T: DeserializeOwned,
{
    serde_json::from_str(raw)
}
