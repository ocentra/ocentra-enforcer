# h01 Money-Critical Classifier

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Money-Critical Classifier`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling completion, product status, PR readiness, or broad DONE.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `rules/security/money-critical.md, src/validators/money-critical-*.ts, tests/fixtures/money-critical/**`
- deps: `d01`
- tier: `P0/P1`

Sources: [PLAN_STATE](../PLAN_STATE.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md), [security-testing source](../refs/security-testing-source.md).

## Where We Are
The registry has no notion of "money-critical code". The ingested spec (§8.2) defines it in prose — "if unsure, treat as money-critical" — but nothing detects or gates it. Every downstream security-testing rule (h02 required categories, h03 threat/invariant mapping) needs a mechanical answer to "is this unit money-critical?" and there is none. Value handling today is invisible to the enforcer.

## Where We Want To Be
A foundational classifier family scaffolded via d01 with full 5-way parity: doc `rules/security/money-critical.md`, validators `src/validators/money-critical-*.ts`, fixtures `tests/fixtures/money-critical/**`. A T2 scored classifier tags any unit that creates/transfers/modifies/destroys value; performs economic calculation; applies rewards/credits/balances/cooldowns; signs or authorizes payments; executes rollback/compensation; changes time-based state; or toggles kill-switches — GENERICALLY across any value system (fiat, Stripe, AWS-billed metering, internal ledger, or the optional crypto/Anchor instance), never crypto-only. A T1 gate then requires every classified unit to be explicitly annotated/registered in a money-critical manifest; unannotated-but-classified code fails CI. Doctrine: silence ≠ permission; if unsure, treat as money-critical.

## Requirement Checklist
Scaffolded with `enforcer rule new <ID>` (d01), landing doc anchor, validators, and fail+pass fixtures.

- [ ] **T2 MONEY-CRIT-CLASSIFY — scored classifier (§8.2).** Score units on the enumerated value-touching signals (balance/credit/reward/cooldown mutation, transfer/mint/burn, economic calc, payment sign/authorize, rollback/compensation, time-based state change, kill-switch toggle). Crossing threshold ⇒ classified money-critical.
- [ ] **T1 MONEY-CRIT-ANNOTATED — annotation/registration gate (§8.2).** A classified unit MUST carry an explicit annotation and appear in the money-critical manifest; classified-but-unannotated ⇒ fail.
- [ ] **T1 MONEY-CRIT-UNSURE-DEFAULT — "if unsure, treat as money-critical".** Ambiguous value-adjacent units default to money-critical unless explicitly annotated otherwise.

## Acceptance And Proof
Tier P0/P1. T2 classifier fixtures assert the score crosses the fail threshold for value-touching code and stays under it for neutral code; the T1 gate fixtures assert flag on unannotated classified units and clean on annotated+registered ones.

Representative triples:
- balance-crediting fn: fail `tests/fixtures/money-critical/classify/fail_credit_balance.ts` (classified, unannotated → flagged), pass `.../pass_credit_balance_annotated.ts`, test `money-crit-classify.test`.
- pure formatter: fail-negative `tests/fixtures/money-critical/classify/pass_pure_formatter.ts` (below threshold, not classified, must stay clean), asserted in `money-crit-classify.test`.
- payment-signing fn unannotated: fail `tests/fixtures/money-critical/annotated/fail_sign_payment_unannotated.ts` (T1 flag), pass `.../pass_sign_payment_registered.ts`, test `money-crit-annotated.test`.

Re-run the d01 `rule-scaffold-parity` oracle; record detection-test artifact paths in TEST_PROOF_EXPECTATIONS.md.

## Parallel Ownership Notes
Owns `rules/security/money-critical.md`, `src/validators/money-critical-*.ts`, and `tests/fixtures/money-critical/**` exclusively; disjoint from siblings. Depends on d01. This pack is foundational: h02, h03, and h04–h06 key off the money-critical manifest it produces but must not redefine classification — they consume the manifest and own their own rule/fixture surfaces. Distinct from d18 security-stop (vulnerability patterns) which does not classify value.
