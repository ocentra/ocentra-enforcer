# arc-25 Crate enforcer-events

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Crate enforcer-events`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `crates/enforcer-events/**`
- deps: `arc-01`, `arc-02`
- tier: `P1`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md), [RUST_ARCHITECTURE](../RUST_ARCHITECTURE.md), [EXECUTION_MODEL](../EXECUTION_MODEL.md).

## Where We Are
The long-lived/observable subsystems (scan lifecycle, coordination lane/claim/lease, proof) currently signal each other with ad hoc callbacks and object literals threaded through `.mjs`; there is no typed in-process event spine. OcentraParent already ships a working, self-contained `ocentra-eventing` crate (expected 0/minimal deps) that is exactly this spine. Per EXECUTION_MODEL §2, the borrow is to **VENDOR that crate as-is**, not re-implement it — don't spend tokens re-deriving a "lean subset."

## Where We Want To Be
`enforcer-events` IS `ocentra-eventing` VENDORED as-is: the crate source copied into `crates/enforcer-events/`, package renamed to `enforcer-events`, any ocentra-specific deps repointed to `enforcer-domain`/`enforcer-core`, keeping the full working implementation (typed `DomainEvent` + `EventEnvelope<E>` with stored-decode-verifies-contract, correlation/causation IDs, panic-isolated dispatch, and whatever the upstream crate ships). It is consumed ONLY by the observable subsystems (arc-15 scan, arc-16 coordination, arc-17 proof); pure-compute crates use plain calls. Any upstream machinery the enforcer doesn't use (contract-registry, aggregate-ordering, TTL queue, request/response, external transport) is left DORMANT and trimmed only in a later optional pass — we do not re-implement to shrink it.

## Requirement Checklist
- [ ] VENDOR the `ocentra-eventing` crate source from OcentraParent into `crates/enforcer-events/` (copy as-is; record origin + attribution, as done for the vendored cybersecurity-skills). Confirm its actual dependency footprint is 0/minimal at vendor time.
- [ ] Rename the crate/package → `enforcer-events`; add it as a workspace member (auto-included by a01's `members = ["crates/*"]`); adopt `[lints] workspace = true`; repoint any ocentra-specific internal deps to `enforcer-domain`/`enforcer-core`; ensure all deps resolve under our workspace.
- [ ] Adapt bare id fields to `enforcer-domain` branded newtypes (correlationId/causationId/etc.) at the boundary; pin camelCase wire casing to match the rest of the engine (only where it does not fight the vendored code — minimal edits).
- [ ] It BUILDS and its OWN vendored tests pass under our workspace: `cargo test -p enforcer-events` green; then wire the consumers arc-15/arc-16/arc-17 to emit typed events through it.
- [ ] (Optional, LATER — not required for green) trim upstream modules the enforcer never uses. Do NOT block on trimming.
- [ ] Clean `cargo clippy` / `cargo fmt --check` (accept upstream's shape; only fix what the lint wall genuinely requires).

## Acceptance And Proof
Tier P1. A proof row in TEST_PROOF_EXPECTATIONS.md asserts `cargo test -p enforcer-events` exits 0 — the vendored crate's own tests plus our boundary tests (envelope round-trip; version/digest-drifted envelope rejected on decode; panic-isolated dispatch) pass under the enforcer workspace. Attribution to OcentraParent recorded. Record the test artifact path.

## Parallel Ownership Notes
Leaf crate — owns ONLY `crates/enforcer-events/**`. Deps arc-01/02 only, so it proceeds in parallel with the rules/validator/lang track. VENDORED wholesale (do not re-implement). Consumed by arc-15 (scan lifecycle), arc-16 (coordination lane/claim/lease), and arc-17 (proof) — those three declare `deps: arc-25` in their frontmatter (wired 2026-07-04); pure-compute crates (domain/config/validator/lang-*) do NOT depend on it.
