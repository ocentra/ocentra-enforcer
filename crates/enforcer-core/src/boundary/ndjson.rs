//! Typed decoding boundary for newline-delimited JSON records.

pub(crate) fn decode_record<T: serde::de::DeserializeOwned>(
    line: &str,
) -> Result<T, serde_json::Error> {
    serde_json::from_str(line)
}
