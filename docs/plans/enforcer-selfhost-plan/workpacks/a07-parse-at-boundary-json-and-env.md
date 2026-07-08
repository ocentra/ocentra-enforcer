# a07 Parse At Boundary Config And Env (enforcer-config)

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Parse At Boundary Config And Env (enforcer-config)`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `crates/enforcer-config/src/load.rs`, `crates/enforcer-config/src/env.rs`, `crates/enforcer-config/src/schema.rs`
- deps: `a01`, `a03`
- tier: `P0`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [RUST_ARCHITECTURE](../RUST_ARCHITECTURE.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md).

## Where We Are
The legacy engine called `JSON.parse(fs.readFileSync(...))` in at least five places (rules.json, config, profile, target config) and fed the raw `any` result straight into logic; `process.env` was read ad hoc with no single validated boundary. In the Rust engine, config load lives in the `enforcer-config` crate (typed config load, parse-at-boundary). Reading files into weakly-typed `serde_json::Value` and threading it around, or scattering `std::env::var(...)` calls, would reintroduce the same two holes in the type story.

## Where We Want To Be
Every config/JSON/RON file is read and immediately `serde`-deserialized into a strongly-typed struct in `enforcer-config` (never a loose `Value` passed into logic), and a single `env` module is the only place `std::env` is read, exposing a decoded, typed config object. This is the `enforcer-config` crate's parse-at-boundary contract.

## Requirement Checklist
- [ ] Define typed config structs in `crates/enforcer-config/src/schema.rs` (rules manifest, engine config, profile, target config) that `#[derive(Deserialize)]` and reject unknown fields (`#[serde(deny_unknown_fields)]` where appropriate).
- [ ] `crates/enforcer-config/src/load.rs` deserializes each file into its typed struct via `serde` (json/RON), returning `Result<T, ConfigError>`; there is no public API returning a loose `serde_json::Value`.
- [ ] Parse/deserialize failure is fail-closed: a typed error carrying the source file path, never a silent `default`/empty struct.
- [ ] `crates/enforcer-config/src/env.rs` is the sole reader of `std::env`; it declares each consumed var, its type, and required/default, and fails-closed on unknown/missing-required.
- [ ] Rule ids parsed from config deserialize into the a03 `RuleId` newtype (parse-at-boundary), not `String`.

## Acceptance And Proof
Tier P0. `cargo test` in `enforcer-config`: malformed and schema-invalid input each produce a deserialize error naming the source path; env decoder rejects unknown/missing-required. A workspace-level check (owned/asserted by a09/a10 dogfood) confirms no `std::env::var` reads outside `enforcer-config::env` and no public loose-`Value` config API. Fail/pass fixtures per RUST_ARCHITECTURE 5-way parity. Rows in TEST_PROOF_EXPECTATIONS.md.

## Parallel Ownership Notes
Depends on a01 (workspace) and a03 (`RuleId` newtype for parsed rule ids). Owns the three `enforcer-config` modules exclusively; disjoint from the `enforcer-domain` newtype packs (a03/a04/a05/a06) and from `enforcer-coordination`. The single-env-reader invariant is what a10's native dogfood later asserts mechanically. Coordinate `mod`/`pub use` in `enforcer-config/src/lib.rs`.
