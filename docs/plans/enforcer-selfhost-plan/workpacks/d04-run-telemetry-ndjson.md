# d04 Run Telemetry NDJSON

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Run Telemetry NDJSON`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `crates/enforcer-core/src/telemetry.rs, crates/enforcer-core/src/ndjson.rs, crates/enforcer-domain/src/run_record.rs, crates/enforcer-core/tests/telemetry.rs, crates/enforcer-core/tests/fixtures/telemetry/**`
- deps: `arc-01`, `arc-02`, `d01-rule-mechanization-engine`
- tier: `P1 unit`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [RUST_ARCHITECTURE](../RUST_ARCHITECTURE.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md).

## Where We Are
The enforcer produces findings but keeps no machine-readable record of each run. The proof harness (arc-17 `enforcer-proof`) stores rich envelopes but there is no per-run telemetry line usable for trend analysis (d05 budget, d10 failure modes). ADBP's "measure everything" is prose. arc-01 stands up the `enforcer-core` skeleton and arc-02 the `enforcer-domain` schema crate; this pack folds in the OcentraParent "logging = structured data" borrow: a versioned `serde` telemetry record in `enforcer-domain` + a generic append-only `NdjsonWriter<T>` and hash-chain primitive in `enforcer-core` (no separate `enforcer-log` crate — that would duplicate `enforcer-proof`). It owns those specific files, not the whole core/domain crates.

## Where We Want To Be
Every enforcer run appends exactly one NDJSON line — a versioned `RunRecord` (`schema_version` + `eventType`, reusing branded newtypes: `RuleId` count, `Severity`, `Sha256`) — via the generic `enforcer-core::NdjsonWriter<T>`, with two-layer redaction (key-based field names + value-pattern secret regexes, both always run) applied before write. The record is `serde`-validated at the decode boundary; a stored-decode re-verifies the contract.

## Requirement Checklist
- [ ] Define the versioned `RunRecord` `serde` struct in `crates/enforcer-domain/src/run_record.rs` (`schema_version`, `eventType`, timestamp, command, ruleIds-in-scope count, findings by `Severity`, duration ms, exit status) — camelCase wire casing; branded newtypes, never bare `String` for ids.
- [ ] Implement the generic append-only `NdjsonWriter<T: Serialize>` + a pure hash-chain primitive in `crates/enforcer-core/src/ndjson.rs`; the run-telemetry sink in `src/telemetry.rs` writes one `RunRecord` line per run to a stable path (e.g. `proof/telemetry/runs.ndjson`).
- [ ] Two-layer redaction (from the `enforcer-core` redaction util) runs on every record before write; the record is parse-at-boundary validated — an invalid shape fails the run (fail-closed serialize/deserialize round-trip).
- [ ] Append is atomic and newline-terminated; a crashed run does not write a half line (write-then-flush a whole line).
- [ ] Telemetry emission never changes exit code or findings (observer, not gate); obey `[workspace.lints]` — `print_*`/`unwrap`/`expect`/`panic` banned, no `pub use` barrels.

## Acceptance And Proof
Tier T1 (P1 unit). Prove via `cargo test -p enforcer-core` (`crates/enforcer-core/tests/telemetry.rs` with `tests/fixtures/telemetry/**`): a scripted run appends exactly one valid NDJSON line; a forced schema violation (fail fixture) is rejected on decode; two runs append two independently-parseable lines and the hash-chain verifies on replay. Mechanism: serde-decode-then-append writer, asserted by re-parsing every emitted line. Rows in TEST_PROOF_EXPECTATIONS.md.

## Parallel Ownership Notes
Deps `arc-01` (owns the `enforcer-core` crate skeleton/`lib.rs`), `arc-02` (owns the `enforcer-domain` schema crate/`lib.rs`), and `d01-rule-mechanization-engine` (for `RuleId` enumeration in scope). Downstream d05 (context budget) and d10 (failure modes) consume this NDJSON. Owns only the named `telemetry.rs`/`ndjson.rs`/`run_record.rs` + `tests/fixtures/telemetry/**` files, disjoint by file from the arc-01/arc-02 skeletons and from d02/d03/d05 — coordinate the `mod`/`pub` lines appended to `enforcer-core/src/lib.rs` and `enforcer-domain/src/lib.rs` with the arc skeleton owners. Runs concurrently with d02/d03.
