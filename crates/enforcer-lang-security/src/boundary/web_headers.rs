//! HTTP response snapshot decoding boundary.
//! Malformed JSON is rejected, with negative coverage in this module's tests.

use std::collections::BTreeMap;

#[derive(Debug, Default, serde::Deserialize)]
pub(crate) struct CookieSnapshot {
    // DEFAULT-JUSTIFICATION: an unnamed cookie remains inspectable by its security attributes.
    #[serde(default)]
    pub(crate) name: String,
    // DEFAULT-JUSTIFICATION: Secure is false when the captured cookie omits the attribute.
    #[serde(default)]
    pub(crate) secure: bool,
    // DEFAULT-JUSTIFICATION: HttpOnly is false when the captured cookie omits the attribute.
    #[serde(default)]
    pub(crate) httponly: bool,
    // DEFAULT-JUSTIFICATION: an omitted SameSite attribute must remain absent for validation.
    #[serde(default)]
    pub(crate) samesite: Option<String>,
}

#[derive(Debug, Default, serde::Deserialize)]
pub(crate) struct HeadersSnapshot {
    // DEFAULT-JUSTIFICATION: a snapshot without headers has no header values to inspect.
    #[serde(default)]
    headers: BTreeMap<String, String>,
    // DEFAULT-JUSTIFICATION: a snapshot without cookies has no cookie attributes to inspect.
    #[serde(default)]
    pub(crate) cookies: Vec<CookieSnapshot>,
}

impl HeadersSnapshot {
    pub(crate) fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }
}

pub(crate) fn decode(source: &str) -> Option<HeadersSnapshot> {
    let value: serde_json::Value = serde_json::from_str(source).ok()?;
    let object = value.as_object()?;
    if !object.contains_key("headers") && !object.contains_key("cookies") {
        return None;
    }
    serde_json::from_value(value).ok()
}

pub(crate) fn samesite_is_valid(samesite: &Option<String>) -> bool {
    matches!(
        samesite.as_deref().map(str::to_ascii_lowercase).as_deref(),
        Some("strict") | Some("lax") | Some("none")
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn malformed_header_snapshot_is_rejected() {
        assert!(super::decode(r#"{"headers":"#).is_none());
    }

    #[test]
    fn arbitrary_json_object_is_not_a_header_snapshot() {
        assert!(super::decode(r#"{"kind":"branded-scalar"}"#).is_none());
    }
}
