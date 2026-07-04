# f03 Project Tie And Native Augment

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Project Tie And Native Augment`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling completion, product status, PR readiness, or broad DONE.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `src/project-config/*`, `.enforce/config` schema (Effect)
- deps: `a01-ts-toolchain-and-build`
- tier: `P1`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md).

## Where We Are
There is no per-project config that says how the enforcer relates to native tools (cargo/tsc/ruff). Nothing declares whether we replace, add to, or run alongside native checks, and nothing bounds our scope. Without this contract, an agent either skips our checks or runs the whole repo by mistake.

## Where We Want To Be
A per-project config schema (Effect Schema, parse-at-boundary) tying native tools WITH the enforcer via a `mode`: `override` (ours instead), `augment` (ours in addition), or `both`. Default = `augment` scoped: let native run AND run our SCOPED checks (crate/file), never our whole-repo by default. This is the agent-facing contract consumed by the c04 deny-hook and the f01 MCP scan.

## Requirement Checklist
- [ ] Effect schema for `.enforce/config` with a `nativeMode` field (`override|augment|both`) per tool.
- [ ] Default resolution = `augment` with scoped (crate/file) enforcer checks; whole-repo is never the default.
- [ ] Config is parsed/validated at the boundary; malformed config fails fast with a typed error.
- [ ] Exposes a resolver API consumed by c04 (deny-hook) and f01 (scan) for "run ours too, scoped."
- [ ] No mode silently suppresses our checks; disabling requires an explicit gated waiver (owner+reason+ruleId) per honesty doctrine.

## Acceptance And Proof
Tier P1. Proof row `project-config-native-mode` in TEST_PROOF_EXPECTATIONS.md:
- fail-fixture: malformed `.enforce/config` (bad `nativeMode`) -> asserts typed boundary parse error, no silent default.
- pass-fixture: valid config -> resolver returns `augment` scoped, and native+enforcer both selected for the crate.
- detection test: absence of config -> resolver returns the scoped `augment` default (never whole-repo), asserted on the resolved scope.

## Parallel Ownership Notes
Owns `src/project-config/*` and the `.enforce/config` schema only. It is the contract that f01, f02, and c04 consume; those packs do not define it. Disjoint from f04 (run-context mode), which is orthogonal to native-tie mode.
