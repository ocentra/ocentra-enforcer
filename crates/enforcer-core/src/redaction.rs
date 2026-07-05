//! Two-layer redaction over structured records.
//!
//! RECONCILED 2026-07-05 against the real OcentraParent `logging-core`
//! source (reachable at vendor time; unreachable when this module was
//! first written, per lesson L12): the canonical `redact_fields` there is
//! SINGLE-layer only (key-name matching over a flat `LogFields` map, no
//! value-pattern scanning, no recursion into nested structures). This
//! module is a deliberate, independent EXTENSION beyond that upstream
//! shape — not a partial or pending port — adding a second value-pattern
//! layer (regex secret detection in free text) and generalizing from the
//! flat `LogFields` type to arbitrary nested `serde_json::Value`, so any
//! structured record can be redacted, not just log lines.
//!
//! Layer 1 (key-name): any field whose key matches a sensitive-key fragment
//! has its entire value replaced.
//! Layer 2 (value-pattern): every remaining string value is scanned with
//! secret-detecting regexes and matching spans are replaced.
//!
//! BOTH layers ALWAYS run, in that order, over every record before it is
//! written anywhere. Neither layer alone is sufficient: key-matching misses
//! secrets embedded in free-text values, and value-patterns miss opaque
//! secrets stored under obvious keys.

use crate::error::{Error, Result};

/// Replacement marker written over redacted content.
pub const REDACTED: &str = "[REDACTED]";

/// Default sensitive key fragments (matched case-insensitively as
/// substrings of the field key).
const DEFAULT_KEY_FRAGMENTS: &[&str] = &[
    "password",
    "passwd",
    "secret",
    "token",
    "api_key",
    "apikey",
    "authorization",
    "credential",
    "private_key",
    "session_id",
    "cookie",
];

/// Default secret-detecting value patterns.
const DEFAULT_VALUE_PATTERNS: &[&str] = &[
    // Bearer-style auth headers embedded in strings.
    r"(?i)bearer\s+[A-Za-z0-9._~+/=-]{8,}",
    // PEM private key blocks.
    r"-----BEGIN [A-Z ]*PRIVATE KEY-----",
    // GitHub personal access tokens.
    r"gh[pousr]_[A-Za-z0-9]{20,}",
    // AWS access key ids.
    r"AKIA[0-9A-Z]{16}",
    // Generic `key=value`-style secret assignments inside free text.
    r"(?i)(password|secret|token|api[_-]?key)\s*[:=]\s*\S+",
];

/// Two-layer structured-record redactor.
#[derive(Debug)]
pub struct Redactor {
    key_fragments: Vec<String>,
    value_patterns: Vec<regex::Regex>,
}

impl Redactor {
    /// Build a redactor with the default key fragments and value patterns.
    pub fn with_defaults() -> Result<Self> {
        Self::new(
            DEFAULT_KEY_FRAGMENTS.iter().map(|s| (*s).to_owned()),
            DEFAULT_VALUE_PATTERNS.iter().copied(),
        )
    }

    /// Build a redactor from custom key fragments and value-pattern sources.
    pub fn new(
        key_fragments: impl IntoIterator<Item = String>,
        value_patterns: impl IntoIterator<Item = &'static str>,
    ) -> Result<Self> {
        let key_fragments: Vec<String> = key_fragments
            .into_iter()
            .map(|f| f.to_ascii_lowercase())
            .collect();
        let mut compiled = Vec::new();
        for pattern in value_patterns {
            let re = regex::Regex::new(pattern)
                .map_err(|e| Error::InvalidConfig(format!("bad redaction pattern: {e}")))?;
            compiled.push(re);
        }
        Ok(Self {
            key_fragments,
            value_patterns: compiled,
        })
    }

    /// Redact a structured record in place. Both layers always run.
    pub fn redact(&self, value: &mut serde_json::Value) {
        self.redact_key_layer(value);
        self.redact_value_layer(value);
    }

    /// Layer 1: replace the entire value of any field whose key matches a
    /// sensitive fragment.
    fn redact_key_layer(&self, value: &mut serde_json::Value) {
        match value {
            serde_json::Value::Object(map) => {
                for (key, entry) in map.iter_mut() {
                    if self.key_is_sensitive(key) {
                        *entry = serde_json::Value::String(REDACTED.to_owned());
                    } else {
                        self.redact_key_layer(entry);
                    }
                }
            }
            serde_json::Value::Array(items) => {
                for item in items.iter_mut() {
                    self.redact_key_layer(item);
                }
            }
            _ => {}
        }
    }

    /// Layer 2: scan every string value with the secret-detecting patterns
    /// and replace matching spans.
    fn redact_value_layer(&self, value: &mut serde_json::Value) {
        match value {
            serde_json::Value::String(s) => {
                let mut current = s.clone();
                for re in &self.value_patterns {
                    if re.is_match(&current) {
                        current = re.replace_all(&current, REDACTED).into_owned();
                    }
                }
                *s = current;
            }
            serde_json::Value::Object(map) => {
                for (_, entry) in map.iter_mut() {
                    self.redact_value_layer(entry);
                }
            }
            serde_json::Value::Array(items) => {
                for item in items.iter_mut() {
                    self.redact_value_layer(item);
                }
            }
            _ => {}
        }
    }

    fn key_is_sensitive(&self, key: &str) -> bool {
        let lowered = key.to_ascii_lowercase();
        self.key_fragments.iter().any(|f| lowered.contains(f))
    }
}

#[cfg(test)]
mod tests {
    use super::{Redactor, REDACTED};
    use crate::error::Result;

    #[test]
    fn key_layer_redacts_sensitive_keys() -> Result<()> {
        let redactor = Redactor::with_defaults()?;
        let mut record = serde_json::json!({
            "user": "alice",
            "password": "hunter2",
            "nested": { "api_key": "abc123", "note": "fine" },
            "items": [ { "authToken": "xyz" } ]
        });
        redactor.redact(&mut record);
        assert_eq!(record["password"], REDACTED);
        assert_eq!(record["nested"]["api_key"], REDACTED);
        assert_eq!(record["items"][0]["authToken"], REDACTED);
        assert_eq!(record["user"], "alice");
        assert_eq!(record["nested"]["note"], "fine");
        Ok(())
    }

    #[test]
    fn value_layer_redacts_secret_patterns_in_free_text() -> Result<()> {
        let redactor = Redactor::with_defaults()?;
        let mut record = serde_json::json!({
            "log_line": "calling api with Bearer abcdef123456789 now",
            "aws": "key AKIAIOSFODNN7EXAMPLE in flight",
            "assignment": "password=supersecret rest",
            "clean": "nothing to see here"
        });
        redactor.redact(&mut record);
        let log_line = record["log_line"].as_str().unwrap_or_default();
        assert!(log_line.contains(REDACTED));
        assert!(!log_line.contains("abcdef123456789"));
        let aws = record["aws"].as_str().unwrap_or_default();
        assert!(aws.contains(REDACTED));
        assert!(!aws.contains("AKIAIOSFODNN7EXAMPLE"));
        let assignment = record["assignment"].as_str().unwrap_or_default();
        assert!(!assignment.contains("supersecret"));
        assert_eq!(record["clean"], "nothing to see here");
        Ok(())
    }

    #[test]
    fn both_layers_run_on_the_same_record() -> Result<()> {
        let redactor = Redactor::with_defaults()?;
        let mut record = serde_json::json!({
            "secret": "opaque-value",
            "message": "auth used Bearer tok_abcdef0123456789"
        });
        redactor.redact(&mut record);
        // Key layer hit.
        assert_eq!(record["secret"], REDACTED);
        // Value layer hit on a non-sensitive key.
        let message = record["message"].as_str().unwrap_or_default();
        assert!(message.contains(REDACTED));
        assert!(!message.contains("tok_abcdef0123456789"));
        Ok(())
    }

    #[test]
    fn clean_record_passes_through_unchanged() -> Result<()> {
        let redactor = Redactor::with_defaults()?;
        let original = serde_json::json!({
            "event": "scan_complete",
            "files": 12,
            "status": "ok",
            "detail": { "durations_ms": [3, 5, 8] }
        });
        let mut record = original.clone();
        redactor.redact(&mut record);
        assert_eq!(record, original);
        Ok(())
    }
}
