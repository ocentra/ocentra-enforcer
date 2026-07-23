//! The SECURITY slice of the shared `generic-scanner` engine (`SEC-2.1`..
//! `SEC-2.20`, 20 rules). This crate owns only this SEC-2 slice's rule
//! specs, NOT the shared `generic-scanner` engine itself — the engine and
//! its common/python/typescript partition are owned by arc-09 (see this
//! workpack's `ENGINE BOUNDARY` note and `enforcer-lang-ts::rules::spec`'s
//! doc comment for the sibling exemplar).
//!
//! `rules/rules.json`'s `triggers` field for every rule in this slice
//! restates the rule TITLE rather than giving a literal source pattern
//! (the same generic-scanner authoring artifact arc-07's mem-arc-07-0002
//! flagged for the TS slice) — each spec below therefore encodes the
//! actual detection regex ported from `src/generic-scanner-shared.mjs`'s
//! `COMMON_SECRET_RULES` table and `src/generic-common-line-rules.mjs`'s
//! `scanSecretLine`, not a copy of the JSON `triggers` string.
//!
//! Three of the 20 rows use a validator name OTHER than the literal
//! string `generic-scanner` in `rules/rules.json` (`SEC-2.15` is
//! `generic-scanner-redaction`; `SEC-2.16`/`.17`/`.18` are
//! `common/security`) — all four are still content-line-scan rules with
//! the identical shape as the other 16, so this crate treats them as one
//! data-driven family rather than splitting into separate modules.
//!
//! Several rows encode a JS-source `condition && !markerPresent(line)`
//! shape (SEC-2.11/.12/.14 "real secret WITHOUT an allowed safe-value
//! marker"; SEC-2.16/.17/.18 "tool invocation WITHOUT its required
//! flag"). Rust's `regex` crate has no negative lookaround, so those rows
//! use [`RuleSpec::suppressed_by_any_of`]: the base `pattern` matches the
//! positive shape, and a hit is suppressed if the marker pattern ALSO
//! matches the same line — the two-regex equivalent of the JS `&& !`.

use crate::boundary::spec::{RuleBehavior, RuleSpec};
use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::ids::BuiltInSecurityRule;

const ANY_SECRET_PATTERN: &str = r#"(?i)\b(?:[A-Z0-9_/-]*(?:api[_-]?key|secret|token|password|private[_-]?key))\b\s*[:=]\s*["'][A-Za-z0-9_./+=:@-]{16,}["']"#;
const ENV_PLACEHOLDER_ALLOWED_PATTERN: &str =
    r"(?i)example|placeholder|changeme|replace_me|dummy|counterfeit|test|<[^>]+>|\$\{[^}]+\}";
const FIXTURE_MARKER_ALLOWED_PATTERN: &str = r"(?i)\bfake\b|\bfixture\b|\bexample\b";

/// Build every one of the 20 `SEC-2.*` specs. Fails closed (propagates the
/// first regex-compile error) rather than constructing a partially valid
/// table — see [`super::registry::build_all`] for the caller contract.
pub(crate) fn specs() -> Result<Vec<RuleSpec>, DecodeError> {
    let compile = |pattern| crate::boundary::regex::compile("securityRulePattern", pattern);
    let any_secret = ANY_SECRET_PATTERN;
    let env_placeholder_allowed = ENV_PLACEHOLDER_ALLOWED_PATTERN;
    let fixture_marker_allowed = FIXTURE_MARKER_ALLOWED_PATTERN;

    let specs = [
        // SEC-2.1 — GitHub tokens (`ghp_`/`gho_`/`ghu_`/`ghs_`/`ghr_...`).
        RuleSpec::new(
            BuiltInSecurityRule::Sec2Rule1,
            "GitHub tokens are forbidden",
            "GitHub token found.",
            compile(r"\bgh[pousr]_[A-Za-z0-9_]{20,}\b")?,
            RuleBehavior::content(),
        ),
        // SEC-2.2 — AWS access keys (`AKIA` + 16 uppercase-alnum).
        RuleSpec::new(
            BuiltInSecurityRule::Sec2Rule2,
            "AWS access keys are forbidden",
            "AWS access key found.",
            compile(r"\bAKIA[0-9A-Z]{16}\b")?,
            RuleBehavior::content(),
        ),
        // SEC-2.3 — Google service account JSON markers.
        RuleSpec::new(
            BuiltInSecurityRule::Sec2Rule3,
            "Google service account JSON is forbidden",
            "Google service account JSON marker found.",
            compile(r#""type"\s*:\s*"service_account"|"private_key_id"\s*:"#)?,
            RuleBehavior::content(),
        ),
        // SEC-2.4 — Azure credential assignments.
        RuleSpec::new(
            BuiltInSecurityRule::Sec2Rule4,
            "Azure credentials are forbidden",
            "Azure credential assignment found.",
            compile(r"(?i)\bAZURE_(?:CLIENT_SECRET|TENANT_ID|CLIENT_ID)\b\s*[:=]")?,
            RuleBehavior::content(),
        ),
        // SEC-2.5 — Slack/Discord tokens.
        RuleSpec::new(
            BuiltInSecurityRule::Sec2Rule5,
            "Slack and Discord tokens are forbidden",
            "Slack or Discord token found.",
            compile(r"(?i)\bxox[baprs]-[A-Za-z0-9-]{10,}\b|discord(?:app)?\.[A-Za-z0-9_-]{20,}")?,
            RuleBehavior::content(),
        ),
        // SEC-2.6 — JWT-looking three-segment tokens.
        RuleSpec::new(
            BuiltInSecurityRule::Sec2Rule6,
            "JWT-looking secrets are forbidden",
            "JWT-looking token found.",
            compile(r"\beyJ[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}\b")?,
            RuleBehavior::content(),
        ),
        // SEC-2.7 — PEM-style private key blocks.
        RuleSpec::new(
            BuiltInSecurityRule::Sec2Rule7,
            "Private key blocks are forbidden",
            "private key block found.",
            compile(r"-----BEGIN (?:RSA |OPENSSH |EC |DSA |)?PRIVATE KEY-----")?,
            RuleBehavior::content(),
        ),
        // SEC-2.8 — npm/PyPI/Cargo registry tokens.
        RuleSpec::new(
            BuiltInSecurityRule::Sec2Rule8,
            "Package registry tokens are forbidden",
            "package registry token found.",
            compile(
                r"\b(?:npm_[A-Za-z0-9]{20,}|pypi-[A-Za-z0-9_-]{20,}|CARGO_REGISTRY_TOKEN\s*=)",
            )?,
            RuleBehavior::content(),
        ),
        // SEC-2.9 — Stripe live/test keys.
        RuleSpec::new(
            BuiltInSecurityRule::Sec2Rule9,
            "Stripe keys are forbidden",
            "Stripe key found.",
            compile(r"\b(?:sk|pk)_(?:live|test)_[A-Za-z0-9]{20,}\b")?,
            RuleBehavior::content(),
        ),
        // SEC-2.10 — high-entropy secret assignments (32+ character encoded
        // value assigned to a secret/token/password/key identifier).
        RuleSpec::new(
            BuiltInSecurityRule::Sec2Rule10,
            "High-entropy secret assignments are forbidden",
            "high-entropy secret assignment found.",
            compile(
                r#"(?i)\b(?:secret|token|password|key)\b\s*[:=]\s*["'][A-Za-z0-9+/=_-]{32,}["']"#,
            )?,
            RuleBehavior::content(),
        ),
        // SEC-2.11 — `.env.example` must carry only sentinel-looking
        // values: fire on a real-looking secret assignment UNLESS the line
        // also carries one of the safe-value forms recognized by
        // `ENV_PLACEHOLDER_ALLOWED` in the JS source.
        RuleSpec::new(
            BuiltInSecurityRule::Sec2Rule11,
            ".env.example may contain placeholders only",
            ".env.example contains a real-looking secret.",
            compile(any_secret)?,
            RuleBehavior::unguarded_content_suppressed(compile(env_placeholder_allowed)?),
        ),
        // SEC-2.12 — same shape as SEC-2.11 for `.env.template`.
        RuleSpec::new(
            BuiltInSecurityRule::Sec2Rule12,
            ".env.template may contain placeholders only",
            ".env.template contains a real-looking secret.",
            compile(any_secret)?,
            RuleBehavior::unguarded_content_suppressed(compile(env_placeholder_allowed)?),
        ),
        // SEC-2.13 — secret-looking values are forbidden in snapshot/test
        // artifacts. The JS source keys this off `context.isTestLike`, a
        // classification this crate's per-line `Validator` contract does not
        // carry; ported as an unconditional real-secret-shape check, with the
        // rule's fixtures living under a `snapshot`-shaped fixture path to
        // document the intended scope (arc-14+'s scan orchestration is
        // expected to route only test/snapshot files to this rule).
        RuleSpec::new(
            BuiltInSecurityRule::Sec2Rule13,
            "Secrets are forbidden in snapshots",
            "secret-looking value found in snapshot/test artifact.",
            compile(any_secret)?,
            RuleBehavior::unguarded_content(),
        ),
        // SEC-2.14 — fixture secrets require an explicit sentinel on the
        // same line: fire on a real-looking secret assignment UNLESS the line
        // also carries one of the accepted fixture sentinels.
        RuleSpec::new(
            BuiltInSecurityRule::Sec2Rule14,
            "Fixture secrets require counterfeit markers",
            "fixture secret lacks explicit counterfeit marker.",
            compile(any_secret)?,
            RuleBehavior::unguarded_content_suppressed(compile(fixture_marker_allowed)?),
        ),
        // SEC-2.15 — secret diagnostics must redact matched values. The
        // scanner's OWN output redaction is enforced structurally by
        // `super::spec::redact_line` (every finding's snippet is redacted
        // before construction), so this rule's fixtures prove the
        // CONTRAPOSITIVE: a line that looks like un-redacted diagnostic
        // output (carries a real secret-shaped literal) is the fail shape; a
        // `[REDACTED]` sentinel in the same position is the pass shape
        // (the shared secret-shape pattern requires 16+ literal chars inside
        // matching quotes, which `[REDACTED]` does not satisfy).
        RuleSpec::new(
            BuiltInSecurityRule::Sec2Rule15,
            "Secret diagnostics must redact matched values",
            "diagnostic output line carries an unredacted secret-looking value.",
            compile(any_secret)?,
            RuleBehavior::unguarded_content(),
        ),
        // SEC-2.16 — Gitleaks invocations must emit SARIF: fire on a
        // `gitleaks detect|protect|dir|git` command line UNLESS it also
        // carries `sarif` or `--report-format sarif`.
        RuleSpec::new(
            BuiltInSecurityRule::Sec2Rule16,
            "Secret scanners must emit SARIF",
            "Gitleaks command does not emit SARIF.",
            compile(r"(?i)\bgitleaks\s+(?:detect|protect|dir|git)\b")?,
            RuleBehavior::command_suppressed(compile(r"(?i)\bsarif\b|--report-format\s+sarif")?),
        ),
        // SEC-2.17 — Gitleaks findings must be normalized through
        // Enforcer/SARIF: same command shape as SEC-2.16, suppressed by
        // either `ocentra-enforcer` or `--report-format sarif`.
        RuleSpec::new(
            BuiltInSecurityRule::Sec2Rule17,
            "Gitleaks findings must be normalized",
            "Gitleaks command is not normalized through Enforcer/SARIF.",
            compile(r"(?i)\bgitleaks\s+(?:detect|protect|dir|git)\b")?,
            RuleBehavior::command_suppressed(compile(
                r"(?i)\bocentra-enforcer\b|--report-format\s+sarif",
            )?),
        ),
        // SEC-2.18 — TruffleHog findings must be normalized through
        // JSON/Enforcer: fire on a bare `trufflehog` invocation UNLESS it
        // also carries `--json`/`json`/`ocentra-enforcer`.
        RuleSpec::new(
            BuiltInSecurityRule::Sec2Rule18,
            "TruffleHog findings must be normalized",
            "TruffleHog command is not normalized through JSON/Enforcer.",
            compile(r"(?i)\btrufflehog\b")?,
            RuleBehavior::command_suppressed(compile(
                r"(?i)--json\b|\bjson\b|\bocentra-enforcer\b",
            )?),
        ),
        // SEC-2.19 — committed SSH private key files (path-based).
        RuleSpec::new(
            BuiltInSecurityRule::Sec2Rule19,
            "Committed SSH keys are forbidden",
            "SSH key file found in source scope.",
            compile(r"(?i)(^|/)(?:id_rsa|id_ed25519|id_ecdsa|id_dsa)(?:\.pub)?$")?,
            RuleBehavior::path(),
        ),
        // SEC-2.20 — mobile secret config files (path-based).
        RuleSpec::new(
            BuiltInSecurityRule::Sec2Rule20,
            "Mobile secret config files are forbidden",
            "mobile secret config file found in source scope.",
            compile(r"(^|/)(?:google-services\.json|GoogleService-Info\.plist)$")?,
            RuleBehavior::path(),
        ),
    ];

    specs.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use crate::boundary::fixture::run_manifest_fixture_parity;
    use enforcer_validator::validator::{ValidationInput, Validator};

    use super::specs;
    use crate::boundary::spec::SpecValidator;

    #[test]
    fn all_twenty_specs_construct_cleanly() -> Result<(), Box<dyn std::error::Error>> {
        let built = specs()?;
        assert_eq!(built.len(), 20);
        Ok(())
    }

    #[test]
    fn sec_2_18_requires_command_context_without_hiding_real_commands(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let spec = specs()?
            .into_iter()
            .find(|spec| spec.rule_id().as_str() == "SEC-2.18")
            .ok_or("SEC-2.18 spec missing")?;
        let validator = SpecValidator::new(spec)?;
        let file = crate::boundary::fixture::rel_path(
            "crates/enforcer-lang-security/src/rules/generic_scanner.rs",
        )?;
        let rule_definition = ["compile(r\"(?i)\\btruffle", "hog\\b\")?"].concat();
        let command = ["run: truffle", "hog filesystem ."].concat();
        let rule_findings = validator.validate(ValidationInput {
            file: &file,
            source: enforcer_domain::boundary::validation::ValidationSource::from_text(
                &rule_definition,
            ),
            scope: enforcer_domain::findings::ScanScope::Files,
        });
        let command_findings = validator.validate(ValidationInput {
            file: &file,
            source: enforcer_domain::boundary::validation::ValidationSource::from_text(&command),
            scope: enforcer_domain::findings::ScanScope::Files,
        });
        assert!(rule_findings.is_empty());
        assert_eq!(command_findings.len(), 1);
        Ok(())
    }

    /// Every `SEC-2.*` spec fires on its fail fixture and stays silent on
    /// its pass fixture. Fixture paths are explicit (not a mechanical
    /// slug fold) because two rules (`SEC-2.19`/`.20`) are path-target
    /// specs whose fixture file must carry the EXACT forbidden basename
    /// (`id_rsa`, `google-services.json`), which cannot be expressed as
    /// `fail.txt`/`pass.txt` like the 18 content-target specs.
    #[test]
    fn every_sec2_spec_fires_on_fail_and_stays_silent_on_pass(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let fixture_pairs: &[(&str, &str, &str)] = &[
            (
                "SEC-2.1",
                "fixtures/sec-2-1/fail.txt",
                "fixtures/sec-2-1/pass.txt",
            ),
            (
                "SEC-2.2",
                "fixtures/sec-2-2/fail.txt",
                "fixtures/sec-2-2/pass.txt",
            ),
            (
                "SEC-2.3",
                "fixtures/sec-2-3/fail.json",
                "fixtures/sec-2-3/pass.json",
            ),
            (
                "SEC-2.4",
                "fixtures/sec-2-4/fail.txt",
                "fixtures/sec-2-4/pass.txt",
            ),
            (
                "SEC-2.5",
                "fixtures/sec-2-5/fail.txt",
                "fixtures/sec-2-5/pass.txt",
            ),
            (
                "SEC-2.6",
                "fixtures/sec-2-6/fail.txt",
                "fixtures/sec-2-6/pass.txt",
            ),
            (
                "SEC-2.7",
                "fixtures/sec-2-7/fail.txt",
                "fixtures/sec-2-7/pass.txt",
            ),
            (
                "SEC-2.8",
                "fixtures/sec-2-8/fail.txt",
                "fixtures/sec-2-8/pass.txt",
            ),
            (
                "SEC-2.9",
                "fixtures/sec-2-9/fail.txt",
                "fixtures/sec-2-9/pass.txt",
            ),
            (
                "SEC-2.10",
                "fixtures/sec-2-10/fail.txt",
                "fixtures/sec-2-10/pass.txt",
            ),
            (
                "SEC-2.11",
                "fixtures/sec-2-11/fail.txt",
                "fixtures/sec-2-11/pass.txt",
            ),
            (
                "SEC-2.12",
                "fixtures/sec-2-12/fail.txt",
                "fixtures/sec-2-12/pass.txt",
            ),
            (
                "SEC-2.13",
                "fixtures/sec-2-13/fail.txt",
                "fixtures/sec-2-13/pass.txt",
            ),
            (
                "SEC-2.14",
                "fixtures/sec-2-14/fail.txt",
                "fixtures/sec-2-14/pass.txt",
            ),
            (
                "SEC-2.15",
                "fixtures/sec-2-15/fail.txt",
                "fixtures/sec-2-15/pass.txt",
            ),
            (
                "SEC-2.16",
                "fixtures/sec-2-16/fail.txt",
                "fixtures/sec-2-16/pass.txt",
            ),
            (
                "SEC-2.17",
                "fixtures/sec-2-17/fail.txt",
                "fixtures/sec-2-17/pass.txt",
            ),
            (
                "SEC-2.18",
                "fixtures/sec-2-18/fail.txt",
                "fixtures/sec-2-18/pass.txt",
            ),
            (
                "SEC-2.19",
                "fixtures/sec-2-19/fail/id_rsa",
                "fixtures/sec-2-19/pass/ssh-key-readme.txt",
            ),
            (
                "SEC-2.20",
                "fixtures/sec-2-20/fail/google-services.json",
                "fixtures/sec-2-20/pass/mobile-config-readme.txt",
            ),
        ];

        let built = specs()?;
        assert_eq!(
            fixture_pairs.len(),
            built.len(),
            "fixture_pairs table must cover exactly the 20 SEC-2 specs"
        );

        for spec in built {
            let (_, fail, pass) = fixture_pairs
                .iter()
                .find(|(rule_id, _, _)| *rule_id == spec.rule_id().as_str())
                .ok_or_else(|| format!("no fixture pair registered for {}", spec.rule_id()))?;
            let validator = SpecValidator::new(spec)?;
            run_manifest_fixture_parity(&validator, fail, pass)
                .map_err(|e| format!("{}: {e}", validator.rule_id()))?;
        }
        Ok(())
    }
}
