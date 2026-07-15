//! d10 resilience auditor — mechanizes the ADBP "red team / resilience
//! reviewer" narrative (ADBP_GAPS.md rows 82-85, `d20-resilience-obligations`
//! folded into this pack) into two kinds of `Finding`:
//!
//! - T1 required-test OBLIGATIONS ([`FailureModeTestValidator`],
//!   `RESIL-FAILURE-MODE-TEST.1`): a trust-boundary function with no
//!   companion failure-mode test is a BLOCKING gap. Per the workpack: "an
//!   unmet required-test obligation fires a `Finding`; a met one is
//!   silent" — this is a T1 hard gate, not the labeled-T3 posture the
//!   broader ADBP_GAPS.md table describes for the un-mechanized narrative
//!   form of this same row; the workpack explicitly upgrades it to
//!   mechanized T1 for this crate's scope.
//! - T2 failure-mode SMELLS ([`AtomicWriteValidator`],
//!   [`IoTimeoutValidator`]): heuristically-detected gaps that carry a
//!   score + confidence in `[0.0, 1.0]` (folded into the finding `detail`,
//!   mirroring [`crate::rules::fsm::FsmTransitionCoverageValidator`]'s
//!   scored-model precedent — [`enforcer_domain::findings::Finding`] has no
//!   dedicated score/confidence field). These are ADVISORY: `Severity::
//!   Warning`, never gate a build.
//!
//! Every rule here is a lightweight line/keyword-oriented text detector
//! (mirroring this crate's other `rules::*` modules) rather than a full
//! per-language AST parse — this crate has no tree-sitter/AST dependency
//! for Python/Dart/CFML/TS/Rust targets.

use enforcer_domain::findings::Finding;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

/// The fixed parts of one rule's finding: its id, severity, and title.
/// Bundled so the per-call-site `finding()` helper stays under clippy's
/// `too_many_arguments` limit (mirrors [`crate::rules::fsm::FindingSpec`]
/// and [`crate::rules::size_shape::FindingSpec`]).
struct FindingSpec<'a> {
    rule_id: &'a RuleId,
    severity: Severity,
    title: &'a str,
}

/// Build a [`Finding`] for one of this module's validators.
fn finding(
    spec: &FindingSpec<'_>,
    detail: String,
    input: &ValidationInput<'_>,
    line: u32,
) -> Finding {
    Finding {
        rule_id: spec.rule_id.clone(),
        severity: spec.severity,
        title: spec.title.to_owned(),
        detail,
        file: input.file.clone(),
        line,
        snippet: None,
    }
}

/// Find the 1-based line number of the first line containing `marker`, or
/// `1` if the caller wants a whole-file finding with no more specific
/// anchor.
fn first_line_containing(source: &str, marker: &str) -> Option<u32> {
    source
        .lines()
        .enumerate()
        .find(|(_, line)| line.contains(marker))
        .map(|(idx, _)| (idx as u32).saturating_add(1))
}

/// Markers that identify a "trust-boundary" function signature: the
/// candidate surface [`FailureModeTestValidator`] enumerates failure modes
/// for. Deliberately permissive across languages (mirrors this crate's
/// other cross-stack marker vocabularies): an HTTP/RPC handler, an
/// untrusted-input parse/decode boundary, or an explicit doc-comment
/// annotation a caller can attach to any function it wants gated.
const TRUST_BOUNDARY_MARKERS: &[&str] = &[
    "trust-boundary",
    "trust_boundary",
    "fn handle_request",
    "async fn handle_request",
    "pub fn handle(",
    "pub async fn handle(",
    "#[trust_boundary]",
];

/// Markers that identify a companion failure-mode test: an explicit
/// assertion that the trust-boundary surface's failure path is covered.
/// Presence of ANY of these anywhere in the same file is treated as "this
/// trust boundary's failure modes are tested" — a coarse, file-granularity
/// heuristic (mirrors [`crate::rules::fsm::FsmTransitionCoverageValidator`]'s
/// same-file scoring shape) rather than a call-graph-precise linkage.
const FAILURE_MODE_TEST_MARKERS: &[&str] = &[
    "failure_mode",
    "failure-mode",
    "test_handles_failure",
    "test_rejects_invalid",
    "test_returns_error_on",
    "#[should_panic]",
    "assert_err",
];

/// `RESIL-FAILURE-MODE-TEST.1` — required-test obligation (T1, BLOCKING): a
/// trust-boundary function with no companion failure-mode test in the same
/// file fires a [`Finding`]. A trust-boundary function paired with at least
/// one recognized failure-mode test marker is silent.
pub struct FailureModeTestValidator {
    rule_id: RuleId,
}

impl FailureModeTestValidator {
    /// Build the validator, parsing its `RuleId` literal at construction
    /// (parse-at-boundary, mirroring every other bespoke validator in this
    /// crate).
    pub fn new() -> Result<Self, enforcer_domain::boundary::decode_error::DecodeError> {
        Ok(Self {
            rule_id: "RESIL-FAILURE-MODE-TEST.1".parse()?,
        })
    }
}

impl Validator for FailureModeTestValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let Some(marker) = TRUST_BOUNDARY_MARKERS
            .iter()
            .copied()
            .find(|marker| input.source.contains(marker))
        else {
            return Vec::new();
        };
        let has_failure_mode_test = FAILURE_MODE_TEST_MARKERS
            .iter()
            .any(|test_marker| input.source.contains(test_marker));
        if has_failure_mode_test {
            return Vec::new();
        }
        vec![finding(
            &FindingSpec {
                rule_id: &self.rule_id,
                severity: Severity::Error,
                title: "resilience: trust-boundary change has no failure-mode test obligation met",
            },
            format!(
                "found trust-boundary marker `{marker}` with no companion failure-mode test \
                 (expected one of the recognized test markers, e.g. `test_rejects_invalid`, \
                 `assert_err`, in the same file); enumerate this boundary's failure modes and add \
                 a test asserting each is handled before this passes review."
            ),
            &input,
            first_line_containing(input.source, marker).unwrap_or(1),
        )]
    }
}

/// Markers of an in-place truncate-then-write pattern:
/// [`AtomicWriteValidator`]'s smell signal. Deliberately cross-language
/// (Rust `std::fs::write`/`File::create`, Python `open(..., "w")`, generic
/// `writeFileSync`) since this crate validates target-repo code across
/// stacks.
const TRUNCATE_WRITE_MARKERS: &[&str] = &["fs::write(", "File::create(", "open(", "writeFileSync("];

/// Markers that indicate the write is already guarded by an atomic
/// temp-file + rename (or an equivalent transactional pattern), which
/// cancels the [`AtomicWriteValidator`] smell.
const ATOMIC_WRITE_GUARD_MARKERS: &[&str] = &[
    "NamedTempFile",
    "tempfile::",
    ".persist(",
    "rename(",
    "atomic_write",
    "os.replace(",
];

/// Score threshold at/above which [`AtomicWriteValidator`] fires (T2,
/// non-blocking): a truncate-then-write marker present with no atomic-write
/// guard scores `1.0`, at/over the `1.0` threshold.
const ATOMIC_WRITE_FIRE_THRESHOLD: f64 = 1.0;

/// `RESIL-ATOMIC-WRITE.1` — atomic write + rollback smell (T2, SCORED,
/// non-blocking): a truncate-then-write call with no temp-file+rename (or
/// equivalent) guard anywhere in the file scores over threshold. Confidence
/// is fixed at `0.7` for this text-level heuristic (a real AST/call-graph
/// pass could raise it) — both score and confidence are reported in
/// `[0.0, 1.0]`.
pub struct AtomicWriteValidator {
    rule_id: RuleId,
}

/// Fixed confidence this heuristic reports for every finding it emits: a
/// text-level marker scan cannot prove the write is unguarded, only suggest
/// it, so confidence stays below full certainty.
const ATOMIC_WRITE_CONFIDENCE: f64 = 0.7;

impl AtomicWriteValidator {
    /// Build the validator, parsing its `RuleId` literal at construction.
    pub fn new() -> Result<Self, enforcer_domain::boundary::decode_error::DecodeError> {
        Ok(Self {
            rule_id: "RESIL-ATOMIC-WRITE.1".parse()?,
        })
    }
}

impl Validator for AtomicWriteValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let Some(marker) = TRUNCATE_WRITE_MARKERS
            .iter()
            .copied()
            .find(|marker| input.source.contains(marker))
        else {
            return Vec::new();
        };
        let mut score = 1.0_f64;
        if ATOMIC_WRITE_GUARD_MARKERS
            .iter()
            .any(|guard| input.source.contains(guard))
        {
            score -= 1.0;
        }
        if score < ATOMIC_WRITE_FIRE_THRESHOLD {
            return Vec::new();
        }
        vec![finding(
            &FindingSpec {
                rule_id: &self.rule_id,
                severity: Severity::Warning,
                title: "resilience: write path may lack atomic write + rollback",
            },
            format!(
                "found truncate-then-write marker `{marker}` with no atomic temp-file+rename (or \
                 equivalent) guard anywhere in this file (score {score:.1} >= threshold \
                 {ATOMIC_WRITE_FIRE_THRESHOLD:.1}, confidence {ATOMIC_WRITE_CONFIDENCE:.1}); a \
                 crash mid-write can leave a truncated/partial file. This is advisory and does \
                 not block."
            ),
            &input,
            first_line_containing(input.source, marker).unwrap_or(1),
        )]
    }
}

/// Markers of a bare external I/O call: [`IoTimeoutValidator`]'s smell
/// signal. Cross-language, mirroring [`TRUNCATE_WRITE_MARKERS`]'s
/// cross-stack shape.
const EXTERNAL_IO_MARKERS: &[&str] = &[
    "reqwest::get(",
    ".send().await",
    "requests.get(",
    "requests.post(",
    "fetch(",
    "http.get(",
];

/// Markers that indicate the external I/O call above is already guarded by
/// an explicit timeout/retry wrapper, which cancels the
/// [`IoTimeoutValidator`] smell.
const IO_TIMEOUT_GUARD_MARKERS: &[&str] = &[
    "timeout(",
    ".timeout(",
    "with_timeout",
    "retry(",
    "backoff::",
];

/// Score threshold at/above which [`IoTimeoutValidator`] fires (T2,
/// non-blocking): a bare external I/O marker present with no
/// timeout/retry guard anywhere in the file scores `1.0`, at/over the `1.0`
/// threshold.
const IO_TIMEOUT_FIRE_THRESHOLD: f64 = 1.0;

/// Fixed confidence this heuristic reports for every finding it emits (see
/// [`ATOMIC_WRITE_CONFIDENCE`] for why a text-level scan caps below full
/// certainty).
const IO_TIMEOUT_CONFIDENCE: f64 = 0.6;

/// `RESIL-IO-TIMEOUT.1` — I/O timeout/retry smell (T2, SCORED,
/// non-blocking): a bare external network call with no timeout/retry guard
/// anywhere in the file scores over threshold. Confidence is fixed at
/// `0.6` (lower than [`AtomicWriteValidator`]'s, since a bare-call marker
/// is a weaker signal than a truncate-write marker) — both score and
/// confidence are reported in `[0.0, 1.0]`.
pub struct IoTimeoutValidator {
    rule_id: RuleId,
}

impl IoTimeoutValidator {
    /// Build the validator, parsing its `RuleId` literal at construction.
    pub fn new() -> Result<Self, enforcer_domain::boundary::decode_error::DecodeError> {
        Ok(Self {
            rule_id: "RESIL-IO-TIMEOUT.1".parse()?,
        })
    }
}

impl Validator for IoTimeoutValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let Some(marker) = EXTERNAL_IO_MARKERS
            .iter()
            .copied()
            .find(|marker| input.source.contains(marker))
        else {
            return Vec::new();
        };
        let mut score = 1.0_f64;
        if IO_TIMEOUT_GUARD_MARKERS
            .iter()
            .any(|guard| input.source.contains(guard))
        {
            score -= 1.0;
        }
        if score < IO_TIMEOUT_FIRE_THRESHOLD {
            return Vec::new();
        }
        vec![finding(
            &FindingSpec {
                rule_id: &self.rule_id,
                severity: Severity::Warning,
                title: "resilience: external I/O call may lack a timeout/retry guard",
            },
            format!(
                "found bare external I/O marker `{marker}` with no timeout/retry guard anywhere \
                 in this file (score {score:.1} >= threshold {IO_TIMEOUT_FIRE_THRESHOLD:.1}, \
                 confidence {IO_TIMEOUT_CONFIDENCE:.1}); an unbounded call can hang the caller \
                 indefinitely on a stalled peer. This is advisory and does not block."
            ),
            &input,
            first_line_containing(input.source, marker).unwrap_or(1),
        )]
    }
}

/// Build every `resilience` family validator this crate owns (d10): one T1
/// required-test obligation plus two T2 scored smells.
pub fn validators(
) -> Result<Vec<Box<dyn Validator>>, enforcer_domain::boundary::decode_error::DecodeError> {
    Ok(vec![
        Box::new(FailureModeTestValidator::new()?),
        Box::new(AtomicWriteValidator::new()?),
        Box::new(IoTimeoutValidator::new()?),
    ])
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use enforcer_validator::harness::run_fixture_parity;

    use super::*;

    fn manifest_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    #[test]
    fn three_validators_registered_with_unique_rule_ids(
    ) -> Result<(), enforcer_domain::boundary::decode_error::DecodeError> {
        let vs = validators()?;
        assert_eq!(vs.len(), 3);
        let mut seen = std::collections::BTreeSet::new();
        for v in &vs {
            assert!(seen.insert(v.rule_id().to_string()));
        }
        assert_eq!(seen.len(), 3);
        Ok(())
    }

    /// `resilience-auditor` T1 obligation leg: an unmet required-test row
    /// (trust-boundary marker, no companion failure-mode test) fails
    /// review; a met one (companion test present) is silent.
    #[test]
    fn resilience_auditor_failure_mode_test_obligation() -> Result<(), Box<dyn std::error::Error>> {
        let validator = FailureModeTestValidator::new()?;
        run_fixture_parity(
            &validator,
            &manifest_dir(),
            "tests/fixtures/resilience/failure_mode/bad/handler.rs",
            "tests/fixtures/resilience/failure_mode/good/handler.rs",
        )?;
        Ok(())
    }

    #[test]
    fn resilience_auditor_atomic_write_smell() -> Result<(), Box<dyn std::error::Error>> {
        let validator = AtomicWriteValidator::new()?;
        run_fixture_parity(
            &validator,
            &manifest_dir(),
            "tests/fixtures/resilience/atomic_write/bad/writer.rs",
            "tests/fixtures/resilience/atomic_write/good/writer.rs",
        )?;
        Ok(())
    }

    #[test]
    fn resilience_auditor_io_timeout_smell() -> Result<(), Box<dyn std::error::Error>> {
        let validator = IoTimeoutValidator::new()?;
        run_fixture_parity(
            &validator,
            &manifest_dir(),
            "tests/fixtures/resilience/io_timeout/bad/client.rs",
            "tests/fixtures/resilience/io_timeout/good/client.rs",
        )?;
        Ok(())
    }

    /// The smell validators are strictly non-blocking: their findings must
    /// always carry `Severity::Warning`, never `Severity::Error`, so a
    /// consumer treating warnings as advisory-only never has a smell gate
    /// a build.
    #[test]
    fn smells_never_carry_blocking_severity() -> Result<(), Box<dyn std::error::Error>> {
        let atomic = AtomicWriteValidator::new()?;
        let timeout = IoTimeoutValidator::new()?;
        let file: enforcer_domain::paths::RelPath = "crates/x/src/lib.rs".parse()?;

        let atomic_findings = atomic.validate(ValidationInput {
            file: &file,
            source: "fn save() { fs::write(\"x\", data)?; }\n",
            scope: enforcer_domain::findings::ScanScope::Files,
        });
        assert!(!atomic_findings.is_empty(), "fixture assumption failed");
        for f in &atomic_findings {
            assert_eq!(f.severity, Severity::Warning);
        }

        let timeout_findings = timeout.validate(ValidationInput {
            file: &file,
            source: "async fn ping() { reqwest::get(\"http://x\").await?; }\n",
            scope: enforcer_domain::findings::ScanScope::Files,
        });
        assert!(!timeout_findings.is_empty(), "fixture assumption failed");
        for f in &timeout_findings {
            assert_eq!(f.severity, Severity::Warning);
        }
        Ok(())
    }

    /// The required-test obligation is strictly blocking: its finding must
    /// always carry `Severity::Error`.
    #[test]
    fn required_test_obligation_is_blocking() -> Result<(), Box<dyn std::error::Error>> {
        let validator = FailureModeTestValidator::new()?;
        let file: enforcer_domain::paths::RelPath = "crates/x/src/lib.rs".parse()?;
        let findings = validator.validate(ValidationInput {
            file: &file,
            source: "pub fn handle(req: Request) -> Response { todo!() }\n",
            scope: enforcer_domain::findings::ScanScope::Files,
        });
        assert!(!findings.is_empty(), "fixture assumption failed");
        for f in &findings {
            assert_eq!(f.severity, Severity::Error);
        }
        Ok(())
    }

    /// Smell scores/confidence are always reported within `[0.0, 1.0]` —
    /// asserted by parsing the fixed constants this module's scored
    /// validators use, since [`Finding`] itself carries no numeric
    /// score/confidence field (folded into `detail` text instead, mirroring
    /// [`crate::rules::fsm::FsmTransitionCoverageValidator`]).
    #[test]
    fn smell_score_and_confidence_stay_within_unit_bounds() {
        let scores = [ATOMIC_WRITE_FIRE_THRESHOLD, IO_TIMEOUT_FIRE_THRESHOLD];
        let confidences = [ATOMIC_WRITE_CONFIDENCE, IO_TIMEOUT_CONFIDENCE];
        for value in scores.iter().chain(confidences.iter()) {
            assert!(
                (0.0..=1.0).contains(value),
                "value {value} must be in [0.0, 1.0]"
            );
        }
    }

    #[test]
    fn clean_source_with_no_markers_is_silent_across_all_three(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let file: enforcer_domain::paths::RelPath = "crates/x/src/lib.rs".parse()?;
        let source = "fn add(a: i32, b: i32) -> i32 { a + b }\n";
        for v in validators()? {
            let findings = v.validate(ValidationInput {
                file: &file,
                source,
                scope: enforcer_domain::findings::ScanScope::Files,
            });
            assert!(
                findings.is_empty(),
                "expected silence for rule {}",
                v.rule_id()
            );
        }
        Ok(())
    }
}
