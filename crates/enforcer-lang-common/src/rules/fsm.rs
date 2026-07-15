//! d16 FSM transition validity — the cross-stack `Validator` family that
//! mechanizes ADBP_GAPS.md rows 41-50: mandatory FSM routing for
//! `status`/`role`/`type` mutation, an explicit transitions map, canonical
//! FSM file layout, fixed-set literal-must-be-enum, `StrEnum`-only enum
//! location, no silent enum-parse fallback, validate-before-mutate,
//! terminal-state no outgoing edge, FSM singleton/stateless discipline, and
//! transition-coverage testing.
//!
//! Every rule here is a lightweight line/keyword-oriented text detector
//! (mirroring [`crate::pattern::PatternValidator`]'s dominant shape) rather
//! than a full per-language AST parse — this crate has no tree-sitter/AST
//! dependency for Python/Dart/CFML, and the ADBP source rules are
//! themselves keyword/shape scans over source text. T1 rules block (return
//! a finding whenever the marker shape is present); T2 rules are SCORED —
//! they emit a finding only once accumulated signal crosses a fixed
//! threshold, mirroring `enforcer-lang-common`'s `LIT-1` scored model.

use enforcer_domain::findings::Finding;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

/// The fixed parts of one rule's finding: its id, severity, and title.
/// Bundled so the per-call-site `finding()` helper stays under clippy's
/// `too_many_arguments` limit while each validator keeps its own
/// rule-id/severity/title as a `const`-like value at the call site.
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
/// `1` if the marker itself is what's absent and the caller wants a
/// whole-file finding.
fn first_line_containing(source: &str, marker: &str) -> Option<u32> {
    source
        .lines()
        .enumerate()
        .find(|(_, line)| line.contains(marker))
        .map(|(idx, _)| (idx as u32).saturating_add(1))
}

/// FSM-1.1 — mandatory FSM for stateful entity: a `status`/`role`/`type`
/// field mutation (`self.status = ...` / `this.status = ...` /
/// `order.status = ...`) must route through a transition call, not a raw
/// assignment. Fires (T1) when a raw assignment to one of the tracked
/// field names is present AND no transition-call marker
/// (`transition(`/`assert_transition(`) appears anywhere in the file.
pub struct MandatoryFsmValidator {
    rule_id: RuleId,
}

impl MandatoryFsmValidator {
    /// Build the validator, parsing its `RuleId` literal at construction.
    pub fn new() -> Result<Self, enforcer_domain::boundary::decode_error::DecodeError> {
        Ok(Self {
            rule_id: "FSM-1.1".parse()?,
        })
    }
}

const TRACKED_FIELDS: &[&str] = &[".status = ", ".role = ", ".type = "];
const TRANSITION_MARKERS: &[&str] = &["transition(", "assert_transition(", ".transition("];

impl Validator for MandatoryFsmValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let has_transition_call = TRANSITION_MARKERS
            .iter()
            .any(|marker| input.source.contains(marker));
        if has_transition_call {
            return Vec::new();
        }
        for (line_idx, line) in input.source.lines().enumerate() {
            let trimmed = line.trim_start();
            // A raw assignment: the field write is not itself the RHS of a
            // transition call and is not a declaration (`status: str`).
            if TRACKED_FIELDS.iter().any(|field| line.contains(field))
                && !trimmed.starts_with('#')
                && !trimmed.starts_with("//")
            {
                return vec![finding(
                    &FindingSpec {
                        rule_id: &self.rule_id,
                        severity: Severity::Error,
                        title: "fsm: raw status/role/type assignment without a transition call",
                    },
                    "A status/role/type field mutation must route through a transition call \
                     (e.g. `fsm.transition(...)`/`assert_transition(...)`), never a raw \
                     assignment."
                        .to_owned(),
                    &input,
                    (line_idx as u32).saturating_add(1),
                )];
            }
        }
        Vec::new()
    }
}

/// FSM-EXPLICITMAP.1 — explicit transitions map required. Fires (T1) when
/// an ad-hoc `setStatus(String ...)`/`set_status(str ...)` mutator is
/// present without a declared `transitions` map (`const transitions =` /
/// `transitions = {` / `transitions()`) anywhere in the file.
pub struct ExplicitTransitionsMapValidator {
    rule_id: RuleId,
}

impl ExplicitTransitionsMapValidator {
    /// Build the validator, parsing its `RuleId` literal at construction.
    pub fn new() -> Result<Self, enforcer_domain::boundary::decode_error::DecodeError> {
        Ok(Self {
            rule_id: "FSM-EXPLICITMAP.1".parse()?,
        })
    }
}

const AD_HOC_SETTERS: &[&str] = &["setStatus(String", "set_status(str"];
const TRANSITIONS_MAP_MARKERS: &[&str] = &["transitions = {", "const transitions", "transitions()"];

impl Validator for ExplicitTransitionsMapValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let has_map = TRANSITIONS_MAP_MARKERS
            .iter()
            .any(|marker| input.source.contains(marker));
        if has_map {
            return Vec::new();
        }
        for setter in AD_HOC_SETTERS {
            if let Some(line) = first_line_containing(input.source, setter) {
                return vec![finding(
                    &FindingSpec {
                        rule_id: &self.rule_id,
                        severity: Severity::Error,
                        title: "fsm: ad-hoc string setter without a declared transitions map",
                    },
                    format!(
                        "`{setter}` mutates state without a declared states->transitions map \
                         (`transitions = {{...}}` / `transitions()`); declare the allowed \
                         transitions explicitly."
                    ),
                    &input,
                    line,
                )];
            }
        }
        Vec::new()
    }
}

/// FSM-LAYOUT.1 — FSM canonical layout: a transitions map must live under a
/// `state_machines/` path segment; one declared under `models/` is a
/// violation (T1, path-keyed).
pub struct FsmCanonicalLayoutValidator {
    rule_id: RuleId,
}

impl FsmCanonicalLayoutValidator {
    /// Build the validator, parsing its `RuleId` literal at construction.
    pub fn new() -> Result<Self, enforcer_domain::boundary::decode_error::DecodeError> {
        Ok(Self {
            rule_id: "FSM-LAYOUT.1".parse()?,
        })
    }
}

impl Validator for FsmCanonicalLayoutValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let has_transitions_map = input.source.contains("transitions = {");
        if !has_transitions_map {
            return Vec::new();
        }
        let path = input.file.as_str();
        if path.contains("/models/") && !path.contains("/state_machines/") {
            return vec![finding(
                &FindingSpec {
                    rule_id: &self.rule_id,
                    severity: Severity::Error,
                    title: "fsm: transitions map declared outside state_machines/",
                },
                format!(
                    "`{path}` declares a states->transitions map under `models/`; transition \
                     maps must live in `state_machines/` (enums belong in `enums/`)."
                ),
                &input,
                first_line_containing(input.source, "transitions = {").unwrap_or(1),
            )];
        }
        Vec::new()
    }
}

/// FSM-LITERALENUM.1 — fixed-set literal must be enum: `if status ==
/// "pending":`-shaped comparisons on a status/role/type field must compare
/// an enum member, not a bare string literal (T1 where the field binding is
/// a known status/role/type symbol, which is the only shape this text-level
/// detector recognizes).
pub struct StatusStringLiteralForbiddenValidator {
    rule_id: RuleId,
}

impl StatusStringLiteralForbiddenValidator {
    /// Build the validator, parsing its `RuleId` literal at construction.
    pub fn new() -> Result<Self, enforcer_domain::boundary::decode_error::DecodeError> {
        Ok(Self {
            rule_id: "FSM-LITERALENUM.1".parse()?,
        })
    }
}

const STATUS_LITERAL_COMPARISONS: &[&str] = &[".status == \"", ".role == \"", ".type == \""];

impl Validator for StatusStringLiteralForbiddenValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        for (line_idx, line) in input.source.lines().enumerate() {
            if STATUS_LITERAL_COMPARISONS
                .iter()
                .any(|marker| line.contains(marker))
            {
                return vec![finding(
                    &FindingSpec {
                        rule_id: &self.rule_id,
                        severity: Severity::Error,
                        title: "fsm: status/role/type compared against a bare string literal",
                    },
                    "A fixed-set status/role/type comparison must reference a typed enum \
                     member (e.g. `Status.PENDING`), never a bare string literal."
                        .to_owned(),
                    &input,
                    (line_idx as u32).saturating_add(1),
                )];
            }
        }
        Vec::new()
    }
}

/// `py.enum.strenum-only` — enums in `enums/`, `StrEnum` base: every class
/// declared under an `enums/` path segment must inherit a typed enum base
/// (`StrEnum`); a plain `Enum` base anywhere, or a `class ...(Enum)`
/// declared outside `enums/`, is a violation.
pub struct EnumLocationStrEnumOnlyValidator {
    rule_id: RuleId,
}

impl EnumLocationStrEnumOnlyValidator {
    /// Build the validator, parsing its `RuleId` literal at construction.
    pub fn new() -> Result<Self, enforcer_domain::boundary::decode_error::DecodeError> {
        Ok(Self {
            rule_id: "FSM-ENUMLOC.1".parse()?,
        })
    }
}

impl Validator for EnumLocationStrEnumOnlyValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let path = input.file.as_str();
        for (line_idx, line) in input.source.lines().enumerate() {
            let trimmed = line.trim_start();
            if !trimmed.starts_with("class ") {
                continue;
            }
            let is_plain_enum = trimmed.contains("(Enum)") && !trimmed.contains("StrEnum");
            let outside_enums_dir = !path.contains("/enums/");
            let declares_enum_outside_enums_dir = trimmed.contains("Enum") && outside_enums_dir;
            if is_plain_enum || declares_enum_outside_enums_dir {
                return vec![finding(
                    &FindingSpec {
                        rule_id: &self.rule_id,
                        severity: Severity::Error,
                        title: "fsm: enum class outside enums/ or not StrEnum-based",
                    },
                    format!(
                        "`{path}` declares an enum class outside `enums/`, or the class \
                         does not inherit `StrEnum`/a typed enum base."
                    ),
                    &input,
                    (line_idx as u32).saturating_add(1),
                )];
            }
        }
        Vec::new()
    }
}

/// DART-TYPE-1.7 — enum parse no silent fallback: no `firstWhere(...,
/// orElse: () => X.item)` / `?? default` variant fallback on enum parse;
/// the parse must throw or return nullable.
pub struct EnumParseNoFallbackValidator {
    rule_id: RuleId,
}

impl EnumParseNoFallbackValidator {
    /// Build the validator, parsing its `RuleId` literal at construction.
    pub fn new() -> Result<Self, enforcer_domain::boundary::decode_error::DecodeError> {
        Ok(Self {
            rule_id: "FSM-ENUMPARSE.1".parse()?,
        })
    }
}

impl Validator for EnumParseNoFallbackValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        if let Some(line) = first_line_containing(input.source, "orElse: () =>") {
            return vec![finding(
                &FindingSpec {
                    rule_id: &self.rule_id,
                    severity: Severity::Error,
                    title: "fsm: enum parse falls back silently via orElse",
                },
                "An enum parse must throw or return nullable on an unrecognized value; \
                 `firstWhere(..., orElse: () => X.item)` silently substitutes a default \
                 variant instead."
                    .to_owned(),
                &input,
                line,
            )];
        }
        Vec::new()
    }
}

/// CF-FSM-2.2 — validate-before-mutate / raise-on-invalid: a
/// `can_transition`/`canTransition` predicate whose result is discarded
/// (called but not checked before the following mutation) is a violation;
/// the illegal-transition path must `assert_transition`-raise instead.
pub struct ValidateBeforeMutateValidator {
    rule_id: RuleId,
}

impl ValidateBeforeMutateValidator {
    /// Build the validator, parsing its `RuleId` literal at construction.
    pub fn new() -> Result<Self, enforcer_domain::boundary::decode_error::DecodeError> {
        Ok(Self {
            rule_id: "FSM-VALIDATEMUTATE.1".parse()?,
        })
    }
}

impl Validator for ValidateBeforeMutateValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let has_assert_transition =
            input.source.contains("assert_transition") || input.source.contains("assertTransition");
        if has_assert_transition {
            return Vec::new();
        }
        for (line_idx, line) in input.source.lines().enumerate() {
            let trimmed = line.trim_start();
            let calls_can_transition = trimmed.starts_with("self.can_transition(")
                || trimmed.starts_with("this.canTransition(");
            if calls_can_transition {
                return vec![finding(
                    &FindingSpec {
                        rule_id: &self.rule_id,
                        severity: Severity::Error,
                        title: "fsm: can_transition result discarded before mutation",
                    },
                    "`can_transition`/`canTransition` is called but its boolean result is not \
                     checked/returned before the state mutation proceeds; use \
                     `assert_transition` to raise `InvalidTransition` on an illegal edge."
                        .to_owned(),
                    &input,
                    (line_idx as u32).saturating_add(1),
                )];
            }
        }
        Vec::new()
    }
}

/// CF-FSM-2.4 — terminal-state no outgoing: a terminal state (`CLOSED`,
/// `CANCELLED`) must map to an empty transitions list; giving it an
/// outgoing edge is a violation.
pub struct TerminalStateNoOutgoingValidator {
    rule_id: RuleId,
}

impl TerminalStateNoOutgoingValidator {
    /// Build the validator, parsing its `RuleId` literal at construction.
    pub fn new() -> Result<Self, enforcer_domain::boundary::decode_error::DecodeError> {
        Ok(Self {
            rule_id: "FSM-TERMINAL.1".parse()?,
        })
    }
}

const TERMINAL_STATE_NAMES: &[&str] = &["CLOSED", "CANCELLED"];

impl Validator for TerminalStateNoOutgoingValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        for (line_idx, line) in input.source.lines().enumerate() {
            let trimmed = line.trim();
            for terminal in TERMINAL_STATE_NAMES {
                let key = format!("\"{terminal}\":");
                if trimmed.starts_with(&key) && !trimmed.contains("[]") {
                    return vec![finding(
                        &FindingSpec {
                            rule_id: &self.rule_id,
                            severity: Severity::Error,
                            title: "fsm: terminal state has an outgoing transition",
                        },
                        format!(
                            "`{terminal}` is a terminal state and must map to an empty \
                             transition list (`[]`); it has a non-empty outgoing edge."
                        ),
                        &input,
                        (line_idx as u32).saturating_add(1),
                    )];
                }
            }
        }
        Vec::new()
    }
}

/// CF-FSM-2.5 — FSM singleton + stateless: an FSM method must be a pure
/// from/to -> decision function; writing per-request instance/`variables`
/// state inside it is a violation.
pub struct FsmSingletonStatelessValidator {
    rule_id: RuleId,
}

impl FsmSingletonStatelessValidator {
    /// Build the validator, parsing its `RuleId` literal at construction.
    pub fn new() -> Result<Self, enforcer_domain::boundary::decode_error::DecodeError> {
        Ok(Self {
            rule_id: "FSM-SINGLETONSTATELESS.1".parse()?,
        })
    }
}

impl Validator for FsmSingletonStatelessValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        for (line_idx, line) in input.source.lines().enumerate() {
            let trimmed = line.trim_start();
            if trimmed.starts_with("self.variables = ") || trimmed.starts_with("this.variables = ")
            {
                return vec![finding(
                    &FindingSpec {
                        rule_id: &self.rule_id,
                        severity: Severity::Error,
                        title: "fsm: per-request instance state written inside an FSM method",
                    },
                    "An FSM method must stay a pure from/to -> decision function; writing \
                     per-request instance state (`self.variables = ...`) inside it violates \
                     the singleton/stateless contract."
                        .to_owned(),
                    &input,
                    (line_idx as u32).saturating_add(1),
                )];
            }
        }
        Vec::new()
    }
}

/// Score threshold above which [`FsmTransitionCoverageValidator`] fires
/// (T2, scored per `enforcer-literal-scan`'s scored model): an FSM
/// definition contributes `1.0`; a companion invalid-transition-raising
/// test subtracts `1.0`. A file scoring `>= FIRE_THRESHOLD` (FSM present,
/// no covering test) is flagged.
const COVERAGE_FIRE_THRESHOLD: f64 = 1.0;

/// py-fastapi-fsm-transition-coverage / FE-TEST-1.6 / TEST-FSM-1.1 /
/// DART-TEST-3.1 — transition-coverage test: an FSM with no test hitting an
/// invalid transition scores over threshold; a test asserting
/// `InvalidTransitionError`/`InvalidTransition` on an illegal edge stays
/// clean.
pub struct FsmTransitionCoverageValidator {
    rule_id: RuleId,
}

impl FsmTransitionCoverageValidator {
    /// Build the validator, parsing its `RuleId` literal at construction.
    pub fn new() -> Result<Self, enforcer_domain::boundary::decode_error::DecodeError> {
        Ok(Self {
            rule_id: "FSM-COVERAGE.1".parse()?,
        })
    }
}

const INVALID_TRANSITION_TEST_MARKERS: &[&str] = &["InvalidTransitionError", "InvalidTransition"];

impl Validator for FsmTransitionCoverageValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let defines_fsm = input.source.contains("transitions = {");
        if !defines_fsm {
            return Vec::new();
        }
        let mut score = 1.0_f64;
        if INVALID_TRANSITION_TEST_MARKERS
            .iter()
            .any(|marker| input.source.contains(marker))
        {
            score -= 1.0;
        }
        if score >= COVERAGE_FIRE_THRESHOLD {
            return vec![finding(
                &FindingSpec {
                    rule_id: &self.rule_id,
                    severity: Severity::Warning,
                    title: "fsm: transitions map defined with no invalid-transition test",
                },
                format!(
                    "This FSM's transitions map has no companion test asserting \
                     `InvalidTransitionError`/`InvalidTransition` on an illegal edge \
                     (score {score:.1} >= threshold {COVERAGE_FIRE_THRESHOLD:.1})."
                ),
                &input,
                first_line_containing(input.source, "transitions = {").unwrap_or(1),
            )];
        }
        Vec::new()
    }
}

/// Build every `FSM` family validator this crate owns (d16).
pub fn validators(
) -> Result<Vec<Box<dyn Validator>>, enforcer_domain::boundary::decode_error::DecodeError> {
    Ok(vec![
        Box::new(MandatoryFsmValidator::new()?),
        Box::new(ExplicitTransitionsMapValidator::new()?),
        Box::new(FsmCanonicalLayoutValidator::new()?),
        Box::new(StatusStringLiteralForbiddenValidator::new()?),
        Box::new(EnumLocationStrEnumOnlyValidator::new()?),
        Box::new(EnumParseNoFallbackValidator::new()?),
        Box::new(ValidateBeforeMutateValidator::new()?),
        Box::new(TerminalStateNoOutgoingValidator::new()?),
        Box::new(FsmSingletonStatelessValidator::new()?),
        Box::new(FsmTransitionCoverageValidator::new()?),
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
    fn ten_validators_registered_with_unique_rule_ids(
    ) -> Result<(), enforcer_domain::boundary::decode_error::DecodeError> {
        let vs = validators()?;
        assert_eq!(vs.len(), 10);
        let mut seen = std::collections::BTreeSet::new();
        for v in &vs {
            assert!(seen.insert(v.rule_id().to_string()));
        }
        assert_eq!(seen.len(), 10);
        Ok(())
    }

    #[test]
    fn fsm_required_mandatory_fsm() -> Result<(), Box<dyn std::error::Error>> {
        let validator = MandatoryFsmValidator::new()?;
        run_fixture_parity(
            &validator,
            &manifest_dir(),
            "tests/fixtures/fsm/mandatory/bad/raw_status_assign.py",
            "tests/fixtures/fsm/mandatory/good/transition_call.py",
        )?;
        Ok(())
    }

    #[test]
    fn fsm_explicit_transitions_map() -> Result<(), Box<dyn std::error::Error>> {
        let validator = ExplicitTransitionsMapValidator::new()?;
        run_fixture_parity(
            &validator,
            &manifest_dir(),
            "tests/fixtures/fsm/explicit-map/bad/ad_hoc_setstatus.dart",
            "tests/fixtures/fsm/explicit-map/good/transitions_map.dart",
        )?;
        Ok(())
    }

    #[test]
    fn fsm_canonical_layout() -> Result<(), Box<dyn std::error::Error>> {
        let validator = FsmCanonicalLayoutValidator::new()?;
        run_fixture_parity(
            &validator,
            &manifest_dir(),
            "tests/fixtures/fsm/layout/bad/models/order.py",
            "tests/fixtures/fsm/layout/good/state_machines/order.py",
        )?;
        Ok(())
    }

    #[test]
    fn fsm_status_literal_forbidden() -> Result<(), Box<dyn std::error::Error>> {
        let validator = StatusStringLiteralForbiddenValidator::new()?;
        run_fixture_parity(
            &validator,
            &manifest_dir(),
            "tests/fixtures/fsm/literal-enum/bad/status_string_literal.py",
            "tests/fixtures/fsm/literal-enum/good/status_enum_member.py",
        )?;
        Ok(())
    }

    #[test]
    fn fsm_enum_location_strenum_only() -> Result<(), Box<dyn std::error::Error>> {
        let validator = EnumLocationStrEnumOnlyValidator::new()?;
        run_fixture_parity(
            &validator,
            &manifest_dir(),
            "tests/fixtures/fsm/enum-location/bad/models/status.py",
            "tests/fixtures/fsm/enum-location/good/enums/status.py",
        )?;
        Ok(())
    }

    #[test]
    fn fsm_enum_parse() -> Result<(), Box<dyn std::error::Error>> {
        let validator = EnumParseNoFallbackValidator::new()?;
        run_fixture_parity(
            &validator,
            &manifest_dir(),
            "tests/fixtures/fsm/enum-parse/bad/orelse_default.dart",
            "tests/fixtures/fsm/enum-parse/good/try_from_nullable.dart",
        )?;
        Ok(())
    }

    #[test]
    fn fsm_validate_before_mutate() -> Result<(), Box<dyn std::error::Error>> {
        let validator = ValidateBeforeMutateValidator::new()?;
        run_fixture_parity(
            &validator,
            &manifest_dir(),
            "tests/fixtures/fsm/validate-before-mutate/bad/can_transition_ignored.py",
            "tests/fixtures/fsm/validate-before-mutate/good/assert_transition_raises.py",
        )?;
        Ok(())
    }

    #[test]
    fn fsm_terminal_state_no_outgoing() -> Result<(), Box<dyn std::error::Error>> {
        let validator = TerminalStateNoOutgoingValidator::new()?;
        run_fixture_parity(
            &validator,
            &manifest_dir(),
            "tests/fixtures/fsm/terminal-state/bad/closed_has_outgoing.py",
            "tests/fixtures/fsm/terminal-state/good/closed_no_outgoing.py",
        )?;
        Ok(())
    }

    #[test]
    fn fsm_singleton_stateless() -> Result<(), Box<dyn std::error::Error>> {
        let validator = FsmSingletonStatelessValidator::new()?;
        run_fixture_parity(
            &validator,
            &manifest_dir(),
            "tests/fixtures/fsm/singleton-stateless/bad/per_request_variables.py",
            "tests/fixtures/fsm/singleton-stateless/good/pure_decision.py",
        )?;
        Ok(())
    }

    #[test]
    fn fsm_coverage() -> Result<(), Box<dyn std::error::Error>> {
        let validator = FsmTransitionCoverageValidator::new()?;
        run_fixture_parity(
            &validator,
            &manifest_dir(),
            "tests/fixtures/fsm/coverage/bad/no_invalid_test/test_order_fsm.py",
            "tests/fixtures/fsm/coverage/good/invalid_covered/test_order_fsm.py",
        )?;
        Ok(())
    }
}
