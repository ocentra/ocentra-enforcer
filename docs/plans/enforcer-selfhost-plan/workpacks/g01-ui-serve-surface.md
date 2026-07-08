# g01 Ui Serve Surface

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Ui Serve Surface`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling completion, product status, PR readiness, or broad DONE.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `crates/enforcer-ui/src/serve.rs`, cli `enforcer serve`/`enforcer ui`, mcp ui tool
- deps: `arc-24`
- tier: `P5`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md), [RUST_ARCHITECTURE](../RUST_ARCHITECTURE.md).

## Where We Are
The `enforcer-ui` crate skeleton (arc-24) exists — a Rust UI-server / Tauri backend serving a self-contained HTML fallback for headless use and backing the Tauri desktop app, plus the `ts_rs` type-gen pipeline — but no first-class serve surface is wired: there is no `enforcer serve` / `enforcer ui` entry, and the served shell exposes no view-mount registry for the g0x feature modules to plug into. The coordination hub state lives in `enforcer-coordination` (arc-16); UI rendering of enforcement is not yet reachable.

## Where We Want To Be
Stand up `crates/enforcer-ui/src/serve.rs`: the first-class serve surface exposing `enforcer serve` / `enforcer ui` (clap CLI in `enforcer-cli`, arc-22) and an `mcp__enforcer__ui` tool (arc-21). It drives the arc-24 UI server — the Tauri shell for the desktop app AND the served self-contained HTML fallback for headless use — loopback (127.0.0.1) default, token required for any non-loopback bind. It provides a neutral shell + a Rust-side VIEW-MOUNT REGISTRY that g02 (report), g05 (settings), g06 (hub), g08 (explorer) MOUNT into. Presentation-only TS lives under `crates/enforcer-ui/frontend/` (types derived from `enforcer-domain` via `ts_rs`); no business logic in TS. This is HUMAN-invoked surface only; inline agent checks stay silent (see f04, `enforcer-core` run-context).

## Requirement Checklist
- [ ] `crates/enforcer-ui/src/serve.rs` drives the arc-24 UI server (Tauri shell + served HTML fallback); it does not fork the transport or reimplement the arc-24 backend root.
- [ ] `enforcer serve` and `enforcer ui` both resolve to this surface via the `enforcer-cli` clap dispatch; exit-code-driven, Windows-first argv handling.
- [ ] Binds loopback (127.0.0.1) by default; any remote/host bind REQUIRES a token or refuses to start (fail-closed).
- [ ] Exposes a Rust view-mount registry that downstream g0x modules register into; the served HTML fallback is self-contained (no external assets), frontend types derived from `enforcer-domain`.
- [ ] MCP `ui` tool returns the served URL, never auto-launches during silent agent runs (`enforcer-core` run-context gate).

## Acceptance And Proof
Tier P5. Fail-fixture: `serve-remote-no-token` (host bind without token) -> server refuses to start. Pass-fixture: `serve-loopback-default` -> binds 127.0.0.1, returns shell HTML with the mount registry present. Detection test: `serve-surface-contract` (`cargo test -p enforcer-ui`) asserts the CLI aliases resolve, loopback-default holds, remote-without-token is rejected, and the arc-24 UI-server backend is reused (not reimplemented). Clean `cargo clippy` / `cargo fmt --check` (obey `[workspace.lints]`; no `pub use` barrels). Rows in TEST_PROOF_EXPECTATIONS.md.

## Parallel Ownership Notes
Owns `crates/enforcer-ui/src/serve.rs` exclusively; does not own the arc-24 crate skeleton (`Cargo.toml`/`lib.rs`/backend root — read/drive only). Foundation for g02/g05/g06/g08 — they register views into the registry, they never re-open the transport. Deps arc-24 (crate skeleton) and is sequenced after it exists. owns stay DISJOINT BY FILE from sibling g0x modules.
