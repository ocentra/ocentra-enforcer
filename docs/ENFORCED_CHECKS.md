# Enforced Checks Catalog

<!-- ai-dense -->
```yaml
scope: high-level catalog of what the enforcer checks today; not the full rule corpus
implementation: every row below is a native Rust Validator (enforcer-lang-{rust,ts,py,common,security,iac,k8s}), never a TS/Node scanner
routing: call mcp__enforcer__route / `enforcer route` before reading detailed rule docs -- the router returns only matching rule records
languages_validated_not_implemented_in: typescript, javascript, python, dart, cfml -- the enforcer inspects these in a TARGET repo; the enforcer's own implementation is Rust end to end
```
<!-- /ai-dense -->

This is a high-level catalog of what the enforcer checks today. It is not the
full rule corpus. Agents should still route through `enforcer route` / MCP
`mcp__enforcer__route` before reading detailed rule docs — every check below
is backed by a native Rust `Validator`, not a TypeScript/Node scanner.

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
| Test organization | Rust crates use organized external tests under `tests/` by default. `inlineTestPolicy` makes source-level `#[cfg(test)]` modules explicit: `forbid` is the default blocking policy, `warn` reports an advisory, and `allow` is an intentional exception. External `tests/` files are exempt. |

## TypeScript And JavaScript

| Area | What Fails |
| --- | --- |
| Runtime schema authority | Zod source usage where a branded/typed contract layer is the configured authority. |
| Naked domain strings | `type FooId = string`, raw branded intersections, and manual string identity aliases where branded types and decode helpers should own the boundary. |
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
| Required tests | Source workspaces without organized tests, empty `.gitkeep`-only test trees in strict mode, and inline tests in production source. Rust's `inlineTestPolicy` is explicit: `forbid` (default, error), `warn` (advisory), or `allow`. External `tests/` files are never inline-test findings. |
| Single-source contracts | Copied values that should be imported, generated, or derived from the owner contract. |
| Portability | Unguarded platform-specific script commands. |
| Package determinism | Missing `package-lock.json`, missing exact `packageManager`, unbounded Node engines, and loose npm dependency ranges such as `^`, `~`, `*`, `latest`, `git:`, or `file:`. |
| Documentation | Public API docs and comment rules are warnings by default; profiles can promote them to hard failures. |
| Agent-rule hygiene | Oversized or unindexed agent/rule docs that would force broad context loading. |

## Dart And CFML

| Area | What Fails |
| --- | --- |
| Dart | tree-sitter-backed structural checks (`enforcer-lang-dart`): the same suppression/bypass, test-integrity, and source-shape families as TS/Python, adapted to Dart syntax. |
| CFML | CFLint-backed checks (`enforcer-lang-cfml`) ingested through the harness, plus native structural checks where CFLint coverage is incomplete. |

## Detect-And-Route Router (f05)

Before any validator runs, the router mechanically detects each touched
file's language/structure and dispatches it to the matching validator
family and native tool — an agent or CI job never has to name the language
by hand. `enforcer scan` (no `--languages` flag) uses the router by default;
passing `--languages` explicitly narrows the router's dispatch rather than
replacing it.

## Scan Modes (f01)

Scans run in one of five typed modes, chosen for the right cost/coverage
tradeoff:

| Mode | When |
| --- | --- |
| `quick` | Fast pre-save/pre-commit pass over a small file set. |
| `full` | Full workspace sweep, e.g. before a release or a PR-ready claim. |
| `scoped` | An explicit `--files`/`--crate` scope. |
| `diff` | `--base <ref> --head <ref>` changed-lines/changed-files scope. |
| `plan-scan` | Scoped to a plan/workpack's declared `owns` surface. |

## Scope Modes

Checks can run against:

- Exact files: `--files <path...>`
- Crate/package roots: `--crate <name>` or package-scoped checks
- Diff scopes: `--base <ref> --head <ref>`
- Full workspace: `--workspace`

Prefer the smallest scope that covers the change. Use full workspace checks for
PR-ready or release gates, not for every edit loop.

## UI Layer (Track G, Optional)

An optional Tauri desktop control-plane app (Rust backend, TS/web frontend)
gives humans a cockpit over the same enforcer any harness drives via MCP —
never required; the CLI/MCP surface is fully functional standalone. It
includes a rules-and-skills explorer (g08 — every rule/skill rendered with
meaning, fail/pass examples, tier, and framework mapping; this is where
`.md` prose lives for human browsing, while the AI still reads only the
structured rule record), a live lane/hub coordination panel (g06), and a
scan report with per-violation fix/ignore/later/waiver actions (g02/g03).

## Multi-Harness Install (Track C)

The enforcer installs into any of 11 AI-harness adapters — Claude Code,
Codex, Cursor, Windsurf, Gemini, Antigravity, OpenCode, Aider, KiloCode,
Kiro, and any generic `.mcp.json`-based harness — at user/global scope by
default, so a single per-machine install covers every repo. See
[SKILL_MCP_SYSTEM.md](SKILL_MCP_SYSTEM.md) and
[CODEX_SETUP.md](CODEX_SETUP.md) (Codex shown there as the worked example,
not the reference target).

## Onboarding + Autoindex (f02)

A one-time, agent-first onboarding loop (install -> inspect the target's real
build system -> configure a fitting `enforcer-config` -> wire CI -> verify
the wiring actually fires) scaffolds a project's `.enforce/` working
directory the first time the enforcer touches an unfamiliar repo. See
[skills/enforcer-onboarding/SKILL.md](../skills/enforcer-onboarding/SKILL.md).

## Silent Vs Human Mode (f04)

Every scan/check carries a `RunContext`: `AgentInline` (silent, terse,
machine-consumed diagnostics) or `HumanReview` (verbose, human-readable
report). The mode changes report *shape* only — never what is enforced.
