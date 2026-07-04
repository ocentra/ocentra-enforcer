# Install Reference Lessons

<!-- ai-dense -->
```yaml
source: DeusData/codebase-memory-mcp setup pattern + local harness-integration postmortems
adopted: "verify server before claiming integration works; separate global-registration verification from target-repo wiring; idempotent config writes with backup; absolute forward-slash paths; empty-result answers to optional MCP probes; dual frame support (Content-Length + NDJSON)"
current_state: "install.sh/install.ps1 + release checksums + update/uninstall + all-11-harness adapters now LAND via c-track (c01-c10), not a future gap -- see docs/plans/enforcer-selfhost-plan/workpacks/c*.md"
```
<!-- /ai-dense -->

This note captures setup lessons from `DeusData/codebase-memory-mcp` and the
local harness-integration postmortem. It is about installation reliability,
not about graph features.

## Lessons Adopted

- Verify the executable/server before claiming harness integration works.
- Treat global MCP registration as a separate verification target from
  target repo wiring.
- Write app-level config idempotently and create a backup before changing
  it.
- Use absolute paths and forward slashes in TOML/JSON on Windows.
- Avoid hand-typed PowerShell JSON for critical setup paths; prefer CLI
  commands that build structured payloads internally.
- Answer optional MCP client probes such as `resources/list`,
  `resources/templates/list`, and `prompts/list` with empty results when the
  server has no such resources.
- Support both Content-Length MCP frames and newline-delimited JSON frames
  so client transport differences do not require a local shim.
- Keep UI or long-running helper services separate from the MCP server core
  (the optional Tauri UI, Track G, is a separate opt-in surface, not part of
  the MCP server process).

## Current Enforcer Commands

```bash
enforcer install --root <repo> --profile <profile> --dry-run
enforcer install --root <repo> --profile <profile>
enforcer doctor --root <repo>
```

`install` writes target harness/MCP files and the global harness MCP config
for every detected harness. `doctor` verifies the enforcer binary, the MCP
server path, harness global config, and target repo helper files.

## Status: Public Packaging Gaps (resolved by Track C)

The gaps this note originally tracked before public packaging are now owned
and landed by the multi-harness install track, not an open TODO here:

- Root `install.sh`/`install.ps1` one-line installers — c10.
- Published release metadata and per-platform checksums — c10.
- `update` and `uninstall` commands for generated harness/target wiring —
  `enforcer-install` crate (arc-23/c01).
- Adapters for all 11 harnesses (not just one) — c03/c06/c07/c08/c09.
- A signed release process for the native binary matrix — c10.

Consult those workpacks under
[docs/plans/enforcer-selfhost-plan/workpacks/](../docs/plans/enforcer-selfhost-plan/workpacks/)
for current status rather than treating this note as the open gap list.
