//! JSON transport operations owned by the domain boundary.

// BOUNDARY-INVARIANT: JSON is decoded only through serde into a caller-selected
// transport shape; domain validation remains owned by each target type.
// boundaryOwnerNote: enforcer-domain owns this shared JSON adapter.
// Negative malformed JSON and invalid typed values are covered by boundary tests.

/// Encode a serializable value into the generic JSON boundary representation.
pub fn to_value<T: serde::Serialize>(value: &T) -> Result<serde_json::Value, serde_json::Error> {
    serde_json::to_value(value)
}

/// Encode a serializable value into JSON text.
pub fn to_string<T: serde::Serialize>(value: &T) -> Result<String, serde_json::Error> {
    serde_json::to_string(value)
}

/// Decode JSON text into the caller-selected boundary type.
pub fn from_str<T: serde::de::DeserializeOwned>(value: &str) -> Result<T, serde_json::Error> {
    serde_json::from_str(value)
}

/// Decode a generic JSON value into the caller-selected boundary type.
pub fn from_value<T: serde::de::DeserializeOwned>(
    value: serde_json::Value,
) -> Result<T, serde_json::Error> {
    serde_json::from_value(value)
}
