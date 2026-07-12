# Architecture

Ocentra Enforcer is a Rust workspace that exposes one enforcement model through
the CLI, MCP, automation, and an optional desktop UI. The product boundary is
the typed rule and finding model: callers request work, validators evaluate a
defined scope, and the result is returned as structured findings with stable
rule identifiers.

## Runtime flow

```mermaid
sequenceDiagram
  participant Client as Developer, CI, or MCP client
  participant Boundary as CLI or MCP boundary
  participant Router as Scope and routing
  participant Engine as Rules and validators
  participant Journal as Diagnostics and proof

  Client->>Boundary: request a command and scope
  Boundary->>Router: resolve policy and target
  Router->>Engine: run applicable validators
  Engine->>Journal: retain compact results and artifacts
  Journal-->>Boundary: structured report
  Boundary-->>Client: verdict and actionable findings
```

## Core boundaries

| Boundary | Purpose |
| --- | --- |
| CLI | Stable commands, output modes, and exit codes for people and CI. |
| MCP | Stdio tools that return compact structured results for assistants. |
| Configuration | Resolves embedded defaults and repository-level policy into an effective configuration. |
| Rules and validators | Own rule metadata, applicability, findings, fixtures, and deterministic decisions. |
| Language analysis | Keeps language-specific parsing and validation behind explicit crates. |
| Harness and proof | Retains command diagnostics and reproducible evidence without making raw terminal output the primary interface. |
| Coordination | Provides optional, external-to-product-repos state for claims and handoffs. |
| UI | Presents the same Rust-owned state and actions; presentation code does not own enforcement logic. |

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
- **MCP:** structured routing, checks, diagnostics, proof, and coordination
  operations over stdio.
- **UI:** optional human control plane for inspecting state and invoking
  Rust-owned actions.

Consult `enforcer --help` and the command-specific help for the exact command
contract available in the current build.
