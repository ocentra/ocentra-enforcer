# x03 Rename Migration

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Rename Migration`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling completion, product status, PR readiness, or broad DONE.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `crates/enforcer-install/src/migrate_legacy_name.rs, crates/enforcer-install/tests/fixtures/migrate_legacy_name/**`
- deps: `arc-23`, `x01`
- tier: `P1`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md), [RUST_ARCHITECTURE](../RUST_ARCHITECTURE.md), [ADBP_GAPS](../ADBP_GAPS.md).

## Where We Are
x01 renamed the shipped name-source surfaces (workspace/crate `[package]` names + the binary/MCP server-name consts) from `ocentra-enforcer` to `enforcer`. But existing installs still carry a legacy `ocentra-enforcer` MCP server registration in harness configs (Claude `.mcp.json`, Codex TOML, and the other adapters' surfaces) plus legacy `rust_rules_*` / `ocentra_enforcer_*` tool-name usages. The arc-23 `enforcer-install` crate skeleton stands up the installer + adapters but registers `src/migrate_legacy_name.rs` as an empty module-root slot — nothing detects or rewrites those already-installed entries, so an upgraded machine would keep serving the old name. Doctrine forbids lingering `ocentra`: the fix is to UPGRADE existing installs, not to keep the old name alive.

## Where We Want To Be
A one-time transitional migration module `crates/enforcer-install/src/migrate_legacy_name.rs` (invoked by the installer's `doctor`/`migrate` path via arc-22 CLI) that: detects any existing `ocentra-enforcer` MCP registration across all supported harness config locations plus legacy `rust_rules_*` / `ocentra_enforcer_*` tool-name usages; rewrites the registration to `enforcer` (tools resolve as `mcp__enforcer__*`, reading the server-name const x01 owns); drops the deprecated aliases with a single one-time migration notice; and reports exactly what changed as a structured c01/arc-23 `Report`/`ApplyResult` record (typed `Finding`s, not println). It reuses the arc-23 config-read/JSON-and-TOML-merge + managed-block/backup helpers rather than reimplementing config parsing. This is transitional migration, NOT a permanent alias — after migration, zero `ocentra-enforcer` entries remain.

## Requirement Checklist
- [ ] Implement `migrate_legacy_name` as a Rust module in the `enforcer-install` crate (a function/type on the installer's `migrate`/`doctor` path), returning structured arc-23/c01 report records — not a standalone binary and not a `.ts`/Node script.
- [ ] Detect existing `ocentra-enforcer` MCP registration across all supported harness config locations (reuse the per-adapter config-locate logic from arc-23/c02, not a bespoke scanner).
- [ ] Detect legacy `rust_rules_*` and `ocentra_enforcer_*` tool-name usages in those configs.
- [ ] Rewrite the registration to `enforcer` (`mcp__enforcer__*`, reading the x01-owned server-name const) via `serde_json` value-merge (JSON harnesses) / TOML upsert (Codex), preserving unrelated keys; drop deprecated aliases with exactly one one-time notice.
- [ ] Report a typed diff of what changed (`Finding`s / `ApplyResult`); migration is idempotent (a second run is a no-op that changes zero files).
- [ ] No permanent alias retained — a post-migration re-scan finds zero `ocentra-enforcer` entries; back up each config with a timestamped copy before rewrite. Obey `[workspace.lints]` (no `unwrap`/`expect`/`panic`/`print_*`; fail-closed typed error on malformed config); no `pub use` barrels.

## Acceptance And Proof
Tier P1 (deterministic). 5-way parity is Rust-native: fail/pass fixtures under `crates/enforcer-install/tests/fixtures/migrate_legacy_name/**` driven by `cargo test -p enforcer-install`.
- **Fail-fixture** `migrate-legacy-config-present`: a harness config containing the old `ocentra-enforcer` server entry (and a `rust_rules_*` tool name) left unmigrated — a re-scan still finds the old entry = fail.
- **Pass-fixture** `migrate-legacy-config-rewritten`: after `migrate` runs on that fixture config, a re-scan finds zero `ocentra-enforcer` entries and the registration reads `enforcer` (`mcp__enforcer__*`).
- **Detection test** `rename-migration-contract`: `cargo test -p enforcer-install` asserts detection of the legacy entry + legacy tool names, the rewrite to `enforcer`, the single one-time notice, idempotent re-run (second run = zero file changes), a byte-for-byte-preserved timestamped backup, and the zero-lingering-`ocentra` post-scan; malformed config yields a typed error, not a silent skip.
- Clean `cargo clippy` / `cargo fmt --check`. Rows in TEST_PROOF_EXPECTATIONS.md.

## Parallel Ownership Notes
Owns only `crates/enforcer-install/src/migrate_legacy_name.rs` (+ `tests/fixtures/migrate_legacy_name/**`) — a SPECIFIC file under the arc-23 crate, NOT the whole crate. Depends on arc-23 (the `enforcer-install` skeleton + config-read/adapter helpers must exist) and x01 (the shipped rename, so the migration target name `enforcer` and the server-name const are stable). It does not edit x01's name-source surfaces or sibling install adapter files (c03..c09 own their own adapter files); it only reads/rewrites already-installed harness configs via the arc-23 helpers. Runs concurrently with the c-track adapter packs (disjoint by file), sequenced after the arc-23 skeleton. Transitional, not a permanent compatibility shim. owns disjoint? = Y.
