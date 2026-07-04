# arc-18 Crate enforcer-harness

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Crate enforcer-harness`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `crates/enforcer-harness/**`
- deps: `arc-01`, `arc-02`
- tier: `P1`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md), [RUST_ARCHITECTURE](../RUST_ARCHITECTURE.md).

## Where We Are
Adapters that run native tools (cargo, tsc, ruff, dart, CFLint, ...) and compact their output into diagnostics live in `src/harness-parsers-*.mjs` and related `.mjs`. There is no Rust crate providing run-adapters + compact-diagnostic parsing.

## Where We Want To Be
`enforcer-harness` is the Rust native-tool run-adapter crate per RUST_ARCHITECTURE.md: it shells out to native tools (cargo/tsc/ruff/dart/CFLint/...), parses their output into `enforcer-domain` diagnostics, and produces compact diagnostics. It is the graceful-skip seam where an external engine is irreplaceable.

## Requirement Checklist
- [ ] Implement run-adapters per RUST_ARCHITECTURE.md for the native tools (cargo/tsc/ruff/dart/CFLint...), each parsing tool output into `enforcer-domain` findings/diagnostics.
- [ ] Implement compact diagnostics (the condensed output format) and graceful-skip when a tool is absent (report skip, do not hard-fail) per the distribution doctrine.
- [ ] Port the `.mjs` harness-parser logic (`src/harness-parsers-*.mjs`) to Rust.
- [ ] `cargo test -p enforcer-harness` passes with fail/pass fixtures: canned tool-output samples parse to the expected diagnostics (fail fixture: a real error line -> finding; pass fixture: clean output -> none), and a missing-tool case yields a graceful skip.
- [ ] Clean `cargo clippy` / `cargo fmt --check`.

## Acceptance And Proof
Tier P1. Proof row asserts `cargo test -p enforcer-harness` exits 0 — canned tool-output parsing (fail/pass) + graceful-skip proven. Record the artifact path in TEST_PROOF_EXPECTATIONS.md.

## Parallel Ownership Notes
Owns only `crates/enforcer-harness/**`. Deps arc-01/02 only, so it can proceed early in parallel with the validator track. Parallel-safe with arc-15/arc-16/arc-17 — disjoint crate trees.
