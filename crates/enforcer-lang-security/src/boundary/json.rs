//! Generic JSON decoding boundary for source-backed security checks.

pub(crate) fn value(source: &str) -> Option<serde_json::Value> {
    serde_json::from_str(source).ok()
}
