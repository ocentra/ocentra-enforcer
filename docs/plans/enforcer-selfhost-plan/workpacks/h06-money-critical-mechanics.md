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

- owns: `crates/enforcer-security/src/rules/{signing,time,economic,rollback,boundary,killswitch}.rs, crates/enforcer-security/tests/fixtures/money_critical_mechanics/**`
- deps: `d01, arc-19, arc-05, arc-04, h01`
- tier: `P0/P1 mixed`

Sources: [PLAN_STATE](../PLAN_STATE.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md), [security-testing source](../refs/security-testing-source.md), [RUST_ARCHITECTURE](../RUST_ARCHITECTURE.md).

## Where We Are
Spec §8.6–8.10 states the mechanical rules for money-critical code — signing/authorization, time/clock, economic cost, rollback/compensation, the untrusted internal boundary, and kill switches — as PROSE. GENERIC: this is any value system behind untrusted infra; Cloudflare/AWS/gateways and internal topology give ZERO security, internal APIs are hostile. Today a backend can sign a client-raw payload, trust client wall-clock in a money path, expose an unauthenticated internal endpoint, or ship an untested rollback, all silently. The retired Node engine had no such detection; the `enforcer-security` (arc-19) crate stands up the Track H skeleton but no `src/rules/<facet>.rs` mechanics module exists yet. Crypto/blockchain (Solana/Anchor signing) is one OPTIONAL instance (e-pack-crypto-blockchain), never assumed.

## Where We Want To Be
A per-facet `Validator` family under `crates/enforcer-security/src/rules/{signing,time,economic,rollback,boundary,killswitch}.rs`, each impl'ing the `Validator` trait (from `enforcer-validator`, arc-05) and returning structured `Finding`s (from `enforcer-domain`, arc-02) with a terse `Fix:` hint, scaffolded through d01 (arc-14) so each rule lands in 5-way parity, with its threat-mapped rule record (`RuleId` + `ThreatId` MITRE/OWASP where applicable + tier + doc-anchor) carried in `enforcer-rules` (arc-04). Target-language code is parsed with the right frontend — `tree-sitter`/`swc` for TS/JS/Python backends, `syn` for Rust backends. Scoped by h01's money-critical classifier (consumed read-only via the `enforcer-security` manifest):
- SIGNING (`src/rules/signing.rs`, T1): backend must NOT sign/authorize client-raw, non-reconstructable, or non-verifiable payloads; require canonical serialization + correlation-id log at the sign site.
- TIME (`src/rules/time.rs`, T1): client time never trusted in money paths; server-time only; explicit skew tolerance; expiry fails-closed.
- ECONOMIC (`src/rules/economic.rs`, T2): attacker-cost >= system-cost; no free retries with non-zero backend cost; dust bounded (score + confidence).
- ROLLBACK (`src/rules/rollback.rs`, T2 semantics): idempotent/replay-safe/atomic/exactly-once; untested rollback forbidden.
- UNTRUSTED-BOUNDARY (`src/rules/boundary.rs`, T1): internal APIs hostile; internal headers untrusted; topology gives ZERO security; an unauthed internal endpoint = fail.
- KILL-SWITCH (`src/rules/killswitch.rs`, T1): halt-all/atomic/authed/audited/replay-safe; untested kill-switch forbidden.

Each `Validator` obeys `[workspace.lints]` (no `unwrap/expect/panic/print_*`; no `pub use` barrels) and registers into the `enforcer-security` rule set through the crate's `Validator`-registration seam owned by arc-19.

## Requirement Checklist
Each rule is scaffolded via `enforcer rule new <ID>` (d01), landing a doc-anchor in its `enforcer-rules` record, a `Validator` impl in `crates/enforcer-security/src/rules/<facet>.rs`, and a fail+pass fixture pair under `crates/enforcer-security/tests/fixtures/money_critical_mechanics/<facet>/{bad,good}/`.
- [x] T1 SIGNING (`signing.rs`, `MCM-SIGNING.1` {#MCM-SIGNING}): signing a client-raw / non-reconstructable / unverified payload emits a `Finding`; canonical+correlation-id sign site clean.
- [x] T1 TIME (`time.rs`, `MCM-TIME.1` {#MCM-TIME}): client-clock use in a money path emits a `Finding`; server-time + explicit skew + fail-closed expiry clean.
- [x] T1 BOUNDARY (`boundary.rs`, `MCM-BOUNDARY.1` {#MCM-BOUNDARY}): an unauthenticated internal endpoint / trusted internal header emits a `Finding`; authed internal endpoint clean.
- [x] T1 KILL-SWITCH (`killswitch.rs`, `MCM-KILLSWITCH.1` {#MCM-KILLSWITCH}): kill-switch not halt-all/atomic/authed/audited/replay-safe or untested emits a `Finding`.
- [x] T2 ECONOMIC (`economic.rs`, `MCM-ECONOMIC.1` {#MCM-ECONOMIC}) + ROLLBACK (`rollback.rs`, `MCM-ROLLBACK.1` {#MCM-ROLLBACK}): cost/retry/dust and rollback idempotency/atomicity emit score+confidence; untested rollback flagged.
- [x] Scoped by h01 money-critical classification (consumed read-only, never redefined); all rows registered via d01 `rule new`; parity oracle green across `RuleId` <-> doc-anchor <-> `Validator` <-> {fail,pass} fixtures <-> `cargo test` detection test.
- [x] Clean `cargo clippy` / `cargo fmt --check` (obey `[workspace.lints]`).

## Acceptance And Proof
Tier P0/P1 mixed. Prove via `cargo test -p enforcer-security`. 5-way parity per rule. Fixtures are target-language sample code parsed by the Rust `Validator`, under `crates/enforcer-security/tests/fixtures/money_critical_mechanics/<facet>/{bad,good}/`.

- SIGNING (T1): fail `signing/bad/sign_client_raw.ts` (backend signs unmodified client payload, no canonical serialize/log — flagged); pass `signing/good/sign_reconstructed.ts` (payload rebuilt from request context, canonical serialize, correlation-id logged); `#[test] mcm_signing`.
- TIME (T1): fail `time/bad/client_clock.ts` (`Date.now()` from client body drives expiry — flagged); pass `time/good/server_clock.ts` (server time, explicit skew const, expiry fails-closed); `#[test] mcm_time`.
- BOUNDARY (T1): fail `boundary/bad/unauthed_internal.ts` (internal endpoint with no auth, trusts `X-Internal` header — flagged); pass `boundary/good/authed_internal.ts` (authenticated, header untrusted); `#[test] mcm_boundary`.
- KILL-SWITCH (T1): fail `killswitch/bad/untested/` (kill switch present, no test) and `killswitch/bad/nonatomic.ts`; pass `killswitch/good/full/` (halt-all, atomic, authed, audited, replay-safe, tested); `#[test] mcm_killswitch`.
- ECONOMIC (T2): fail `economic/bad/free_retry.ts` (retry with backend cost, no charge/bound — score crosses); pass `economic/good/bounded_cost.ts`; `#[test] mcm_economic`.
- ROLLBACK (T2): fail `rollback/bad/nonidempotent.ts`; pass `rollback/good/exactly_once.ts`; `#[test] mcm_rollback`.

Prove via the per-facet detection tests and the d01 `rule-scaffold-parity` oracle over every money-critical-mechanics `RuleId` this pack adds. Update TEST_PROOF_EXPECTATIONS.md rows before DONE.

## Parallel Ownership Notes
`owns:` set is disjoint BY FILE: exclusively creates `crates/enforcer-security/src/rules/{signing,time,economic,rollback,boundary,killswitch}.rs` and `crates/enforcer-security/tests/fixtures/money_critical_mechanics/**`. Lands inside the `enforcer-security` crate whose SKELETON arc-19 owns (`Cargo.toml`, `src/lib.rs`, the `src/rules/` module-root + `Validator`-registration, the no-bypass meta-check) — must NOT edit that skeleton, arc-19's no-bypass meta-check, or any sibling `src/rules/<name>.rs`. Depends on `d01` (scaffolder + parity oracle — consumed, not redefined), `arc-19` (crate skeleton, sequences this after the skeleton exists), `arc-05` (the `Validator` trait + fixture/parity harness), `arc-04` (the `enforcer-rules` records — added via d01's scaffolder, not by hand), and `h01` (the money-critical classifier manifest, consumed read-only to scope which code these rules apply to). Sibling h04 (test-shape bans) and h05 (invariant property suites) are not touched here; h06 owns the runtime/mechanical rules, not test-quality or invariant-presence. Crypto signing (Solana/Anchor) is handled as one optional instance (e-pack-crypto-blockchain) that composes with the generic SIGNING facet read-only, never a required assumption. `owns disjoint? = Y` (deps arc-19 sequences it after the crate skeleton; deps h01 after the classifier manifest exists).
