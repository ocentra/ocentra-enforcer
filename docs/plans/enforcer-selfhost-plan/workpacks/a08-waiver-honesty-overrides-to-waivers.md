# a08 Waiver Honesty Overrides To Waivers (enforcer-rules)

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Waiver Honesty Overrides To Waivers (enforcer-rules)`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `crates/enforcer-rules/src/waiver.rs`, `crates/enforcer-rules/waivers.ron`
- deps: `a01`, `a03`
- tier: `P1`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [RUST_ARCHITECTURE](../RUST_ARCHITECTURE.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md).

## Where We Are
The legacy config carried `sourceShapeOverrides` (51 entries, e.g. a source file with `maxBranches: 122`, `maxFunctionLines: 540`) — per-file numeric limit bumps with no reason, no owner, no expiry. NOT all 51 are equal, and the earlier framing of "all overrides = dishonesty to waive" is too blunt. Two kinds live in that one list: (a) HONEST per-file policy tuning — a legitimately different budget for a file whose shape is intentional (a large generated/vendor file, a boundary module), which is a real policy decision that belongs in the resolved `sourceShapePolicies` shape, not a waiver; and (b) DISHONEST silent limit-bumps — a file quietly excused past the shared budget with no reason/owner/expiry, which is the enforcer lying to itself. The base per-root/per-ext `sourceShapePolicies` shape (the honest budgets themselves) is NOT owned here — it is a first-class config field homed in **arc-03** (base shape) and consumed by **arc-04** (the source-shape rule); a08 references it and owns ONLY the dishonest-bump -> `Waiver` conversion + the honesty validator. In the Rust engine, rules are structured data (`enforcer-rules` registry); a naked numeric override in a config file has no home and no audit trail.

## Where We Want To Be
Waivers are **structured data** in the `enforcer-rules` registry: a typed `Waiver` record (`path`, `rule_id: RuleId`, non-empty `reason`, `owner`, optional `expires`) loaded via `serde`, and a Rust validator enforces that every waiver is honest (real reason, valid rule id, not expired) and that no silent numeric limit-bumps remain. Every excuse is visible, typed, and auditable — and the enforcer's own dogfood (a10) reads these waivers rather than trusting a naked override.

## Requirement Checklist
- [ ] Define a `Waiver` struct in `crates/enforcer-rules/src/waiver.rs` (`path`, `rule_id: RuleId` from a03, `reason: String` required non-empty, `owner: String`, `expires: Option<...>`), `#[derive(Deserialize)]` with `deny_unknown_fields`.
- [ ] Store waivers as structured data (`crates/enforcer-rules/waivers.ron` or equivalent registry file) loaded/parsed at boundary; no naked numeric limit-bumps anywhere.
- [ ] **Triage, don't blanket-waive [G4].** Classify each of the 51 legacy `sourceShapeOverrides` entries (fields observed: `maxNestingDepth`, `maxBranches`, `maxFunctionLines`, `maxLines`, `maxExports`) into: (1) HONEST per-file policy tuning -> re-expressed as/subsumed by the resolved `sourceShapePolicies` shape owned by arc-03/arc-04 (a08 does NOT model this shape; it only records that the entry was reclassified to policy, not waived); (2) DISHONEST silent limit-bump -> converted to a typed `Waiver` (owner + non-empty reason + `rule_id`/`RuleId` for the source-shape rule it excuses + optional `expires`); (3) dropped because the underlying file was fixed. The migration is a documented triage, not a mechanical "everything becomes a waiver."
- [ ] Migrate every legacy `sourceShapeOverrides` entry per the triage above: dropped (fixed), reclassified-to-policy (honest), or converted to a `Waiver` (dishonest); count parity `waived + reclassified + fixed == original 51`, recorded as data (each entry's disposition auditable).
- [ ] A Rust validator (`enforcer-rules`/`enforcer-validator`) fails-closed on: empty `reason`, unknown/invalid `rule_id`, or (if enabled) an expired waiver.
- [ ] The validator is registered so the scan honors waivers by rule + path, never as a blanket silent skip.

## Acceptance And Proof
Tier P1. `cargo test` in `enforcer-rules` asserts: the waiver registry deserializes; a waiver with empty `reason` or invalid `rule_id` is rejected fail-closed; triage count parity (`waived + reclassified-to-policy + fixed == 51`) holds with each entry's disposition recorded; no naked numeric limit-bump shape is representable (the honest `sourceShapePolicies` budgets live in arc-03/arc-04, referenced here, not modeled here). Fail/pass fixtures per RUST_ARCHITECTURE 5-way parity. Rows in TEST_PROOF_EXPECTATIONS.md.

## Parallel Ownership Notes
Depends on a01 (workspace) and a03 (`RuleId` newtype for the waiver's rule reference). Owns `crates/enforcer-rules/src/waiver.rs` and the waiver registry file exclusively; its enforcement is exercised by a09 (honest scan) and a10 (native dogfood). Coordinate `mod`/`pub use` in `enforcer-rules/src/lib.rs`.
