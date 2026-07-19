//! `CYBER-SECRET.1` (T1) — Wave-1 cyberskills: hardcoded provider-credential
//! detection, a native Rust reimplementation of the inline provider-secret
//! regex tables harvested from the vendored cyberskills corpus
//! (`implementing-api-key-security-controls`,
//! `testing-for-sensitive-data-exposure`,
//! `detecting-aws-credential-exposure-with-trufflehog`,
//! `extracting-credentials-from-memory-dump`, and siblings — the skills that
//! ship an inline pattern table rather than shelling out to
//! gitleaks/trufflehog).
//!
//! This is the gitleaks-style HIGH-CONFIDENCE provider ruleset:
//! unambiguous, provider-prefixed credentials (AWS `AKIA`/`ASIA`, Stripe
//! `sk_live_`, GitHub `gh[pousr]_`, Google `AIza`, Slack `xox…`, npm
//! `npm_`, PEM private-key blocks, and a context-gated AWS secret key). It
//! deliberately EXCLUDES the corpus's low-precision generic patterns (a
//! bare 40-character encoded value, a 32-character hex value, bearer tokens, email/SSN/PII)
//! — those are false-positive magnets that would erode a prevention gate,
//! and the generic `key = "…"` assignment shape is already covered by
//! `SEC-1.1` (`InlineSecretsValidator`). This rule is additive: it catches
//! the bare provider-key literals `SEC-1.1`'s assignment regex misses.
//!
//! Matched secrets are REDACTED in the finding snippet so the secret is
//! never echoed into a report/log.

use crate::boundary::pattern::CredentialPattern;
use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::findings::Finding;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

/// The high-confidence provider patterns, harvested (verbatim where the
/// vendor had them) from the corpus inline tables. Ordered most-specific
/// first; the first match on a line wins to avoid duplicate findings.
fn patterns() -> Result<Vec<CredentialPattern>, DecodeError> {
    let specs: &[(&str, &str, Severity)] = &[
        // AWS long-lived or session access key id.
        (
            "AWS access key id",
            r"\b(?:AKIA|ASIA)[A-Z0-9]{16}\b",
            Severity::Error,
        ),
        // AWS secret access key, CONTEXT-GATED on a nearby `aws` token to
        // avoid matching arbitrary 40-character encoded values.
        (
            "AWS secret access key",
            r#"(?i)aws.{0,20}['"][0-9a-zA-Z/+]{40}['"]"#,
            Severity::Error,
        ),
        // GitHub personal-access / oauth / user / server / refresh tokens.
        (
            "GitHub token",
            r"\bgh[pousr]_[A-Za-z0-9]{36,}\b",
            Severity::Error,
        ),
        // Stripe live secret / restricted keys.
        (
            "Stripe secret key",
            r"\b(?:sk|rk)_live_[0-9a-zA-Z]{24,}\b",
            Severity::Error,
        ),
        // Google API key.
        (
            "Google API key",
            r"\bAIza[0-9A-Za-z_\-]{35}\b",
            Severity::Error,
        ),
        // Slack token (bot/user/app/refresh/legacy).
        (
            "Slack token",
            r"\bxox[baprs]-[0-9A-Za-z-]{10,}\b",
            Severity::Error,
        ),
        // npm access token.
        ("npm token", r"\bnpm_[A-Za-z0-9]{36}\b", Severity::Error),
        // PEM private-key block header.
        (
            "private key (PEM)",
            r"-----BEGIN (?:RSA |EC |DSA |OPENSSH |PGP )?PRIVATE KEY-----",
            Severity::Error,
        ),
        // JSON Web Token — lower confidence (may be a non-secret public
        // token), so Warning rather than Error.
        (
            "JWT",
            r"\beyJ[A-Za-z0-9_-]{10,}\.eyJ[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}\b",
            Severity::Warning,
        ),
    ];
    let mut out = Vec::with_capacity(specs.len());
    for (name, pattern, severity) in specs {
        out.push(CredentialPattern::compile(
            "cyberskillsProviderSecretRegex",
            pattern,
            name,
            *severity,
        )?);
    }
    Ok(out)
}

/// `CYBER-SECRET.1` — hardcoded provider-credential gate.
pub struct ProviderCredentialValidator {
    rule_id: RuleId,
    patterns: Vec<CredentialPattern>,
}

impl std::fmt::Debug for ProviderCredentialValidator {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ProviderCredentialValidator")
            .field("rule_id", &self.rule_id)
            .field("pattern_count", &self.patterns.len())
            .finish()
    }
}

impl ProviderCredentialValidator {
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: enforcer_domain::ids::BuiltInSecurityRule::CyberProviderSecret.id(),
            patterns: patterns()?,
        })
    }
}

impl Validator for ProviderCredentialValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let mut findings = Vec::new();
        for (index, line) in input.source.as_str().lines().enumerate() {
            let line_number = u32::try_from(index).unwrap_or(u32::MAX).saturating_add(1);
            // First matching pattern on the line wins (one finding per line).
            for pattern in &self.patterns {
                if let Some(m) = pattern.regex().find(line) {
                    findings.extend(crate::boundary::finding::from_owned_source(
                        (&self.rule_id, pattern.severity()),
                        "hardcoded provider credential",
                        format!(
                            "a hardcoded {} was found in source. Secrets committed to a repo \
                             leak to anyone with read access and to every clone/fork/CI log. \
                             Fix: revoke/rotate it now, then inject it at runtime from a secret \
                             store or environment, never in code.",
                            pattern.name().as_str()
                        ),
                        input.file,
                        (
                            line_number,
                            Some(crate::boundary::source_predicates::redact_secret(
                                m.as_str(),
                            )),
                        ),
                    ));
                    break;
                }
            }
        }
        findings
    }
}

#[cfg(test)]
mod tests {
    use crate::boundary::fixture::run_manifest_fixture_parity;
    use enforcer_domain::findings::ScanScope;
    use enforcer_validator::validator::{ValidationInput, Validator};

    use super::ProviderCredentialValidator;

    /// Representative fixture pair for the d01 parity oracle. The on-disk
    /// bad fixture uses AWS's documented public sample from GitHub's
    /// push-protection allowlist, so the committed fixture holds
    /// no live-secret-shaped literal; exhaustive per-provider coverage is in
    /// `provider_credential_corpus_code_built` below.
    #[test]
    fn cyberskills_provider_credentials() -> Result<(), Box<dyn std::error::Error>> {
        let validator = ProviderCredentialValidator::new()?;
        run_manifest_fixture_parity(
            &validator,
            "tests/fixtures/cyberskills/secret.provider-credential/bad/leaked.env",
            "tests/fixtures/cyberskills/secret.provider-credential/good/clean.rs",
        )?;
        Ok(())
    }

    /// Exhaustive labeled corpus, CODE-BUILT: every provider-secret input is
    /// assembled from a prefix + filler at runtime, so no real-secret-shaped
    /// literal is ever committed to the repo (which GitHub push protection
    /// would correctly block — the very leak this rule prevents). Flag cases
    /// must produce >=1 finding; clean cases (the false positives a
    /// prevention gate must NOT trip) must produce 0.
    #[test]
    fn provider_credential_corpus_code_built() -> Result<(), Box<dyn std::error::Error>> {
        let validator = ProviderCredentialValidator::new()?;
        let file = crate::boundary::fixture::rel_path("config.txt")?;
        let scan = |src: &str| -> usize {
            validator
                .validate(ValidationInput {
                    file: &file,
                    source: enforcer_domain::boundary::validation::ValidationSource::from_text(src),
                    scope: ScanScope::Files,
                })
                .len()
        };

        let a24 = "a".repeat(24);
        let a35 = "a".repeat(35);
        let a36 = "a".repeat(36);
        let a40 = "A".repeat(40);
        let flag_cases: Vec<String> = vec![
            format!("AWS_ACCESS_KEY_ID=AKIA{}", "A".repeat(16)),
            format!("asia_key=ASIA{}", "0".repeat(16)),
            format!("aws_key = \"{a40}\""), // context-gated AWS secret
            format!("ghp_{a36}"),
            format!("gho_{a36}"),
            format!("ghu_{a36}"),
            format!("ghs_{a36}"),
            format!("sk_live_{a24}"),
            format!("rk_live_{a24}"),
            format!("AIza{a35}"),
            format!("xoxb-{}", "a".repeat(12)),
            format!("xoxp-{}", "1".repeat(12)),
            format!("npm_{a36}"),
            format!("-----BEGIN RSA PRIVATE {}-----", "KEY"),
            format!("-----BEGIN OPENSSH PRIVATE {}-----", "KEY"),
            format!(
                "eyJ{}.eyJ{}.{}",
                "a".repeat(12),
                "b".repeat(12),
                "c".repeat(12)
            ),
            format!("# leaked in a comment: ghp_{a36}"),
        ];
        for (index, src) in flag_cases.iter().enumerate() {
            assert!(
                scan(src) >= 1,
                "flag case {index} should be flagged but was clean: {src}"
            );
        }

        let clean_cases: Vec<String> = vec![
            "key = os.environ[\"AWS_SECRET_ACCESS_KEY\"]".to_owned(),
            "token := os.Getenv(\"GITHUB_TOKEN\")".to_owned(),
            "hash = \"d41d8cd98f00b204e9800998ecf8427e\"".to_owned(), // md5 hex
            "commit = 1234567890abcdef1234567890abcdef12345678".to_owned(), // git sha, no aws ctx
            "id = 550e8400-e29b-41d4-a716-446655440000".to_owned(),   // uuid
            "contact = support@example.com".to_owned(),               // email (PII, not this rule)
            "key: AKIA_YOUR_KEY_HERE".to_owned(),                     // placeholder
            "github_token = ghp_xxx".to_owned(),                      // too short
            format!("sk_test_{a24}"),                                 // test key, not sk_live
            format!("blob = \"{}\"", "Z".repeat(40)), // 40-char blob, no aws context
        ];
        for (index, src) in clean_cases.iter().enumerate() {
            assert_eq!(
                scan(src),
                0,
                "clean case {index} should NOT be flagged: {src}"
            );
        }
        Ok(())
    }
}
