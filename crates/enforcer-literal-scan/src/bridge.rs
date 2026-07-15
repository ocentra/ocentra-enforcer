//! `LIT-2.1` — the universal literal-scan T2 advisory bridge.
//!
//! Wires the folded scored literal-risk scanner (arc-13) into the
//! `enforcer-validator` (arc-05) `Validator` boundary as an **always-on,
//! non-blocking** T2 layer: it runs over every scan target regardless of
//! which (if any) bespoke lang-family `Validator` matched the file's
//! language, giving every one of the ~65 registered languages a baseline
//! mechanical floor.
//!
//! # Purity
//!
//! [`Validator::validate`] must be pure over [`ValidationInput`] (same
//! input, same findings, no I/O) per the trait's own contract. The crate's
//! existing [`crate::scan_runtime::run_scan`] entry point is filesystem
//! rooted (it discovers and reads files from disk), so this bridge does
//! NOT call it. Instead it composes the same pure per-file primitives
//! `run_scan` itself calls internally, operating directly over
//! `input.source: &str`:
//! 1. [`crate::detect_language`] — classify the target's language from its
//!    path (extension/basename), independent of file contents.
//! 2. [`crate::classify_file_role`] — classify the target's role (domain,
//!    boundary, config, test, ...) from its repo-relative path.
//! 3. [`crate::lexer::lex_literals`] — lex string/template literal
//!    candidates out of the in-memory source text.
//! 4. [`crate::risk::classify_literal`] — score each candidate, producing
//!    the crate's own (richer) `Finding` DTO with `score` + `confidence`.
//!
//! Each crate-local `Finding` is then mapped into an
//! `enforcer_domain::findings::Finding` (see [`to_domain_finding`]).
//!
//! # Non-blocking, by construction
//!
//! [`enforcer_domain::findings::Violation`] only exists for a `Finding`
//! whose `severity` is [`Severity::Error`] — `Violation::try_from` fails
//! closed otherwise. This bridge NEVER constructs a `Finding` with
//! `Severity::Error`, regardless of the crate-local scorer's own
//! `blocking` flag (which drives ITS severity string, `"error"` /
//! `"warning"` / `"info"`, for the crate's own standalone CLI/report
//! surface, not this boundary): every mapped finding carries
//! [`Severity::Warning`] when the crate-local score crosses the advisory
//! threshold, and no finding at all is emitted below it. That makes it
//! structurally impossible for this validator's output to promote to a
//! blocking `Violation` or flip a `Report.ok` to `false` on its own — the
//! T2 doctrine (scored/advisory, never a hard gate by itself).

use enforcer_domain::findings::Finding as DomainFinding;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

use crate::lexer::lex_literals;
use crate::models::Finding as ScanFinding;
use crate::risk::classify_literal;
use crate::{classify_file_role, detect_language, DEFAULT_MIN_SCORE};

/// The `LIT-2.1` universal literal-scan T2 advisory bridge `Validator`.
///
/// `min_score` is the advisory threshold: a crate-local finding whose
/// `score` is `>= min_score` crosses the threshold and is mapped into an
/// advisory [`DomainFinding`]; findings scoring below it are dropped
/// (matches the crate's own [`DEFAULT_MIN_SCORE`] filtering behavior so
/// this bridge stays consistent with the standalone CLI/report surface).
pub struct LiteralScanBridgeValidator {
    rule_id: RuleId,
    min_score: u8,
}

impl LiteralScanBridgeValidator {
    /// Build the bridge validator with the crate's default advisory
    /// threshold, parsing its own `RuleId` literal at construction
    /// (parse-at-boundary, mirroring every other bespoke validator in this
    /// workspace).
    pub fn new() -> Result<Self, enforcer_domain::boundary::decode_error::DecodeError> {
        Self::with_min_score(DEFAULT_MIN_SCORE)
    }

    /// Build the bridge validator with an explicit advisory threshold
    /// (e.g. for a caller that wires a project-specific `fail_above`-style
    /// override through to the universal floor).
    pub fn with_min_score(
        min_score: u8,
    ) -> Result<Self, enforcer_domain::boundary::decode_error::DecodeError> {
        Ok(Self {
            rule_id: "LIT-2.1".parse()?,
            min_score,
        })
    }
}

impl Validator for LiteralScanBridgeValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<DomainFinding> {
        let path = std::path::Path::new(input.file.as_str());
        // `include_unknown = true`: the universal floor must still cover
        // targets no bespoke language entry recognizes (the whole point of
        // an "always-on" layer over all ~65+ languages).
        let Some(language) = detect_language(path, true) else {
            return Vec::new();
        };
        let rel = input.file.as_str();
        let role = classify_file_role(rel, language);
        let candidates = lex_literals(input.source, language, rel);

        let mut findings = Vec::new();
        for candidate in &candidates {
            let scan_finding = classify_literal(
                candidate,
                rel,
                language.id,
                role,
                // This bridge scores each file independently (no
                // cross-file repeated-literal context at the per-file
                // `Validator` boundary); `repeated_files = 0` matches
                // "no repetition signal available here".
                0,
                None,
            );
            if scan_finding.score < self.min_score {
                continue;
            }
            findings.push(to_domain_finding(&self.rule_id, input, &scan_finding));
        }
        findings
    }
}

/// Map the crate-local (richer) scored `Finding` into the
/// `enforcer-domain` `Finding` DTO the `Validator` boundary requires.
/// Always non-blocking: `severity` is [`Severity::Warning`], never
/// [`Severity::Error`], regardless of the scan finding's own `blocking`
/// flag or severity string (see the module doc for why).
fn to_domain_finding(
    rule_id: &RuleId,
    input: ValidationInput<'_>,
    scan_finding: &ScanFinding,
) -> DomainFinding {
    DomainFinding {
        rule_id: rule_id.clone(),
        severity: Severity::Warning,
        title: format!(
            "literal-scan advisory: {} (score {}, confidence {})",
            scan_finding.category.as_str(),
            scan_finding.score,
            scan_finding.confidence
        ),
        detail: format!(
            "{} (language: {}, role: {})",
            scan_finding.reason, scan_finding.language, scan_finding.context
        ),
        file: input.file.clone(),
        line: u32::try_from(scan_finding.line).unwrap_or(u32::MAX),
        snippet: Some(scan_finding.literal_preview.clone()),
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use enforcer_domain::findings::{ScanScope, Violation};
    use enforcer_domain::paths::RelPath;
    use enforcer_validator::harness::run_fixture_parity;
    use enforcer_validator::validator::{ValidationInput, Validator};

    use super::LiteralScanBridgeValidator;

    fn manifest_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    /// Named proof row `literal-scan-universal-threshold`: the Dart
    /// fail/pass pair proves the score-threshold crossing behavior via the
    /// standard fail/pass parity oracle.
    #[test]
    fn dart_fail_crosses_threshold_and_pass_stays_under() -> Result<(), Box<dyn std::error::Error>>
    {
        let validator = LiteralScanBridgeValidator::new()?;
        run_fixture_parity(
            &validator,
            &manifest_dir(),
            "tests/fixtures/universal/fail/dart-secret.dart",
            "tests/fixtures/universal/pass/dart-clean.dart",
        )?;
        Ok(())
    }

    /// Same threshold-crossing proof over the newly-registered CFML
    /// (`.cfm`/`.cfc`) language entry, proving `.dart`/`.cfc`/`.cfm` are all
    /// recognized by the registry and covered by the universal floor.
    #[test]
    fn cfml_fail_crosses_threshold_and_pass_stays_under() -> Result<(), Box<dyn std::error::Error>>
    {
        let validator = LiteralScanBridgeValidator::new()?;
        run_fixture_parity(
            &validator,
            &manifest_dir(),
            "tests/fixtures/universal/fail/cfml-secret.cfm",
            "tests/fixtures/universal/pass/cfml-clean.cfm",
        )?;
        Ok(())
    }

    /// Named proof row `literal-scan-graceful-skip` /
    /// `literal-scan-advisory-nonblocking`: an advisory finding never
    /// promotes to a blocking `Violation` (structurally enforced by
    /// `Violation::try_from` requiring `Severity::Error`), and an
    /// unrecognized/degenerate target degrades to silence rather than
    /// erroring.
    #[test]
    fn advisory_findings_never_promote_to_violation() -> Result<(), Box<dyn std::error::Error>> {
        let validator = LiteralScanBridgeValidator::new()?;
        let file: RelPath = "tests/fixtures/universal/fail/dart-secret.dart".parse()?;
        let source = std::fs::read_to_string(manifest_dir().join(file.as_str()))?;
        let findings = validator.validate(ValidationInput {
            file: &file,
            source: &source,
            scope: ScanScope::Files,
        });
        assert!(
            !findings.is_empty(),
            "fail fixture must still cross the threshold"
        );
        for finding in findings {
            assert_ne!(
                finding.severity,
                enforcer_domain::severity::Severity::Error,
                "a universal literal-scan advisory finding must never carry Severity::Error"
            );
            assert!(
                Violation::try_from(finding).is_err(),
                "an advisory finding must fail to promote to a blocking Violation"
            );
        }
        Ok(())
    }

    /// A target whose path carries no recognizable extension/basename
    /// still degrades gracefully (empty findings, no panic) rather than
    /// treating "unknown language" as an error condition — the run stays
    /// exit-0-capable even when native language detection has nothing to
    /// key off.
    #[test]
    fn unrecognized_target_degrades_to_silence() -> Result<(), Box<dyn std::error::Error>> {
        let validator = LiteralScanBridgeValidator::new()?;
        let file: RelPath = "tests/fixtures/universal/fail/no-extension".parse()?;
        let findings = validator.validate(ValidationInput {
            file: &file,
            source: "some plain text with no literals of interest",
            scope: ScanScope::Files,
        });
        assert!(findings.is_empty());
        Ok(())
    }

    /// `.dart`, `.cfc`, and `.cfm` must all resolve to a concrete language
    /// entry in the registry (not the `unknown` fallback) — the additive
    /// registry rows this pack owns.
    #[test]
    fn dart_and_cfml_extensions_are_registered() {
        for path in [
            "crates/x/lib/main.dart",
            "crates/x/handlers/OrderService.cfc",
            "crates/x/views/index.cfm",
        ] {
            let language = crate::detect_language(std::path::Path::new(path), false);
            assert!(
                language.is_some(),
                "expected a registered language for {path}"
            );
        }
    }

    /// A validator using a threshold no real finding can cross behaves
    /// exactly like the harness's own `SilentValidator` negative case:
    /// the fixture-parity oracle must catch it as "did not fire on fail".
    #[test]
    fn impossible_threshold_is_caught_as_silent_by_the_parity_oracle(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let validator = LiteralScanBridgeValidator::with_min_score(255)?;
        let outcome = run_fixture_parity(
            &validator,
            &manifest_dir(),
            "tests/fixtures/universal/fail/dart-secret.dart",
            "tests/fixtures/universal/pass/dart-clean.dart",
        );
        assert!(outcome.is_err());
        Ok(())
    }
}
