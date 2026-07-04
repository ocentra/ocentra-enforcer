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
The legacy config carried `sourceShapeOverrides` (51 entries, e.g. a source file with `maxBranches: 122`, `maxFunctionLines: 540`) — silent limit bumps with no reason, no owner, no expiry. The enforcer excused itself without saying so, which is dishonest self-enforcement. In the Rust engine, rules are structured data (`enforcer-rules` registry); a naked numeric override in a config file has no home and no audit trail.

## Where We Want To Be
Waivers are **structured data** in the `enforcer-rules` registry: a typed `Waiver` record (`path`, `rule_id: RuleId`, non-empty `reason`, `owner`, optional `expires`) loaded via `serde`, and a Rust validator enforces that every waiver is honest (real reason, valid rule id, not expired) and that no silent numeric limit-bumps remain. Every excuse is visible, typed, and auditable — and the enforcer's own dogfood (a10) reads these waivers rather than trusting a naked override.

## Requirement Checklist
- [ ] Define a `Waiver` struct in `crates/enforcer-rules/src/waiver.rs` (`path`, `rule_id: RuleId` from a03, `reason: String` required non-empty, `owner: String`, `expires: Option<...>`), `#[derive(Deserialize)]` with `deny_unknown_fields`.
- [ ] Store waivers as structured data (`crates/enforcer-rules/waivers.ron` or equivalent registry file) loaded/parsed at boundary; no naked numeric limit-bumps anywhere.
- [ ] Migrate every legacy `sourceShapeOverrides` entry: either dropped (rule fixed) or converted to a `Waiver` with a real reason; count parity waived + fixed == original 51 recorded as data.
- [ ] A Rust validator (`enforcer-rules`/`enforcer-validator`) fails-closed on: empty `reason`, unknown/invalid `rule_id`, or (if enabled) an expired waiver.
- [ ] The validator is registered so the scan honors waivers by rule + path, never as a blanket silent skip.

## Acceptance And Proof
Tier P1. `cargo test` in `enforcer-rules` asserts: the waiver registry deserializes; a waiver with empty `reason` or invalid `rule_id` is rejected fail-closed; count parity (waived + fixed == 51) holds; no naked numeric limit-bump shape is representable. Fail/pass fixtures per RUST_ARCHITECTURE 5-way parity. Rows in TEST_PROOF_EXPECTATIONS.md.

## Parallel Ownership Notes
Depends on a01 (workspace) and a03 (`RuleId` newtype for the waiver's rule reference). Owns `crates/enforcer-rules/src/waiver.rs` and the waiver registry file exclusively; its enforcement is exercised by a09 (honest scan) and a10 (native dogfood). Coordinate `mod`/`pub use` in `enforcer-rules/src/lib.rs`.
