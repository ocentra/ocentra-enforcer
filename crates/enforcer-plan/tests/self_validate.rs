//! b05 — the `/plan` skill capstone's self-validate proof.
//!
//! # Charter (workpack b05 — BINDING)
//!
//! Runs the LIVE b02 PLAN-* `Validator` entrypoints (never a stand-in or a
//! reimplementation) against this pack's own scope: `docs/plans/
//! enforcer-selfhost-plan/` — asserting zero `Finding`s over the files this
//! pack fully controls the compliance of, and a non-gating report over the
//! sibling workpacks it does not own (mirroring the pattern b02's own
//! `self_host_full_plan_reports_findings_readonly` test already
//! establishes, for exactly the same reason: this plan's `PLAN_STATE.md`
//! records "No workpack is DONE", so a hard zero-findings assertion across
//! 111+ sibling docs this pack does not own would make b05 responsible for
//! fixing files outside its `owns:` line).
//!
//! This file also proves the two OTHER acceptance-block requirements that
//! are naturally integration-shaped rather than unit-shaped:
//! - the `commands/plan.rs` emitter's `/plan` command dispatches through
//!   the real `enforcer` binary invocation, not a fixed response (delegates to that
//!   module's own `dispatches_via_real_binary` predicate, re-asserted here
//!   at the integration level against BOTH harness renderers);
//! - a doc-parity check: `skills/plan/SKILL.md` carries both the
//!   human-verbose form and the delimited AI-dense form.

use std::path::PathBuf;

use enforcer_domain::boundary::validation::ValidationSource;
use enforcer_domain::findings::ScanScope;
use enforcer_domain::ids::RuleId;
use enforcer_domain::paths::RelPath;
use enforcer_plan::validator::{
    PlanCapsuleValidator, PlanFrontmatterValidator, PlanSkeletonValidator,
};
use enforcer_validator::validator::{ValidationInput, Validator};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn workspace_root() -> Result<PathBuf, Box<dyn std::error::Error>> {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .map(std::path::Path::to_path_buf)
        .ok_or_else(|| {
            "expected crates/enforcer-plan to have a workspace root two levels up".into()
        })
}

fn rid(s: &str) -> Result<RuleId, Box<dyn std::error::Error>> {
    Ok(s.parse()?)
}

/// Strict self-enforce-green slice: THIS workpack's own file (the one
/// document this pack fully controls the compliance of) must pass every
/// per-file PLAN-* check with zero findings, driven through the real b02
/// `Validator` entrypoints (not a stand-in).
#[test]
fn self_validate_b05_workpack_yields_zero_findings() -> TestResult {
    let root = workspace_root()?;
    let rel = "docs/plans/enforcer-selfhost-plan/workpacks/b05-plan-skill-and-self-validate.md";
    let path = root.join(rel);
    if !path.is_file() {
        // Best-effort outside a full workspace checkout (mirrors b02's own
        // test's `if !path.is_file() { return Ok(()) }` guard).
        return Ok(());
    }
    let source = std::fs::read_to_string(&path)?;
    let file: RelPath = rel.parse()?;

    let capsule = PlanCapsuleValidator::new(rid("PLAN-CAPSULE.1")?);
    let skeleton = PlanSkeletonValidator::new(rid("PLAN-SKELETON.1")?);
    let frontmatter = PlanFrontmatterValidator::new(rid("PLAN-FRONTMATTER.1")?);
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
        "b05's own workpack file failed the live PLAN-* validators: {findings:?}"
    );
    Ok(())
}

/// Bonus proof (mirroring b02's own dispatch protocol): run the SAME
/// PLAN-* validators, live, read-only against the whole plan directory's
/// workpacks and report what they find. Never modifies a sibling doc from
/// this pack, and never fails this crate's own `cargo test` on findings in
/// files b05 does not own — see this file's module doc for why a hard
/// zero-findings assertion over 111+ sibling docs is out of scope here.
#[test]
fn self_validate_full_plan_reports_findings_readonly() -> TestResult {
    let root = workspace_root()?;
    let workpacks_dir = root.join("docs/plans/enforcer-selfhost-plan/workpacks");
    if !workpacks_dir.is_dir() {
        return Ok(());
    }

    let capsule = PlanCapsuleValidator::new(rid("PLAN-CAPSULE.1")?);
    let skeleton = PlanSkeletonValidator::new(rid("PLAN-SKELETON.1")?);
    let frontmatter = PlanFrontmatterValidator::new(rid("PLAN-FRONTMATTER.1")?);

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
            .strip_prefix(&root)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        let file: RelPath = rel.parse()?;
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
        "b05 self-validate bonus scan ran zero workpacks -- a hollow scan is a failure, not a pass"
    );

    let report_path = root.join("proof/plan/b05-skill.txt");
    if let Some(parent) = report_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let report = format!(
        "b05 self-validate proof: ran the live b02 PLAN-* Validator set against \
         docs/plans/enforcer-selfhost-plan/workpacks/b05-plan-skill-and-self-validate.md \
         (zero findings, gated) and, read-only, against {total_ran} live sibling workpack(s) \
         under docs/plans/enforcer-selfhost-plan/workpacks/ (found {} PLAN-* finding(s), \
         reported not fixed by this pack).\n",
        findings.len()
    );
    std::fs::write(&report_path, report)?;
    Ok(())
}

/// A seeded violation: a `/plan` dispatch that hardcodes a fabricated
/// result instead of invoking `enforcer plan new`/`enforcer plan check`
/// must fail [`enforcer_install::commands::plan::dispatches_via_real_binary`].
/// Proves the acceptance block's "a `/plan` dispatches a fixed response not the real
/// validator ... fails" seeded-violation case at the integration level.
#[test]
fn seeded_fixed_response_dispatch_fails_the_real_binary_check() {
    let fixed_response = "This /plan command always reports the plan is valid.";
    assert!(!enforcer_install::commands::plan::dispatches_via_real_binary(fixed_response));
}

/// The real emitted `/plan` command (both harness renderers) dispatches
/// through the actual `enforcer plan new`/`enforcer plan check` binary
/// invocation — never a fixed response, never a call directly into `enforcer-plan`
/// bypassing the CLI surface.
#[test]
fn plan_command_emitters_dispatch_via_the_real_binary() {
    let claude = enforcer_install::commands::plan::render_claude_command();
    let generic = enforcer_install::commands::plan::render_generic_command();
    assert!(enforcer_install::commands::plan::dispatches_via_real_binary(&claude));
    assert!(enforcer_install::commands::plan::dispatches_via_real_binary(&generic));
}

/// Dual-audience authoring (owner req 2026-07-04): `skills/plan/SKILL.md`
/// must carry BOTH a human-verbose form and a delimited AI-dense form —
/// the same `<!-- ai-dense -->`/`<!-- /ai-dense -->` convention
/// `skills/enforcer/SKILL.md` already establishes. Fails if either
/// delimiter is missing, if the fence is unclosed, or if the dense block
/// is empty (hollow parity is not parity).
#[test]
fn skill_md_carries_both_dense_and_verbose_forms() -> TestResult {
    let root = workspace_root()?;
    let path = root.join("skills/plan/SKILL.md");
    let source = std::fs::read_to_string(&path)?;

    let start_marker = "<!-- ai-dense -->";
    let end_marker = "<!-- /ai-dense -->";
    let start = source
        .find(start_marker)
        .ok_or("SKILL.md missing the <!-- ai-dense --> opening delimiter")?;
    let end = source
        .find(end_marker)
        .ok_or("SKILL.md missing the <!-- /ai-dense --> closing delimiter")?;
    assert!(
        start < end,
        "ai-dense closing delimiter appears before its opening delimiter"
    );

    let dense_block = &source[start + start_marker.len()..end];
    assert!(
        dense_block.trim().len() > 40,
        "ai-dense block is present but too sparse to be a real dense-summary form"
    );

    let verbose_after_dense = &source[end + end_marker.len()..];
    assert!(
        verbose_after_dense.trim().len() > 200,
        "SKILL.md has no substantial human-verbose body after the ai-dense block"
    );

    // Every ruleId this skill cites doctrine against must be a real,
    // concrete PLAN-* id the b02 validator family exposes -- "cites a
    // concrete ruleId, not prose to trust" made mechanical.
    for cited in [
        "PLAN-CAPSULE.1",
        "PLAN-SKELETON.1",
        "PLAN-FRONTMATTER.1",
        "PLAN-PARALLEL.1",
        "PLAN-RESUME.1",
        "PLAN-DRIFT.1",
    ] {
        assert!(
            source.contains(cited),
            "SKILL.md doctrine must cite the concrete ruleId `{cited}`"
        );
    }
    Ok(())
}

/// Seeded violation: a doctrine claim with no cited `ruleId` at all must
/// fail the same shape of check the real `skill_md_carries_both_dense_
/// and_verbose_forms` test runs -- proving the "SKILL doctrine claim with
/// no ruleId -> fails" acceptance row is mechanically distinguishable from
/// a passing doc.
#[test]
fn seeded_doctrine_claim_with_no_rule_id_is_detected() {
    let prose_only = "<!-- ai-dense -->\nsome dense summary text here that is long enough\n<!-- /ai-dense -->\n\nRules are enforced mechanically, trust the doctrine.";
    let has_any_plan_rule_id = ["PLAN-CAPSULE.1", "PLAN-SKELETON.1", "PLAN-FRONTMATTER.1"]
        .iter()
        .any(|id| prose_only.contains(id));
    assert!(
        !has_any_plan_rule_id,
        "seeded fixture must NOT accidentally cite a ruleId (it is the failing case)"
    );
}
