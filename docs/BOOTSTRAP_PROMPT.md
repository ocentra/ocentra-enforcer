# Bootstrap Prompt For Future Codex

Use this when asking a fresh Codex thread to install Ocentra Enforcer and wire a
target repo.

```text
You are setting up Ocentra Enforcer.

Source repo:
https://github.com/ocentra/ocentra-enforcer

Install location:
Windows preferred: E:\ocentra-enforcer
macOS/Linux preferred: ~/tools/ocentra-enforcer

Target repo:
<ABSOLUTE_TARGET_REPO_PATH>

Profile:
<strict OR ocentra-parent OR another named profile>

Tasks:
1. Clone https://github.com/ocentra/ocentra-enforcer to the install location if it does not exist.
2. Run npm install in the enforcer repo.
3. Run npm test, npm run rust:rules:scan, npm run rust:rules, and npm run mcp:smoke.
4. Run Codex install dry-run:
   node <ENFORCER_PATH>/scripts/ocentra-enforcer.mjs codex install --root <TARGET_REPO> --profile <PROFILE> --dry-run
5. If the plan is correct, run the non-dry-run installer:
   node <ENFORCER_PATH>/scripts/ocentra-enforcer.mjs codex install --root <TARGET_REPO> --profile <PROFILE>
6. Verify the global MCP registry and target wiring:
   node <ENFORCER_PATH>/scripts/ocentra-enforcer.mjs codex doctor --root <TARGET_REPO>
   codex mcp list
   If local CLI config parsing is blocked by unrelated config settings, use:
   codex -c service_tier='"fast"' mcp list
7. Restart Codex Desktop or start a new thread so the app reloads MCP servers.
8. Run:
   node <ENFORCER_PATH>/scripts/mcp-smoke.mjs --root <TARGET_REPO> --profile <PROFILE> --file Cargo.toml
   node <ENFORCER_PATH>/scripts/mcp-smoke.mjs --root <TARGET_REPO> --profile <PROFILE> --file Cargo.toml --framing ndjson
9. For hooks and CI, run target adapter dry-run:
   node <ENFORCER_PATH>/scripts/ocentra-enforcer.mjs init --root <TARGET_REPO> --profile <PROFILE> --adapters precommit,github-actions --dry-run
10. Do not write hook or CI files until the dry-run plan is reviewed.

Rules:
- The MCP server runs from the enforcer install path.
- The target repo is always passed as root.
- The installer updates Codex config directly and creates a backup before writing.
- `codex doctor` verifies global config separately from MCP server smoke.
- `mcp-smoke --framing ndjson` exists because some Codex MCP transports use newline JSON rather than Content-Length frames.
- Use profile for pack-owned policy.
- Use configPath for target-owned policy.
- Do not copy enforcer source into the target repo.
- Do not remove target repo's existing guards until old-vs-new parity is proven.
- Use E:/path style paths in TOML/JSON on Windows to avoid backslash escaping issues.
```

## MCP Verification Prompt

After setup, ask Codex:

```text
Use the ocentra-enforcer MCP server. Call ocentra_enforcer_route with:
root = <TARGET_REPO>
profile = <PROFILE>
scope = files
files = ["Cargo.toml"]

Report the returned docs, rule IDs, and whether the route avoided loading the full Rust rulebook.
```

Expected result:

- `ok: true`.
- `index: "rules/INDEX.md"`.
- `docs` contains only matching docs.
- `rules` contains compact rule metadata.
