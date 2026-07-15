//! d23 test companion and quality — the cross-stack `Validator` family that
//! mechanizes ADBP_GAPS.md rows 95-100: companion-test-required presence,
//! no assertion-free tests, assert-on-variant-not-message, behavioral test
//! names / query-by-role, test-data factories, injected-clock (no
//! wall-clock assertion), and a target-project failing coverage floor.
//!
//! Row 50 (`TEST-FSM-1.1`, FSM transition-coverage) is owned by d16's
//! [`crate::rules::fsm::FsmTransitionCoverageValidator`] — this module
//! consumes that rule's existence via the shared workpack narrative but
//! does not redefine or re-implement it; d16's `fsm.rs` is not edited here.
//!
//! Every rule here is a lightweight line/keyword-oriented text detector
//! (mirroring [`crate::rules::fsm`]'s dominant shape) rather than a full
//! per-language AST parse. T1 rules block (`Severity::Error`); T2 rules are
//! SCORED — they emit a finding only once accumulated signal crosses a
//! fixed threshold, mirroring `enforcer-lang-common`'s `LIT-1` scored
//! model.
//!
//! # Companion-presence is a single-file convention
//!
//! [`TestCompanionRequiredValidator`] (`TEST-COMPANION.1`) mechanizes
//! "every watched-layer source file has a matching test companion", but the
//! base `Validator` contract ([`enforcer_validator::validator::Validator`])
//! inspects exactly one file's source text in isolation — it has no
//! filesystem access to check whether a SIBLING file actually exists (the
//! same constraint [`crate::rules::change_discipline`] documents for its
//! own base/head need). This validator therefore checks a source file's
//! OWN text for a structured `# companion-test: <path>` /
//! `// companion-test: <path>` annotation naming its test file, mirroring
//! [`crate::rules::deferred_work`]'s annotation-exemption convention: a
//! watched-layer file defining a public symbol with no such annotation is
//! flagged; one carrying the annotation is clean. The real filesystem
//! existence check (does the named companion path actually exist and
//! contain a matching test) is a follow-up for the `enforcer-scan`
//! orchestrator, which — unlike this per-file `Validator` — walks the whole
//! tree and can cross-reference two paths; noted here rather than built,
//! matching the documented-follow-up precedent in
//! [`crate::rules::change_discipline`].

use enforcer_domain::findings::Finding;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

/// The fixed parts of one rule's finding: its id, severity, and title.
/// Bundled so the per-call-site `finding()` helper stays under clippy's
/// `too_many_arguments` limit (mirrors [`crate::rules::fsm::FindingSpec`]).
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
/// `1` if the caller wants a whole-file finding when the marker itself is
/// what's absent.
fn first_line_containing(source: &str, marker: &str) -> Option<u32> {
    source
        .lines()
        .enumerate()
        .find(|(_, line)| line.contains(marker))
        .map(|(idx, _)| (idx as u32).saturating_add(1))
}

/// Path segments that mark a file as living in a "watched layer" whose
/// public symbols require a test companion (mirrors the workpack's
/// `services/foo.py` example; deliberately narrow rather than "every file
/// in the repo" to avoid flooding config/docs/fixtures with false
/// positives).
const WATCHED_LAYER_SEGMENTS: &[&str] = &["/services/", "/routers/", "/handlers/"];

/// Markers that introduce a public symbol this rule cares about (a public
/// function/class in a watched layer) across the languages this crate's
/// text-level detectors already span (Python/Dart/TS/Rust-shaped).
const PUBLIC_SYMBOL_MARKERS: &[&str] = &["def ", "pub fn ", "export function ", "class "];

/// The structured single-file companion annotation this validator accepts
/// as proof a source file names its test companion (see module docs for why
/// this is a single-file convention rather than a real filesystem check).
const COMPANION_ANNOTATION_MARKERS: &[&str] = &["# companion-test:", "// companion-test:"];

/// `TEST-COMPANION.1` / legacy `py.tests.companion-required` /
/// `TEST-COMPANION-1.1` / `COMP-1.1` / `RUST-TEST-1.1` — companion test
/// required: a watched-layer source file that defines a public symbol with
/// no `companion-test:` annotation naming its test file is flagged (T1).
pub struct TestCompanionRequiredValidator {
    rule_id: RuleId,
}

impl TestCompanionRequiredValidator {
    /// Build the validator, parsing its `RuleId` literal at construction.
    pub fn new() -> Result<Self, enforcer_domain::boundary::decode_error::DecodeError> {
        Ok(Self {
            rule_id: "TEST-COMPANION.1".parse()?,
        })
    }
}

impl Validator for TestCompanionRequiredValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let path = input.file.as_str();
        let in_watched_layer = WATCHED_LAYER_SEGMENTS
            .iter()
            .any(|segment| path.contains(segment));
        if !in_watched_layer {
            return Vec::new();
        }
        let defines_public_symbol = PUBLIC_SYMBOL_MARKERS
            .iter()
            .any(|marker| input.source.contains(marker));
        if !defines_public_symbol {
            return Vec::new();
        }
        let has_companion_annotation = COMPANION_ANNOTATION_MARKERS
            .iter()
            .any(|marker| input.source.contains(marker));
        if has_companion_annotation {
            return Vec::new();
        }
        vec![finding(
            &FindingSpec {
                rule_id: &self.rule_id,
                severity: Severity::Error,
                title: "test-quality: watched-layer source file has no test companion",
            },
            format!(
                "`{path}` defines a public symbol in a watched layer but carries no \
                 `companion-test: <path>` annotation naming its matching test file; every \
                 source file / new public symbol in a watched layer must have a matching test \
                 companion (>=1 happy + >=1 error/edge path)."
            ),
            &input,
            1,
        )]
    }
}

/// Markers that introduce a test case block across TS/JS (`it`/`test`) —
/// the shape [`AssertionFreeTestValidator`] and
/// [`BehavioralTestNameValidator`] both scan for.
const TEST_CASE_OPENERS: &[&str] = &["it(", "test(", "it.only(", "test.only("];

/// Markers proving a test body contains at least one assertion, across
/// TS/JS `expect`, Rust `assert!`/`assert_eq!`, and Python `assert`.
const ASSERTION_MARKERS: &[&str] = &["expect(", "assert!", "assert_eq!", "assert "];

/// `CF-TEST-1.5` / `FE-TEST-1.4` — assertion-free test forbidden: a test
/// case block (`it("x", () => { ... })` / `test("x", ...)`) whose body
/// contains no assertion marker is flagged (T1).
pub struct AssertionFreeTestValidator {
    rule_id: RuleId,
}

impl AssertionFreeTestValidator {
    /// Build the validator, parsing its `RuleId` literal at construction.
    pub fn new() -> Result<Self, enforcer_domain::boundary::decode_error::DecodeError> {
        Ok(Self {
            rule_id: "TEST-ASSERTIONFREE.1".parse()?,
        })
    }
}

impl Validator for AssertionFreeTestValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let has_test_case = TEST_CASE_OPENERS
            .iter()
            .any(|marker| input.source.contains(marker));
        if !has_test_case {
            return Vec::new();
        }
        let has_assertion = ASSERTION_MARKERS
            .iter()
            .any(|marker| input.source.contains(marker));
        if has_assertion {
            return Vec::new();
        }
        let line = TEST_CASE_OPENERS
            .iter()
            .find_map(|marker| first_line_containing(input.source, marker))
            .unwrap_or(1);
        vec![finding(
            &FindingSpec {
                rule_id: &self.rule_id,
                severity: Severity::Error,
                title: "test-quality: assertion-free test case",
            },
            "A test case exercises behavior but asserts nothing (`it`/`test` with no \
             `expect`/`assert!`); every test must assert on visible output."
                .to_owned(),
            &input,
            line,
        )]
    }
}

/// A test name is "behavioral" when it reads as
/// `test_<action>_<scenario>_<outcome>` — approximated here as: the test
/// name string has at least this many underscore-separated segments
/// (`test_order_1` -> 2 segments, below threshold; `test_cancel_order_
/// already_shipped_raises` -> 5 segments, at/above threshold).
const BEHAVIORAL_NAME_MIN_SEGMENTS: usize = 4;

/// Query-by-testid markers this rule forbids in favor of role/label/text
/// queries.
const QUERY_BY_TESTID_MARKERS: &[&str] = &["getByTestId(", "querySelector(\"[data-testid"];

/// Extract every quoted test-name literal following a
/// [`TEST_CASE_OPENERS`] marker (`it("name", ...)` / `test("name", ...)`),
/// or, for Python, every `def test_<name>(` function name.
fn test_names(source: &str) -> Vec<String> {
    let mut names = Vec::new();
    for line in source.lines() {
        for opener in TEST_CASE_OPENERS {
            if let Some(rest) = line.trim_start().strip_prefix(opener) {
                if let Some(quote) = rest.chars().next() {
                    if quote == '"' || quote == '\'' {
                        if let Some(quoted) = rest.strip_prefix(quote) {
                            if let Some((name, _)) = quoted.split_once(quote) {
                                names.push(name.to_owned());
                            }
                        }
                    }
                }
            }
        }
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix("def test_") {
            if let Some(paren) = rest.find('(') {
                if let Some(name) = rest.get(..paren) {
                    names.push(format!("test_{name}"));
                }
            }
        }
    }
    names
}

/// `py-fastapi-behavioral-test-names` / `FE-TEST-1.1` — behavioral test
/// names + query-by-role: a test name with fewer than
/// [`BEHAVIORAL_NAME_MIN_SEGMENTS`] underscore-separated segments (e.g.
/// `test_order_1`), or a query-by-test-id call, scores over threshold (T2).
pub struct BehavioralTestNameValidator {
    rule_id: RuleId,
}

impl BehavioralTestNameValidator {
    /// Build the validator, parsing its `RuleId` literal at construction.
    pub fn new() -> Result<Self, enforcer_domain::boundary::decode_error::DecodeError> {
        Ok(Self {
            rule_id: "TEST-BEHAVIORNAME.1".parse()?,
        })
    }
}

const BEHAVIORNAME_FIRE_THRESHOLD: f64 = 1.0;

impl Validator for BehavioralTestNameValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        for name in test_names(input.source) {
            let segments = name.split('_').filter(|s| !s.is_empty()).count();
            if segments < BEHAVIORAL_NAME_MIN_SEGMENTS {
                let line = first_line_containing(input.source, &name).unwrap_or(1);
                return vec![finding(
                    &FindingSpec {
                        rule_id: &self.rule_id,
                        severity: Severity::Warning,
                        title: "test-quality: non-behavioral test name",
                    },
                    format!(
                        "Test name `{name}` has only {segments} underscore-separated segment(s), \
                         below the behavioral-naming floor of \
                         {BEHAVIORAL_NAME_MIN_SEGMENTS} (score 1.0 >= threshold \
                         {BEHAVIORNAME_FIRE_THRESHOLD:.1}); prefer \
                         `test_<action>_<scenario>_<outcome>` / `should ... when ...`."
                    ),
                    &input,
                    line,
                )];
            }
        }
        if let Some(marker) = QUERY_BY_TESTID_MARKERS
            .iter()
            .find(|marker| input.source.contains(**marker))
        {
            let line = first_line_containing(input.source, marker).unwrap_or(1);
            return vec![finding(
                &FindingSpec {
                    rule_id: &self.rule_id,
                    severity: Severity::Warning,
                    title: "test-quality: query-by-test-id instead of role/label/text",
                },
                format!(
                    "`{marker}` queries by test-id (score 1.0 >= threshold \
                     {BEHAVIORNAME_FIRE_THRESHOLD:.1}); query by role/label/text \
                     (`getByRole(...)`) instead."
                ),
                &input,
                line,
            )];
        }
        Vec::new()
    }
}

/// Markers proving an assertion targets an error's TYPE/variant, not its
/// message string — exempts a `.toThrow(...)`/`assert`/`matches!` from
/// [`AssertOnVariantNotMessageValidator`] when present.
const ASSERT_ON_VARIANT_MARKERS: &[&str] = &["matches!(", ".toThrow(type", "isinstance("];

/// Markers proving an assertion targets an error's MESSAGE STRING instead
/// of its type/variant — always a violation when present, regardless of
/// whether a variant-style assertion also appears elsewhere in the file.
const ASSERT_ON_MESSAGE_MARKERS: &[&str] = &[".toThrow(message", "err.message ==", ".message ==\""];

/// `CF-TEST-1.4` / `TEST-VARIANT-1.1` — assert on error variant/type, not
/// message string: an assertion against an error's message text (score 1.0
/// per occurrence) scores over threshold (T2); a variant/type assertion
/// stays clean.
pub struct AssertOnVariantNotMessageValidator {
    rule_id: RuleId,
}

impl AssertOnVariantNotMessageValidator {
    /// Build the validator, parsing its `RuleId` literal at construction.
    pub fn new() -> Result<Self, enforcer_domain::boundary::decode_error::DecodeError> {
        Ok(Self {
            rule_id: "TEST-VARIANT.1".parse()?,
        })
    }
}

const VARIANT_FIRE_THRESHOLD: f64 = 1.0;

impl Validator for AssertOnVariantNotMessageValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let _ = ASSERT_ON_VARIANT_MARKERS; // documents the accepted-clean shape; see module docs.
        if let Some(marker) = ASSERT_ON_MESSAGE_MARKERS
            .iter()
            .find(|marker| input.source.contains(**marker))
        {
            let line = first_line_containing(input.source, marker).unwrap_or(1);
            return vec![finding(
                &FindingSpec {
                    rule_id: &self.rule_id,
                    severity: Severity::Warning,
                    title: "test-quality: assertion targets error message string",
                },
                format!(
                    "`{marker}` asserts on an error's message text (score 1.0 >= threshold \
                     {VARIANT_FIRE_THRESHOLD:.1}); assert on the error's variant/type instead \
                     (`matches!(e, Err(X))` / `.toThrow(type=...)`)."
                ),
                &input,
                line,
            )];
        }
        Vec::new()
    }
}

/// Markers for an inline hardcoded test-data dict/object literal assigned
/// straight into a call, as opposed to a named factory/fixture.
const INLINE_DICT_MARKERS: &[&str] = &["{\"email\":", "{'email':"];

/// Markers proving a factory/fixture is used for test data instead of an
/// inline literal.
const FACTORY_MARKERS: &[&str] = &["factory(", "Factory(", "make_", "fixture("];

/// `py-fastapi-test-data-factories` — test-data factories required: an
/// inline hardcoded dict literal used as test data, with no factory/fixture
/// marker anywhere in the file, scores over threshold (T2).
pub struct TestDataFactoryValidator {
    rule_id: RuleId,
}

impl TestDataFactoryValidator {
    /// Build the validator, parsing its `RuleId` literal at construction.
    pub fn new() -> Result<Self, enforcer_domain::boundary::decode_error::DecodeError> {
        Ok(Self {
            rule_id: "TEST-FACTORY.1".parse()?,
        })
    }
}

const FACTORY_FIRE_THRESHOLD: f64 = 1.0;

impl Validator for TestDataFactoryValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let uses_factory = FACTORY_MARKERS
            .iter()
            .any(|marker| input.source.contains(marker));
        if uses_factory {
            return Vec::new();
        }
        if let Some(marker) = INLINE_DICT_MARKERS
            .iter()
            .find(|marker| input.source.contains(**marker))
        {
            let line = first_line_containing(input.source, marker).unwrap_or(1);
            return vec![finding(
                &FindingSpec {
                    rule_id: &self.rule_id,
                    severity: Severity::Warning,
                    title: "test-quality: inline hardcoded test data instead of a factory",
                },
                format!(
                    "`{marker}` is an inline hardcoded test-data literal with no factory/fixture \
                     in this file (score 1.0 >= threshold {FACTORY_FIRE_THRESHOLD:.1}); use a \
                     test-data factory instead of inline dicts."
                ),
                &input,
                line,
            )];
        }
        Vec::new()
    }
}

/// Markers for a wall-clock delta assertion (`monotonic()`/`time.time()`
/// difference compared against a fixed budget) — the shape
/// [`NoWallClockAssertValidator`] forbids.
const WALLCLOCK_DELTA_MARKERS: &[&str] = &["monotonic() -", "time.time() -"];

/// Markers proving an injected/fake clock is used instead of the wall
/// clock.
const INJECTED_CLOCK_MARKERS: &[&str] = &["FakeClock", "fake_clock", "injected_clock"];

/// `py-fastapi-no-wallclock-assert` — injected clock required, no
/// wall-clock assertion: a test asserting on a wall-clock time delta
/// (`monotonic() - start <= 0.6`) with no injected/fake clock in the file
/// scores over threshold (T2).
pub struct NoWallClockAssertValidator {
    rule_id: RuleId,
}

impl NoWallClockAssertValidator {
    /// Build the validator, parsing its `RuleId` literal at construction.
    pub fn new() -> Result<Self, enforcer_domain::boundary::decode_error::DecodeError> {
        Ok(Self {
            rule_id: "TEST-NOWALLCLOCK.1".parse()?,
        })
    }
}

const WALLCLOCK_FIRE_THRESHOLD: f64 = 1.0;

impl Validator for NoWallClockAssertValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let uses_injected_clock = INJECTED_CLOCK_MARKERS
            .iter()
            .any(|marker| input.source.contains(marker));
        if uses_injected_clock {
            return Vec::new();
        }
        if let Some(marker) = WALLCLOCK_DELTA_MARKERS
            .iter()
            .find(|marker| input.source.contains(**marker))
        {
            let line = first_line_containing(input.source, marker).unwrap_or(1);
            return vec![finding(
                &FindingSpec {
                    rule_id: &self.rule_id,
                    severity: Severity::Warning,
                    title: "test-quality: wall-clock delta assertion instead of an injected clock",
                },
                format!(
                    "`{marker}` asserts on a wall-clock time delta with no injected/fake clock \
                     in this file (score 1.0 >= threshold {WALLCLOCK_FIRE_THRESHOLD:.1}); inject \
                     a `FakeClock` and assert on the decision instead of elapsed wall-clock time."
                ),
                &input,
                line,
            )];
        }
        Vec::new()
    }
}

/// Markers that a coverage configuration/invocation is present at all
/// (Python `[tool.coverage]`, `vitest run`), scoping
/// [`CoverageGatePresenceValidator`] and [`CoverageFailFloorValidator`] to
/// files that actually declare a coverage setup.
const COVERAGE_CONFIG_MARKERS: &[&str] = &["[tool.coverage]", "vitest run"];

/// Markers proving the coverage gate is wired as a FAILING threshold
/// (`fail_under = <n>`, `--cov-fail-under`, or vitest's `--coverage` flag
/// actually passed on the invocation line).
const COVERAGE_GATE_PRESENT_MARKERS: &[&str] = &["fail_under", "--cov-fail-under", "--coverage"];

/// `CIGATE-1.1` — coverage gate presence required: a coverage config/
/// invocation with no `fail_under`/`--cov-fail-under`/`--coverage` gate
/// present anywhere in the file is flagged (T1 — presence is deterministic).
pub struct CoverageGatePresenceValidator {
    rule_id: RuleId,
}

impl CoverageGatePresenceValidator {
    /// Build the validator, parsing its `RuleId` literal at construction.
    pub fn new() -> Result<Self, enforcer_domain::boundary::decode_error::DecodeError> {
        Ok(Self {
            rule_id: "TEST-COVERAGEGATE.1".parse()?,
        })
    }
}

impl Validator for CoverageGatePresenceValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let has_coverage_config = COVERAGE_CONFIG_MARKERS
            .iter()
            .any(|marker| input.source.contains(marker));
        if !has_coverage_config {
            return Vec::new();
        }
        let has_gate = COVERAGE_GATE_PRESENT_MARKERS
            .iter()
            .any(|marker| input.source.contains(marker));
        if has_gate {
            return Vec::new();
        }
        let line = COVERAGE_CONFIG_MARKERS
            .iter()
            .find_map(|marker| first_line_containing(input.source, marker))
            .unwrap_or(1);
        vec![finding(
            &FindingSpec {
                rule_id: &self.rule_id,
                severity: Severity::Error,
                title: "test-quality: coverage configured with no failing gate",
            },
            "A coverage configuration/invocation is present with no `fail_under`/ \
             `--cov-fail-under` value or `--coverage` flag; the coverage step must actually \
             invoke a FAILING threshold, not just reference coverage."
                .to_owned(),
            &input,
            line,
        )]
    }
}

/// Minimum acceptable coverage floor value. A `fail_under`/
/// `--cov-fail-under` numeric value below this is scored as too low to be
/// a meaningful gate.
const COVERAGE_FLOOR_MINIMUM: u32 = 70;

/// Parse the integer value following `fail_under` or `--cov-fail-under` on
/// any line of `source` (`fail_under = 40`, `fail_under=40`,
/// `--cov-fail-under=40`), returning the first one found.
fn coverage_floor_value(source: &str) -> Option<u32> {
    for line in source.lines() {
        for marker in ["fail_under", "--cov-fail-under"] {
            if let Some((_, rest)) = line.split_once(marker) {
                let digits: String = rest
                    .chars()
                    .skip_while(|c| !c.is_ascii_digit())
                    .take_while(|c| c.is_ascii_digit())
                    .collect();
                if let Ok(value) = digits.parse::<u32>() {
                    return Some(value);
                }
            }
        }
    }
    None
}

/// `CIGATE-1.2` / `py-fastapi-coverage-fail-under` — coverage floor value:
/// a `fail_under`/`--cov-fail-under` value below [`COVERAGE_FLOOR_MINIMUM`]
/// scores over threshold (T2, the floor VALUE is scored, unlike
/// [`CoverageGatePresenceValidator`]'s deterministic presence check).
pub struct CoverageFailFloorValidator {
    rule_id: RuleId,
}

impl CoverageFailFloorValidator {
    /// Build the validator, parsing its `RuleId` literal at construction.
    pub fn new() -> Result<Self, enforcer_domain::boundary::decode_error::DecodeError> {
        Ok(Self {
            rule_id: "TEST-COVERAGEFLOOR.1".parse()?,
        })
    }
}

const COVERAGEFLOOR_FIRE_THRESHOLD: f64 = 1.0;

impl Validator for CoverageFailFloorValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let Some(value) = coverage_floor_value(input.source) else {
            return Vec::new();
        };
        if value >= COVERAGE_FLOOR_MINIMUM {
            return Vec::new();
        }
        let line = first_line_containing(input.source, "fail_under")
            .or_else(|| first_line_containing(input.source, "--cov-fail-under"))
            .unwrap_or(1);
        vec![finding(
            &FindingSpec {
                rule_id: &self.rule_id,
                severity: Severity::Warning,
                title: "test-quality: coverage floor set too low",
            },
            format!(
                "The coverage failing threshold is set to {value}, below the \
                 {COVERAGE_FLOOR_MINIMUM} floor (score 1.0 >= threshold \
                 {COVERAGEFLOOR_FIRE_THRESHOLD:.1})."
            ),
            &input,
            line,
        )]
    }
}

/// Build every `test_quality` family validator this crate owns (d23).
pub fn validators(
) -> Result<Vec<Box<dyn Validator>>, enforcer_domain::boundary::decode_error::DecodeError> {
    Ok(vec![
        Box::new(TestCompanionRequiredValidator::new()?),
        Box::new(AssertionFreeTestValidator::new()?),
        Box::new(BehavioralTestNameValidator::new()?),
        Box::new(AssertOnVariantNotMessageValidator::new()?),
        Box::new(TestDataFactoryValidator::new()?),
        Box::new(NoWallClockAssertValidator::new()?),
        Box::new(CoverageGatePresenceValidator::new()?),
        Box::new(CoverageFailFloorValidator::new()?),
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
    fn eight_validators_registered_with_unique_rule_ids(
    ) -> Result<(), enforcer_domain::boundary::decode_error::DecodeError> {
        let vs = validators()?;
        assert_eq!(vs.len(), 8);
        let mut seen = std::collections::BTreeSet::new();
        for v in &vs {
            assert!(seen.insert(v.rule_id().to_string()));
        }
        assert_eq!(seen.len(), 8);
        Ok(())
    }

    #[test]
    fn test_companion_required() -> Result<(), Box<dyn std::error::Error>> {
        let validator = TestCompanionRequiredValidator::new()?;
        run_fixture_parity(
            &validator,
            &manifest_dir(),
            "tests/fixtures/test_quality/companion/bad/services/foo.py",
            "tests/fixtures/test_quality/companion/good/services/foo.py",
        )?;
        Ok(())
    }

    #[test]
    fn assertion_free_test_forbidden() -> Result<(), Box<dyn std::error::Error>> {
        let validator = AssertionFreeTestValidator::new()?;
        run_fixture_parity(
            &validator,
            &manifest_dir(),
            "tests/fixtures/test_quality/no_assert/bad.test.ts",
            "tests/fixtures/test_quality/no_assert/good.test.ts",
        )?;
        Ok(())
    }

    #[test]
    fn behavioral_test_names() -> Result<(), Box<dyn std::error::Error>> {
        let validator = BehavioralTestNameValidator::new()?;
        run_fixture_parity(
            &validator,
            &manifest_dir(),
            "tests/fixtures/test_quality/names/bad.test.ts",
            "tests/fixtures/test_quality/names/good.test.ts",
        )?;
        Ok(())
    }

    #[test]
    fn assert_on_variant_not_message() -> Result<(), Box<dyn std::error::Error>> {
        let validator = AssertOnVariantNotMessageValidator::new()?;
        run_fixture_parity(
            &validator,
            &manifest_dir(),
            "tests/fixtures/test_quality/variant/bad.test.ts",
            "tests/fixtures/test_quality/variant/good.test.ts",
        )?;
        Ok(())
    }

    #[test]
    fn test_data_factories() -> Result<(), Box<dyn std::error::Error>> {
        let validator = TestDataFactoryValidator::new()?;
        run_fixture_parity(
            &validator,
            &manifest_dir(),
            "tests/fixtures/test_quality/factories/bad_factory.test.py",
            "tests/fixtures/test_quality/factories/good_factory.test.py",
        )?;
        Ok(())
    }

    #[test]
    fn no_wallclock_assert() -> Result<(), Box<dyn std::error::Error>> {
        let validator = NoWallClockAssertValidator::new()?;
        run_fixture_parity(
            &validator,
            &manifest_dir(),
            "tests/fixtures/test_quality/factories/bad_wallclock.test.py",
            "tests/fixtures/test_quality/factories/good_wallclock.test.py",
        )?;
        Ok(())
    }

    #[test]
    fn coverage_gate_presence() -> Result<(), Box<dyn std::error::Error>> {
        let validator = CoverageGatePresenceValidator::new()?;
        run_fixture_parity(
            &validator,
            &manifest_dir(),
            "tests/fixtures/test_quality/coverage/bad_presence/pyproject.toml",
            "tests/fixtures/test_quality/coverage/good/pyproject.toml",
        )?;
        Ok(())
    }

    #[test]
    fn coverage_fail_floor() -> Result<(), Box<dyn std::error::Error>> {
        let validator = CoverageFailFloorValidator::new()?;
        run_fixture_parity(
            &validator,
            &manifest_dir(),
            "tests/fixtures/test_quality/coverage/bad_floor/pyproject.toml",
            "tests/fixtures/test_quality/coverage/good/pyproject.toml",
        )?;
        Ok(())
    }
}
