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
//! shape (SEC-2.11/.12/.14 "real secret WITHOUT an allowed placeholder
//! marker"; SEC-2.16/.17/.18 "tool invocation WITHOUT its required
//! flag"). Rust's `regex` crate has no negative lookaround, so those rows
//! use [`RuleSpec::suppressed_by_any_of`]: the base `pattern` matches the
//! positive shape, and a hit is suppressed if the marker pattern ALSO
//! matches the same line — the two-regex equivalent of the JS `&& !`.

use enforcer_domain::boundary::decode_error::DecodeError;
use regex::Regex;

use super::spec::{MatchTarget, RuleSpec};

/// Build every one of the 20 `SEC-2.*` specs. Fails closed (propagates the
/// first regex-compile error) rather than constructing a partially valid
/// table — see [`super::registry::build_all`] for the caller contract.
#[allow(clippy::too_many_lines, clippy::vec_init_then_push)]
pub fn specs() -> Result<Vec<RuleSpec>, DecodeError> {
    let any_secret = any_secret_pattern();
    let env_placeholder_allowed = env_placeholder_allowed_pattern();
    let fixture_marker_allowed = fixture_marker_allowed_pattern();

    let mut specs = Vec::new();

    // SEC-2.1 — GitHub tokens (`ghp_`/`gho_`/`ghu_`/`ghs_`/`ghr_...`).
    specs.push(RuleSpec {
        rule_id: "SEC-2.1",
        title: "GitHub tokens are forbidden",
        detail: "GitHub token found.",
        pattern: compile(r"\bgh[pousr]_[A-Za-z0-9_]{20,}\b")?,
        target: MatchTarget::Content,
        comment_guard: true,
        suppressed_by_any_of: None,
    });

    // SEC-2.2 — AWS access keys (`AKIA` + 16 uppercase-alnum).
    specs.push(RuleSpec {
        rule_id: "SEC-2.2",
        title: "AWS access keys are forbidden",
        detail: "AWS access key found.",
        pattern: compile(r"\bAKIA[0-9A-Z]{16}\b")?,
        target: MatchTarget::Content,
        comment_guard: true,
        suppressed_by_any_of: None,
    });

    // SEC-2.3 — Google service account JSON markers.
    specs.push(RuleSpec {
        rule_id: "SEC-2.3",
        title: "Google service account JSON is forbidden",
        detail: "Google service account JSON marker found.",
        pattern: compile(r#""type"\s*:\s*"service_account"|"private_key_id"\s*:"#)?,
        target: MatchTarget::Content,
        comment_guard: true,
        suppressed_by_any_of: None,
    });

    // SEC-2.4 — Azure credential assignments.
    specs.push(RuleSpec {
        rule_id: "SEC-2.4",
        title: "Azure credentials are forbidden",
        detail: "Azure credential assignment found.",
        pattern: compile(r"(?i)\bAZURE_(?:CLIENT_SECRET|TENANT_ID|CLIENT_ID)\b\s*[:=]")?,
        target: MatchTarget::Content,
        comment_guard: true,
        suppressed_by_any_of: None,
    });

    // SEC-2.5 — Slack/Discord tokens.
    specs.push(RuleSpec {
        rule_id: "SEC-2.5",
        title: "Slack and Discord tokens are forbidden",
        detail: "Slack or Discord token found.",
        pattern: compile(
            r"(?i)\bxox[baprs]-[A-Za-z0-9-]{10,}\b|discord(?:app)?\.[A-Za-z0-9_-]{20,}",
        )?,
        target: MatchTarget::Content,
        comment_guard: true,
        suppressed_by_any_of: None,
    });

    // SEC-2.6 — JWT-looking three-segment tokens.
    specs.push(RuleSpec {
        rule_id: "SEC-2.6",
        title: "JWT-looking secrets are forbidden",
        detail: "JWT-looking token found.",
        pattern: compile(r"\beyJ[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}\b")?,
        target: MatchTarget::Content,
        comment_guard: true,
        suppressed_by_any_of: None,
    });

    // SEC-2.7 — PEM-style private key blocks.
    specs.push(RuleSpec {
        rule_id: "SEC-2.7",
        title: "Private key blocks are forbidden",
        detail: "private key block found.",
        pattern: compile(r"-----BEGIN (?:RSA |OPENSSH |EC |DSA |)?PRIVATE KEY-----")?,
        target: MatchTarget::Content,
        comment_guard: true,
        suppressed_by_any_of: None,
    });

    // SEC-2.8 — npm/PyPI/Cargo registry tokens.
    specs.push(RuleSpec {
        rule_id: "SEC-2.8",
        title: "Package registry tokens are forbidden",
        detail: "package registry token found.",
        pattern: compile(
            r"\b(?:npm_[A-Za-z0-9]{20,}|pypi-[A-Za-z0-9_-]{20,}|CARGO_REGISTRY_TOKEN\s*=)",
        )?,
        target: MatchTarget::Content,
        comment_guard: true,
        suppressed_by_any_of: None,
    });

    // SEC-2.9 — Stripe live/test keys.
    specs.push(RuleSpec {
        rule_id: "SEC-2.9",
        title: "Stripe keys are forbidden",
        detail: "Stripe key found.",
        pattern: compile(r"\b(?:sk|pk)_(?:live|test)_[A-Za-z0-9]{20,}\b")?,
        target: MatchTarget::Content,
        comment_guard: true,
        suppressed_by_any_of: None,
    });

    // SEC-2.10 — high-entropy secret assignments (32+ char base64-ish
    // value assigned to a secret/token/password/key identifier).
    specs.push(RuleSpec {
        rule_id: "SEC-2.10",
        title: "High-entropy secret assignments are forbidden",
        detail: "high-entropy secret assignment found.",
        pattern: compile(
            r#"(?i)\b(?:secret|token|password|key)\b\s*[:=]\s*["'][A-Za-z0-9+/=_-]{32,}["']"#,
        )?,
        target: MatchTarget::Content,
        comment_guard: true,
        suppressed_by_any_of: None,
    });

    // SEC-2.11 — `.env.example` must carry only placeholder-looking
    // values: fire on a real-looking secret assignment UNLESS the line
    // also carries an allowed placeholder marker (`example`,
    // `placeholder`, `changeme`, `replace_me`, `dummy`, `fake`, `test`,
    // `<...>`, `${...}` — `ENV_PLACEHOLDER_ALLOWED` in the JS source).
    specs.push(RuleSpec {
        rule_id: "SEC-2.11",
        title: ".env.example may contain placeholders only",
        detail: ".env.example contains a real-looking secret.",
        pattern: compile(&any_secret)?,
        target: MatchTarget::Content,
        comment_guard: false,
        suppressed_by_any_of: Some(compile(&env_placeholder_allowed)?),
    });

    // SEC-2.12 — same shape as SEC-2.11 for `.env.template`.
    specs.push(RuleSpec {
        rule_id: "SEC-2.12",
        title: ".env.template may contain placeholders only",
        detail: ".env.template contains a real-looking secret.",
        pattern: compile(&any_secret)?,
        target: MatchTarget::Content,
        comment_guard: false,
        suppressed_by_any_of: Some(compile(&env_placeholder_allowed)?),
    });

    // SEC-2.13 — secret-looking values are forbidden in snapshot/test
    // artifacts. The JS source keys this off `context.isTestLike`, a
    // classification this crate's per-line `Validator` contract does not
    // carry; ported as an unconditional real-secret-shape check, with the
    // rule's fixtures living under a `snapshot`-shaped fixture path to
    // document the intended scope (arc-14+'s scan orchestration is
    // expected to route only test/snapshot files to this rule).
    specs.push(RuleSpec {
        rule_id: "SEC-2.13",
        title: "Secrets are forbidden in snapshots",
        detail: "secret-looking value found in snapshot/test artifact.",
        pattern: compile(&any_secret)?,
        target: MatchTarget::Content,
        comment_guard: false,
        suppressed_by_any_of: None,
    });

    // SEC-2.14 — fixture secrets require an explicit fake marker on the
    // same line: fire on a real-looking secret assignment UNLESS the line
    // also carries `fake`, `fixture`, or `example`.
    specs.push(RuleSpec {
        rule_id: "SEC-2.14",
        title: "Fixture secrets require fake markers",
        detail: "fixture secret lacks explicit fake marker.",
        pattern: compile(&any_secret)?,
        target: MatchTarget::Content,
        comment_guard: false,
        suppressed_by_any_of: Some(compile(&fixture_marker_allowed)?),
    });

    // SEC-2.15 — secret diagnostics must redact matched values. The
    // scanner's OWN output redaction is enforced structurally by
    // `super::spec::redact_line` (every finding's snippet is redacted
    // before construction), so this rule's fixtures prove the
    // CONTRAPOSITIVE: a line that looks like un-redacted diagnostic
    // output (carries a real secret-shaped literal) is the fail shape; a
    // `[REDACTED]` placeholder in the same position is the pass shape
    // (the shared secret-shape pattern requires 16+ literal chars inside
    // matching quotes, which `[REDACTED]` does not satisfy).
    specs.push(RuleSpec {
        rule_id: "SEC-2.15",
        title: "Secret diagnostics must redact matched values",
        detail: "diagnostic output line carries an unredacted secret-looking value.",
        pattern: compile(&any_secret)?,
        target: MatchTarget::Content,
        comment_guard: false,
        suppressed_by_any_of: None,
    });

    // SEC-2.16 — Gitleaks invocations must emit SARIF: fire on a
    // `gitleaks detect|protect|dir|git` command line UNLESS it also
    // carries `sarif` or `--report-format sarif`.
    specs.push(RuleSpec {
        rule_id: "SEC-2.16",
        title: "Secret scanners must emit SARIF",
        detail: "Gitleaks command does not emit SARIF.",
        pattern: compile(r"(?i)\bgitleaks\s+(?:detect|protect|dir|git)\b")?,
        target: MatchTarget::Content,
        comment_guard: true,
        suppressed_by_any_of: Some(compile(r"(?i)\bsarif\b|--report-format\s+sarif")?),
    });

    // SEC-2.17 — Gitleaks findings must be normalized through
    // Enforcer/SARIF: same command shape as SEC-2.16, suppressed by
    // either `ocentra-enforcer` or `--report-format sarif`.
    specs.push(RuleSpec {
        rule_id: "SEC-2.17",
        title: "Gitleaks findings must be normalized",
        detail: "Gitleaks command is not normalized through Enforcer/SARIF.",
        pattern: compile(r"(?i)\bgitleaks\s+(?:detect|protect|dir|git)\b")?,
        target: MatchTarget::Content,
        comment_guard: true,
        suppressed_by_any_of: Some(compile(
            r"(?i)\bocentra-enforcer\b|--report-format\s+sarif",
        )?),
    });

    // SEC-2.18 — TruffleHog findings must be normalized through
    // JSON/Enforcer: fire on a bare `trufflehog` invocation UNLESS it
    // also carries `--json`/`json`/`ocentra-enforcer`.
    specs.push(RuleSpec {
        rule_id: "SEC-2.18",
        title: "TruffleHog findings must be normalized",
        detail: "TruffleHog command is not normalized through JSON/Enforcer.",
        pattern: compile(r"(?i)\btrufflehog\b")?,
        target: MatchTarget::Content,
        comment_guard: true,
        suppressed_by_any_of: Some(compile(r"(?i)--json\b|\bjson\b|\bocentra-enforcer\b")?),
    });

    // SEC-2.19 — committed SSH private key files (path-based).
    specs.push(RuleSpec {
        rule_id: "SEC-2.19",
        title: "Committed SSH keys are forbidden",
        detail: "SSH key file found in source scope.",
        pattern: compile(r"(?i)(^|/)(?:id_rsa|id_ed25519|id_ecdsa|id_dsa)(?:\.pub)?$")?,
        target: MatchTarget::Path,
        comment_guard: false,
        suppressed_by_any_of: None,
    });

    // SEC-2.20 — mobile secret config files (path-based).
    specs.push(RuleSpec {
        rule_id: "SEC-2.20",
        title: "Mobile secret config files are forbidden",
        detail: "mobile secret config file found in source scope.",
        pattern: compile(r"(^|/)(?:google-services\.json|GoogleService-Info\.plist)$")?,
        target: MatchTarget::Path,
        comment_guard: false,
        suppressed_by_any_of: None,
    });

    Ok(specs)
}

fn compile(pattern: &str) -> Result<Regex, DecodeError> {
    Regex::new(pattern).map_err(|source| {
        DecodeError::new("securityRulePattern", format!("invalid regex: {source}"))
    })
}

/// The generic real-looking-secret-assignment shape shared by several
/// SEC-2 rows (mirrors `SECRET_RE` in `src/generic-scanner-shared.mjs`).
fn any_secret_pattern() -> String {
    r#"(?i)\b(?:[A-Z0-9_/-]*(?:api[_-]?key|secret|token|password|private[_-]?key))\b\s*[:=]\s*["'][A-Za-z0-9_./+=:@-]{16,}["']"#
        .to_owned()
}

/// `ENV_PLACEHOLDER_ALLOWED` from `src/generic-scanner-shared.mjs`: the set
/// of markers that make an otherwise real-looking `.env.example`/
/// `.env.template` value acceptable.
fn env_placeholder_allowed_pattern() -> String {
    r"(?i)example|placeholder|changeme|replace_me|dummy|fake|test|<[^>]+>|\$\{[^}]+\}".to_owned()
}

/// The fixture-fake-marker allowance for SEC-2.14 (`fake`/`fixture`/
/// `example`, matching `scanSecretLine`'s own inline check).
fn fixture_marker_allowed_pattern() -> String {
    r"(?i)\bfake\b|\bfixture\b|\bexample\b".to_owned()
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use enforcer_validator::harness::run_fixture_parity;
    use enforcer_validator::validator::Validator;

    use super::specs;
    use crate::rules::spec::SpecValidator;

    fn manifest_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    #[test]
    fn all_twenty_specs_construct_cleanly() -> Result<(), Box<dyn std::error::Error>> {
        let built = specs()?;
        assert_eq!(built.len(), 20);
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
                .find(|(rule_id, _, _)| *rule_id == spec.rule_id)
                .ok_or_else(|| format!("no fixture pair registered for {}", spec.rule_id))?;
            let validator = SpecValidator::new(spec)?;
            run_fixture_parity(&validator, &manifest_dir(), fail, pass)
                .map_err(|e| format!("{}: {e}", validator.rule_id()))?;
        }
        Ok(())
    }
}
