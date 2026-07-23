//! BOUNDARY-INVARIANT: this boundary module validates raw wire values and converts only through typed domain contracts.
//! Azure storage snapshot decoding boundary.
//! Malformed JSON is rejected, with negative coverage in this module's tests.

#[derive(Debug, serde::Deserialize)]
pub(crate) struct StorageAccountSnapshot {
    #[serde(rename = "name")]
    name: Option<String>,
    #[serde(rename = "allow_blob_public_access")]
    pub(crate) allow_blob_public_access: Option<bool>,
    #[serde(rename = "enable_https_traffic_only")]
    pub(crate) enable_https_traffic_only: Option<bool>,
    #[serde(rename = "minimum_tls_version")]
    pub(crate) minimum_tls_version: Option<String>,
}

impl StorageAccountSnapshot {
    pub(crate) fn label(&self) -> &str {
        self.name.as_deref().unwrap_or("<unnamed>")
    }
}

pub(crate) fn decode(source: &str) -> Option<StorageAccountSnapshot> {
    serde_json::from_str(source).ok()
}

#[cfg(test)]
mod tests {
    #[test]
    fn malformed_storage_snapshot_is_rejected() {
        assert!(super::decode(r#"{"name":"unfinished""#).is_none());
    }
}
