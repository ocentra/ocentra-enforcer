# a06 Branded HubName LaneId Newtypes (enforcer-domain)

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Branded HubName LaneId Newtypes (enforcer-domain)`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `crates/enforcer-domain/src/coordination_ids.rs`
- deps: `a01`
- tier: `P0`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [RUST_ARCHITECTURE](../RUST_ARCHITECTURE.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md).

## Where We Are
The legacy coordination code threaded hub names and lane ids as raw `string`. In the Rust engine the coordination hub is the `enforcer-coordination` crate (ported from `src/coordination/vendor/*.js`), which still builds filesystem paths under the coordination root from these ids. A hub name passed where a lane id is expected, or an unsanitized id used to build a path, would be invisible to the compiler and only surface as a mislocated or colliding coordination artifact. This is a **domain-modeling** workpack that populates the `enforcer-domain` crate — it FOLDS INTO arc-02-enforcer-domain.

## Where We Want To Be
`HubName` and `LaneId` branded **newtypes** in `enforcer-domain`, each with a private inner field, minted only via `parse`-at-boundary (and `serde`), so the two cannot be swapped and every id is validated (filesystem-safe charset) before `enforcer-coordination` uses it in path construction or presence/claim APIs.

## Requirement Checklist
- [ ] Define `HubName` and `LaneId` newtypes in `crates/enforcer-domain/src/coordination_ids.rs` with private inner fields and `parse`-at-boundary constructors; derive `Debug, Clone, PartialEq, Eq, Hash`, `Display`, and `serde` via the validator.
- [ ] Parse enforces a filesystem-safe charset (no path separators, no `..`, bounded length, non-empty).
- [ ] `enforcer-coordination` context/claim/presence signatures accept `HubName`/`LaneId`, never bare `String`; path construction consumes only these newtypes.
- [ ] Being distinct types, swapping a `HubName` for a `LaneId` (or vice versa) is a compile error.
- [ ] Parse fails-closed (`Err`) on empty / unsafe-charset / oversized ids; serde rejects the same.

## Acceptance And Proof
Tier P0. `cargo test` in `enforcer-domain`: valid mint for both newtypes, rejection of unsafe charset / empty / oversize, and a test confirming path helpers accept only branded ids. Fail/pass fixtures per RUST_ARCHITECTURE 5-way parity. Distinct types make `HubName` != `LaneId` a compile-time guarantee. Rows in TEST_PROOF_EXPECTATIONS.md.

## Parallel Ownership Notes
Depends on a01. Populates the `enforcer-domain` crate (folds into arc-02-enforcer-domain); owns `crates/enforcer-domain/src/coordination_ids.rs` exclusively; disjoint from a03/a04/a05 newtype modules and from a07's config/env boundary. The broader `enforcer-coordination` crate (port of `vendor/*.js`) adopts these types in its own workpacks. Coordinate `mod`/`pub use` in `enforcer-domain/src/lib.rs`.
