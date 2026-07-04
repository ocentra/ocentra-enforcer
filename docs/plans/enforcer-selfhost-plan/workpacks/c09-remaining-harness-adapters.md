# c09 Remaining Harness Adapters

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Remaining Harness Adapters`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling completion, product status, PR readiness, or broad DONE.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `crates/enforcer-install/src/adapters/antigravity.rs, crates/enforcer-install/src/adapters/windsurf.rs, crates/enforcer-install/src/adapters/opencode.rs, crates/enforcer-install/src/adapters/aider.rs, crates/enforcer-install/src/adapters/kilocode.rs, crates/enforcer-install/src/adapters/kiro.rs, crates/enforcer-install/tests/fixtures/remaining/**`
- deps: `c01-install-core-and-cli-contract, c02-harness-autodetect, arc-23`
- tier: `P5 install-proof`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md), [RUST_ARCHITECTURE](../RUST_ARCHITECTURE.md).

## Where We Are
The Track C adapters cover Claude (c03), Codex (c06), generic (c07), and gemini/cursor/zed (c08). But codebase-memory supports six more harnesses the plan currently omits entirely: Antigravity, Windsurf, OpenCode, Aider, KiloCode, Kiro. There is no adapter module, no autodetect entry, and no doctor coverage for any of them, so `enforcer install` cannot self-register into those harnesses.

## Where We Want To Be
Six real adapter modules implementing the arc-23 `Adapter` trait (`plan`/`apply`/`verify`), each writing its NATIVE MCP registration format idempotently (registering the `enforcer` binary as the stdio MCP server), surfaced by c02 autodetect and verified by the c07 shared doctor. With c03+c06+c07+c08+this, all 11 harnesses are covered. Native surfaces:
- **Antigravity**: detect `~/.gemini/antigravity-cli`; upsert MCP entry into `~/.gemini/config/mcp_config.json` (`serde_json`).
- **Windsurf**: upsert `mcpServers` JSON (`~/.codeium/windsurf/mcp_config.json`).
- **KiloCode**: VS Code `globalStorage/kilocode.kilo-code` MCP settings JSON.
- **Kiro**: `~/.kiro` MCP config.
- **OpenCode / Aider**: CLI-shim / CLI detected; if no MCP config surface exists, detect+document (emit a T3-labeled `deferred: no mcp surface` verify `Check`, write nothing).

## Requirement Checklist
- [ ] Each adapter implements the `Adapter` trait (`plan`/`apply`/`verify`) and registers in the arc-23 registry (keyed by `AdapterId`) so c02 autodetect surfaces it.
- [ ] JSON-config harnesses (antigravity, windsurf, kilocode, kiro) upsert the `enforcer` server entry idempotently via `serde_json` value edits (second apply = no diff), preserving unrelated keys.
- [ ] CLI-only harnesses (opencode, aider) detect the binary via `enforcer-harness` (arc-18) and, absent an MCP surface, return a `Tier::T3` `deferred` verify `Check` writing zero files.
- [ ] Absent harness -> `verify` returns `Status::Skipped` with reason `not-detected` (honest, never silent).
- [ ] The c07 shared doctor re-reads disk and reports per-adapter pass/fail; modules obey `[workspace.lints]` (no `unwrap`/`expect`/`panic`/`print_*`, no `pub use` barrels).

## Acceptance And Proof
P5 install-proof (`remaining-harness-adapters` in TEST_PROOF_EXPECTATIONS.md), proved by `cargo test -p enforcer-install`. Fixtures live under `crates/enforcer-install/tests/fixtures/remaining/`. For each of the six adapters:
- **fail fixture**: harness present but server entry missing/renamed on disk -> `verify` reports the named failing `Check`.
- **pass fixture**: `apply` against a temp-home fixture yields the correct native config; re-reading matches the golden; a second `apply` is byte-identical (idempotent).
- **not-detected fixture**: no harness marker -> `Status::Skipped` (`not-detected`), zero writes.
- **detection test** (`remaining-adapters-detect`): autodetect enumerates all six `AdapterId`s and doctor aggregates their checks. The CLI-only pair additionally asserts the `Tier::T3` `deferred` label with a stated reason.

## Parallel Ownership Notes
Owns only the six new adapter modules under `crates/enforcer-install/src/adapters/` (+ `tests/fixtures/remaining/**`) — disjoint by file from c03/c06/c07/c08; the crate skeleton, trait, and registry belong to arc-23. Depends on c01 (interface/core), c02 (autodetect), and arc-23. Runs concurrently with all other Track C adapter packs. owns disjoint? = Y
