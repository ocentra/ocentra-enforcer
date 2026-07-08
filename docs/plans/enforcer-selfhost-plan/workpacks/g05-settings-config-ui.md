# g05 Settings Config UI

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Settings Config UI`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling completion, product status, PR readiness, or broad DONE.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `crates/enforcer-ui/src/settings/*`, `crates/enforcer-ui/src/bin/export_ts.rs`, `crates/enforcer-ui/tests/ts_drift.rs`
- deps: `arc-24, arc-03`
- tier: `P1/P5`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md).

## Where We Are
The `enforcer-config` crate (arc-03) is the single declarative control-plane — typed load with parse-at-boundary — and arc-24 (`enforcer-ui`) provides the served HTML shell + view-mount registry (Tauri backend, served-HTML fallback). But there is no config CONTROL surface: a human still hand-edits `enforcer-config` files to discover/enable projects, apply-by-language, pick the active profile, and toggle per-rule severity/waiver — the exact writes an AI agent must never make silently. The UI's TS frontend types are also still hand-drifted from the Rust `enforcer-domain` schema, with no derive/drift guard.

## Where We Want To Be
A per-project SETTINGS control-plane module `crates/enforcer-ui/src/settings/` mounted into the arc-24 shell for human-only config: auto-discover projects and enable-per-project; apply-by-language; per-rule toggle (on/off, severity, waiver); global-vs-project scope; pick the active profile. Reads and writes flow through the typed `enforcer-config` (arc-03) load/store API — never raw file edits — so nobody hunts TOML/JSON to flip switches. This pack ALSO owns the Rust->TS type-gen pipeline for the whole UI: a `#[derive(ts_rs::TS)]` export bin (`src/bin/export_ts.rs`) that emits the committed frontend `.ts` types from `enforcer-domain`, guarded by a fail-closed `cargo test` drift test (`tests/ts_drift.rs`) that byte-compares committed vs freshly-emitted. camelCase wire casing throughout.

## Requirement Checklist
- [ ] `crates/enforcer-ui/src/settings/` renders settings from live config state (discovered projects, per-project enable, apply-by-language, active profile, per-rule severity/waiver) read through the arc-03 `enforcer-config` typed load API; no hardcoded defaults in the view.
- [ ] Every write routes ONLY through the arc-03 `enforcer-config` store API (parse-at-boundary, typed); the settings module never touches config files directly on disk.
- [ ] Waiver authoring constructs an EXPLICIT gated waiver newtype (owner + reason + `RuleId`, all validated at the boundary) persisted via arc-03 — never a silent suppression or inline-disable.
- [ ] Toggle operations are idempotent: re-enabling an already-enabled rule/project re-serializes to byte-identical config (round-trip through the typed model, no duplicated entries).
- [ ] Type-gen: `src/bin/export_ts.rs` emits the committed frontend TS types from `enforcer-domain` via `ts_rs`; `tests/ts_drift.rs` FAILS CLOSED if committed `.ts` differs from freshly-emitted (byte-compare). camelCase casing asserted.
- [ ] Human surface only: loopback + token gated (via g07 guards); no popups during silent agent runs.

## Acceptance And Proof
T1 (`settings-config-writes`): fail-fixture — `cargo test -p enforcer-ui settings::` — a waiver save missing owner/reason/`RuleId` is rejected at the boundary (typed error) and writes nothing (temp-dir config unchanged). pass-fixture — toggling a rule severity writes the correct config once through the arc-03 store API (temp-dir fixture, serialized config matches golden). detection test — re-toggling ON twice round-trips to byte-identical config (idempotency assert). Type-gen T1 (`ts-rs-drift`): `cargo test -p enforcer-ui --test ts_drift` FAILS if committed TS drifts from `enforcer-domain`-emitted (fail-closed byte-compare); passes when in sync. Record artifact paths in TEST_PROOF_EXPECTATIONS.md.

## Parallel Ownership Notes
Owns only `crates/enforcer-ui/src/settings/*`, the type-gen bin `crates/enforcer-ui/src/bin/export_ts.rs`, and its drift test `crates/enforcer-ui/tests/ts_drift.rs`. Consumes the arc-24 shell and arc-03 `enforcer-config` typed API read/write; disjoint from g04/g06/g07 by file. Does NOT own the `enforcer-ui` crate skeleton (`Cargo.toml`/`lib.rs`/view registry — arc-24) nor the `enforcer-config` schema (arc-03) — it delegates all config load/store to the arc-03 owner rather than duplicating parse logic.
