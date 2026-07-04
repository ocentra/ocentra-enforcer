# c07 Generic Writer And Doctor

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Generic Writer And Doctor`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `crates/enforcer-install/src/adapters/generic.rs, crates/enforcer-install/src/doctor.rs, crates/enforcer-install/tests/fixtures/generic/**, crates/enforcer-install/tests/fixtures/doctor/**`
- deps: `c01-install-core-and-cli-contract, c02-harness-autodetect, arc-23`
- tier: `P1 unit`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md), [RUST_ARCHITECTURE](../RUST_ARCHITECTURE.md).

## Where We Are
The reference doctor logic is Codex-specific and tangled into one install path. Harnesses that only need a plain `.mcp.json` server entry have no adapter today, and there is no shared, mechanical doctor in `enforcer-install` that verifies an install regardless of adapter. Once c01/arc-23 land the `Adapter` trait and typed `Check`/`Report` records, both gaps get first-class Rust modules.

## Where We Want To Be
A generic adapter (`crates/enforcer-install/src/adapters/generic.rs`) that upserts a standard `.mcp.json` server entry for harnesses with no bespoke needs, plus a shared doctor module (`crates/enforcer-install/src/doctor.rs`) that mechanically re-reads disk and aggregates per-adapter `verify` checks into one typed `Report` across all adapters.

## Requirement Checklist
- [ ] Generic adapter upserts `mcpServers["ocentra-enforcer"]` into a target `.mcp.json` (`serde_json` value edit, preserving unrelated keys) given a resolved home path from arc-23; command points at the `enforcer` binary.
- [ ] Shared doctor aggregates each registered adapter's `verify` checks into one `Report` with typed `Severity` (from `enforcer-domain`, arc-02).
- [ ] Doctor is mechanical: every check re-reads the actual file and resolves the server binary path from disk, never trusts the plan.
- [ ] Doctor result is fail-closed — any `Severity::Error` check drives a non-zero CLI exit (arc-22); `Severity::Warning` checks do not fail.
- [ ] Generic adapter and doctor are pure over an injected filesystem abstraction (or a temp-dir root) for fixture testing; obey `[workspace.lints]` (no `unwrap`/`expect`/`panic`/`print_*`, no `pub use` barrels).

## Acceptance And Proof
T1 (`generic-writer` and `install-doctor` in TEST_PROOF_EXPECTATIONS.md), proved by `cargo test -p enforcer-install`: a `#[test]` asserts the generic adapter's `.mcp.json` output against a golden file under `tests/fixtures/generic/`, and doctor returns all-green on a good fixture and red (naming the failing check) on a `tests/fixtures/doctor/` fixture with a missing/renamed server binary.

## Parallel Ownership Notes
Owns `crates/enforcer-install/src/adapters/generic.rs` and `crates/enforcer-install/src/doctor.rs` (+ their `tests/fixtures/`) only — the crate skeleton, `Adapter` trait, and registry belong to arc-23. Disjoint by file from codex (c06), claude (c03), and stub (c08) adapters. Depends on c01/c02 and arc-23. Runs concurrently with all other adapter workpacks. owns disjoint? = Y
