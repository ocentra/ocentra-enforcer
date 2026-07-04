# arc-02 Crate enforcer-domain

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Crate enforcer-domain`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `crates/enforcer-domain/**`
- deps: `arc-01`
- tier: `P0`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md), [RUST_ARCHITECTURE](../RUST_ARCHITECTURE.md).

## Where We Are
Domain identifiers and records are untyped strings/objects threaded through the legacy `.mjs` (rule ids, paths, hashes, hub/lane ids, findings, reports). The old plan brand-domained these via Effect-Schema; the pivot replaces that with Rust newtypes + serde. Nothing yet defines the single-source schema the whole workspace parses at its boundary, and the OcentraParent "Logging = structured data" + "Schema + Rust->TS" borrows have no home: no versioned telemetry/audit records, no correlation/causation id newtypes, and no `ts_rs`-derived UI types.

## Where We Want To Be
`enforcer-domain` is the SINGLE-SOURCE schema crate — a serde-only, dependency-light LEAF that OWNS DTO SHAPE, NOT BEHAVIOR (per the OcentraParent borrow; the crate opens with a charter doc-comment stating exactly that, and ONE `enforcer-domain` crate with modules — NOT per-feature `*-domain` crates). It provides branded newtypes + serde for `RuleId`, `RepoRoot`, `RelPath`, `Sha256`, `HubName`, `LaneId`, `Severity`, `Tier`, `Finding`, `Violation`, `Report`, `ScanScope`, `ThreatId` (MITRE/OWASP), with parse-at-boundary constructors so no invalid value can be constructed. Per the "Logging = structured data" borrow it ALSO owns the versioned serde RECORDS (`schema_version` + `eventType`) that ride the `enforcer-core` NDJSON sink and the `enforcer-events` spine — `EnforcerEvent` (internally-tagged on `eventType`), `RunEvent`, `DiagnosticRecord`, `ArtifactRef`, `ScanEvent` — plus `correlationId`/`causationId` newtypes. UI-facing types carry `#[derive(ts_rs::TS)]` for the Rust->TS drift pipeline. Every downstream crate imports these types instead of raw strings.

## Requirement Checklist
- [ ] Open the crate with a charter doc-comment: `enforcer-domain` OWNS DTO SHAPE, NOT BEHAVIOR — a serde-only dependency-light leaf; ONE crate with modules, never per-feature `*-domain` crates; opt in via `[lints] workspace = true`; no `pub use` barrels.
- [ ] Implement all branded newtypes + records listed in RUST_ARCHITECTURE.md (`RuleId`, `RepoRoot`, `RelPath`, `Sha256`, `HubName`, `LaneId`, `Severity`, `Tier`, `Finding`, `Violation`, `Report`, `ScanScope`, `ThreatId`) with serde `Serialize`/`Deserialize`.
- [ ] Each newtype validates on construction (fallible `try_from`/`parse` returning the `enforcer-core` structured decode/validation error), preventing illegal values; no public raw-string constructor.
- [ ] Add `correlationId`/`causationId` as branded newtypes (reused by `enforcer-events` envelopes and the telemetry records) — never bare `String`.
- [ ] Implement the versioned serde RECORDS (OcentraParent "Logging = structured data" borrow), each carrying `schema_version` and reusing the branded newtypes above: `EnforcerEvent` (serde internally-tagged on `eventType`), `RunEvent`, `DiagnosticRecord`, `ArtifactRef`, `ScanEvent`. Records are DTO shape only; the NDJSON sink + hash-chain + redaction MECHANISM live in `enforcer-core` (arc-01).
- [ ] Pin camelCase serde wire casing (`#[serde(rename_all = "camelCase")]`) on the MCP/UI-facing types (per the locked wire-casing decision).
- [ ] `#[derive(ts_rs::TS)]` on the UI-facing types (findings/reports/records the UI renders) so arc-24's export bin + fail-closed drift test can regenerate the committed `.ts`.
- [ ] Port the Effect-Schema / ad-hoc `.mjs` shapes (rule ids, paths, sha, coordination ids, findings/reports, telemetry records) to these Rust types as the authoritative definition.
- [ ] `cargo test -p enforcer-domain` passes with fail/pass fixtures per newtype (valid inputs parse; malformed inputs are rejected), serde round-trip tests, and a round-trip for each versioned record asserting the `eventType` tag + `schema_version` + camelCase field names on the wire.
- [ ] Clean `cargo clippy` / `cargo fmt --check`.

## Acceptance And Proof
Tier P0 (schema is the load-bearing single source of truth). A proof row in TEST_PROOF_EXPECTATIONS.md asserts `cargo test -p enforcer-domain` exits 0 with per-newtype fail/pass fixtures (rejects malformed, accepts valid), serde round-trip coverage, and versioned-record round-trips proving the `eventType` tag + camelCase casing. Record the artifact path. (The `ts_rs` -> committed-`.ts` drift test is proven by the arc-24 UI/type pack, not here.)

## Parallel Ownership Notes
Foundation schema — nearly every other crate deps arc-02. Owns only `crates/enforcer-domain/**` (newtypes + versioned records; DTO SHAPE only). The NDJSON sink / hash-chain / redaction MECHANISM is owned by arc-01 (core) and the event envelope/dispatch by arc-25 (events) — disjoint by file. Must land immediately after arc-01; downstream lang/validator/scan/coordination/proof/events crates cannot begin real work until these types exist.
