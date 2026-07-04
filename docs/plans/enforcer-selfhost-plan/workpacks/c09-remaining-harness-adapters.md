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

- owns: `src/install/adapters/{antigravity,windsurf,opencode,aider,kilocode,kiro}.*`
- deps: `c01-install-core-and-cli-contract, c02-harness-autodetect`
- tier: `P5 install-proof`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md).

## Where We Are
The Track C adapters cover Claude (c03), Codex (c06), generic (c07), and gemini/cursor/zed (c08). But codebase-memory supports six more harnesses the plan currently omits entirely: Antigravity, Windsurf, OpenCode, Aider, KiloCode, Kiro. There is no adapter, no autodetect entry, and no doctor coverage for any of them, so enforcer cannot self-register into those harnesses.

## Where We Want To Be
Six real adapters over the c01 interface (`plan/apply/verify`), each writing its NATIVE MCP registration format idempotently, surfaced by c02 autodetect and verified by the shared doctor. With c03+c06+c07+c08+this, all 11 harnesses are covered. Native surfaces:
- **Antigravity**: detect `~/.gemini/antigravity-cli`; write MCP entry into `~/.gemini/config/mcp_config.json`.
- **Windsurf**: upsert `mcpServers` JSON (`~/.codeium/windsurf/mcp_config.json`).
- **KiloCode**: VS Code `globalStorage/kilocode.kilo-code` MCP settings JSON.
- **Kiro**: `~/.kiro` MCP config.
- **OpenCode / Aider**: CLI-shim / CLI detected; if no MCP config surface exists, detect+document (emit a T3-labeled `deferred: no mcp surface` verify check, write nothing).

## Requirement Checklist
- [ ] Each adapter implements `plan/apply/verify` and registers in the c01 registry so c02 autodetect surfaces it.
- [ ] JSON-config harnesses (antigravity, windsurf, kilocode, kiro) upsert the `enforcer` server entry idempotently (second apply = no diff).
- [ ] CLI-only harnesses (opencode, aider) detect the binary and, absent an MCP surface, return a T3 `deferred` verify check writing zero files.
- [ ] Absent harness -> `verify` returns `skipped:not-detected` (honest, never silent).
- [ ] Shared doctor re-reads disk and reports per-adapter pass/fail.

## Acceptance And Proof
P5 install-proof (`remaining-harness-adapters` in TEST_PROOF_EXPECTATIONS.md). For each of the six adapters:
- **fail fixture**: harness present but server entry missing/renamed on disk -> `verify` reports the named failing check.
- **pass fixture**: apply against a temp-home fixture yields the correct native config; re-reading matches the golden; a second apply is byte-identical (idempotent).
- **not-detected fixture**: no harness marker -> `skipped:not-detected`, zero writes.
- **detection test** (`remaining-adapters-detect`): autodetect enumerates all six ids and doctor aggregates their checks. CLI-only pair additionally asserts the `deferred` T3 label with a stated reason.

## Parallel Ownership Notes
Owns only the six new adapter files under `src/install/adapters/` — disjoint from c03/c06/c07/c08. Depends on c01 (interface/core) and c02 (autodetect). Runs concurrently with all other Track C adapter packs.
