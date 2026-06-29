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
4. Register the MCP server with Codex:
   codex mcp add ocentra-enforcer -- node <ENFORCER_PATH>/mcp/rust-rules-mcp.mjs
5. Verify:
   codex mcp get ocentra-enforcer
   codex mcp list
6. If codex mcp add is unavailable, edit ~/.codex/config.toml or %USERPROFILE%\.codex\config.toml manually with:
   [mcp_servers.ocentra-enforcer]
   command = "node"
   args = ["<ENFORCER_PATH>/mcp/rust-rules-mcp.mjs"]
   startup_timeout_sec = 20
   enabled = true
7. Run:
   node <ENFORCER_PATH>/scripts/mcp-smoke.mjs --root <TARGET_REPO> --profile <PROFILE> --file Cargo.toml
8. Run target dry-run:
   node <ENFORCER_PATH>/scripts/rust-rules.mjs init --root <TARGET_REPO> --profile <PROFILE> --adapters codex,mcp,precommit,github-actions --dry-run
9. Do not write target repo files until the dry-run plan is reviewed.
10. After approval, run the non-dry-run init or manually add only the requested target wiring.

Rules:
- The MCP server runs from the enforcer install path.
- The target repo is always passed as root.
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
