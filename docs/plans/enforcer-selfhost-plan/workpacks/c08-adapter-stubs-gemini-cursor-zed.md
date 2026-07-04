# c08 Adapter Stubs Gemini Cursor Zed

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Adapter Stubs Gemini Cursor Zed`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `crates/enforcer-install/src/adapters/gemini.rs, crates/enforcer-install/src/adapters/cursor.rs, crates/enforcer-install/src/adapters/zed.rs, crates/enforcer-install/tests/fixtures/stubs/**`
- deps: `c01-install-core-and-cli-contract, arc-23`
- tier: `P0 contract/schema`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md), [RUST_ARCHITECTURE](../RUST_ARCHITECTURE.md).

## Where We Are
Only Codex has a real adapter, and c03/c06/c07 add Claude/generic. Gemini, Cursor, and Zed each have differing config surfaces we are not building fully yet, but the arc-23 `Adapter` registry must still resolve them by id rather than silently no-op or crash. The registry keys off a typed `AdapterId` newtype (from `enforcer-domain`, arc-02), so an unknown id is a typed error, not a silent skip.

## Where We Want To Be
Contract-only stub adapter modules for `gemini`, `cursor`, and `zed` that implement the arc-23 `Adapter` trait, declare themselves not-yet-implemented explicitly, and note that ADBP-style config converters are deferred.

## Requirement Checklist
- [ ] Each stub implements the `Adapter` trait (`plan`/`apply`/`verify`) returning a well-formed typed `Report` with `Status::Deferred` and a reason string.
- [ ] `apply` on a stub is a safe no-op that writes zero files and returns `Ok` (never `panic`/`unwrap`).
- [ ] `verify` returns a single advisory `Check` labeled `deferred: no mechanization yet` (T3-labeled `Tier::T3` with the reason stated), not an `Error`-severity check.
- [ ] A source `//` comment records that ADBP-style converters for these harnesses are deferred (link Track B once numbered).
- [ ] Each stub registers in the arc-23 adapter registry under its id (`gemini`/`cursor`/`zed`) so c02 autodetect and the arc-22 CLI can surface it; modules obey `[workspace.lints]` (no `unwrap`/`expect`/`print_*`, no `pub use` barrels).

## Acceptance And Proof
P0 contract (`adapter-stub-contract` in TEST_PROOF_EXPECTATIONS.md), proved by `cargo test -p enforcer-install`: a `#[test]` iterates all three stubs and asserts each conforms to the `Adapter` trait, returns `Status::Deferred`, and performs zero filesystem writes when `apply`-ed against a temp-dir fixture under `tests/fixtures/stubs/`. Registry lookup for each `AdapterId` must resolve (`Ok`, no panic).

## Parallel Ownership Notes
Owns only the three stub adapter modules (+ `tests/fixtures/stubs/**`) — disjoint by file from generic (c07 `generic.rs`), codex (c06), and claude (c03); the crate skeleton, trait, and registry belong to arc-23. Depends on c01 and arc-23. Deliberately scoped as contract-only so Track B ADBP converter work can land later without touching these modules' interface. owns disjoint? = Y
