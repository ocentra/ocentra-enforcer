---
name: security-testing
description: Money-critical & security-testing mandate walkthrough — route, classify, threat-map, invariants, mechanics, and CI tooling for any system handling money/payments/value behind untrusted infrastructure. Use when a change touches value transfer, signing, time-based state, rollback/compensation, or kill switches.
---

# Security Testing (Money-Critical Mandate)

<!-- ai-dense -->
```yaml
tier: "T3 advisory — no mechanization possible: this SKILL sequences human/agent judgment across
  Track H (h01-h07); it produces no Finding and gates nothing. Mechanized enforcement lives in the
  h01-h07 Rust Validator rows and the profiles/money-critical-security.json profile, not here."
stages: "route -> h01 classify -> h02 required-categories -> h03 threat-map -> h05 invariants -> h06 mechanics -> h07 tooling/CI"
domain: "GENERIC: any system handling money/payments/value behind untrusted infra (Cloudflare/AWS/API
  gateways are NOT security boundaries; internal APIs are hostile). NOT game- or crypto-specific;
  crypto/blockchain is one OPTIONAL instance, never assumed."
doctrine: "AI output is untrusted input. Silence != permission. If a change cannot be proven safe
  under adversarial conditions, it is unsafe. AI cannot waive/simplify/remove tests or downgrade
  security tests to unit tests."
mechanized_home: "profiles/money-critical-security.json (neutral, no branding) + crates/enforcer-security/src/policy_ingest.rs (ingest a project's own spec into that profile shape)"
```
<!-- /ai-dense -->

This SKILL is prose walkthrough, not an enforcement engine. It carries the
label above deliberately: nothing in this file emits a `Finding`, and
nothing in this file gates a merge. The T1/T2 mechanized gates live in the
Track H `Validator` rows (`crates/enforcer-security/src/rules/*.rs`) and the
loadable profile (`profiles/money-critical-security.json`). Use this SKILL
to decide WHICH of those gates apply to the change in front of you, in
order.

## When To Use This

Any change that creates, transfers, modifies, or destroys value; triggers a
blockchain instruction; signs a message; computes an economic/reward/credit/
balance/cooldown value; performs a rollback or compensation; depends on
time-based state; or touches a kill switch. If unsure whether a unit is
money-critical, treat it as money-critical (fail-closed default, doctrine
§8.2).

## The Seven Stages

1. **Route.** Identify which money-critical surface the change touches
   (a value-transfer path, a signing path, a time-gated path, a rollback
   path, a kill switch). This determines which later stages apply.
2. **Classify (h01).** Decide, and get the change explicitly annotated,
   as money-critical or not. An unannotated value-touching unit is a T1
   flag under `MONEY-CRIT-ANNOTATED.1`; the T2 `MONEY-CRIT-CLASSIFY.1`
   scorer catches borderline cases heuristically.
3. **Required categories (h02).** Every money-critical unit needs test
   coverage across the seven baseline categories: negative, replay,
   concurrency, rollback/compensation, economic-exhaustion, time-based,
   signing/verification — and, per the full spec, all twenty categories in
   `profiles/money-critical-security.json`'s `requiredTestCategories`.
4. **Threat-map (h03).** Every classified unit must map to at least one
   threat, at least one invariant, and at least one property/concurrency/
   replay test. A classified unit absent from the threat map, or a threat
   with zero tests, is forbidden — "unmapped logic is forbidden logic."
5. **Invariants (h05).** Confirm the ten economic/logic invariants in
   `profiles/money-critical-security.json`'s `invariants` list are each
   backed by a property-based test (a generator-driven refutation, not a
   single hand-picked literal case).
6. **Mechanics (h06).** Check the per-facet mechanical rules: signing
   (never sign client-raw/non-reconstructable payloads), time (server time
   only, fail-closed expiry), economic (attacker-cost >= system-cost, no
   free retries), rollback (idempotent/replay-safe/atomic), boundary
   (internal APIs are hostile, no trusted internal headers), kill-switch
   (halt-all/atomic/authenticated/audited/replay-safe, tested).
7. **Tooling/CI (h07).** Confirm coverage floors (>=90% line, >=80%
   branch), fuzz/property seeds are persisted and reproducible, security
   events are logged with correlation IDs and not sampled away, and static
   findings stay signal-only unless mapped to an exploitable threat.

## Ingesting A Project's Own Spec

If the target project has its own security/testing `.mdc` spec doc (per
the "target repo owns policy" convention), do not manually re-derive a
profile from it. Use `crates/enforcer-security/src/policy_ingest.rs`
(`parse_spec` + `map_to_profile`) to turn that spec into a
`MechanizedProfile`. Two outcomes:

- A rule the spec asserts that a real mechanized `Validator` already
  backs becomes an ENABLED row (`backed: true`) — actually enforced at the
  tier the spec claims.
- A rule the spec asserts with no backing validator yet becomes a
  `backed: false` row AND a `Finding` flagging it for mechanization (fed
  to d01's rule-scaffold engine). It is never silently treated as
  enforced just because the spec says so — that would be fake-green.

This honesty seam is the point of ingestion: an un-mechanized assertion
must stay visibly un-enforced until someone builds the validator for it.
