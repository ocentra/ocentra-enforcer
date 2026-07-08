# g06 Hub Coordination Dashboard

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Hub Coordination Dashboard`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling completion, product status, PR readiness, or broad DONE.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `crates/enforcer-ui/src/hub/*`
- deps: `arc-24, arc-16`
- tier: `P3`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md).

## Where We Are
The `enforcer-coordination` crate (arc-16) already materializes and exposes the live coordination ledger — presence, lanes, claims, leases, releases, tasks, workers, mail/intercommunication — as typed state (the ported-to-Rust hub, superseding the old vendored `dashboard.js`/`server.js` Node surface). But that live state is not surfaced in the arc-24 (`enforcer-ui`) shell: there is no enforcement-adjacent view of who is working which lane, what is claimed/leased, or what mail is flowing while Codex/Claude edit in parallel.

## Where We Want To Be
The hub dashboard — presence, lanes, claims, leases/releases, tasks, workers, mail, sync — surfaced as a `crates/enforcer-ui/src/hub/` module mounted into the SAME arc-24 shell as report + settings, so coordination + enforcement + config live under one local `enforcer ui`. The panel binds to LIVE materialized ledger state via the arc-16 `enforcer-coordination` API (typed `HubName`/`LaneId`/claim/lease records from `enforcer-domain`), streaming updates via the `enforcer-events` (arc-25) event spine that coordination already emits. Rendered as a self-contained served-HTML view (and Tauri frontend), all types DERIVED from `enforcer-domain` via `ts_rs` (owned by g05) — no hand-written wire shapes.

## Requirement Checklist
- [ ] Mount a hub panel `crates/enforcer-ui/src/hub/` into the arc-24 shell registry as a self-contained view (Tauri frontend + served-HTML fallback), no separate transport.
- [ ] Bind panels (presence / lanes / claims / leases / tasks / workers / mail / sync) to LIVE materialized state via the arc-16 `enforcer-coordination` API, consuming its `enforcer-events` (arc-25) stream — not a separate standalone server.
- [ ] Single served origin: one loopback + token gate shared with report/settings (via g07 guards); no forked HTTP layer.
- [ ] Read-only surface — the panel reflects ledger state and issues zero mutating calls against the coordination API (mutations remain g04's Run-dispatch path).
- [ ] Empty/degraded/unmaterialized ledger renders a stable empty state, never a panic (obey the workspace deny-wall: no `unwrap`/`expect`/`panic`).

## Acceptance And Proof
T1 (`hub-dashboard-mount`): fail-fixture — `cargo test -p enforcer-ui hub::` — a missing/unmaterialized coordination view renders the empty state (returns the placeholder view, no panic). pass-fixture — a seeded ledger with one lane + one claim renders those exact rows in the mounted panel (assert the rendered view carries the lane `LaneId` + claim ids from the arc-16 state). detection test — the panel issues zero mutating calls against the coordination API (spy/mock arc-16 facade asserts read-only). Record artifact paths in TEST_PROOF_EXPECTATIONS.md.

## Parallel Ownership Notes
Owns only `crates/enforcer-ui/src/hub/*`. Consumes the arc-24 shell and the arc-16 `enforcer-coordination` typed facade + arc-25 event stream read-only; reuses g05's `ts_rs`-derived types (does not redefine them). Disjoint from g04/g05/g07 by file. Does NOT own the `enforcer-ui` crate skeleton (arc-24) nor any `enforcer-coordination` internals (arc-16) — it reads the materialized state the arc-16 owner exposes.
