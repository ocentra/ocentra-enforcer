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

- owns: `src/ui/serve.*`, cli `enforcer serve`/`enforcer ui`, mcp ui tool
- deps: `a01`
- tier: `P5`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md).

## Where We Are
The hub UI is already vendored at `src/coordination/vendor/server.js` (13.5KB Node `http` createServer + token gate) and `src/coordination/vendor/dashboard.js` (18KB self-contained "Ocentra Ledger" HTML, zero framework, inline CSS). It is reachable ONLY via a buried `coordination ledger:dashboard` command and renders only coordination/ledger state — not enforcement.

## Where We Want To Be
Promote the vendored server into a first-class `src/ui/serve.*` module exposing `enforcer serve` / `enforcer ui` (CLI) and an `mcp__enforcer__ui` tool. It reuses the vendored Node HTTP + self-contained HTML shell — loopback default, token required for any non-loopback bind — and provides a neutral HTML shell + view registry that g02 (report) and later settings/hub views MOUNT into. No framework, no bundler, no binary. This is HUMAN-invoked surface only; inline agent checks stay silent (see f04).

## Requirement Checklist
- [ ] `src/ui/serve.*` wraps the vendored `server.js` HTTP core; no fork of the transport.
- [ ] `enforcer serve` and `enforcer ui` both resolve to this surface; the old buried path still works.
- [ ] Binds loopback (127.0.0.1) by default; any remote/host bind REQUIRES a token or refuses to start.
- [ ] Serves a self-contained HTML shell exposing a view-mount registry for downstream packs.
- [ ] MCP `ui` tool returns the served URL, never auto-launches during silent agent runs.

## Acceptance And Proof
Tier P5. Fail-fixture: `serve-remote-no-token` (host bind without token) -> server refuses to start. Pass-fixture: `serve-loopback-default` -> binds 127.0.0.1, returns shell HTML with mount registry present. Detection test: `serve-surface-contract` asserts the CLI aliases resolve, loopback-default holds, remote-without-token is rejected, and the vendored HTTP core is reused (not reimplemented). Rows in TEST_PROOF_EXPECTATIONS.md.

## Parallel Ownership Notes
Owns `src/ui/serve.*` exclusively; does not touch `src/coordination/vendor/*` (read/wrap only). Foundation for g02/g03 — they mount views, they never re-open the HTTP layer. Depends on a01 toolchain for the build.
