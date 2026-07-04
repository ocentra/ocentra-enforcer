# f01 Scan Modes And MCP

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Scan Modes And MCP`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling completion, product status, PR readiness, or broad DONE.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `crates/enforcer-scan/src/modes.rs`, `crates/enforcer-scan/tests/modes.rs`, `crates/enforcer-scan/tests/fixtures/modes/**`
- deps: `arc-15-enforcer-scan, d01-rule-mechanization-engine`
- tier: `P1/P3`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md), [RUST_ARCHITECTURE](../RUST_ARCHITECTURE.md).

## Where We Are
Scanning is all-or-nothing: the agent can only run a broad check with no named scope. There is no typed selector that lets an AI agent, while coding, pick "just this crate" or "just the diff." Whole-repo runs are the only path, which is slow and wrong as an inline default. The `enforcer-scan` crate skeleton (arc-15) provides the rayon fan-out engine and the `enforcer-domain` `Report`, but no `modes` module bounds a run's scope/depth.

## Where We Want To Be
A `ScanMode` selector in `crates/enforcer-scan/src/modes.rs`, surfaced by the `enforcer_scan` MCP tool and the `enforcer scan --mode <m>` CLI, with named MODES the agent selects: `quick` (fast most-common T1 subset), `full` (everything the enforcer can do), `repo`/`workspace`, `scoped` (crate-or-folder), `diff` (changed files only), and `plan-scan` (validate a plan dir). Scope + depth are a typed `serde` enum + newtypes from `enforcer-domain` (`ScanScope`, `Tier`), parsed at the MCP/CLI boundary; default is scoped-not-whole-repo. The tool is agent-callable so the AI decides what to run inline over the arc-15 scan engine.

## Requirement Checklist
- [ ] A `ScanMode` enum + a `ScanRequest` struct (scope path, depth, tier filter) built on `enforcer-domain` newtypes (`ScanScope`, `RelPath`, `Tier`), `serde`-deserialized and validated at the MCP/CLI boundary (parse-at-boundary; malformed mode/scope is a typed `thiserror` error, never a silent default).
- [ ] Each mode maps to a deterministic rule/scope selection driving the arc-15 fan-out over the d01-scaffolded rule set; `quick` = a named T1 subset, `full` = all tiers.
- [ ] Default when no scope given is `scoped` (cwd crate/folder), never whole-repo.
- [ ] `diff` mode reads changed paths (base/head, via the tri-modal scope contract); `plan-scan` targets a plan dir.
- [ ] MCP tool name is `enforcer_scan` (registered in `enforcer-mcp`, arc-21); CLI is `enforcer scan --mode` (clap, `enforcer-cli`, arc-22). Both call the same `modes.rs` resolver — no logic duplicated in a surface.

## Acceptance And Proof
Tier P1/P3. Proof row `scan-modes-select` in TEST_PROOF_EXPECTATIONS.md asserts `cargo test -p enforcer-scan --test modes` exits 0:
- fail-fixture: a `full`-only violation seeded outside the scoped path in `tests/fixtures/modes/**` -> asserts `scoped`/`quick` does NOT report it (scope honored).
- pass-fixture: same violation inside scope -> `scoped` reports it; `full` always reports it.
- detection test: an invalid mode string is rejected at the deserialization boundary (typed error, non-zero), and each mode resolves to its expected rule/scope set.

## Parallel Ownership Notes
Owns ONLY `crates/enforcer-scan/src/modes.rs` + its `tests/modes.rs` and `tests/fixtures/modes/**` — disjoint files inside the arc-15 crate, which owns the crate skeleton (`Cargo.toml`, `lib.rs`, module root, fan-out engine). Deps arc-15 (sequenced after the skeleton exists) and d01 (rule set/parity). Disjoint by file from f02 (`onboard.rs`) and f05 (`router/**`), which also live in `enforcer-scan`; disjoint from f03 (project-config) and f04 (run-context mode), which f01 references but does not own. `owns disjoint? = Y`.
