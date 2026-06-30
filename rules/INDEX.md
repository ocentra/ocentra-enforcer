# Enforcer Rule Index

Read this file before opening detailed rule docs. This index is the routing layer
for the future `ocentra-enforcer` repo: load only the docs needed for the target
language, files, and task.

## Decision Tree

1. Identify the target root and scope.
   - File edit: route by exact touched files.
   - Crate/package work: route by crate/package and changed manifests.
   - Diff/PR work: route by changed files first, then escalate if needed.
   - Workspace readiness: route all implemented families, but still prefer file or diff scope first.

2. Route by file kind.
   - `*.rs`: read Rust source, domain, imports/modules, async/runtime, common source, common security, and documentation advisory docs.
   - `Cargo.toml`: read Rust toolchain/Cargo and dependency docs.
   - `Cargo.lock`, `deny.toml`: read Rust dependency docs.
   - `rust-toolchain.toml`, `clippy.toml`, `rustfmt.toml`: read Rust toolchain/Cargo docs.
   - `*.ts`, `*.tsx`, `*.js`, `*.jsx`, `*.mjs`, `*.cjs`: read TypeScript source docs plus common source/security/generated-artifact/documentation advisory docs.
   - `scripts/**/*.mjs`: also read common portability docs.
   - `*.test.*`, `*.spec.*`, `tests/**`, `__tests__/**`: also read the TypeScript test docs and common test-double docs.
   - `package.json`: read TypeScript source/toolchain docs plus common security docs.
   - `tsconfig*.json`, `eslint.config.*`, `vitest.config.*`, `jest.config.*`, `playwright.config.*`: read TypeScript toolchain docs.
   - `*.py`: read Python source docs plus common source/security/generated-artifact/documentation advisory docs.
   - `test_*.py`, `*_test.py`, `tests/**`: also read Python test docs and common test-double docs.
   - `pyproject.toml`, `requirements*.txt`, `uv.lock`, `poetry.lock`, `pytest.ini`, `mypy.ini`, `ruff.toml`: read Python toolchain docs.
   - Unknown files: do not load detailed rules unless a failing rule ID exists.

3. Route by explicit failure.
   - If a validator returns `RR-*`, `TS-*`, `PY-*`, `SEC-*`, `GEN-*`, `TEST-*`, `PORT-*`, `SRC-*`, `CONTRACT-*`, `DEP-*`, `SBOM-*`, `AI-*`, `DOC-*`, or `HAR-*`,
     open only the doc listed for that rule in `rules/rules.json`.
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

Rust, TypeScript, Python, and common security/generated/test/portability/documentation/harness rules are
implemented as indexed families. TypeScript and Python start with practical hard
guards and harness ingestion, not a complete replacement for each ecosystem's
native tools.

```text
rules/rust/        Rust source, domain, Cargo, dependency, async/runtime rules
rules/typescript/  TS/JS source, tests, and toolchain routing
rules/python/      Python source, tests, and toolchain routing
rules/common/      source integrity, security, generated artifacts, test-double, portability, source-shape, contracts, dependency policy, SBOM, agent-rule index, documentation, and harness diagnostics
```

Documentation/comment rules are advisory by default. They appear as warnings
unless a target config upgrades them to `error` or includes `warning` in
`failOn`.

## Harness First

For compiler, lint, and test commands, prefer the harness wrapper so raw terminal
walls become compact diagnostics:

```bash
ocentra-enforcer run --root C:/path/to/project --tool cargo-check -- cargo check --message-format=json
ocentra-enforcer runs last-failure --root C:/path/to/project --json
```

Open raw artifacts only if `runs diagnostics` or `runs last-failure` does not
contain enough information.

## Schema Contract

`schemas/effect/enforcer-schemas.mjs` is the runtime contract layer for configs,
profiles, registry, route requests, scan/check reports, violations, init requests, and
MCP payloads. `rules/rules.json` is decoded through that layer, and
`schemas/json/*.schema.json` provides JSON-schema-compatible artifacts for docs,
tests, MCP clients, and non-Effect consumers.

Tests enforce that every scanner rule has one registry entry, every registry
entry points to a real doc, route output stays compact, and harness diagnostics
can be queried without opening raw logs.
