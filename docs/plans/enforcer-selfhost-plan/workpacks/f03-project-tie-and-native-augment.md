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

- owns: `crates/enforcer-config/src/project_tie.rs`, `crates/enforcer-config/src/policy.rs`, `crates/enforcer-config/tests/project_tie.rs`, `crates/enforcer-config/tests/fixtures/project_tie/**`
- deps: `arc-03-enforcer-config`
- tier: `P1`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md), [RUST_ARCHITECTURE](../RUST_ARCHITECTURE.md).

## Where We Are
There is no per-project config that says how the enforcer relates to native tools (`cargo`/`tsc`/`ruff`/`dart`/`CFLint`). Nothing declares whether we replace, add to, or run alongside native checks, and nothing bounds our scope. Rule suppression, when it happens at all, lives inline in source (an anti-pattern the enforcer itself bans). The arc-03 `enforcer-config` skeleton provides typed load + parse-at-boundary, but no `project_tie` module or committed declarative policy.

## Where We Want To Be
A per-project config type in `crates/enforcer-config/src/project_tie.rs` (`serde` structs built on `enforcer-domain` newtypes, parsed at boundary by the arc-03 loader) tying native tools WITH the enforcer via a `native_mode`: `Override` (ours instead), `Augment` (ours in addition), or `Both` — per tool. Default = `Augment` scoped: let native run AND run our SCOPED checks (crate/file), never our whole-repo by default. Alongside it, a DECLARATIVE committed policy in `src/policy.rs` (the OcentraParent config-externalization borrow): owner/exempt globs, allow-regex lists, `cfg(test)` skipping, and per-rule toggles (on/off, severity, waiver) — all data in `.enforce/config`, NEVER an inline `#[allow]`/comment disable in the target source. This is the agent-facing contract consumed by the c04 deny-hook, the f01 MCP scan, and f05's native-tie step.

## Requirement Checklist
- [ ] A `serde` `ProjectConfig` (`.enforce/config`) with a `native_mode` field (`Override|Augment|Both`) per tool, built on `enforcer-domain` newtypes; loaded and validated through the arc-03 parse-at-boundary loader (malformed `native_mode` -> typed `thiserror` error, no silent default).
- [ ] Declarative policy externalization (`src/policy.rs`): committed owner globs, exempt globs, allow-regex lists, `cfg(test)`/test-path skipping, and per-rule toggles (enable/disable, severity override, waiver) — all read from `.enforce/config`, never an inline disable.
- [ ] Default resolution = `Augment` with scoped (crate/file) enforcer checks; whole-repo is never the default.
- [ ] Exposes a resolver API (returning a resolved, total policy view) consumed by c04 (deny-hook), f01 (scan), and f05 (native-tie) for "run ours too, scoped."
- [ ] No mode silently suppresses our checks; disabling a rule requires an explicit gated waiver (owner + reason + `RuleId`) per the honesty doctrine, and inline suppression stays banned (enforced elsewhere by the no-bypass meta-check in `enforcer-security`).

## Acceptance And Proof
Tier P1. Proof row `project-config-native-mode` in TEST_PROOF_EXPECTATIONS.md asserts `cargo test -p enforcer-config --test project_tie` exits 0:
- fail-fixture: malformed `.enforce/config` (bad `native_mode`) in `tests/fixtures/project_tie/**` -> asserts a typed boundary parse error, no silent default.
- pass-fixture: valid config -> resolver returns `Augment` scoped, and native + enforcer are both selected for the crate; a per-rule toggle + owner/exempt glob + allow-regex round-trip through serde and take effect.
- detection test: absence of config -> resolver returns the scoped `Augment` default (never whole-repo), asserted on the resolved scope; an inline-disable in a fixture is NOT honored (only declarative policy is).

## Parallel Ownership Notes
Owns ONLY `crates/enforcer-config/src/{project_tie,policy}.rs` + its `tests/project_tie.rs` and `tests/fixtures/project_tie/**` — disjoint files inside the arc-03 crate, which owns the crate skeleton (`Cargo.toml`, `lib.rs`, the base loader). Deps arc-03 (sequenced after the loader exists). It is the contract that f01, f02, f05, and c04 consume; those packs do not define it. Disjoint from f04 (run-context mode), which is an orthogonal axis. `owns disjoint? = Y`.
