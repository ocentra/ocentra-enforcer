# Enforced Checks Catalog

This is a high-level catalog of what Enforcer checks today. It is not the full
rulebook. Agents should still route through `rules/INDEX.md` and MCP
`ocentra_enforcer_route` before reading detailed rule docs.

## Rust

| Area | What Fails |
| --- | --- |
| Toolchain and manifests | Missing/invalid `rust-toolchain.toml`, `Cargo.lock`, `clippy.toml`, `deny.toml`, or Rust manifest policy. |
| Unsafe and panics | `unsafe`, undocumented unsafe contracts, raw pointers in public signatures, `transmute`, `static mut`, `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, `unreachable`, `dbg`, `println`, and `eprintln`. |
| Error handling | `Result<T, String>`, `Result<T, &'static str>`, `Err("literal")`, `Err(format!(...))`, `map_err(...to_string())`, ignored fallible-looking results, `.ok()`, and `.unwrap_or_default()` on fallible domain/config data. |
| Domain typing | Naked `String`, `str`, `PathBuf`, raw primitive public signatures, `AsRef<str>`, `Into<String>`, `Vec<String>`, `HashMap<String, _>`, raw public fields, raw type aliases, weak tuple newtypes, boolean state clusters, and serialized public primitives where brands/newtypes should own the boundary. |
| Re-exports and imports | Wildcard imports and public Rust re-exports such as `pub use`, `pub(crate) use`, `pub(super) use`, and `pub(in ...) use` unless a profile explicitly allows facade-only behavior. |
| Allocation and copies | `clone`, `to_string`, `to_owned`, indexing/slicing, and casts without required justification policy. |
| Async/runtime | Untracked `tokio::spawn`, unbounded channels without `CHANNEL-JUSTIFICATION:`, blocking work, and async/runtime shape issues routed through Rust async-runtime rules. |
| Serde and tests | Direct non-boundary `Deserialize` derives, unjustified `#[serde(untagged)]`, weak `assert!(x.is_ok())`, and weak `assert!(x.is_some())`. |
| Dependencies | Wildcard versions, blocked dependency shapes, git/path dependency policy, cargo-audit, cargo-deny, license policy, and SBOM generation. |
| Test organization | Rust crates need organized tests under `tests/`; inline `#[cfg(test)] mod tests` and `#[test]` blocks in `src/` are hard failures. |

## TypeScript And JavaScript

| Area | What Fails |
| --- | --- |
| Runtime schema authority | Zod source usage where Effect Schema is the configured contract layer. |
| Naked domain strings | `type FooId = string`, raw branded intersections, and manual string identity aliases where Effect Schema brands and decode helpers should own the boundary. |
| Strict source slop | `any`, unsafe `as` casts, double assertions, non-null assertions, default exports, `process.env` outside config boundaries, `JSON.parse` outside decoder boundaries, console debugging, and thrown string errors. |
| Barrel exports and re-exports | `export *`, `export * as`, `export { X } from`, `export type { X } from`, and `export { default as X } from`. |
| Suppression and bypass | `eslint-disable`, `ts-ignore`, formatter bypasses, and validation-bypass comments unless a profile explicitly permits them. |
| Weak or hidden tests | Weak assertions, skipped/focused/todo tests, test-double vocabulary/packages, and inline `describe`/`it`/`test` blocks inside production `src/`. |
| Source shape | Oversized files, too many exports/classes/functions/types, long functions, and generated-output files treated as source unless configured as generated artifacts. |
| Import boundaries | Profile-backed forbidden imports and package/layer boundary violations. |

## Python

| Area | What Fails |
| --- | --- |
| Suppression and bypass | Broad `noqa`, `type: ignore`, linter disables, and validation-bypass comments. |
| Naked domain strings | Raw string aliases and unbranded identity values where a project profile requires schema-owned domain boundaries. |
| Strict source slop | `Any`, untyped defs, mutable default args, bare/broad exceptions, print debugging, `subprocess` with `shell=True`, wildcard imports, and `requests` calls without timeout. |
| Test integrity | Skipped/focused tests, weak assertions, broad test doubles, and inline `def test_*` or `class Test*` declarations inside production `src/`. |
| Toolchain diagnostics | Ruff JSON, Pyright JSON, mypy output, pytest text/JUnit, Bandit, and pip-audit can be ingested through the harness. |

## Common Checks

| Area | What Fails |
| --- | --- |
| Policy integrity | Immutable rule disables/downgrades, strict `failOn` bypasses, unsafe/build/git/path dependency escape hatches without waiver, and strict public re-export allow mode. |
| Rule registry integrity | Doc-only rule IDs, registry rows missing routed doc anchors, scanner-emitted unregistered IDs, duplicate IDs, and validator-backed rules missing required fixture evidence. |
| Waiver governance | Missing waiver metadata, broad scopes, expired waivers, AI-owned waivers, immutable-rule waivers without registry permission, and missing remediation plans. |
| CI and repo governance | CI must use least permissions, run on pull requests and main, avoid `continue-on-error`/`|| true`, use `npm ci`, cover Linux/macOS/Windows where required, and protect rule/validator/schema/workflow/package files with CODEOWNERS. |
| Secrets | Inline secret-like assignments and staged secret leaks. |
| Generated artifacts | Generated markers or tracked output/proof artifacts in source scope. |
| Test doubles | Mock/fake/stub/spy vocabulary and common packages by default. |
| Required tests | Source workspaces without organized tests, empty `.gitkeep`-only test trees in strict mode, and inline tests in production source. |
| Single-source contracts | Copied values that should be imported, generated, or derived from the owner contract. |
| Portability | Unguarded platform-specific script commands. |
| Package determinism | Missing `package-lock.json`, missing exact `packageManager`, unbounded Node engines, and loose npm dependency ranges such as `^`, `~`, `*`, `latest`, `git:`, or `file:`. |
| Documentation | Public API docs and comment rules are warnings by default; profiles can promote them to hard failures. |
| Agent-rule hygiene | Oversized or unindexed agent/rule docs that would force broad context loading. |

## Scope Modes

Checks can run against:

- Exact files: `--files <path...>`
- Crate/package roots: `--crate <name>` or package-scoped checks
- Diff scopes: `--base <ref> --head <ref>`
- Full workspace: `--workspace`

Prefer the smallest scope that covers the change. Use full workspace checks for
PR-ready or release gates, not for every edit loop.
