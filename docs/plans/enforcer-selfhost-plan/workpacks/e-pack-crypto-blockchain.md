# e-pack-crypto-blockchain Crypto And Blockchain Money-Critical Pack

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Crypto And Blockchain Money-Critical Pack`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling completion, product status, PR readiness, or broad DONE.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `crates/enforcer-lang-crypto/**`
- deps: `arc-05-validator, arc-04-rules, d01, d17, d18, h01`
- tier: `P0/P1` (OPTIONAL / opt-in, OFF by default)

Sources: [PLAN_STATE](../PLAN_STATE.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md), [security-testing source](../refs/security-testing-source.md), [RUST_ARCHITECTURE](../RUST_ARCHITECTURE.md).

## Where We Are
Crypto/blockchain is ONE optional money-critical instance, not assumed by the enforcer. The spec's §2.5 on-chain abuse surface (Solana/Anchor as the example) is prose; no `enforcer-lang-crypto` crate exists, no crypto language is registered as rule records in `enforcer-rules`, and no crypto `Validator` is mechanized. The workspace ships `enforcer-lang-{rust,ts,py,common,security,iac,k8s}` but nothing crypto-specific.

## Where We Want To Be
An OPTIONAL, clearly-labeled, opt-in **NEW workspace crate `enforcer-lang-crypto`** (default OFF) that this pack stands up itself (no arc-* pack pre-builds it), registering the crypto language(s) (e.g. Solana/Anchor Rust on-chain) ONLY when the project opts in via `enforcer-config`. Concretely:

1. **This pack stands up the crate skeleton itself.** `crates/enforcer-lang-crypto/Cargo.toml` (`[lints] workspace=true`, deps on `enforcer-domain`, `enforcer-rules` (arc-04), `enforcer-validator` (arc-05); Anchor/Solana Rust on-chain targets are parsed with `syn` reusing the `enforcer-lang-rust` (d17) machinery), `src/lib.rs` (crate root + module tree), and a `register()` fn that adds every crypto `Validator` to the shared rule set **only when the opt-in config flag is set** (default OFF — the crate no-ops when disabled). Each validator impls the `Validator` trait (arc-05) and emits `enforcer-domain::Finding`s with a `Fix:` hint; obeys `[workspace.lints]` (no `unwrap/expect/panic/print_*`, no `pub use` barrels).
2. It mechanizes §2.5 as typed rule records in `enforcer-rules` (via d01, 5-way parity): instruction execution paths, replay, slot-timing edges, CPI boundaries, simulation-vs-execution mismatch, key-lifecycle abuse, signing abuse (blind-sign / wrong-message), nonce/blockhash reuse, signer order/injection, PDA security, Anchor account validation, and program state transitions. T1 = structural `syn` validators (account constraints present, signer checks present); T2 = heuristic scored (CPI/PDA risk scoring, folded into the `enforcer-literal-scan` scored floor); labeled T3 where only runtime proof on localnet suffices, run via the h07 crypto-localnet adapter (through `enforcer-harness` arc-18). Composes with h06 signing rules and the h01 `enforcer-security` classifier (read-only).

## Requirement Checklist
- [ ] `enforcer-lang-crypto` crate skeleton stood up (Cargo.toml + lib.rs + `register()`); pack is OFF by default; enabling requires explicit opt-in config in `enforcer-config`; the crate no-ops (registers no validators) when disabled.
- [ ] T1: missing Anchor account constraint / absent signer check blocks (structural `syn` `Validator`).
- [ ] T1: instruction path signing a non-reconstructable message blocks (via h06 signing rules).
- [ ] T2: CPI-boundary / PDA-derivation heuristics scored (feeds the `enforcer-literal-scan` scored floor).
- [ ] T3: replay / slot-timing / sim-vs-exec / nonce-reuse labeled for localnet proof (h07 adapter via arc-18); each carries an `advisory, no mechanization possible + <reason>` label whose presence d01 verifies.
- [ ] No product/company/game branding in rules or fixtures.

## Acceptance And Proof
Tier P0/P1, optional. Per-rule fixtures under `crates/enforcer-lang-crypto/tests/fixtures/`: `bad/missing-signer-check` + `good/signer-checked`; `bad/unconstrained-account` + `good/constrained-account`; `bad/blind-sign` + `good/reconstructable-sign`; `bad/pda-unvalidated` + `good/pda-validated`; `bad/cpi-unbounded` + `good/cpi-scoped`; T3 `label/replay-needs-localnet`. Detection test `cargo test -p enforcer-lang-crypto` asserts each fail blocks/scores/labels, each pass clean, and the pack **no-ops when opt-in is off** (registers no validators, emits no findings). 5-way parity oracle over every crypto `RuleId`. Rows in TEST_PROOF_EXPECTATIONS.md. (Crate-map delta: this pack ADDS the OPT-IN `enforcer-lang-crypto` crate to the workspace — the reconciliation pass records the crate note; do not edit shared index files here.)

## Parallel Ownership Notes
Owns `crates/enforcer-lang-crypto/**` exclusively (the whole new crate: `Cargo.toml`, `src/**`, `tests/fixtures/crypto/**`) — disjoint from all siblings by file. This pack builds its OWN crate skeleton since no arc-* pack pre-builds it. Depends on arc-05 (the `Validator` trait + fixture/parity harness), arc-04 (`enforcer-rules` record load), d01 (mechanization + 5-way parity), d17 (Rust error handling / reuses the `enforcer-lang-rust` `syn` machinery for Anchor Rust targets), d18 (`enforcer-lang-security` / `enforcer-security` security-stop watchlist), and h01 (`enforcer-security` classifier). Consumes h06 signing rules + the h07 localnet adapter (run through `enforcer-harness` arc-18) read-only. Being OFF by default (opt-in via `enforcer-config`) keeps it disjoint from all non-crypto lanes — it never runs, registers a validator, or emits a finding unless a project opts in.
