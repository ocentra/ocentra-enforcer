# Ocentra Enforcer

Standalone enforcement system for humans, CI, Codex skills, and MCP clients.
Rust is the only implemented language module in this slice. The repo reserves
structure for future TypeScript and Python modules without implementing them yet.

Default consumption is package plus Codex plugin/MCP. Git submodules are allowed
only for projects that need source pinning; they are not the default install
model.

## Commands

```bash
npm test
npm run test:rules
npm run test:mcp
npm run enforcer:init:dry-run
npm run enforcer:rules:scan
npm run enforcer:rules
```

Direct CLI forms:

```bash
node scripts/rust-rules.mjs init --root C:/path/to/repo --profile strict --adapters codex,mcp,precommit,github-actions --dry-run
node scripts/rust-rules.mjs scan --root C:/path/to/repo --files src/lib.rs
node scripts/rust-rules.mjs scan --root C:/path/to/repo --crate my-crate
node scripts/rust-rules.mjs scan --root C:/path/to/repo --workspace
node scripts/rust-rules.mjs scan --root C:/path/to/repo --base origin/main --head HEAD
node scripts/rust-rules.mjs cargo --root C:/path/to/repo --crate my-crate
node scripts/rust-rules.mjs doctor --root C:/path/to/repo --workspace
node scripts/rust-rules.mjs explain RR-7.3
```

After package install, the canonical entrypoints are:

```bash
ocentra-enforcer scan --root C:/path/to/project --files src/lib.rs
ocentra-enforcer-mcp
```

Compatibility aliases remain for one Rust-pack release:

```bash
rust-rules scan --root C:/path/to/project --files src/lib.rs
rust-rules-mcp
```

## Install / Init Model

Run init from the enforcer install path and pass the target repo explicitly:

```bash
ocentra-enforcer init --root C:/path/to/repo --profile strict --adapters codex,mcp,precommit,github-actions --dry-run
```

`--dry-run` prints the exact file plan without writing. The default hook adapter
is a plain Git hook for cross-platform use. Husky is generated only when
requested or when the target repo already uses Husky. Lefthook is opt-in.

MCP runs from the enforcer install path. Target projects always pass `root` plus
either `configPath` for project-specific policy or `profile` for a named pack
policy such as `strict` or `ocentra-parent`.

Example global MCP wiring:

```json
{
  "mcpServers": {
    "ocentra-enforcer": {
      "command": "node",
      "args": ["C:/path/to/ocentra-enforcer/mcp/rust-rules-mcp.mjs"]
    }
  }
}
```

Canonical MCP tools:

```text
ocentra_enforcer_route
ocentra_enforcer_scan
ocentra_enforcer_doctor
ocentra_enforcer_explain
```

Legacy `rust_rules_*` MCP tool aliases remain for one Rust-pack compatibility
release.

## Indexed Rules

Agents should read `rules/INDEX.md` first, call MCP `ocentra_enforcer_route`,
and then open only the returned docs. `docs/RustRules.md` remains the legacy
monolithic reference, not the default context load.

`rules/rules.json` is validated by Effect Schema at runtime and by tests.
JSON-schema-compatible artifacts live under `schemas/json/` for docs, MCP
clients, and non-Effect consumers.

## Main Files

- `scripts/rust-rules.mjs`: Rust CLI and scanner engine, exposed through the `ocentra-enforcer` and legacy `rust-rules` bins.
- `mcp/rust-rules-mcp.mjs`: MCP stdio server with canonical `ocentra_enforcer_*` tools and legacy `rust_rules_*` aliases.
- `schemas/effect/enforcer-schemas.mjs`: Effect Schema contract source for configs, profiles, registry, route requests, reports, violations, init, and MCP payloads.
- `schemas/json/*.schema.json`: JSON-schema-compatible contract artifacts.
- `rules/INDEX.md`: agent-facing routing index.
- `rules/rules.json`: machine-readable rule registry.
- `rules/rust/*.md`: small Rust rule-family docs for selective loading.
- `adapters/`: templates for Codex/MCP wiring, plain Git hooks, Husky, Lefthook, GitHub Actions, CodeQL, dependency policy, secret scan, and SBOM.
- `profiles/ocentra-parent.json`: migrated Ocentra Parent strict Rust profile.
- `rust-rules.config.json`: legacy strict default profile file, still supported.

## Migration Model

Generic guards should move into Ocentra Enforcer as reusable, profile-backed
checks. Ocentra Parent keeps ledger, hub, dev server, release packaging, and
product proof scripts in the main repo.

For current Ocentra Parent migration:

1. Add generic guard parity in Ocentra Enforcer.
2. Keep old repo guards as thin wrappers.
3. Prove file, crate, workspace, and diff scopes match or exceed old behavior.
4. Wire CI and hooks to the pack command.
5. Remove duplicated repo-local guards only after parity is proven.
