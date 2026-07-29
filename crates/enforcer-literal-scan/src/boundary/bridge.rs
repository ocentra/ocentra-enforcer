//! BOUNDARY-INVARIANT: this boundary module validates raw wire values and converts only through typed domain contracts.
//! `LIT-2.1` â€” the universal literal-scan T2 advisory bridge.
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
//! 1. [`crate::detect_language`] â€” classify the target's language from its
//!    path (extension/basename), independent of file contents.
//! 2. [`crate::classify_file_role`] â€” classify the target's role (domain,
//!    boundary, config, test, ...) from its repo-relative path.
//! 3. [`crate::lexer::lex_literals`] â€” lex string/template literal
//!    candidates out of the in-memory source text.
//! 4. [`crate::risk::classify_literal`] â€” score each candidate, producing
//!    the crate's own (richer) `Finding` DTO with `score` + `confidence`.
//!
//! Each crate-local `Finding` is then mapped into an
//! `enforcer_domain::findings::Finding` (see [`to_domain_finding`]).
//!
//! # Non-blocking, by construction
//!
//! [`enforcer_domain::findings::Violation`] only exists for a `Finding`
//! whose `severity` is [`Severity::Error`] â€” `Violation::try_from` fails
//! closed otherwise. This bridge NEVER constructs a `Finding` with
//! `Severity::Error`, regardless of the crate-local scorer's own
//! `blocking` flag (which drives ITS severity string, `"error"` /
//! `"warning"` / `"info"`, for the crate's own standalone CLI/report
//! surface, not this boundary): every mapped finding carries
//! [`Severity::Warning`] when the crate-local score crosses the advisory
//! threshold, and no finding at all is emitted below it. That makes it
//! structurally impossible for this validator's output to promote to a
//! blocking `Violation` or flip a `Report.ok` to `false` on its own â€” the
//! T2 doctrine (scored/advisory, never a hard gate by itself).

use std::num::NonZeroU32;

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::findings::{
    Finding as DomainFinding, FindingDetail, FindingLine, FindingSnippet, FindingTitle,
};
use enforcer_domain::ids::{BuiltInLiteralRule, RuleId};
use enforcer_domain::scan_types::{LiteralFindingPath, LiteralLanguageId, LiteralRiskScore};
use enforcer_domain::severity::Severity;
use enforcer_domain::telemetry_types::SourceLine;
use enforcer_validator::validator::{ValidationInput, Validator};

use crate::file_role::classify_file_role;
use crate::language_registry::detect_language;
use crate::lexer::lex_literals;
use crate::risk::{classify_literal, ClassificationInput};
use crate::Finding as ScanFinding;

/// The `LIT-2.1` universal literal-scan T2 advisory bridge `Validator`.
///
/// `min_score` is the advisory threshold: a crate-local finding whose
/// `score` is `>= min_score` crosses the threshold and is mapped into an
/// advisory [`DomainFinding`]; findings scoring below it are dropped
/// (matches the crate's own [`DEFAULT_MIN_SCORE`] filtering behavior so
/// this bridge stays consistent with the standalone CLI/report surface).
#[derive(Debug)]
pub struct LiteralScanBridgeValidator {
    rule_id: RuleId,
    min_score: LiteralRiskScore,
}

impl LiteralScanBridgeValidator {
    /// Build the bridge validator with the crate's default advisory
    /// threshold, parsing its own `RuleId` literal at construction
    /// (parse-at-boundary, mirroring every other bespoke validator in this
    /// workspace).
    pub fn new() -> Result<Self, enforcer_domain::boundary::decode_error::DecodeError> {
        Self::with_min_score(LiteralRiskScore::default())
    }

    /// Build the bridge validator with an explicit advisory threshold
    /// (e.g. for a caller that wires a project-specific `fail_above`-style
    /// override through to the universal floor).
    pub fn with_min_score(
        min_score: LiteralRiskScore,
    ) -> Result<Self, enforcer_domain::boundary::decode_error::DecodeError> {
        Ok(Self {
            rule_id: BuiltInLiteralRule::Lit2Rule1.id(),
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
        let Ok(finding_path) = LiteralFindingPath::try_new(String::from(rel)) else {
            return Vec::new();
        };
        let language_id = LiteralLanguageId::from(language.id);
        let role = classify_file_role(&finding_path, language);
        let candidates = lex_literals(input.source.as_str(), language, rel);

        let mut findings = Vec::new();
        for candidate in &candidates {
            let scan_finding = classify_literal(ClassificationInput {
                candidate,
                file: &finding_path,
                language: &language_id,
                role,
                // This bridge scores each file independently (no
                // cross-file repeated-literal context at the per-file
                // `Validator` boundary); `repeated_files = 0` matches
                // "no repetition signal available here".
                repeated_files: 0.into(),
                fail_above: None,
            });
            if scan_finding.score < self.min_score {
                continue;
            }
            match to_domain_finding(input, &scan_finding) {
                Ok(finding) => findings.push(finding),
                Err(_) => continue,
            }
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
    input: ValidationInput<'_>,
    scan_finding: &ScanFinding,
) -> Result<DomainFinding, DecodeError> {
    let title = FindingTitle::new(format!(
        "literal-scan advisory: {} (score {}, confidence {})",
        scan_finding.category.wire_name(),
        scan_finding.score,
        scan_finding.confidence.wire_name()
    ))?;
    let detail = FindingDetail::new(format!(
        "{} (language: {}, role: {})",
        scan_finding.reason, scan_finding.language, scan_finding.context
    ))?;
    let source_line = match u32::try_from(scan_finding.line.get()) {
        Ok(line) => match NonZeroU32::new(line) {
            Some(line) => FindingLine::known(SourceLine::try_new(line)),
            None => FindingLine::Unspecified,
        },
        Err(_) => FindingLine::Unspecified,
    };
    let snippet = FindingSnippet::new(scan_finding.literal_preview.to_string())
        .into_iter()
        .next();

    Ok(DomainFinding {
        rule_id: BuiltInLiteralRule::Lit2Rule1.id(),
        severity: Severity::Warning,
        title,
        detail,
        file: enforcer_domain::paths::RelPath::try_from(String::from(input.file.as_str()))?,
        line: source_line,
        snippet,
    })
}

#[cfg(test)]
mod tests {
    use enforcer_domain::boundary::decode_error::DecodeError;
    use enforcer_domain::findings::{ScanScope, Violation};
    use enforcer_domain::paths::{RelPath, RepoRoot};
    use enforcer_domain::scan_types::LiteralRiskScore;
    use enforcer_validator::error::HarnessError;
    use enforcer_validator::harness::run_fixture_parity;
    use enforcer_validator::validator::{ValidationInput, Validator};

    use super::LiteralScanBridgeValidator;

    fn manifest_root() -> Result<RepoRoot, DecodeError> {
        RepoRoot::try_from(env!("CARGO_MANIFEST_DIR").to_owned())
    }

    /// Named proof row `literal-scan-universal-threshold`: the Dart
    /// fail/pass pair proves the score-threshold crossing behavior via the
    /// standard fail/pass parity oracle.
    #[test]
    fn dart_fail_crosses_threshold_and_pass_stays_under() -> Result<(), Box<dyn std::error::Error>>
    {
        let validator = LiteralScanBridgeValidator::new()?;
        let root = manifest_root()?;
        let fail: RelPath = "tests/fixtures/universal/fail/dart-secret.dart".parse()?;
        let pass: RelPath = "tests/fixtures/universal/pass/dart-clean.dart".parse()?;
        run_fixture_parity(&validator, &root, &fail, &pass)?;
        Ok(())
    }

    /// Same threshold-crossing proof over the newly-registered CFML
    /// (`.cfm`/`.cfc`) language entry, proving `.dart`/`.cfc`/`.cfm` are all
    /// recognized by the registry and covered by the universal floor.
    #[test]
    fn cfml_fail_crosses_threshold_and_pass_stays_under() -> Result<(), Box<dyn std::error::Error>>
    {
        let validator = LiteralScanBridgeValidator::new()?;
        let root = manifest_root()?;
        let fail: RelPath = "tests/fixtures/universal/fail/cfml-secret.cfm".parse()?;
        let pass: RelPath = "tests/fixtures/universal/pass/cfml-clean.cfm".parse()?;
        run_fixture_parity(&validator, &root, &fail, &pass)?;
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
        let root = manifest_root()?;
        let file: RelPath = "tests/fixtures/universal/fail/dart-secret.dart".parse()?;
        let source = std::fs::read_to_string(root.resolve(&file))?;
        let findings = validator.validate(ValidationInput {
            file: &file,
            source: enforcer_domain::boundary::validation::ValidationSource::from_text(&source),
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
    /// treating "unknown language" as an error condition â€” the run stays
    /// exit-0-capable even when native language detection has nothing to
    /// key off.
    #[test]
    fn unrecognized_target_degrades_to_silence() -> Result<(), Box<dyn std::error::Error>> {
        let validator = LiteralScanBridgeValidator::new()?;
        let file: RelPath = "tests/fixtures/universal/fail/no-extension".parse()?;
        let findings = validator.validate(ValidationInput {
            file: &file,
            source: enforcer_domain::boundary::validation::ValidationSource::from_text(
                "some plain text with no literals of interest",
            ),
            scope: ScanScope::Files,
        });
        assert!(findings.is_empty());
        Ok(())
    }

    /// `.dart`, `.cfc`, and `.cfm` must all resolve to a concrete language
    /// entry in the registry (not the `unknown` fallback) â€” the additive
    /// registry rows this pack owns.
    #[test]
    fn dart_and_cfml_extensions_are_registered() {
        for path in [
            "crates/x/lib/main.dart",
            "crates/x/handlers/OrderService.cfc",
            "crates/x/views/index.cfm",
        ] {
            let language =
                crate::language_registry::detect_language(std::path::Path::new(path), false);
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
        let threshold = std::num::NonZeroU8::new(100).ok_or("non-zero test threshold")?;
        let validator =
            LiteralScanBridgeValidator::with_min_score(LiteralRiskScore::try_from(threshold)?)?;
        let root = manifest_root()?;
        let fail: RelPath = "tests/fixtures/universal/fail/dart-secret.dart".parse()?;
        let pass: RelPath = "tests/fixtures/universal/pass/dart-clean.dart".parse()?;
        let outcome = run_fixture_parity(&validator, &root, &fail, &pass);
        assert!(matches!(
            outcome,
            Err(HarnessError::DidNotFireOnFail { .. })
        ));
        Ok(())
    }
}
