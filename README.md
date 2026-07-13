# Ocentra Enforcer

<!-- ai-dense -->
```yaml
product: native Rust repository-enforcement engine
entrypoints: "enforcer CLI | enforcer serve (MCP stdio) | optional Tauri UI"
core_model: "typed rules + deterministic validators -> structured findings and proof"
scopes: "check/scan/verify: paths | git diff (--base/--head) | --all; verify also selects a named mode"
workflow: "route or choose the smallest scope -> repair findings -> widen validation"
implementation: "Rust workspace; TypeScript is presentation-only in the optional UI"
```
<!-- /ai-dense -->

Ocentra Enforcer is a native Rust enforcement engine for local development,
CI, and MCP-enabled coding assistants. It makes repository policy executable:
the same rule system can route work, scan a scope, retain compact diagnostics,
record proof, and coordinate parallel contributors.

The goal is straightforward: code should be accepted because deterministic
checks support it, not because a person or agent remembered every convention.

## What it provides

- A Rust CLI and MCP stdio server from one codebase.
- Typed rule records with documented IDs, validators, fixtures, and findings.
- File, crate, workspace, and diff-oriented validation scopes.
- Native Rust validation plus structural validation for TypeScript/JavaScript,
  Python, Dart, CFML, configuration, CI, dependencies, and generated files.
- Compact, durable run diagnostics and proof artifacts.
- Optional coordination for exact-file claims, handoffs, and parallel work.
- An optional Tauri control plane; the CLI and MCP surfaces remain standalone.

## Architecture

```mermaid
flowchart LR
  A["Developer, CI, or MCP client"] --> B["CLI / MCP boundary"]
  B --> C["Routing and scope"]
  C --> D["Typed rule registry"]
  D --> E["Native validators and tool adapters"]
  E --> F["Structured findings and proof"]
  F --> G["Accept, repair, or escalate"]
```

The Rust workspace separates the concerns deliberately:

| Area | Responsibility |
| --- | --- |
| `enforcer-cli` | Command-line contract and human-readable output. |
| `enforcer-mcp` | Stdio MCP boundary and structured tool responses. |
| `enforcer-rules` / `enforcer-validator` | Rule registry, policy evaluation, and validation. |
| Language crates | Syntax-aware Rust, TypeScript/JavaScript, Python, Dart, CFML, infrastructure, and security analysis. |
| `enforcer-proof` / `enforcer-harness` | Durable run records, artifacts, and tool-result ingestion. |
| `enforcer-coordination` | Optional claims, guard decisions, mail, and ledger state. |
| `enforcer-ui` | Optional desktop control plane. |

See [the architecture guide](docs/ARCHITECTURE.md) for boundaries and runtime
flow.

## Build from source

Requirements:

- Rust stable with Cargo
- Node.js only when building or working on the optional Tauri frontend

```powershell
cargo check --workspace
cargo test --workspace
cargo build --release -p enforcer-cli
```

During development, invoke the CLI through Cargo:

```powershell
cargo run -p enforcer-cli -- --help
cargo run -p enforcer-cli -- check --help
cargo run -p enforcer-cli -- serve --help
```

The current CLI surface is `check`, `scan`, `serve`, `ui`, `verify`, `advise`,
`architecture`, and `onboard`. `check`, `scan`, and `verify` accept one
explicit scope: paths, a `--base`/`--head` diff pair, or `--all`; `verify`
also selects a named verification mode. Use `--help` as the source of truth
for the build you are running.

## Typical workflow

1. For MCP work, route the request through the installed Enforcer MCP tool;
   for CLI work, choose an explicit scope with `check` or `scan`.
2. Run the smallest meaningful validation scope.
3. Read compact diagnostics and repair the reported condition.
4. Record or inspect proof for work that requires reproducible evidence.
5. Run the wider local or CI gate before accepting the change.

For repositories wired to Enforcer, start with a route before reading broad
rule documentation. This keeps automated work scoped to the code and policy
that actually apply.

## Documentation

- [Architecture](docs/ARCHITECTURE.md)
- [Enforced checks](docs/ENFORCED_CHECKS.md)
- [Target repository wiring](docs/TARGET_REPO_WIRING.md)
- [MCP and harness setup](docs/CODEX_SETUP.md)
- [Coordination model](docs/COORDINATION.md)
- [Proof system](docs/PROOF_SYSTEM_DESIGN.md)
- [Release policy](docs/RELEASE_POLICY.md)

Internal planning material is intentionally kept under `docs/plans/`; it is
not the product contract. The current CLI, tests, and validators define the
supported runtime behavior.

## Contribution and verification

Use the smallest scoped Enforcer check while editing, then run the appropriate
workspace gate before declaring work complete. Do not weaken a policy, suppress
a test, or add a broad waiver simply to produce a green result.

```powershell
cargo fmt --check
cargo clippy --workspace -- -D warnings
cargo test --workspace
```

See [CONTRIBUTING.md](CONTRIBUTING.md) and [SECURITY.md](SECURITY.md) for
contribution and security-reporting expectations.
