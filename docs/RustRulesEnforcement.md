# Ocentra Enforcer Rust Integration Guide

Ocentra Enforcer is the standalone enforcement repo. This slice implements the
Rust module only and reserves future `rules/typescript`, `rules/python`, and
matching validators for later releases.

## Consumption Model

For new installs, read [../INSTALL.md](../INSTALL.md) first. For Codex MCP
setup, read [CODEX_SETUP.md](CODEX_SETUP.md). For target repo wiring, read
[TARGET_REPO_WIRING.md](TARGET_REPO_WIRING.md).

Default use is package plus Codex plugin/MCP:

```bash
ocentra-enforcer init --root C:/path/to/repo --profile strict --adapters codex,mcp,precommit,github-actions --dry-run
ocentra-enforcer scan --root C:/path/to/repo --files crates/example/src/lib.rs
ocentra-enforcer cargo --root C:/path/to/repo --crate example-crate
```

Git submodules are optional for source pinning only. MCP should run from the
enforcer install path, and each tool call must pass the target repo as `root`.

## Indexed Rule Routing

Agents should read `rules/INDEX.md` first, then call:

```text
ocentra_enforcer_route
```

Use the returned `docs` list instead of loading `docs/RustRules.md` by default.
The monolithic Rust rulebook remains a fallback for broad policy review, missing
registry entries, or unknown failures.

Legacy MCP aliases remain for one Rust-pack compatibility release:

```text
rust_rules_route
rust_rules_scan
rust_rules_doctor
rust_rules_explain
```

## Contract Layer

Effect Schema is the runtime contract source:

```text
schemas/effect/enforcer-schemas.mjs
```

It decodes configs, profiles, registry data, route requests, scan reports,
violations, init requests, and MCP tool payloads. JSON-schema-compatible
artifacts live under `schemas/json/` for docs, tests, MCP clients, and
non-Effect consumers.

## Adapter Layer

`ocentra-enforcer init --dry-run` reports the exact file plan before writing.
Use [BOOTSTRAP_PROMPT.md](BOOTSTRAP_PROMPT.md) when handing setup to a fresh
Codex thread.

Supported adapters in this slice:

- `codex`: project-local Codex skill wiring.
- `mcp`: project MCP server config pointing to the enforcer install path.
- `precommit`: plain Git pre-commit hook, the default cross-platform hook.
- `husky`: opt-in or auto-added only when the target already uses Husky.
- `lefthook`: opt-in Lefthook config.
- `github-actions`: expands to enforcer, CodeQL, dependency policy, secret scan, and SBOM workflows.
- `codeql`, `dependency-policy`, `secret-scan`, `sbom`: individual workflow adapters.

## Ocentra Parent Migration Boundary

Move generic guards into Ocentra Enforcer over time:

- re-export bans;
- architecture/source-shape guards;
- validation bypass checks;
- weak assertions and skipped-test checks;
- generated artifact checks;
- Rust dependency/domain/string guards;
- secret/dependency/SBOM gates.

Keep product-specific dev server, release packaging, and product proof semantics
in Ocentra Parent. Move generic ledger, hub, lane, mail, exact-file-claim, and
architecture enforcement into Ocentra Enforcer behind profile-backed CLI/MCP
interfaces. Parent wrappers should stay thin until old-vs-new parity is proven
for file, crate, diff, workspace, and coordination scopes.
