# a03 Branded RuleId Newtype (enforcer-domain)

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Branded RuleId Newtype (enforcer-domain)`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `crates/enforcer-domain/src/rule_id.rs`
- deps: `a01`
- tier: `P0`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [RUST_ARCHITECTURE](../RUST_ARCHITECTURE.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md).

## Where We Are
Rule ids flowed through the legacy engine as raw `string` (Effect-Schema `.mjs` + `rule-registry`/`policy`). A typo, or a policy id with no registry entry, was only caught at runtime if at all. In the Rust engine the rule registry is structured data (rules-as-data) loaded via `serde`; without a dedicated newtype there is no compile-time distinction between "an arbitrary string" and "a validated rule id". This is a **domain-modeling** workpack that populates the `enforcer-domain` crate — it FOLDS INTO arc-02-enforcer-domain (the single-source schema crate).

## Where We Want To Be
A `RuleId` branded **newtype** in `enforcer-domain` (`struct RuleId(String)` or `Box<str>`) with a private field, minted only via `RuleId::parse` (and `serde` deserialization that calls the same validator), so every downstream crate receives `RuleId` and a raw `String` cannot flow into the registry/policy APIs.

## Requirement Checklist
- [ ] Define `RuleId` newtype in `crates/enforcer-domain/src/rule_id.rs` with a private inner field, `TryFrom<&str>`/`FromStr`, and a `parse`-at-boundary constructor validating the id grammar.
- [ ] Implement `serde::Deserialize` via the same validator (parse-at-boundary), and `Serialize`/`Display` for round-trip; derive `Debug, Clone, PartialEq, Eq, Hash` so it can key a registry map.
- [ ] The registry keys/lookups and policy rule-references consume `RuleId`, never bare `String`; the inner value is not publicly constructible.
- [ ] Parse rejects unknown/malformed ids fail-closed (returns `Err`), never silently coerces.
- [ ] A parity check (owned by the rules crate that consumes this newtype) can assert every policy `RuleId` resolves to a registry entry — this pack ships the type + parse boundary that makes that check well-typed.

## Acceptance And Proof
Tier P0 (contract/schema). `cargo test` in `enforcer-domain` asserts `RuleId::parse` mints for valid ids and returns `Err` for invalid (empty / malformed / wrong grammar); a serde round-trip test proves deserialization rejects bad ids. Fail/pass fixtures (RUST_ARCHITECTURE 5-way parity) cover the boundary. The private field enforces at compile time that a bare `String` cannot substitute for `RuleId`. Rows in TEST_PROOF_EXPECTATIONS.md.

## Parallel Ownership Notes
Depends on a01. Populates the `enforcer-domain` crate (folds into arc-02-enforcer-domain); owns `crates/enforcer-domain/src/rule_id.rs` exclusively. a04 (paths), a05 (sha256), a06 (hub/lane ids) add disjoint newtype modules to the same crate — all P0 domain packs run concurrently after a01 as long as each owns its own `src/*.rs` file and `lib.rs` re-exports are appended without collision (coordinate the `mod`/`pub use` lines in `enforcer-domain/src/lib.rs`).
