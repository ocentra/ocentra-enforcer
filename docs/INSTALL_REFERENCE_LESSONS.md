# Install Reference Lessons

This note captures setup lessons from `DeusData/codebase-memory-mcp` and the
local Codex integration postmortem. It is about installation reliability, not
about graph features.

## Lessons Adopted

- Verify the executable/server before claiming Codex integration works.
- Treat Codex global MCP registration as a separate verification target from
  target repo wiring.
- Write app-level config idempotently and create a backup before changing it.
- Use absolute paths and forward slashes in Codex TOML/JSON on Windows.
- Avoid hand-typed PowerShell JSON for critical setup paths; prefer CLI commands
  that build structured payloads internally.
- Answer optional MCP client probes such as `resources/list`,
  `resources/templates/list`, and `prompts/list` with empty results when the
  server has no such resources.
- Support both Content-Length MCP frames and newline-delimited JSON frames so
  client transport differences do not require a local shim.
- Keep UI or long-running helper services separate from the MCP server. Enforcer
  currently has no UI service.

## Current Enforcer Commands

```bash
ocentra-enforcer codex install --root <repo> --profile <profile> --dry-run
ocentra-enforcer codex install --root <repo> --profile <profile>
ocentra-enforcer codex doctor --root <repo>
npm run mcp:smoke
npm run mcp:smoke:ndjson
```

`codex install` writes target Codex/MCP files and the global Codex MCP config.
`codex doctor` verifies Node, package dependencies, the MCP server path, Codex
global config, and target repo helper files.

## Still Needed Before Public Packaging

- Root `install.ps1` and `install.sh` one-line installers.
- Published package metadata and release checksums.
- `update` and `uninstall` commands for generated Codex/target wiring.
- Optional multi-agent adapters beyond Codex, MCP JSON, hooks, and CI.
- A signed release process when binary/native helpers are introduced.
