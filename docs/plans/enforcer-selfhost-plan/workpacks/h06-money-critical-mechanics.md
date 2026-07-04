# h06 Money Critical Mechanics

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Money Critical Mechanics`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling completion, product status, PR readiness, or broad DONE.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `rules/security/money-critical-mechanics.md, src/validators/{signing,time,economic,rollback,boundary,killswitch}-*.ts, tests/fixtures/money-critical-mechanics/**`
- deps: `d01, h01`
- tier: `P0/P1 mixed`

Sources: [PLAN_STATE](../PLAN_STATE.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md), [security-testing source](../refs/security-testing-source.md).

## Where We Are
Spec §8.6–8.10 states the mechanical rules for money-critical code — signing/authorization, time/clock, economic cost, rollback/compensation, the untrusted internal boundary, and kill switches — as PROSE. GENERIC: this is any value system behind untrusted infra; Cloudflare/AWS/gateways and internal topology give ZERO security, internal APIs are hostile. Today a backend can sign a client-raw payload, trust client wall-clock in a money path, expose an unauthenticated internal endpoint, or ship an untested rollback, all silently. Crypto/blockchain (Solana/Anchor signing) is one OPTIONAL instance, never assumed.

## Where We Want To Be
A `rules/security/money-critical-mechanics.md` doc plus per-facet validators, scaffolded through d01, scoped by h01's money-critical classifier:
- SIGNING (`signing-*.ts`, T1): backend must NOT sign/authorize client-raw, non-reconstructable, or non-verifiable payloads; require canonical serialization + correlation-id log at the sign site.
- TIME (`time-*.ts`, T1): client time never trusted in money paths; server-time only; explicit skew tolerance; expiry fails-closed.
- ECONOMIC (`economic-*.ts`, T2): attacker-cost >= system-cost; no free retries with non-zero backend cost; dust bounded.
- ROLLBACK (`rollback-*.ts`, T2 semantics): idempotent/replay-safe/atomic/exactly-once; untested rollback forbidden.
- UNTRUSTED-BOUNDARY (`boundary-*.ts`, T1): internal APIs hostile; internal headers untrusted; topology gives ZERO security; an unauthed internal endpoint = fail.
- KILL-SWITCH (`killswitch-*.ts`, T1): halt-all/atomic/authed/audited/replay-safe; untested kill-switch forbidden.

## Requirement Checklist
- [ ] `rules/security/money-critical-mechanics.md` created with one anchored section per ruleId, each carrying its tier.
- [ ] T1 SIGNING: signing a client-raw / non-reconstructable / unverified payload flagged; canonical+correlation-id sign site clean.
- [ ] T1 TIME: client-clock use in a money path flagged; server-time + explicit skew + fail-closed expiry clean.
- [ ] T1 BOUNDARY: an unauthenticated internal endpoint / trusted internal header flagged; authed internal endpoint clean.
- [ ] T1 KILL-SWITCH: kill-switch not halt-all/atomic/authed/audited/replay-safe or untested flagged.
- [ ] T2 ECONOMIC + ROLLBACK: cost/retry/dust and rollback idempotency/atomicity emit score+confidence; untested rollback flagged.
- [ ] Scoped by h01 money-critical classification; all rows registered via d01 `rule new`; parity oracle green across ruleId <-> doc <-> validator <-> {fail,pass} fixtures <-> detection-test.

## Acceptance And Proof
5-way parity per rule. Fixtures live under `tests/fixtures/money-critical-mechanics/`.

- SIGNING (T1): fail `sign-client-raw.fail.ts` (backend signs unmodified client payload, no canonical serialize/log — flagged); pass `sign-reconstructed.pass.ts` (payload rebuilt from request context, canonical serialize, correlation-id logged).
- TIME (T1): fail `client-clock.fail.ts` (`Date.now()` from client body drives expiry — flagged); pass `server-clock.pass.ts` (server time, explicit skew const, expiry fails-closed).
- BOUNDARY (T1): fail `unauthed-internal.fail.ts` (internal endpoint with no auth, trusts `X-Internal` header — flagged); pass `authed-internal.pass.ts` (authenticated, header untrusted).
- KILL-SWITCH (T1): fail `killswitch-untested.fail/` (kill switch present, no test) and `killswitch-nonatomic.fail.ts`; pass `killswitch-full.pass/` (halt-all, atomic, authed, audited, replay-safe, tested).
- ECONOMIC (T2): fail `free-retry.fail.ts` (retry with backend cost, no charge/bound — score crosses); pass `bounded-cost.pass.ts`.
- ROLLBACK (T2): fail `rollback-nonidempotent.fail.ts`; pass `rollback-exactly-once.pass.ts`.

Prove via per-facet detection tests and the d01 `rule-scaffold-parity` oracle. Update TEST_PROOF_EXPECTATIONS.md rows before DONE.

## Parallel Ownership Notes
`owns:` set is disjoint: exclusively creates `rules/security/money-critical-mechanics.md`, the `src/validators/{signing,time,economic,rollback,boundary,killswitch}-*.ts` family, and `tests/fixtures/money-critical-mechanics/**`. Depends on `d01` (scaffolder + parity) and `h01` (money-critical classifier, consumed read-only to scope which code these rules apply to). Sibling h04 (test-shape bans) and h05 (invariant property suites) are not touched here; h06 owns the runtime/mechanical rules, not test-quality or invariant-presence. Crypto signing (Solana/Anchor) is handled as one optional instance under the generic SIGNING facet, never a required assumption. Can start once d01 and h01 land.
