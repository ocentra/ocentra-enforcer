//! `common/secret-scan` validators: `SEC-1.1` (inline secrets forbidden)
//! and `SEC-1.2` (sensitive files forbidden). Both `lockLevel: immutable`,
//! `canDisable`/`canDowngrade: false` in `rules/rules.json` — the strictest
//! tier, ported verbatim (pattern-for-pattern) from:
//!
//! - `src/generic-common-line-rules.mjs`'s `scanSecretLine` (the
//!   `OPENAI_KEY_RE`/`SECRET_RE` branches that fire `SEC-1.1`), and
//! - `src/source-policy-common-security-sensitive.mjs`'s
//!   `scanSensitivePathPolicy` (which fires `SEC-1.2` via
//!   `isForbiddenSensitivePath` in `src/source-policy-helpers.mjs`).

use enforcer_core::error::DecodeError;
use enforcer_domain::findings::Finding;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};
use regex::Regex;

use super::spec::redact_line;
use super::text_scan::{is_comment_only_line, lines};

/// `SEC-1.1` — inline secrets are forbidden (`appliesTo: **/*`).
///
/// Fires on either an OpenAI-shaped key (`sk-...`/`sk-proj-...`) or a
/// generic `<api_key|secret|token|password|private_key> = "<16+ chars>"`
/// assignment, mirroring the JS source's `OPENAI_KEY_RE` and `SECRET_RE`
/// exactly (case-insensitive, same character classes/length floors).
pub struct InlineSecretsValidator {
    rule_id: RuleId,
    openai_key: Regex,
    generic_secret: Regex,
}

impl InlineSecretsValidator {
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: "SEC-1.1".parse()?,
            #[allow(clippy::unwrap_used)]
            openai_key: Regex::new(r"\bsk-(?:proj-)?[A-Za-z0-9_-]{20,}\b").unwrap(),
            #[allow(clippy::unwrap_used)]
            generic_secret: Regex::new(
                r#"(?i)\b(?:[A-Z0-9_/-]*(?:api[_-]?key|secret|token|password|private[_-]?key))\b\s*[:=]\s*["'][A-Za-z0-9_./+=:@-]{16,}["']"#,
            )
            .unwrap(),
        })
    }
}

impl Validator for InlineSecretsValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let mut findings = Vec::new();
        for line in lines(input.source) {
            if is_comment_only_line(line.text) {
                continue;
            }
            let hit = if self.openai_key.is_match(line.text) {
                Some("OpenAI key found.")
            } else if self.generic_secret.is_match(line.text) {
                Some("Inline secret-like assignment found.")
            } else {
                None
            };
            if let Some(detail) = hit {
                findings.push(Finding {
                    rule_id: self.rule_id.clone(),
                    severity: Severity::Error,
                    title: "Inline secrets are forbidden".to_owned(),
                    detail: detail.to_owned(),
                    file: input.file.clone(),
                    line: line.number,
                    snippet: Some(redact_line(line.text)),
                });
            }
        }
        findings
    }
}

/// `SEC-1.2` — sensitive files are forbidden in source scope.
///
/// A path-only check (no line scan): the file's repo-relative path is
/// tested against the same allow/forbid pattern pair as
/// `isForbiddenSensitivePath` in `src/source-policy-helpers.mjs` — allowed
/// `.env.example`/`.env.sample`/`.env.template` paths are excluded FIRST
/// (checked before the forbidden set, matching the JS `some(allowed) ->
/// return false` short-circuit), then the forbidden set (`.env*`,
/// `google-services.json`, `GoogleService-Info.plist`, `id_rsa[.pub]`,
/// `*.pem`/`*.p12`/`*.pfx`/`*.key`) is tested.
pub struct SensitiveFilesValidator {
    rule_id: RuleId,
    allowed: Vec<Regex>,
    forbidden: Vec<Regex>,
}

impl SensitiveFilesValidator {
    pub fn new() -> Result<Self, DecodeError> {
        #[allow(clippy::unwrap_used)]
        let allowed = vec![
            Regex::new(r"(?i)(^|/)\.env\.example$").unwrap(),
            Regex::new(r"(?i)(^|/)\.env\.sample$").unwrap(),
            Regex::new(r"(?i)(^|/)\.env\.template$").unwrap(),
        ];
        #[allow(clippy::unwrap_used)]
        let forbidden = vec![
            Regex::new(r"(?i)(^|/)\.env(\..+)?$").unwrap(),
            Regex::new(r"(?i)(^|/)google-services\.json$").unwrap(),
            Regex::new(r"(^|/)GoogleService-Info\.plist$").unwrap(),
            Regex::new(r"(?i)(^|/)id_rsa(\.pub)?$").unwrap(),
            Regex::new(r"(?i)\.(pem|p12|pfx|key)$").unwrap(),
        ];
        Ok(Self {
            rule_id: "SEC-1.2".parse()?,
            allowed,
            forbidden,
        })
    }

    fn is_forbidden(&self, rel: &str) -> bool {
        if self.allowed.iter().any(|pattern| pattern.is_match(rel)) {
            return false;
        }
        self.forbidden.iter().any(|pattern| pattern.is_match(rel))
    }
}

impl Validator for SensitiveFilesValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let rel = input.file.as_str();
        if self.is_forbidden(rel) {
            vec![Finding {
                rule_id: self.rule_id.clone(),
                severity: Severity::Error,
                title: "Sensitive files are forbidden in source scope".to_owned(),
                detail: "forbidden sensitive file path".to_owned(),
                file: input.file.clone(),
                line: 1,
                snippet: Some(rel.to_owned()),
            }]
        } else {
            Vec::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use enforcer_domain::findings::ScanScope;
    use enforcer_domain::paths::RelPath;
    use enforcer_validator::harness::run_fixture_parity;

    use super::{InlineSecretsValidator, SensitiveFilesValidator};
    use enforcer_validator::validator::{ValidationInput, Validator};

    fn manifest_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    fn rel(path: &str) -> RelPath {
        #[allow(clippy::unwrap_used)]
        path.parse().unwrap()
    }

    #[test]
    fn inline_secrets_fires_on_openai_key() -> Result<(), Box<dyn std::error::Error>> {
        let validator = InlineSecretsValidator::new()?;
        let file = rel("src/config.rs");
        let findings = validator.validate(ValidationInput {
            file: &file,
            source: "let key = \"sk-proj-abcdefghijklmnopqrstuvwx\";",
            scope: ScanScope::Files,
        });
        assert_eq!(findings.len(), 1);
        Ok(())
    }

    #[test]
    fn inline_secrets_silent_on_placeholder() -> Result<(), Box<dyn std::error::Error>> {
        let validator = InlineSecretsValidator::new()?;
        let file = rel("src/config.rs");
        let findings = validator.validate(ValidationInput {
            file: &file,
            source: "let api_key = std::env::var(\"API_KEY\")?;",
            scope: ScanScope::Files,
        });
        assert!(findings.is_empty());
        Ok(())
    }

    #[test]
    fn sensitive_files_fires_on_dotenv() -> Result<(), Box<dyn std::error::Error>> {
        let validator = SensitiveFilesValidator::new()?;
        let file = rel(".env");
        let findings = validator.validate(ValidationInput {
            file: &file,
            source: "SECRET=abc",
            scope: ScanScope::Files,
        });
        assert_eq!(findings.len(), 1);
        Ok(())
    }

    #[test]
    fn sensitive_files_allows_dotenv_example() -> Result<(), Box<dyn std::error::Error>> {
        let validator = SensitiveFilesValidator::new()?;
        let file = rel(".env.example");
        let findings = validator.validate(ValidationInput {
            file: &file,
            source: "SECRET=changeme",
            scope: ScanScope::Files,
        });
        assert!(findings.is_empty());
        Ok(())
    }

    #[test]
    fn sec_1_1_inline_secrets_fixture_parity() -> Result<(), Box<dyn std::error::Error>> {
        let validator = InlineSecretsValidator::new()?;
        run_fixture_parity(
            &validator,
            &manifest_dir(),
            "fixtures/sec-1-1/fail.rs",
            "fixtures/sec-1-1/pass.rs",
        )?;
        Ok(())
    }

    #[test]
    fn sec_1_2_sensitive_files_fixture_parity() -> Result<(), Box<dyn std::error::Error>> {
        let validator = SensitiveFilesValidator::new()?;
        run_fixture_parity(
            &validator,
            &manifest_dir(),
            "fixtures/sec-1-2/fail/.env",
            "fixtures/sec-1-2/pass/.env.example",
        )?;
        Ok(())
    }
}
