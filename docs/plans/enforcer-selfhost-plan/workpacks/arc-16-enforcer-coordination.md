# arc-16 Crate enforcer-coordination

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Crate enforcer-coordination`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `crates/enforcer-coordination/**`
- deps: `arc-01`, `arc-02`
- tier: `P1`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md), [RUST_ARCHITECTURE](../RUST_ARCHITECTURE.md).

## Where We Are
The multi-agent coordination hub is a vendored JS tree at `src/coordination/vendor/*.js` (domain, paths, root, events, identity, claim-policy, lock-policy, stream, materialize, presence, health-presence, guard, repair, server, daemon, cli, notify, peers, retention, read-index, manifest, dashboard, context). It is not Rust and not a workspace crate.

## Where We Want To Be
`enforcer-coordination` is the Rust coordination engine per RUST_ARCHITECTURE.md: hub/lane/claim/guard/ledger/presence/sync built on `enforcer-domain` (`HubName`, `LaneId`), porting the entire `src/coordination/vendor/*.js` tree to Rust. This is the crate that MUST port `src/coordination/vendor/*.js` -> Rust.

## Requirement Checklist
- [ ] Port `src/coordination/vendor/*.js` to Rust in `crates/enforcer-coordination` per RUST_ARCHITECTURE.md — explicitly: this crate ports `src/coordination/vendor/*.js` -> Rust.
- [ ] Cover the subsystems: hub, lane, claim + claim-policy, guard, ledger/events + materialize/read-index, presence + health-presence, sync + stream, plus identity, lock-policy, repair, retention, peers, notify, manifest, dashboard, context, root/paths.
- [ ] Build all coordination identifiers/records on `enforcer-domain` newtypes (`HubName`, `LaneId`, etc.); parse-at-boundary for the on-disk ledger/index.
- [ ] `cargo test -p enforcer-coordination` passes with fail/pass fixtures for the load-bearing invariants (claim conflict rejected vs. granted; guard fires on a corrupt ledger vs. passes on a healthy one; sync/materialize round-trip).
- [ ] Clean `cargo clippy` / `cargo fmt --check`.

## Acceptance And Proof
Tier P1. Proof row asserts `cargo test -p enforcer-coordination` exits 0 — claim/guard/sync invariants proven with fail/pass fixtures, and behavior parity with the ported `vendor/*.js` semantics. Record the artifact path in TEST_PROOF_EXPECTATIONS.md.

## Parallel Ownership Notes
Owns only `crates/enforcer-coordination/**` (the Rust port). Deps arc-01/02 only, so it can proceed in parallel with the rules/validator/lang track. Parallel-safe with arc-15/arc-17/arc-18 — disjoint crate trees. Consumed by arc-21 (mcp) for the coordination tool surface.
