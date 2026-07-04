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

### Orphaned MCP mechanics adopted from the Node server (AUDIT_FINDINGS WAVE 4; no prior crate owner)
These four rows port live, provable behaviors from `mcp/rust-rules-mcp-*.mjs` that currently have NO Rust owner. Each carries an explicit fail/pass fixture intent so it cannot silently drop in the port.

- [ ] **Legacy `rust_rules_*` alias surface + defined deprecation window.** The Node registry (`rust-rules-mcp-tool-registry.mjs`) doubles EVERY canonical `ocentra_enforcer_*` tool with a `rust_rules_`-prefixed alias (`LEGACY_ALIAS_TOOLS`, description "Legacy alias for …; kept for one Rust-pack compatibility release"), and dispatch (`rust-rules-mcp-dispatch.mjs` `callTool` -> `normalizeToolName`, `rust-rules-mcp-fingerprint.mjs`) folds any `rust_rules_*` name back to `ocentra_enforcer_*` before handler lookup. The port MUST either (a) emit + accept the legacy alias set for exactly ONE compatibility release (a DEFINED deprecation window, matching the current description contract) then remove it, or (b) coordinate the removal with x03 (rename-migration) so the alias is retired in one place. Both alias and canonical names must appear in the tool-surface enumeration the d05 measure consumes, so the alias bloat is visible in the context-budget baseline. Fixture — pass: a `rust_rules_check` (aliased) call resolves to the same handler/result as `ocentra_enforcer_check`; the alias appears in `tools/list`. Fail (deprecation-close guard): once the window is declared closed, an alias call is rejected as Unknown tool and the alias is absent from `tools/list`.
- [ ] **Stale-server write-gate + `ocentra_enforcer_run` CLI fallback (fail-closed safety invariant).** Own the dispatch-boundary gate ported from `rust-rules-mcp-fallback.mjs` (`shouldBlockStaleMcpTool`, `mcpStaleError`, `buildStaleFallback`), `rust-rules-mcp-context.mjs` (`COORDINATION_WRITE_TOOLS` set: init/claim/closeout/release/report/message/sync/ensure/compact), and `WRITE_ACTIONS_BY_TOOL` (mail:send/ack, peer:add/remove/sync). When the running server's code fingerprint != disk OR the coordination hash-compat check fails (`directWritesAllowed === (!stale && hashCompatible)`; the hash-compat source-of-truth references **arc-16**), every coordination-WRITE tool is REFUSED and dispatch returns a STRUCTURED fallback: `{ ok:false, fallbackAvailable, reloadRequired:true, fallback:{ recommendedTool:"ocentra_enforcer_run", command, commandLine, enforcerRunArguments } }` pointing at the on-disk CLI run from the pack root. Read-only tools and `ocentra_enforcer_mcp_status` are NEVER gated. This is a live fail-closed invariant (hit in real smoke tests). Fixture — fail: a stale/hash-incompatible server refuses `ocentra_enforcer_coordination_claim` (a WRITE tool) and returns a well-formed `ocentra_enforcer_run` fallback naming the CLI command; pass: a fresh, hash-compatible server dispatches the same WRITE tool to its handler; and a read-only tool (e.g. `coordination_status`) is allowed even while stale.
- [ ] **coordination `repair` write/dry-run gating at the MCP boundary.** `repair` is a conditional write: the gate treats it as a write (and therefore stale-refuses per the row above) ONLY when `args.write === true || args.dryRun === false` (`shouldBlockStaleMcpTool` special case for `ocentra_enforcer_coordination_repair`); a pure dry-run/read `repair` is NOT gated. The port MUST reproduce this exact predicate so a dry-run repair stays available on a stale server while a real repair is refused with the CLI fallback. (hash-compat / repair semantics reference **arc-16**.) Fixture — pass: `coordination_repair {dryRun:true}` (or `write` unset) is allowed on a stale server; fail: `coordination_repair {write:true}` OR `{dryRun:false}` on a stale server is refused and returns the structured fallback.
- [ ] **`check` named-check enum parity (no silent drop).** The `ocentra_enforcer_check` tool exposes a fixed enum of ~20 named checks (`rust-rules-mcp-tool-registry-rules.mjs`: no-zod-source, no-naked-domain-strings, no-test-doubles, weak-assertions, …, source-shape, required-tests, single-source-contracts, dependency-policy, sbom, literal-risk, ai-rule-index, import-boundaries, architecture-policy). The port MUST carry an EXPLICIT parity checklist/test asserting the advertised enum == the set of registered check validators (arc-15 / d01 own the validators themselves) so no individual check can silently disappear from the MCP surface or gain an unbacked entry. Fixture — pass: enum set == registered-validator set (bidirectional equality); fail: a check present in the enum with no backing validator (or a registered validator missing from the enum) fails the parity assertion.

## Acceptance And Proof
Tier P1. Proof row asserts `cargo test -p enforcer-mcp` exits 0 — transport + tool dispatch proven with fail/pass request fixtures and a deterministic tool-surface enumeration. Record the artifact path in TEST_PROOF_EXPECTATIONS.md.

## Parallel Ownership Notes
Owns the `enforcer-mcp` crate SKELETON: `crates/enforcer-mcp/Cargo.toml`, `src/lib.rs`, the transport + tool-registry + router modules, and the single stdio-protocol sink module (the sanctioned print site). Deps the engine frontier (arc-15/16/17/18) + foundation/rules.

Parallel Ownership Note (disjoint feature packs): d05 context-budget owns its own ratchet/baseline files (`enforcer-mcp` surface measure consumed via the seam above, its baseline + `enforcer-core` meter owned by d05) — NOT this crate; d05 `deps:` arc-21 and is sequenced after the skeleton. owns stay DISJOINT BY FILE. Parallel-safe with arc-22 (cli) at the transport/registry boundary — both are FIRST-CLASS surfaces over the same engine crates, neither secondary. Precedes/pairs with arc-22 which serves this on stdio.
