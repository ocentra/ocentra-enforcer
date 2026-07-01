# Rust Integration Guide

Ocentra Enforcer is a standalone, multi-language enforcement platform. Rust is
one implemented module alongside TypeScript/JavaScript, Python, common
security/generated-artifact checks, proof harnessing, compact diagnostics, and
coordination. This document explains the Rust surface only; it is not the
system architecture document.

For the full system model, use:

- [../README.md](../README.md)
- [ENFORCED_CHECKS.md](ENFORCED_CHECKS.md)
- [COORDINATION.md](COORDINATION.md)
- [TARGET_REPO_WIRING.md](TARGET_REPO_WIRING.md)

## Consumption Model

Default use is package plus Codex plugin/MCP:

```bash
ocentra-enforcer init --root C:/path/to/repo --profile strict --adapters codex,mcp,precommit,github-actions --dry-run
ocentra-enforcer scan --root C:/path/to/repo --files crates/example/src/lib.rs
ocentra-enforcer cargo --root C:/path/to/repo --crate example-crate
```

Git submodules are optional for source pinning only. MCP should run from the
Enforcer install path, and each tool call must pass the target repo as `root`.

## Indexed Rule Routing

Agents must read `rules/INDEX.md` first, then call:

```text
ocentra_enforcer_route
```

Use the returned `docs` list instead of loading `docs/RustRules.md` by default.
The monolithic Rust rulebook remains a fallback for broad Rust policy review,
missing registry entries, or unknown failures.

Legacy MCP aliases remain for one compatibility release:

```text
rust_rules_route
rust_rules_scan
rust_rules_doctor
rust_rules_explain
```

## Rust Gate Shape

Rust enforcement currently covers:

- toolchain and manifest determinism;
- lint suppression and validator bypass comments;
- unsafe, panic, unwrap, expect, debug/console macros, and erased errors;
- raw strings, raw primitives, raw public fields, raw aliases, and weak tuple
  newtypes in domain surfaces;
- wildcard imports and public Rust re-exports;
- clone/allocation/cast/indexing justification policy;
- async/runtime shape checks;
- dependency, cargo-deny, cargo-audit, fmt, clippy, test, and doc gates.

Use `rules/rust/*.md` for routed remediation details and
[ENFORCED_CHECKS.md](ENFORCED_CHECKS.md) for the high-level catalog.

## Meta-Enforcement

Rust rules are protected by the shared policy layer:

```bash
ocentra-enforcer check rule-coverage --root <repo>
ocentra-enforcer check policy-integrity --root <repo>
ocentra-enforcer check waiver-policy --root <repo>
```

Immutable Rust rules cannot be disabled or downgraded by project config. Waivers
must be narrow, visible, expiring, and owner-backed where a rule permits them.

## Contract Layer

Effect Schema is the runtime contract source:

```text
schemas/effect/enforcer-schemas.mjs
```

It decodes configs, profiles, registry data, route requests, scan reports,
violations, init requests, and MCP tool payloads. JSON-schema-compatible
artifacts live under `schemas/json/` for docs, tests, MCP clients, and
non-Effect consumers.

## Target Repo Boundary

A target repo should keep product code, product-specific dev servers, release
packaging, and domain-specific proof meaning. Enforcer owns reusable guards,
coordination, compact diagnostics, proof running, policy integrity, and
architecture checks. Existing repo-local wrappers should stay thin until
old-vs-new parity is proven for file, crate/package, diff, workspace, hook, and
CI scopes.
