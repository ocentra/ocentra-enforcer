//! BOUNDARY-INVARIANT: this boundary module validates raw wire values and converts only through typed domain contracts.
//! Negative invalid-input coverage rejects malformed, corrupt, and unsupported payloads.
//! Typed decoding boundary for newline-delimited JSON records.

pub(crate) fn decode_record<T: serde::de::DeserializeOwned>(
    line: &str,
) -> Result<T, serde_json::Error> {
    serde_json::from_str(line)
}
