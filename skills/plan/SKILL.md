---
name: plan
description: Author, scaffold, and self-validate Ocentra-methodology workpack plans. Use when creating a new docs/plans/<name>/ directory, authoring or auditing workpack documents, running the PLAN-* structure validators against a plan, or wiring the /plan harness command.
---

# Plan

<!-- ai-dense -->
```yaml
workflow: "enforcer plan new <name> (b01 scaffold) -> author workpacks -> enforcer plan check (b02 validate) -> orchestrate (b04)"
doctrine: "rules are conditions, enforcement is mechanical (T1 hard-block / T2 warn-surface / T3 review-assist), no prose-without-check"
scaffolder: "crates/enforcer-plan/src/scaffolder.rs::scaffold_plan -- deterministic, refuses overwrite without force, checklist DERIVED from caller ScopeFacts (L24: never sibling copy-paste)"
validators:
  PLAN-CAPSULE.1: "crates/enforcer-plan/src/validator.rs::PlanCapsuleValidator -- exact agent-capsule marker block, only Doc: line varies"
  PLAN-SKELETON.1: "crates/enforcer-plan/src/validator.rs::PlanSkeletonValidator -- required section headings present, in order"
  PLAN-FRONTMATTER.1: "crates/enforcer-plan/src/validator.rs::PlanFrontmatterValidator -- owns/deps/tier lines well-formed, tier in P0..P5 [+ T1..T3]"
  PLAN-PARALLEL.1: "crates/enforcer-plan/src/validator.rs::check_parallel_safety -- no-dep-edge workpacks must have disjoint owns globs"
  PLAN-RESUME.1: "crates/enforcer-plan/src/validator.rs::PlanResumeStateValidator -- Where We Are + CHECKLIST/TASKLIST/PROGRESS + PREV/NEXT present"
  PLAN-DRIFT.1: "crates/enforcer-plan/src/validator.rs::check_checklist_drift -- checklist must not contradict this doc's own Where We Are (L24)"
templates: "crates/enforcer-plan/templates/*.tpl via crates/enforcer-plan/src/templates.rs -- frozen, include_str!, {{placeholder}} substitution, no inline capsule literal outside this dir"
self_validate: "crates/enforcer-plan/tests/self_validate.rs -- runs the live PLAN-* Validator set against docs/plans/enforcer-selfhost-plan/ (this plan's OWN capsule/skeleton/frontmatter files); zero Finding assertion scoped to the files this pack owns, non-gating report over sibling docs it does not own"
dispatch: "/plan command (crates/enforcer-install/src/commands/plan.rs) emits a harness command whose body invokes `enforcer plan new`/`enforcer plan check` via the real enforcer binary -- never a hand-written per-harness hook, never a stub that fakes success"
cli_status: "enforcer plan is a RESERVED clap subcommand (crates/enforcer-cli/src/main.rs Command::Plan) not yet wired to enforcer-plan's scaffolder/validator -- a known, separate gap outside this skill's owns-set; the emitted command still targets the real binary invocation, not a workaround"
banned: "hardcoded checklist boilerplate copy-pasted from a sibling workpack (L24); inline capsule literal outside templates/; a /plan dispatch that short-circuits to a fixed/fake result"
```
<!-- /ai-dense -->

Use this skill to create a new plan directory, author or audit workpacks against
this repository's Ocentra-methodology contract, and self-validate a plan
mechanically instead of trusting prose review.

The methodology used to live as tribal knowledge and scattered prose: the
capsule contract, the scaffold/validate loop, and the disjoint-ownership rule
for parallel work were things people remembered rather than things a machine
checked. This skill closes that gap by wiring the mechanical backing
(`enforcer-plan`'s scaffolder and PLAN-* validators) into one documented
workflow and a self-validating `cargo test`.

## Workflow

1. **Scaffold** a new plan directory with `enforcer plan new <name>` (routes
   to [`enforcer_plan::scaffolder::scaffold_plan`]) — writes `PLAN_STATE.md`,
   `PLAN_EXECUTION_BLUEPRINT.md`, `TEST_PROOF_EXPECTATIONS.md`,
   `WORKPACK_INDEX.md`, `RESUME_STATE.md`, and a capsule-stamped workpack
   stub under `docs/plans/<name>/`. Refuses to overwrite an existing plan
   directory unless `--force` is passed. The Requirement Checklist is
   derived from the caller-supplied scope facts — never copied from a
   sibling plan (see the scaffolder module doc's "L24" section).
2. **Author** each workpack from its own scope facts: `owns`/`deps`/`tier`
   frontmatter, the standard Where-We-Are / Where-We-Want-To-Be /
   Requirement-Checklist / Acceptance-And-Proof / Parallel-Ownership-Notes
   section skeleton, and the exact `agent-capsule` marker block.
3. **Validate** with `enforcer plan check` (routes to the PLAN-* `Validator`
   family in `enforcer-plan`'s `validator` module) before treating any
   workpack as ready to assign. A workpack failing `PLAN-CAPSULE.1`,
   `PLAN-SKELETON.1`, or `PLAN-FRONTMATTER.1` is not ready. Two workpacks
   declaring no dependency edge between them must have disjoint `owns`
   globs (`PLAN-PARALLEL.1`) — this is what makes them safe to run
   concurrently.
4. **Orchestrate** (b04, `enforcer_plan::orchestrator`) maps validated plan
   structure onto coordination-lane claims for parallel execution — read the
   validated `owns`/`deps` graph, never a second, hand-maintained copy of it.
5. **Self-validate**: before calling a plan doc "DONE", run the same PLAN-*
   `Validator` set this skill documents against the doc itself. A plan or
   workpack is not compliant because it reads well — it is compliant because
   the validator returns zero `Finding`s.

## Doctrine

Rules are conditions a machine can evaluate, not review guidance a human is
trusted to remember. Enforcement follows the mechanical-enforcement ladder:

- **T1** (hard `Violation`, `Severity::Error`): blocks — a workpack missing
  its capsule block or required sections is not assignable.
- **T2** (`Severity::Warning`): surfaces but does not block — e.g. a
  resume-state marker present but sparse.
- **T3** (review-assist): no mechanical signal exists yet; a human judgment
  call, not silently promoted to a hard gate.

No prose-without-check: every doctrine claim this skill makes cites a
concrete `ruleId` and its `Validator` entrypoint (see the `validators:` map
above) — a claim with no `ruleId` is exactly the gap `PLAN-CAPSULE.1` /
`PLAN-SKELETON.1` / `PLAN-FRONTMATTER.1` exist to catch mechanically instead
of trusting a reviewer to notice.

## Dual-Audience Authoring

Every doc this skill authors or generates — including this `SKILL.md` — ships
both forms: the `<!-- ai-dense -->...<!-- /ai-dense -->` block above (a
super-high-information-density YAML summary an agent parses without reading
prose) and this human-verbose body (the same facts, explained). The dense
form is authored to map onto a future typed record once the `.md` authoring
surface is retired in favor of a typed system/db/schema — it is TRANSITIONAL,
not the permanent home of this knowledge. A doc-parity check
(`crates/enforcer-plan/tests/self_validate.rs`) asserts both forms are
present on every doc this skill's scaffolder emits and on this file itself.

## CLI

```bash
enforcer plan new <name>
enforcer plan new <name> --force
enforcer plan check
```

`enforcer plan` is reserved in the CLI grammar
(`crates/enforcer-cli/src/cli.rs::Command::Plan`) but its dispatch to
`enforcer-plan`'s scaffolder/validator is a separate, not-yet-wired gap
outside this skill's own owns-set (`skills/plan/**`,
`crates/enforcer-install/src/commands/plan.rs`,
`crates/enforcer-plan/tests/self_validate.rs`) — the `/plan` harness command
this skill's installer emitter ships still targets this real binary
invocation, never a hand-rolled per-harness workaround.

## Mechanical Backing (never trust prose over these)

| Concern | Entrypoint |
|---|---|
| Scaffold a plan | `enforcer_plan::scaffolder::scaffold_plan` |
| Capsule contract | `enforcer_plan::validator::PlanCapsuleValidator` (`PLAN-CAPSULE.1`) |
| Section skeleton | `enforcer_plan::validator::PlanSkeletonValidator` (`PLAN-SKELETON.1`) |
| owns/deps/tier | `enforcer_plan::validator::PlanFrontmatterValidator` (`PLAN-FRONTMATTER.1`) |
| Disjoint parallel ownership | `enforcer_plan::validator::check_parallel_safety` (`PLAN-PARALLEL.1`) |
| Resume-state presence | `enforcer_plan::validator::PlanResumeStateValidator` (`PLAN-RESUME.1`) |
| Checklist/Where-We-Are drift | `enforcer_plan::validator::check_checklist_drift` (`PLAN-DRIFT.1`) |
| Frozen templates | `enforcer_plan::templates` (`crates/enforcer-plan/templates/*.tpl`) |
| Orchestration binding | `enforcer_plan::orchestrator` |

## Failure Handling

Report the exact `ruleId` that fired, the file/line the `Finding` points at,
and the `Fix:` implied by that validator's own doc comment. Never weaken a
PLAN-* check to make a workpack pass; fix the workpack, or fix the caller's
scope facts if the checklist item does not belong.
