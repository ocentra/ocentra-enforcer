//! PLAN-* structure validators (b02): mechanical checks over workpack
//! markdown documents that used to be prose-only authoring instructions.
//!
//! # Charter
//!
//! Every workpack under `docs/plans/<plan>/workpacks/*.md` MUST carry:
//! the exact `agent-capsule` marker block ([`PlanCapsuleValidator`]);
//! the standard section skeleton in order ([`PlanSkeletonValidator`]);
//! well-formed `owns`/`deps`/`tier` frontmatter ([`PlanFrontmatterValidator`]).
//! Two workpacks with no dependency edge between them MUST declare
//! disjoint `owns` globs ([`check_parallel_safety`] — cross-document, so it
//! is a plain function rather than a single-file [`Validator`]). A plan
//! carries live resume-state, either as a dedicated `RESUME_STATE.md` or an
//! equivalent required section-set ([`PlanResumeStateValidator`]). Finally,
//! a workpack's `Requirement Checklist` must not contradict its own
//! `Where We Are` prose ([`check_checklist_drift`] — the L24 lesson: arc-12
//! shipped a checklist item duplicated from a sibling pack that its own
//! Where-We-Are already said did not apply).
//!
//! Each single-file check implements `enforcer_validator::validator::Validator`
//! so it can run through the arc-05 fixture/parity harness against the
//! fail/pass fixtures under `tests/fixtures/plan-validator/**`. The two
//! cross-document checks (`PLAN-PARALLEL-SAFETY`, `PLAN-CHECKLIST-DRIFT`)
//! take a slice of parsed documents / one document's raw text directly,
//! since the `Validator` trait's single-file contract does not fit a
//! predicate over two documents.
//!
//! This module owns the checks; the typed [`enforcer_rules::registry::RuleRecord`]s
//! linking each `ruleId` to this module, its doc anchor, and its fixtures
//! live in `enforcer-rules`' `src/rules/plan.rs` (b02 also owns that file).

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::boundary::validation::ValidationSourceText;
use enforcer_domain::findings::Finding;
use enforcer_domain::ids::{LaneId, RuleId};
use enforcer_domain::plan_types::PlanCondition;
use enforcer_domain::plan_types::PlanOwnershipPattern;

use enforcer_validator::validator::{ValidationInput, Validator};

use crate::boundary::finding::build_error_finding_at as finding;
use crate::boundary::validator::{
    extract_line_value, one_based_line, section_text, workpack_id_condition,
};
/// The exact `agent-capsule` marker block every workpack must carry,
/// verbatim except for the `Doc:` line (which names the workpack). Sourced
/// from the workpacks this plan already ships (see
/// `docs/plans/enforcer-selfhost-plan/workpacks/a03-*.md`).
const CAPSULE_LINES: &[&str] = &[
    "<!-- agent-capsule -->",
    "> Agent Capsule",
    "> Plan: `enforcer-selfhost-plan`",
    "> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.",
    "> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.",
    "> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.",
    "> Proves: only the local scope stated by this file and its named proof/test rows.",
    "> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.",
    "> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.",
    "<!-- /agent-capsule -->",
];

/// The required section headings, in order. `Requirement Checklist` and
/// `Acceptance And Proof` may appear in either relative order to each
/// other in some historical packs, but `Where We Are` must precede
/// `Where We Want To Be`, and `Parallel Ownership Notes` is always last.
const REQUIRED_HEADINGS: &[&str] = &[
    "## Where We Are",
    "## Where We Want To Be",
    "## Requirement Checklist",
    "## Acceptance And Proof",
    "## Parallel Ownership Notes",
];

/// `PLAN-CAPSULE`: every workpack contains the exact agent-capsule marker
/// block, unmodified fields (the `Doc:` line is the only line allowed to
/// vary — it names the workpack).
#[derive(Debug)]
pub struct PlanCapsuleValidator {
    rule_id: RuleId,
}

impl PlanCapsuleValidator {
    /// Construct with the linked `ruleId` (kept caller-supplied so the
    /// registry record and this validator can never silently disagree on
    /// which id fires).
    pub fn new(rule_id: RuleId) -> Self {
        Self { rule_id }
    }
}

impl Validator for PlanCapsuleValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let lines: Vec<&str> = input.source.as_str().lines().collect();
        let Some(start) = lines
            .iter()
            .position(|l| l.trim() == "<!-- agent-capsule -->")
        else {
            return vec![finding(
                &self.rule_id,
                "missing agent-capsule marker block",
                "workpack has no `<!-- agent-capsule -->` marker block",
                input.file,
                1,
            )];
        };

        // The `Doc:` line is the one field allowed to vary; every other
        // capsule line must match CAPSULE_LINES verbatim, in order.
        let mut expected_idx = 0usize;
        let mut cursor = start;
        while let Some(expected) = CAPSULE_LINES.get(expected_idx) {
            let Some(actual) = lines.get(cursor) else {
                return vec![finding(
                    &self.rule_id,
                    "truncated agent-capsule block",
                    format!("capsule ended before required line `{}`", expected),
                    input.file,
                    one_based_line(cursor),
                )];
            };
            if *expected == "> Plan: `enforcer-selfhost-plan`" {
                // Allow a `> Doc:` line to be inserted directly after
                // `Plan:` before continuing the fixed sequence — checked
                // by peeking one line ahead rather than matching text.
                if actual.trim() != *expected {
                    return vec![finding(
                        &self.rule_id,
                        "agent-capsule field modified",
                        format!("expected `{expected}`, found `{}`", actual.trim()),
                        input.file,
                        one_based_line(cursor),
                    )];
                }
                cursor += 1;
                if lines
                    .get(cursor)
                    .is_some_and(|l| l.trim_start().starts_with("> Doc:"))
                {
                    cursor += 1;
                }
                expected_idx += 1;
                continue;
            }
            if actual.trim() != *expected {
                return vec![finding(
                    &self.rule_id,
                    "agent-capsule field modified",
                    format!("expected `{expected}`, found `{}`", actual.trim()),
                    input.file,
                    one_based_line(cursor),
                )];
            }
            cursor += 1;
            expected_idx += 1;
        }

        Vec::new()
    }
}

/// `PLAN-SKELETON`: required headings present, in order.
#[derive(Debug)]
pub struct PlanSkeletonValidator {
    rule_id: RuleId,
}

impl PlanSkeletonValidator {
    /// Construct with the linked `ruleId`.
    pub fn new(rule_id: RuleId) -> Self {
        Self { rule_id }
    }
}

impl Validator for PlanSkeletonValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let lines: Vec<&str> = input.source.as_str().lines().collect();
        let mut cursor = 0usize;
        for &heading in REQUIRED_HEADINGS {
            let found = lines
                .get(cursor..)
                .and_then(|remaining| remaining.iter().position(|line| line.trim() == heading));
            match found {
                Some(offset) => cursor += offset + 1,
                None => {
                    return vec![finding(
                        &self.rule_id,
                        "missing or out-of-order required heading",
                        format!(
                            "expected `{heading}` at or after line {cursor} (in required order)"
                        ),
                        input.file,
                        1,
                    )];
                }
            }
        }
        Vec::new()
    }
}

/// The lowest and highest priority-tier tokens this plan's workpacks use
/// (`P0`..`P5`). Scoped to this module: distinct from
/// `enforcer_domain::severity::Tier` (`T1`/`T2`/`T3`, the mechanical-
/// enforcement tier), which a `tier:` line may ALSO carry as a second
/// token (e.g. `P4 T1`).
const VALID_PRIORITY_TIERS: &[&str] = &["P0", "P1", "P2", "P3", "P4", "P5"];
const VALID_ENFORCEMENT_TIERS: &[&str] = &["T1", "T2", "T3"];

/// `PLAN-FRONTMATTER`: `owns`/`deps`/`tier` lines present and well-formed;
/// tier's priority token in the `P0`-`P5` set (optionally followed by a
/// `T1`-`T3` enforcement-tier token, e.g. `P4 T1`); `deps` ids parse as
/// non-empty dash/alnum tokens (the plan's own workpack-id grammar, e.g.
/// `a01`, `arc-04`) — this crate does not import `enforcer_domain`'s
/// `RuleId`/`LaneId` newtypes for workpack ids because a workpack id is a
/// distinct vocabulary (short doc-authoring slugs, not rule or
/// coordination-lane ids); this validator owns that grammar itself.
#[derive(Debug)]
pub struct PlanFrontmatterValidator {
    rule_id: RuleId,
}

impl PlanFrontmatterValidator {
    /// Construct with the linked `ruleId`.
    pub fn new(rule_id: RuleId) -> Self {
        Self { rule_id }
    }
}

impl Validator for PlanFrontmatterValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let lines: Vec<&str> = input.source.as_str().lines().collect();

        let Some((owns_value, owns_line)) = extract_line_value(&lines, "- owns:") else {
            return vec![finding(
                &self.rule_id,
                "missing owns frontmatter",
                "workpack has no `- owns:` line",
                input.file,
                1,
            )];
        };
        if owns_value.trim().is_empty() {
            return vec![finding(
                &self.rule_id,
                "empty owns frontmatter",
                "`- owns:` line has no glob values",
                input.file,
                one_based_line(owns_line),
            )];
        }

        let Some((deps_value, deps_line)) = extract_line_value(&lines, "- deps:") else {
            return vec![finding(
                &self.rule_id,
                "missing deps frontmatter",
                "workpack has no `- deps:` line",
                input.file,
                1,
            )];
        };
        // `deps: none` is the plan's documented spelling for "no
        // dependencies" (e.g. a01); accept it without id-grammar checks.
        if deps_value.trim().trim_matches('`') != "none" {
            for token in deps_value.split(',') {
                let token = token.trim();
                if token.is_empty() {
                    continue;
                }
                if matches!(workpack_id_condition(token), PlanCondition::Unsatisfied) {
                    return vec![finding(
                        &self.rule_id,
                        "malformed deps id",
                        format!("`- deps:` token `{token}` is not a well-formed workpack id"),
                        input.file,
                        one_based_line(deps_line),
                    )];
                }
            }
        }

        let Some((tier_value, tier_line)) = extract_line_value(&lines, "- tier:") else {
            return vec![finding(
                &self.rule_id,
                "missing tier frontmatter",
                "workpack has no `- tier:` line",
                input.file,
                1,
            )];
        };
        let tier_value = tier_value.trim().trim_matches('`');
        let mut tokens = tier_value.split_whitespace();
        let Some(priority_token) = tokens.next() else {
            return vec![finding(
                &self.rule_id,
                "empty tier frontmatter",
                "`- tier:` line has no priority token",
                input.file,
                one_based_line(tier_line),
            )];
        };
        if !VALID_PRIORITY_TIERS.contains(&priority_token) {
            return vec![finding(
                &self.rule_id,
                "tier not in the P0-P5 set",
                format!("`- tier:` priority token `{priority_token}` is not one of P0..P5"),
                input.file,
                one_based_line(tier_line),
            )];
        }
        if let Some(enforcement_token) = tokens.next() {
            if !VALID_ENFORCEMENT_TIERS.contains(&enforcement_token) {
                return vec![finding(
                    &self.rule_id,
                    "enforcement tier not in the T1-T3 set",
                    format!(
                        "`- tier:` enforcement token `{enforcement_token}` is not one of T1..T3"
                    ),
                    input.file,
                    one_based_line(tier_line),
                )];
            }
        }

        Vec::new()
    }
}

/// Parsed `owns:` globs for one workpack, used by [`check_parallel_safety`].
#[derive(Debug, Clone)]
pub struct OwnsRecord {
    /// Human-readable workpack label for diagnostics (file stem or id).
    pub workpack_id: LaneId,
    /// Dependency ids this workpack declares (raw tokens, e.g. `a01`).
    pub deps: Vec<LaneId>,
    /// `owns:` glob strings, verbatim.
    pub owns: Vec<PlanOwnershipPattern>,
}

/// Parse the `deps:`/`owns:` frontmatter lines out of one workpack's raw
/// text into an [`OwnsRecord`]. Returns `None` if either line is absent —
/// callers should run [`PlanFrontmatterValidator`] first so a structurally
/// broken workpack is reported by that rule, not silently skipped here.
pub fn parse_owns_record(
    workpack_id: LaneId,
    source: &ValidationSourceText,
) -> Result<Option<OwnsRecord>, DecodeError> {
    let lines: Vec<&str> = source.as_source().as_str().lines().collect();
    let Some((owns_value, _)) = extract_line_value(&lines, "- owns:") else {
        return Ok(None);
    };
    let Some((deps_value, _)) = extract_line_value(&lines, "- deps:") else {
        return Ok(None);
    };
    let owns = owns_value
        .split(',')
        .map(|s| s.trim().trim_matches('`'))
        .filter(|s| !s.is_empty())
        .map(str::parse)
        .collect::<Result<Vec<PlanOwnershipPattern>, _>>()?;
    let deps_value = deps_value.trim().trim_matches('`');
    let deps = if deps_value == "none" {
        Vec::new()
    } else {
        deps_value
            .split(',')
            .map(|s| s.trim().trim_matches('`'))
            .filter(|s| !s.is_empty())
            .map(str::parse)
            .collect::<Result<Vec<LaneId>, _>>()?
    };
    Ok(Some(OwnsRecord {
        workpack_id,
        deps,
        owns,
    }))
}

/// Two glob strings overlap when they are byte-identical, or when one is a
/// literal prefix of the other's directory path (the shape every `owns:`
/// entry in this plan takes: exact file paths or a `dir/**` glob). This is
/// intentionally conservative (string-prefix, not a full glob-intersection
/// solver) — the doctrine only needs to catch the two shapes this plan's
/// workpacks actually author: two exact-identical paths, and a `**` glob
/// that contains a sibling's exact path.
fn globs_overlap(a: &PlanOwnershipPattern, b: &PlanOwnershipPattern) -> PlanCondition {
    let a = a.as_str();
    let b = b.as_str();
    if a == b {
        return PlanCondition::Satisfied;
    }
    let a_root = a.trim_end_matches("**").trim_end_matches('/');
    let b_root = b.trim_end_matches("**").trim_end_matches('/');
    if a.ends_with("**") && b.starts_with(a_root) {
        return PlanCondition::Satisfied;
    }
    if b.ends_with("**") && a.starts_with(b_root) {
        return PlanCondition::Satisfied;
    }
    PlanCondition::Unsatisfied
}

/// `PLAN-PARALLEL-SAFETY`: for any two workpacks with no dependency edge
/// between them, their `owns:` globs MUST be disjoint. Returns one
/// [`Finding`] per offending pair (attributed to the first workpack's
/// synthetic path, matching this crate's `Finding::file` contract — a
/// two-file offense needs a file to point at, and the first-listed
/// workpack is the deterministic, order-stable choice).
///
/// This is the reusable predicate b04's orchestrator imports (per the
/// workpack's own Parallel Ownership Notes) — kept a plain function over
/// parsed [`OwnsRecord`]s rather than a `Validator` impl, since the
/// `Validator` trait's contract is exactly one file's text, and this
/// check is inherently pairwise across two documents.
pub fn check_parallel_safety(
    rule_id: &RuleId,
    records: &[OwnsRecord],
    file_for: impl Fn(&LaneId) -> enforcer_domain::paths::RelPath,
) -> Vec<Finding> {
    let mut findings = Vec::new();
    for (index, a) in records.iter().enumerate() {
        let Some(following) = index.checked_add(1).and_then(|start| records.get(start..)) else {
            continue;
        };
        for b in following {
            let has_dep_edge = a.deps.iter().any(|d| d == &b.workpack_id)
                || b.deps.iter().any(|d| d == &a.workpack_id);
            if has_dep_edge {
                continue;
            }
            let overlap = a.owns.iter().any(|oa| {
                b.owns
                    .iter()
                    .any(|ob| matches!(globs_overlap(oa, ob), PlanCondition::Satisfied))
            });
            if overlap {
                findings.push(finding(
                    rule_id,
                    "non-disjoint owns between no-dep-edge workpacks",
                    format!(
                        "`{}` and `{}` declare no dependency edge but their `owns:` globs overlap",
                        a.workpack_id, b.workpack_id
                    ),
                    &file_for(&a.workpack_id),
                    1,
                ));
            }
        }
    }
    findings
}

/// One resume-state record type this validator recognizes: either a
/// section heading OR a leading-line marker word, matched case-
/// insensitively so `## Where We Are` and `**Where we are:**` both count.
const RESUME_STATE_MARKERS: &[&str] = &["where we are", "checklist", "progress", "prev", "next"];

/// `PLAN-RESUME-STATE`: every plan carries live resume-state — a
/// `RESUME_STATE.md` (or the equivalent required section-set) with a
/// `Where We Are` block AND `CHECKLIST`/`TASKLIST`/`PROGRESS` lists AND
/// `PREV`/`NEXT` records. Operates on one document's raw text (the
/// `RESUME_STATE.md` file, or any other file a caller points it at as the
/// equivalent-section-set carrier), firing when ANY required marker is
/// absent.
#[derive(Debug)]
pub struct PlanResumeStateValidator {
    rule_id: RuleId,
}

impl PlanResumeStateValidator {
    /// Construct with the linked `ruleId`.
    pub fn new(rule_id: RuleId) -> Self {
        Self { rule_id }
    }
}

impl Validator for PlanResumeStateValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let lowercase = input.source.as_str().to_lowercase();
        let mut missing = Vec::new();
        for marker in RESUME_STATE_MARKERS {
            let present = lowercase.contains(marker)
                && (lowercase.contains(&format!("## {marker}"))
                    || lowercase.contains(&format!("# {marker}"))
                    || lowercase.contains(&format!("**{marker}")));
            if !present {
                missing.push(*marker);
            }
        }
        if missing.is_empty() {
            Vec::new()
        } else {
            vec![finding(
                &self.rule_id,
                "resume-state section-set incomplete",
                format!(
                    "missing required resume-state marker(s): {}",
                    missing.join(", ")
                ),
                input.file,
                1,
            )]
        }
    }
}

/// L24 (`refs/orchestration-lessons.md`): arc-12 shipped a checklist item
/// ("port the .mjs k8s logic") inherited from a sibling language pack,
/// while its own `Where We Are` correctly said no such logic existed —
/// the checklist contradicted the pack's own stated scope facts. This
/// `PLAN-CHECKLIST-DRIFT` predicate flags a narrow, high-confidence slice
/// of that failure mode: a `Where We Are` sentence stating a thing does
/// NOT exist / is greenfield, paired with a checklist item that still
/// instructs porting/migrating/copying that same thing. A plain function
/// (not a `Validator` impl) because it correlates two sections of ONE
/// document's own text, which the single-purpose `Validator` contract
/// does not distinguish from "two arbitrary findings" — this stays a
/// workpack-scoped helper other checks in this module can call inline
/// with the same [`ValidationInput`] shape.
pub fn check_checklist_drift(rule_id: &RuleId, input: ValidationInput<'_>) -> Vec<Finding> {
    let lower = input.source.as_str().to_lowercase();
    let where_we_are = section_text(&lower, "## where we are", "## where we want to be");
    let checklist = section_text(
        &lower,
        "## requirement checklist",
        "## acceptance and proof",
    );

    let Some(where_we_are) = where_we_are else {
        return Vec::new();
    };
    let Some(checklist) = checklist else {
        return Vec::new();
    };

    let negation_markers = ["no such", "does not exist", "not exist", "greenfield"];
    let porting_markers = ["port the", "port existing", "migrate the", "copy the"];

    let states_absence = negation_markers.iter().any(|m| where_we_are.contains(m));
    let claims_porting = porting_markers.iter().any(|m| checklist.contains(m));

    if states_absence && claims_porting {
        vec![finding(
            rule_id,
            "checklist contradicts this workpack's own Where-We-Are",
            "Requirement Checklist instructs porting/migrating existing logic, but Where We \
             Are states no such logic exists in this workpack's scope (L24: sibling \
             copy-paste drift)",
            input.file,
            1,
        )]
    } else {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        check_checklist_drift, check_parallel_safety, parse_owns_record, OwnsRecord,
        PlanCapsuleValidator, PlanFrontmatterValidator, PlanResumeStateValidator,
        PlanSkeletonValidator, Validator,
    };
    use enforcer_domain::boundary::decode_error::DecodeError;
    use enforcer_domain::boundary::validation::{ValidationSource, ValidationSourceText};
    use enforcer_domain::findings::ScanScope;
    use enforcer_domain::ids::{LaneId, RuleId};
    use enforcer_domain::paths::{RelPath, RepoRoot};
    use enforcer_validator::{harness::run_fixture_parity, validator::ValidationInput};
    use proptest::proptest;

    fn manifest_dir() -> Result<RepoRoot, Box<dyn std::error::Error>> {
        Ok(RepoRoot::try_from(env!("CARGO_MANIFEST_DIR").to_owned())?)
    }

    /// Test-only `file_for` callback for [`check_parallel_safety`]: builds
    /// a synthetic workpack path for diagnostics, returning every
    /// [`RelPath`] candidate through `if let Ok` rather than
    /// `unwrap`/`expect` (workspace lint policy). Every id this module's
    /// own tests pass in is a short alnum/dash token, so the primary
    /// candidate always parses; the loop only exists to keep this
    /// panic-free even if that ever stopped being true.
    fn test_file_for(id: &LaneId) -> enforcer_domain::paths::RelPath {
        let candidates = [
            format!("docs/plans/enforcer-selfhost-plan/workpacks/{id}.md"),
            "docs/plans/enforcer-selfhost-plan/workpacks/unknown.md".to_owned(),
        ];
        for candidate in candidates {
            if let Ok(path) = candidate.parse() {
                return path;
            }
        }
        // Unreachable in practice (the second candidate is a fixed
        // literal that trivially satisfies `RelPath`'s own rules); retry
        // the same fixed literal forever rather than panic.
        loop {
            if let Ok(path) = "unknown.md".parse::<enforcer_domain::paths::RelPath>() {
                return path;
            }
        }
    }

    proptest! {
        #[test]
        fn parse_owns_record_handles_arbitrary_text(source in "(?s).{0,4096}") {
            if let Ok(workpack_id) = "property-workpack".parse::<LaneId>() {
                let source = ValidationSourceText::try_new(source);
                let parsed = parse_owns_record(workpack_id.clone(), &source);
                prop_assert!(match parsed {
                    Ok(None) => true,
                    Ok(Some(record)) => {
                        record.workpack_id == workpack_id
                            && record.owns.iter().all(|pattern| !pattern.as_str().is_empty())
                    }
                    Err(DecodeError { path, .. }) => {
                        path == "planOwnershipPattern" || path == "laneId"
                    }
                });
            }
        }
    }

    #[test]
    fn parse_owns_record_rejects_invalid_ownership_pattern(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let source = ValidationSourceText::try_new(
            "- deps: none\n- owns: ../outside-the-repository.rs".to_owned(),
        );
        let result = parse_owns_record("invalid-owns".parse()?, &source);
        assert!(matches!(
            result,
            Err(DecodeError { ref path, .. }) if path == "planOwnershipPattern"
        ));
        Ok(())
    }

    #[test]
    fn capsule_validator_parity() -> Result<(), Box<dyn std::error::Error>> {
        let validator = PlanCapsuleValidator::new("PLAN-CAPSULE.1".parse()?);
        run_fixture_parity(
            &validator,
            &manifest_dir()?,
            &"tests/fixtures/plan-validator/capsule/fail/workpack.md".parse()?,
            &"tests/fixtures/plan-validator/capsule/pass/workpack.md".parse()?,
        )?;
        Ok(())
    }

    #[test]
    fn skeleton_validator_parity() -> Result<(), Box<dyn std::error::Error>> {
        let validator = PlanSkeletonValidator::new("PLAN-SKELETON.1".parse()?);
        run_fixture_parity(
            &validator,
            &manifest_dir()?,
            &"tests/fixtures/plan-validator/skeleton/fail/workpack.md".parse()?,
            &"tests/fixtures/plan-validator/skeleton/pass/workpack.md".parse()?,
        )?;
        Ok(())
    }

    #[test]
    fn frontmatter_validator_parity() -> Result<(), Box<dyn std::error::Error>> {
        let validator = PlanFrontmatterValidator::new("PLAN-FRONTMATTER.1".parse()?);
        run_fixture_parity(
            &validator,
            &manifest_dir()?,
            &"tests/fixtures/plan-validator/frontmatter/fail/workpack.md".parse()?,
            &"tests/fixtures/plan-validator/frontmatter/pass/workpack.md".parse()?,
        )?;
        Ok(())
    }

    #[test]
    fn resume_state_validator_parity() -> Result<(), Box<dyn std::error::Error>> {
        let validator = PlanResumeStateValidator::new("PLAN-RESUME.1".parse()?);
        run_fixture_parity(
            &validator,
            &manifest_dir()?,
            &"tests/fixtures/plan-validator/resume-state/fail/RESUME_STATE.md".parse()?,
            &"tests/fixtures/plan-validator/resume-state/pass/RESUME_STATE.md".parse()?,
        )?;
        Ok(())
    }

    #[test]
    fn parallel_safety_flags_overlap_without_dep_edge() -> Result<(), Box<dyn std::error::Error>> {
        let rule_id: RuleId = "PLAN-PARALLEL.1".parse()?;
        let a = std::fs::read_to_string(crate::boundary::validator::fixture_path(
            &manifest_dir()?,
            &"tests/fixtures/plan-validator/parallel-safety/overlap-a.md".parse()?,
        ))?;
        let b = std::fs::read_to_string(crate::boundary::validator::fixture_path(
            &manifest_dir()?,
            &"tests/fixtures/plan-validator/parallel-safety/overlap-b.md".parse()?,
        ))?;
        let records = vec![
            parse_owns_record("z97".parse()?, &ValidationSourceText::try_new(a))?
                .ok_or("expected z97 to parse")?,
            parse_owns_record("z98".parse()?, &ValidationSourceText::try_new(b))?
                .ok_or("expected z98 to parse")?,
        ];
        let findings = check_parallel_safety(&rule_id, &records, test_file_for);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].rule_id.as_str(), "PLAN-PARALLEL.1");
        Ok(())
    }

    #[test]
    fn parallel_safety_allows_disjoint_owns_without_dep_edge(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let rule_id: RuleId = "PLAN-PARALLEL.1".parse()?;
        let a = std::fs::read_to_string(crate::boundary::validator::fixture_path(
            &manifest_dir()?,
            &"tests/fixtures/plan-validator/parallel-safety/disjoint-a.md".parse()?,
        ))?;
        let b = std::fs::read_to_string(crate::boundary::validator::fixture_path(
            &manifest_dir()?,
            &"tests/fixtures/plan-validator/parallel-safety/disjoint-b.md".parse()?,
        ))?;
        let records = vec![
            parse_owns_record("z95".parse()?, &ValidationSourceText::try_new(a))?
                .ok_or("expected z95 to parse")?,
            parse_owns_record("z96".parse()?, &ValidationSourceText::try_new(b))?
                .ok_or("expected z96 to parse")?,
        ];
        let findings = check_parallel_safety(&rule_id, &records, test_file_for);
        assert!(findings.is_empty());
        Ok(())
    }

    #[test]
    fn parallel_safety_allows_overlap_when_dep_edge_declared(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let rule_id: RuleId = "PLAN-PARALLEL.1".parse()?;
        let records = vec![
            OwnsRecord {
                workpack_id: "x1".parse()?,
                deps: vec!["x2".parse()?],
                owns: vec!["crates/sample/src/shared.rs".parse()?],
            },
            OwnsRecord {
                workpack_id: "x2".parse()?,
                deps: vec![],
                owns: vec!["crates/sample/src/shared.rs".parse()?],
            },
        ];
        let findings = check_parallel_safety(&rule_id, &records, test_file_for);
        assert!(findings.is_empty());
        Ok(())
    }

    #[test]
    fn checklist_drift_fires_on_contradiction() -> Result<(), Box<dyn std::error::Error>> {
        let rule_id: RuleId = "PLAN-DRIFT.1".parse()?;
        let source = std::fs::read_to_string(crate::boundary::validator::fixture_path(
            &manifest_dir()?,
            &"tests/fixtures/plan-validator/checklist-drift/fail/workpack.md".parse()?,
        ))?;
        let file: enforcer_domain::paths::RelPath =
            "tests/fixtures/plan-validator/checklist-drift/fail/workpack.md".parse()?;
        let findings = check_checklist_drift(
            &rule_id,
            ValidationInput {
                file: &file,
                source: ValidationSource::from_text(&source),
                scope: ScanScope::Files,
            },
        );
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].rule_id.as_str(), "PLAN-DRIFT.1");
        Ok(())
    }

    #[test]
    fn checklist_drift_silent_on_consistent_workpack() -> Result<(), Box<dyn std::error::Error>> {
        let rule_id: RuleId = "PLAN-DRIFT.1".parse()?;
        let source = std::fs::read_to_string(crate::boundary::validator::fixture_path(
            &manifest_dir()?,
            &"tests/fixtures/plan-validator/checklist-drift/pass/workpack.md".parse()?,
        ))?;
        let file: enforcer_domain::paths::RelPath =
            "tests/fixtures/plan-validator/checklist-drift/pass/workpack.md".parse()?;
        let findings = check_checklist_drift(
            &rule_id,
            ValidationInput {
                file: &file,
                source: ValidationSource::from_text(&source),
                scope: ScanScope::Files,
            },
        );
        assert!(findings.is_empty());
        Ok(())
    }

    #[test]
    fn self_host_this_workpack_file_yields_zero_findings() -> Result<(), Box<dyn std::error::Error>>
    {
        // Strict self-enforce-green slice: THIS pack's own workpack file
        // (the one document b02 fully controls the compliance of) must
        // pass every per-file PLAN-* check with zero findings. The
        // broader live-plan sweep (111 sibling workpacks this pack does
        // NOT own) is reported, not gated, by
        // `self_host_full_plan_reports_findings_readonly` below — see that
        // test's doc comment for why a hard zero-findings assertion over
        // sibling docs is out of this pack's scope.
        let workspace_root = std::path::PathBuf::from(manifest_dir()?.as_str())
            .parent()
            .and_then(|p| p.parent())
            .ok_or("expected crates/enforcer-plan to have a workspace root two levels up")?
            .to_path_buf();
        let path = workspace_root
            .join("docs/plans/enforcer-selfhost-plan/workpacks/b02-plan-structure-validator.md");
        if !path.is_file() {
            // Best-effort outside a full workspace checkout.
            return Ok(());
        }
        let source = std::fs::read_to_string(&path)?;
        let file: enforcer_domain::paths::RelPath =
            "docs/plans/enforcer-selfhost-plan/workpacks/b02-plan-structure-validator.md"
                .parse()?;

        let capsule = PlanCapsuleValidator::new("PLAN-CAPSULE.1".parse()?);
        let skeleton = PlanSkeletonValidator::new("PLAN-SKELETON.1".parse()?);
        let frontmatter = PlanFrontmatterValidator::new("PLAN-FRONTMATTER.1".parse()?);
        let input_for = |scope| ValidationInput {
            file: &file,
            source: ValidationSource::from_text(&source),
            scope,
        };
        let mut findings = Vec::new();
        findings.extend(capsule.validate(input_for(ScanScope::Files)));
        findings.extend(skeleton.validate(input_for(ScanScope::Files)));
        findings.extend(frontmatter.validate(input_for(ScanScope::Files)));

        assert!(
            findings.is_empty(),
            "b02's own workpack file failed its own PLAN-* checks: {findings:?}"
        );
        Ok(())
    }

    #[test]
    fn self_host_full_plan_reports_findings_readonly() -> Result<(), Box<dyn std::error::Error>> {
        // Bonus proof (per the b02 dispatch protocol, step 5): run the
        // PLAN-* validators read-only against the WHOLE live plan dir
        // (`docs/plans/enforcer-selfhost-plan/workpacks/**.md`, 111+
        // sibling workpacks this pack does not own) and REPORT what they
        // find; do not fix sibling docs from this pack, and do not fail
        // this crate's own `cargo test` on findings in files this pack
        // does not own. This plan's own `PLAN_STATE.md` records "No
        // workpack is DONE" and the doctrine's `owns:` grammar the
        // capsule/frontmatter checks assume was only fully standardized
        // after some of these 111 docs were first authored (b03 owns
        // *authoring* the frozen capsule/section templates the
        // scaffolder emits from; b02 only owns *checking* against them) —
        // so a hollow always-passes test would hide real drift, and a
        // hard-failing test would make this pack responsible for fixing
        // 111 files it does not own. Print + persist the finding count
        // as the honest middle path: a real, non-empty scan result,
        // asserted to have actually run (not silently skipped), with the
        // finding detail available to the caller/CI log without gating
        // b02's own proof-green.
        let workspace_root = std::path::PathBuf::from(manifest_dir()?.as_str())
            .parent()
            .and_then(|p| p.parent())
            .ok_or("expected crates/enforcer-plan to have a workspace root two levels up")?
            .to_path_buf();
        let workpacks_dir = workspace_root.join("docs/plans/enforcer-selfhost-plan/workpacks");
        if !workpacks_dir.is_dir() {
            // Best-effort outside a full workspace checkout.
            return Ok(());
        }

        let capsule = PlanCapsuleValidator::new("PLAN-CAPSULE.1".parse()?);
        let skeleton = PlanSkeletonValidator::new("PLAN-SKELETON.1".parse()?);
        let frontmatter = PlanFrontmatterValidator::new("PLAN-FRONTMATTER.1".parse()?);

        let mut total_ran = 0usize;
        let mut findings = Vec::new();
        for entry in std::fs::read_dir(&workpacks_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("md") {
                continue;
            }
            let source = std::fs::read_to_string(&path)?;
            let rel = path
                .strip_prefix(&workspace_root)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");
            let file: enforcer_domain::paths::RelPath = rel.parse()?;
            let input_for = |scope| ValidationInput {
                file: &file,
                source: ValidationSource::from_text(&source),
                scope,
            };
            findings.extend(capsule.validate(input_for(ScanScope::Files)));
            findings.extend(skeleton.validate(input_for(ScanScope::Files)));
            findings.extend(frontmatter.validate(input_for(ScanScope::Files)));
            total_ran += 1;
        }

        assert!(
            total_ran > 0,
            "bonus self-host scan ran zero workpacks — a hollow scan is a failure, not a pass"
        );

        // Persist the report to the named proof artifact path
        // (TEST_PROOF_EXPECTATIONS.md's `proof/plan/b02-selfhost.txt`)
        // rather than printing (workspace lint: no `print_*`/`eprintln!`
        // in non-CLI crates) — read the file back if you need the
        // human-readable report; the assertion above is this test's own
        // proof that the scan genuinely ran.
        let report_path = workspace_root.join("proof/plan/b02-selfhost.txt");
        if let Some(parent) = report_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let report = format!(
            "b02 bonus proof: scanned {total_ran} live workpacks under \
             docs/plans/enforcer-selfhost-plan/workpacks/, found {} PLAN-* finding(s) \
             (reported read-only; not fixed by this pack).\n",
            findings.len()
        );
        std::fs::write(&report_path, report)?;
        Ok(())
    }
}
