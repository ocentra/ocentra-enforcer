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

- owns: `rules/crypto/**.md`, `src/validators/crypto-*.ts`, `tests/fixtures/crypto/**`
- deps: `d01`, `d17`, `d18`, `h01`
- tier: `P0/P1` (OPTIONAL / opt-in)

Sources: [PLAN_STATE](../PLAN_STATE.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md), [security-testing source](../refs/security-testing-source.md).

## Where We Are
Crypto/blockchain is ONE optional money-critical instance, not assumed by the enforcer. The spec's §2.5 on-chain abuse surface (Solana/Anchor as the example) is prose; no language is registered and no crypto rule is mechanized.

## Where We Want To Be
An OPTIONAL, clearly-labeled, opt-in pack that registers the crypto language(s) (e.g. Solana/Anchor Rust on-chain) ONLY when the project opts in (default OFF). It mechanizes §2.5: instruction execution paths, replay, slot-timing edges, CPI boundaries, simulation-vs-execution mismatch, key-lifecycle abuse, signing abuse (blind-sign / wrong-message), nonce/blockhash reuse, signer order/injection, PDA security, Anchor account validation, and program state transitions. T1 = structural (account constraints present, signer checks present); T2 = heuristic (CPI/PDA risk scoring); labeled T3 where only runtime proof on localnet suffices, run via the h07 crypto-localnet adapter. Composes with h06 signing and the h01 classifier.

## Requirement Checklist
- [ ] Pack is OFF by default; enabling requires explicit opt-in config.
- [ ] T1: missing Anchor account constraint / absent signer check blocks.
- [ ] T1: instruction path signing a non-reconstructable message blocks (via h06).
- [ ] T2: CPI-boundary / PDA-derivation heuristics scored.
- [ ] T3: replay / slot-timing / sim-vs-exec / nonce-reuse labeled for localnet proof (h07).
- [ ] No product/company/game branding in rules or fixtures.

## Acceptance And Proof
Tier P0/P1, optional. Per-rule fixtures under `tests/fixtures/crypto/`: `fail/missing-signer-check` + `pass/signer-checked`; `fail/unconstrained-account` + `pass/constrained-account`; `fail/blind-sign` + `pass/reconstructable-sign`; `fail/pda-unvalidated` + `pass/pda-validated`; `fail/cpi-unbounded` + `pass/cpi-scoped`; T3 `label/replay-needs-localnet`. Detection test `crypto-validators.test` asserts each fail blocks/scores/labels, each pass clean, and the pack no-ops when opt-in is off. 5-way parity oracle. Rows in TEST_PROOF_EXPECTATIONS.md.

## Parallel Ownership Notes
Depends on d01 (mechanization), d17 (Rust error handling), d18 (security-stop watchlist), h01 (classifier). Consumes h06 signing rules + h07 localnet adapter read-only. Owns `rules/crypto/**`, `src/validators/crypto-*.ts` and its fixtures exclusively; being OFF by default keeps it disjoint from all non-crypto lanes.
