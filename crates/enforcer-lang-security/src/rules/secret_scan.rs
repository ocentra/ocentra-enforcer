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

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::findings::Finding;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};
use regex::Regex;

use crate::boundary::spec::redact_line;
use crate::boundary::text_scan::{is_comment_only_line, lines};

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

impl std::fmt::Debug for InlineSecretsValidator {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("InlineSecretsValidator")
            .field("rule_id", &self.rule_id)
            .field("patterns", &"<redacted>")
            .finish()
    }
}

impl InlineSecretsValidator {
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: enforcer_domain::ids::BuiltInSecurityRule::Sec1Rule1.id(),
            openai_key: crate::boundary::regex::compile(
                "sec1OpenAiKey",
                r"\bsk-(?:proj-)?[A-Za-z0-9_-]{20,}\b",
            )?,
            generic_secret: crate::boundary::regex::compile(
                "sec1GenericSecret",
                r#"(?i)\b(?:[A-Z0-9_/-]*(?:api[_-]?key|secret|token|password|private[_-]?key))\b\s*[:=]\s*["'][A-Za-z0-9_./+=:@-]{16,}["']"#,
            )?,
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
            let hit = if self.openai_key.is_match(line.text.as_str()) {
                Some("OpenAI key found.")
            } else if self.generic_secret.is_match(line.text.as_str()) {
                Some("Inline secret-like assignment found.")
            } else {
                None
            };
            if let Some(detail) = hit {
                findings.extend(crate::boundary::finding::from_owned_source(
                    (&self.rule_id, Severity::Error),
                    "Inline secrets are forbidden",
                    detail,
                    input.file,
                    (line.number, Some(redact_line(line.text.as_str()))),
                ));
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
#[derive(Debug)]
pub struct SensitiveFilesValidator {
    rule_id: RuleId,
    allowed: Vec<Regex>,
    forbidden: Vec<Regex>,
}

impl SensitiveFilesValidator {
    pub fn new() -> Result<Self, DecodeError> {
        let allowed = vec![
            crate::boundary::regex::compile("sec1EnvExample", r"(?i)(^|/)\.env\.example$")?,
            crate::boundary::regex::compile("sec1EnvSample", r"(?i)(^|/)\.env\.sample$")?,
            crate::boundary::regex::compile("sec1EnvTemplate", r"(?i)(^|/)\.env\.template$")?,
        ];
        let forbidden = vec![
            crate::boundary::regex::compile("sec1Env", r"(?i)(^|/)\.env(\..+)?$")?,
            crate::boundary::regex::compile(
                "sec1GoogleServices",
                r"(?i)(^|/)google-services\.json$",
            )?,
            crate::boundary::regex::compile(
                "sec1GoogleServiceInfo",
                r"(^|/)GoogleService-Info\.plist$",
            )?,
            crate::boundary::regex::compile("sec1IdRsa", r"(?i)(^|/)id_rsa(\.pub)?$")?,
            crate::boundary::regex::compile(
                "sec1PrivateKeyExtension",
                r"(?i)\.(pem|p12|pfx|key)$",
            )?,
        ];
        Ok(Self {
            rule_id: enforcer_domain::ids::BuiltInSecurityRule::Sec1Rule2.id(),
            allowed,
            forbidden,
        })
    }
}

impl Validator for SensitiveFilesValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let rel = input.file.as_str();
        if crate::boundary::source_predicates::path_is_forbidden(
            input.file,
            &self.allowed,
            &self.forbidden,
        ) {
            crate::boundary::finding::from_source(
                (&self.rule_id, Severity::Error),
                "Sensitive files are forbidden in source scope",
                "forbidden sensitive file path",
                input.file,
                (1, Some(rel)),
            )
            .into_iter()
            .collect()
        } else {
            Vec::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::boundary::fixture::run_manifest_fixture_parity;
    use enforcer_domain::findings::ScanScope;

    use super::{InlineSecretsValidator, SensitiveFilesValidator};
    use enforcer_validator::validator::{ValidationInput, Validator};

    #[test]
    fn inline_secrets_fires_on_openai_key() -> Result<(), Box<dyn std::error::Error>> {
        let validator = InlineSecretsValidator::new()?;
        let file = crate::boundary::fixture::rel_path("src/config.rs")?;
        let source = ["let key = \"sk-", "proj-abcdefghijklmnopqrstuvwx\";"].concat();
        let findings = validator.validate(ValidationInput {
            file: &file,
            source: enforcer_domain::boundary::validation::ValidationSource::from_text(&source),
            scope: ScanScope::Files,
        });
        assert_eq!(findings.len(), 1);
        Ok(())
    }

    #[test]
    fn inline_secrets_silent_on_placeholder() -> Result<(), Box<dyn std::error::Error>> {
        let validator = InlineSecretsValidator::new()?;
        let file = crate::boundary::fixture::rel_path("src/config.rs")?;
        let findings = validator.validate(ValidationInput {
            file: &file,
            source: enforcer_domain::boundary::validation::ValidationSource::from_text(
                "let api_key = std::env::var(\"API_KEY\")?;",
            ),
            scope: ScanScope::Files,
        });
        assert!(findings.is_empty());
        Ok(())
    }

    #[test]
    fn sensitive_files_fires_on_dotenv() -> Result<(), Box<dyn std::error::Error>> {
        let validator = SensitiveFilesValidator::new()?;
        let file = crate::boundary::fixture::rel_path(".env")?;
        let findings = validator.validate(ValidationInput {
            file: &file,
            source: enforcer_domain::boundary::validation::ValidationSource::from_text(
                "SECRET=abc",
            ),
            scope: ScanScope::Files,
        });
        assert_eq!(findings.len(), 1);
        Ok(())
    }

    #[test]
    fn sensitive_files_allows_dotenv_example() -> Result<(), Box<dyn std::error::Error>> {
        let validator = SensitiveFilesValidator::new()?;
        let file = crate::boundary::fixture::rel_path(".env.example")?;
        let findings = validator.validate(ValidationInput {
            file: &file,
            source: enforcer_domain::boundary::validation::ValidationSource::from_text(
                "SECRET=changeme",
            ),
            scope: ScanScope::Files,
        });
        assert!(findings.is_empty());
        Ok(())
    }

    #[test]
    fn sec_1_1_inline_secrets_fixture_parity() -> Result<(), Box<dyn std::error::Error>> {
        let validator = InlineSecretsValidator::new()?;
        run_manifest_fixture_parity(
            &validator,
            "fixtures/sec-1-1/fail.rs",
            "fixtures/sec-1-1/pass.rs",
        )?;
        Ok(())
    }

    #[test]
    fn sec_1_2_sensitive_files_fixture_parity() -> Result<(), Box<dyn std::error::Error>> {
        let validator = SensitiveFilesValidator::new()?;
        run_manifest_fixture_parity(
            &validator,
            "fixtures/sec-1-2/fail/.env",
            "fixtures/sec-1-2/pass/.env.example",
        )?;
        Ok(())
    }
}
