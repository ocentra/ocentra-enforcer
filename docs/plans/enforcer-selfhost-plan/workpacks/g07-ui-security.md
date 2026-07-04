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

- owns: `src/ui/security/*`, `tests/ui-security/**`
- deps: `g01, g04`
- tier: `P5 T1`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md).

## Where We Are
Track G introduced a NEW UI attack surface: the g01 local HTTP server (wrapping the vendored `server.js` loopback + token gate), the g03 waiver-write endpoint, the g05 CI/pre-commit-config-write endpoint, and the g04 Run-dispatch that executes agents and writes coordination fix-intents. Loopback-default + token are inherited from `server.js` but not asserted as a security contract, and the mutation/dispatch endpoints have no same-origin/CSRF or per-action authorization. Any local web page could POST to them.

## Where We Want To Be
A dedicated `src/ui/security/*` layer enforced by every g0x endpoint: (1) bind is loopback-by-default and a non-loopback bind without a token REFUSES to start; (2) mutation endpoints (g03 waiver-write, g05 config-write) require same-origin + a valid session/CSRF token; cross-origin POSTs are rejected; (3) g04 Run-dispatch requires a valid intent token and runs sandboxed under explicit authorization so a page cannot trigger arbitrary agent runs. Waivers remain honest, gated, never silent (T1). This guards HUMAN surface only; silent agent-inline runs are unaffected.

## Requirement Checklist
- [ ] `src/ui/security/*` exposes loopback-bind assertion, origin/CSRF check, and dispatch-authorization guards reused by all g0x endpoints.
- [ ] Non-loopback bind without token refuses to start (assert the inherited `server.js` behavior).
- [ ] Waiver-write (g03) and config-write (g05) reject cross-origin / missing-CSRF-token POSTs.
- [ ] Run-dispatch (g04) refuses without a valid intent token; authorized runs are sandboxed.

## Acceptance And Proof
Tier P5 T1. Fail-fixtures: `sec-xorigin-waiver-reject` (cross-origin POST to waiver/config endpoint rejected), `sec-remote-bind-no-token` (non-loopback bind without token refused), `sec-dispatch-no-token` (dispatch without valid intent token refused). Pass-fixture: `sec-same-origin-token-ok` (same-origin + valid token to waiver/config/dispatch succeeds). Detection test: `ui-security-contract` asserts all three rejections and the pass path, and that guards come from `src/ui/security/*` (not re-inlined per endpoint). Rows in TEST_PROOF_EXPECTATIONS.md.

## Parallel Ownership Notes
Owns `src/ui/security/*` and `tests/ui-security/**` exclusively. Depends on g01 (serve surface) and g04 (dispatch) landing; wraps/guards their endpoints without re-opening the HTTP transport or forking dispatch logic. g03/g05 endpoints consume these guards but are owned by their own packs.
