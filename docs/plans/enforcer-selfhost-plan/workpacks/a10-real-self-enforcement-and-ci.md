# a10 Real Self Enforcement Native Dogfood And CI

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Real Self Enforcement Native Dogfood And CI`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `.github/workflows/dogfood.yml`, `xtask/src/dogfood.rs`, `crates/enforcer-cli/tests/self_enforce.rs`
- deps: `a01`, `a08`, `a09`
- tier: `P4`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [RUST_ARCHITECTURE](../RUST_ARCHITECTURE.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md).

## Where We Are
Legacy self-enforcement ran only `check source-shape`, leaned on silent overrides to stay green, and did not hard-fail on real findings. In the Rust engine, "eat your own dog food" is **native**: the enforcer's own Rust rules (from `enforcer-rules` + the `enforcer-lang-rust`/validator crates) plus the standard Rust toolchain (`cargo clippy`/`fmt`/`deny`/`audit`) plus the built `enforcer` binary should all run on the enforcer's OWN crates — but that dogfood loop and its hard-fail CI gate do not exist yet.

## Where We Want To Be
A native dogfood pipeline: the `enforcer` binary runs its own Rust rule set (via `enforcer scan`) against the workspace's `crates/**`, alongside `cargo fmt --check` + `cargo clippy -D warnings` + `cargo deny check` + `cargo audit`, and any real finding hard-fails. This runs in CI on push/PR (extending the base gate a01 lays down) and honors a08 waivers as the only sanctioned exceptions. With a09's honest coverage, a hollow (zero-ran) self-scan hard-fails rather than passing.

## Requirement Checklist
- [ ] Add an `xtask dogfood` command (`xtask/src/dogfood.rs`) that builds `enforcer` and runs it (`enforcer scan crates/`) with the enforcer's own Rust rules, plus `cargo fmt --all --check`, `cargo clippy --all-targets -- -D warnings`, `cargo deny check`, `cargo audit` — aggregating exit codes and hard-failing on any non-zero.
- [ ] `crates/enforcer-cli/tests/self_enforce.rs` invokes the built binary on the workspace's own crates and asserts it runs a nonzero number of checks (a09 coverage) and exits zero on the clean tree.
- [ ] `.github/workflows/dogfood.yml` runs the `xtask dogfood` gate on push/PR across the target matrix, extending a01's base workflow (a01 owns the base fmt/clippy/test/deny/audit gate; a10 owns the native `enforcer`-on-its-own-crates step).
- [ ] With a09's honest skips, a self-scan that ran zero checks fails CI rather than passing.
- [ ] No bypass flag; a08 waivers (structured data) are the only sanctioned exceptions and are honored by the dogfood run.

## Acceptance And Proof
Tier P4. Proof: seed a self-violating fixture in a crate (clippy lint / rule violation) and show `xtask dogfood` + the CI job exit non-zero; show a formatting violation fails `cargo fmt --check`; show the workflow file invokes the native `enforcer scan crates/` step; a green run on the workspace with a visible nonzero ran-count. Fail/pass fixtures per RUST_ARCHITECTURE 5-way parity. Rows in TEST_PROOF_EXPECTATIONS.md.

## Parallel Ownership Notes
Depends on a01 (workspace/toolchain + base CI), a08 (waivers as structured data, the only exceptions), and a09 (honest coverage, so zero-ran hard-fail is meaningful). Owns the dogfood workflow, the `xtask dogfood` command, and the CLI self-enforce integration test exclusively; a01 owns `.github/workflows/ci.yml` (base gate); a10 owns `.github/workflows/dogfood.yml` — disjoint files, sequenced (a10 deps a01). This is the capstone that turns the whole track green under real native gates: the enforcer enforcing itself.
