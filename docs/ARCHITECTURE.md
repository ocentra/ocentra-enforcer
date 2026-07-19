# Architecture

<!-- ai-dense -->
```yaml
engine: native Rust workspace
public_boundaries: "CLI | MCP stdio | optional desktop UI"
decision_owner: "typed rules and validators return structured findings"
target_languages: "Rust, TypeScript/JavaScript, Python, Dart, CFML, infrastructure, security"
ui_boundary: "Tauri presentation invokes Rust-owned state and actions; it does not make policy decisions"
runtime_truth: "enforcer --help and command-specific help for the binary under test"
```
<!-- /ai-dense -->

Ocentra Enforcer is a Rust workspace whose validation model is currently
exposed through the CLI, with narrower MCP and desktop control surfaces. The product boundary is
the typed rule and finding model: callers request work, validators evaluate a
defined scope, and the result is returned as structured findings with stable
rule identifiers.

## Runtime flow

```mermaid
sequenceDiagram
  participant Client as Developer or CI
  participant Boundary as CLI validation boundary
  participant Scope as Scope resolution
  participant Engine as Rules and validators
  participant Journal as Diagnostics and proof

  Client->>Boundary: request a command and scope
  Boundary->>Scope: resolve policy and target
  Scope->>Engine: run applicable validators
  Engine->>Journal: retain compact results and artifacts
  Journal-->>Boundary: structured report
  Boundary-->>Client: verdict and actionable findings
```

## Core boundaries

| Boundary | Purpose |
| --- | --- |
| CLI | Stable commands, output modes, and exit codes for people and CI. |
| MCP | Stdio boundary. The current Rust router wires status, coordination status and claim, and UI only. |
| Configuration | Resolves embedded defaults and repository-level policy into an effective configuration. |
| Rules and validators | Own rule metadata, applicability, findings, fixtures, and deterministic decisions. |
| Language analysis | Keeps language-specific parsing and validation behind explicit crates. |
| Harness and proof | Retains command diagnostics and reproducible evidence without making raw terminal output the primary interface. |
| Coordination | Provides optional, external-to-product-repos state for claims and handoffs. |
| UI | Presents Rust-owned state where wired; presentation code does not own enforcement logic. |

## Enforcement model

Enforcer treats documentation as an explanation layer, not a bypass path. A
rule that blocks work should have a stable identifier, a validator, fixtures,
and a route for finding the relevant guidance. A policy change must be proven
by the corresponding test and validation surface.

The supported implementation languages are deliberately separated from target
languages. Enforcer itself is Rust. The optional desktop frontend is a thin
Tauri presentation layer; language analyzers inspect source in the target
repository rather than moving engine decisions into frontend code.

## Operational modes

- **CLI:** local and CI use through commands such as `check`, `scan`, and
  `verify`.
- **MCP:** server status, coordination status, exact-path claim, and UI
  launch/status over stdio. Other registered tool contracts are not wired.
- **UI:** optional human control plane for inspecting state and invoking
  Rust-owned actions.

## Capability map

The command boundary supports focused repository checks and scans, named
verification modes, a stdio MCP server, onboarding, architecture checks, and
an optional desktop surface. The visible `plan`, `proof`, and `coordination`
CLI groups are reserved boundaries and currently return not-wired errors.

The workspace also separates human and unattended use. MCP and CI consumers
receive compact structured results suitable for automation, while the CLI and
desktop surface provide a human-readable control plane over the same Rust-owned
contracts. Coordination state, when accessed through a wired boundary, is
installation-level state rather than application state in a target repository.

The desktop currently supports project and settings persistence, scope and
scan-target management, proof inspection, graph/search, and basic coordination
status, messages, acknowledgements, and claims. Runs are read-only. Fix
dispatch, adapter repair and hook installation, assurance CI gating, canonical
Rust scan persistence, Rust-native analysis history, finding waivers, run
execution/maintenance, and incremental memory refresh are not implemented.
Analysis still uses the compatibility Node bridge.

Consult `enforcer --help` and the command-specific help for the exact command
contract available in the current build.
