# Enforcer Rule Index

Read this file before opening detailed rule docs. This index is the routing layer
for the future `ocentra-enforcer` repo: load only the docs needed for the target
language, files, and task.

## Decision Tree

1. Identify the target root and scope.
   - File edit: route by exact touched files.
   - Crate/package work: route by crate/package and changed manifests.
   - Diff/PR work: route by changed files first, then escalate if needed.
   - Workspace readiness: route all Rust families.

2. Route by file kind.
   - `*.rs`: read Rust source, domain, imports/modules, and async/runtime docs.
   - `Cargo.toml`: read Rust toolchain/Cargo and dependency docs.
   - `Cargo.lock`, `deny.toml`: read Rust dependency docs.
   - `rust-toolchain.toml`, `clippy.toml`, `rustfmt.toml`: read Rust toolchain/Cargo docs.
   - Unknown files: do not load detailed rules unless a failing rule ID exists.

3. Route by explicit failure.
   - If a validator returns `RR-*`, open only the doc listed for that rule in
     `rules/rules.json`.
   - Use the old full `docs/RustRules.md` only as a legacy fallback for broad
     policy review or missing registry entries.

## MCP First

Prefer `ocentra_enforcer_route` before reading rule details:

```json
{
  "name": "ocentra_enforcer_route",
  "arguments": {
    "root": "C:/path/to/project",
    "profile": "strict",
    "scope": "files",
    "files": ["src/lib.rs"]
  }
}
```

Then read only the returned `docs`.

The legacy `rust_rules_route` tool name remains a temporary alias for one
Rust-pack compatibility release.

## Current Language Modules

Rust is implemented now. TypeScript and Python are reserved for the future
`ocentra-enforcer` repo but intentionally not implemented in this slice.

```text
rules/rust/        implemented
rules/typescript/  reserved, not implemented
rules/python/      reserved, not implemented
```

## Schema Contract

`schemas/effect/enforcer-schemas.mjs` is the runtime contract layer for configs,
profiles, registry, route requests, scan reports, violations, init requests, and
MCP payloads. `rules/rules.json` is decoded through that layer, and
`schemas/json/*.schema.json` provides JSON-schema-compatible artifacts for docs,
tests, MCP clients, and non-Effect consumers.

Tests enforce that every scanner rule has one registry entry, every registry
entry points to a real doc, and no non-Rust language entries exist in this
implementation slice.
