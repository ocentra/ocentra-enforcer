# g07 Ui Security

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Ui Security`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling completion, product status, PR readiness, or broad DONE.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `crates/enforcer-ui/src/security/*`, `crates/enforcer-ui/tests/ui_security/**`
- deps: `arc-24, g04`
- tier: `P5 T1`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md).

## Where We Are
Track G introduced a NEW UI attack surface: the arc-24 (`enforcer-ui`) local HTTP surface (Tauri backend + served-HTML fallback), the g03 waiver-write endpoint, the g05 config-write endpoint, and the g04 Run-dispatch that writes coordination fix-intents. The Rust serve surface binds loopback + issues a token, but that is not yet asserted as an enforced security CONTRACT, and the mutation/dispatch handlers have no same-origin/CSRF or per-action authorization. Any local web page could POST to them.

## Where We Want To Be
A dedicated `crates/enforcer-ui/src/security/` guard layer, reused by every g0x handler in the `enforcer-ui` backend: (1) bind is loopback-by-default and a non-loopback bind WITHOUT a token REFUSES to start (typed error, fail-closed); (2) mutation handlers (g03 waiver-write, g05 config-write) require same-origin + a valid session/CSRF token — cross-origin POSTs are rejected; (3) the g04 Run-dispatch handler requires a valid intent token and runs under explicit authorization so a page cannot trigger arbitrary agent runs. Guards are Rust functions/middleware (validated tokens as branded newtypes from `enforcer-domain`), applied once and shared — not re-inlined per handler. Waivers remain honest, gated, never silent (T1). This guards the HUMAN surface only; silent agent-inline runs are unaffected.

## Requirement Checklist
- [ ] `crates/enforcer-ui/src/security/` exposes loopback-bind assertion, origin/CSRF check, and dispatch-authorization guards reused by all g0x handlers (single source, not per-handler copies).
- [ ] Non-loopback bind without token refuses to start (returns a typed fail-closed error from the serve entrypoint; obeys the deny-wall — no `panic`).
- [ ] Waiver-write (g03) and config-write (g05) reject cross-origin / missing-CSRF-token POSTs (guard returns rejection before any `enforcer-config`/`.enforce` write).
- [ ] Run-dispatch (g04) refuses without a valid intent token (branded token newtype validated at the boundary); authorized runs are gated behind explicit authorization.
- [ ] Tokens/session ids are validated branded newtypes (`enforcer-domain`), parse-at-boundary — never bare `String`.

## Acceptance And Proof
Tier P5 T1 (`ui-security-contract`): `cargo test -p enforcer-ui --test ui_security`. Fail-fixtures: `sec-xorigin-waiver-reject` (cross-origin POST to waiver/config handler rejected, zero write), `sec-remote-bind-no-token` (non-loopback bind without token -> serve entrypoint refuses with typed error), `sec-dispatch-no-token` (dispatch without valid intent token refused). Pass-fixture: `sec-same-origin-token-ok` (same-origin + valid token to waiver/config/dispatch succeeds). Detection test asserts all three rejections and the pass path, and that guards resolve from `crates/enforcer-ui/src/security/*` (not re-inlined per handler). Rows in TEST_PROOF_EXPECTATIONS.md.

## Parallel Ownership Notes
Owns `crates/enforcer-ui/src/security/*` and `crates/enforcer-ui/tests/ui_security/**` exclusively. Depends on arc-24 (serve surface) and g04 (dispatch) landing; wraps/guards their handlers without re-opening the HTTP transport or forking dispatch logic. g03/g05 handlers consume these guards but are owned by their own packs. Does NOT own the `enforcer-ui` crate skeleton (arc-24) — it adds the guard module and reuses `enforcer-domain` token newtypes rather than redefining them.
