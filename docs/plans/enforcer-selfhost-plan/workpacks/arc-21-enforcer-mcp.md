# arc-21 Crate enforcer-mcp

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Crate enforcer-mcp`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `crates/enforcer-mcp/**`
- deps: `arc-01`, `arc-02`, `arc-04`, `arc-15`, `arc-16`, `arc-17`, `arc-18`
- tier: `P1`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md), [RUST_ARCHITECTURE](../RUST_ARCHITECTURE.md).

## Where We Are
The MCP stdio server was a large Node tree (transport frames/messages, input schemas, tool registry per family, route/fallback/fingerprint, runner/dispatch, compact output). That engine is dropped; there is no Rust stdio server, and nothing measures or ratchets the tool-description surface the MCP exposes to agents.

## Where We Want To Be
`enforcer-mcp` is the Rust stdio MCP server per RUST_ARCHITECTURE.md and a FIRST-CLASS product surface (the harness-native, install-once, zero-per-repo-config agent UX — the primary agent experience; NOT secondary to the CLI). It stands up the crate SKELETON that exposes the consolidated tool surface (scan/check/proof/coordination/diagnostics) via the router, speaks MCP over stdio (JSON-RPC framing, camelCase wire casing), and delegates to `enforcer-scan`/`enforcer-proof`/`enforcer-coordination`/`enforcer-harness`. No business logic lives here — it is transport + tool registry + routing, with typed I/O via `enforcer-domain`. The `print_stdout`/`print_stderr` deny-wall lints are allowed in exactly ONE stdio-protocol sink module (the frame writer that owns the JSON-RPC channel), never elsewhere. It hosts the d05 context-budget tool-surface measure.

## Requirement Checklist
- [ ] Stand up the `enforcer-mcp` crate skeleton per RUST_ARCHITECTURE.md: transport (framing + JSON-RPC messages), the consolidated tool registry, and the router that dispatches tools to engine crates.
- [ ] Confine all stdout/stderr writes to ONE stdio-protocol sink module (the frame writer) carrying a scoped, documented `#![allow(clippy::print_stdout, clippy::print_stderr)]` at module scope; every other module obeys the `[workspace.lints]` deny wall. This is the ONLY sanctioned print site in the crate.
- [ ] Consolidate the tool surface via the router (as the compact/consolidated MCP output does), delegating to arc-15/16/17/18; keep typed I/O via `enforcer-domain` branded newtypes with camelCase wire casing.
- [ ] Provide the d05 context-budget tool-surface measure seam: expose the registered-tool enumeration + total description byte/token count so the d05 ratchet (which owns its baseline + `enforcer-core` meter) can measure the enforcer's own MCP surface against a committed baseline; this pack owns the measurable surface, d05 owns the baseline/ratchet files.
- [ ] `cargo test -p enforcer-mcp` passes with fail/pass fixtures: a canned MCP request over the transport yields the expected tool result (pass fixture), and a malformed request is rejected with a proper error frame (fail fixture); tool-registry schema round-trips; the tool-surface enumeration is deterministic (for d05).
- [ ] Clean `cargo clippy` / `cargo fmt --check` (deny wall honored everywhere except the single sink module; no `pub use` barrels).

## Acceptance And Proof
Tier P1. Proof row asserts `cargo test -p enforcer-mcp` exits 0 — transport + tool dispatch proven with fail/pass request fixtures and a deterministic tool-surface enumeration. Record the artifact path in TEST_PROOF_EXPECTATIONS.md.

## Parallel Ownership Notes
Owns the `enforcer-mcp` crate SKELETON: `crates/enforcer-mcp/Cargo.toml`, `src/lib.rs`, the transport + tool-registry + router modules, and the single stdio-protocol sink module (the sanctioned print site). Deps the engine frontier (arc-15/16/17/18) + foundation/rules.

Parallel Ownership Note (disjoint feature packs): d05 context-budget owns its own ratchet/baseline files (`enforcer-mcp` surface measure consumed via the seam above, its baseline + `enforcer-core` meter owned by d05) — NOT this crate; d05 `deps:` arc-21 and is sequenced after the skeleton. owns stay DISJOINT BY FILE. Parallel-safe with arc-22 (cli) at the transport/registry boundary — both are FIRST-CLASS surfaces over the same engine crates, neither secondary. Precedes/pairs with arc-22 which serves this on stdio.
