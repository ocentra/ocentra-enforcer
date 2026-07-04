# Money-Critical & Security-Testing Mandate (ingested reference spec)

GENERIC domain: ANY system handling money / payments / value behind UNTRUSTED infrastructure — Cloudflare,
AWS, API gateways are NOT security boundaries; internal APIs are hostile. NOT game- or crypto-specific.
Crypto/blockchain (e.g. Solana/Anchor) is ONE OPTIONAL instance of a money-critical domain, never assumed.
Ingested from a project security-testing .mdc (provenance kept generic — no product branding). This is the
"HOW to test security" spec, PROSE today (hope the agent obeys); Track H mechanizes it into T1/T2 rules per
our tested-enforcement doctrine. Its §8 IS our doctrine (AI-output-untrusted, no-exceptions,
silence≠permission, fail-CI-by-default).

SIBLING DEPENDENCY (not yet supplied — refines per-category numbered rules):
- `ocentra-security-rules.mdc` — guarantees G1–G5 + Rules 0.1.1–15.x.
- `ocentra-test-rules.mdc` — test-quality §0.2/§7/§11/§20.
Track H mechanizes the fully-enumerated content below now; per-rule numbered detail expands when siblings land.

## Money-critical code (§8.2) — "if unsure, treat as money-critical"
Creates/transfers/modifies/destroys value · triggers Solana instructions · signs messages · economic
calculations · rewards/credits/balances/cooldowns · rollbacks/compensation · time-based state · kill switches.

## Invariants (§2.3) — the economic/logic guarantees (map to G1–G6)
same-request-twice ≠ more value · failure ≠ reward · retry ≠ mutation · partial-failure ≠ profit ·
order-of-independent-actions ≠ advantage · attacker-cost ≥ system-cost · compensation idempotent+replay-safe ·
time-assumptions fail-closed · backend never signs what it can't independently verify · emergency controls
reduce blast radius.

## Threat model (§0.5)
Attacker: full client control, replay/mutate/automate, timing control, arbitrary signatures, tx spam,
partial-failure exploit, forged internal headers, serialization manipulation. Rejected assumptions: client
clock accurate, client randomness unpredictable, latency bounded, RPC honest, internal services trusted,
error handlers safe, rollback correct-by-default, time-logic safe-by-default, admin actions safe.
RULE: every threat MUST map to ≥1 property test + ≥1 integration test + ≥1 fuzz test (where applicable);
unmappable threat = incomplete threat model.

## Required test categories (§3.1–3.20)
auth/authz · CORS/origin · input-validation/schema · URL/path manipulation · replay/idempotency ·
partial-failure exploitation · state/logic abuse · DDoS/resource exhaustion · Solana/blockchain abuse ·
request-smuggling/protocol · API/WebSocket · error-handling/info-leak · key/signing · time/clock/drift ·
economic exhaustion (distinct from DDoS) · rollback/recovery abuse · cross-component trust · serialization/
encoding · kill-switch/circuit-breaker · test-integrity (meta).

## CI MUST FAIL IF (§4, §8.3)
Any money-critical endpoint lacks: negative | replay | concurrency | rollback/compensation | economic-
exhaustion | time-based | signing/verification tests. Any invariant test fails. Schemathesis finds new
failing cases. Coverage drops (≥90% line, ≥80% branch). Kill-switch tests missing. Test-integrity violations
(non-deterministic, shared state, order-dependent). Property tests fail w/o counterexample. Fuzz fails w/o seed.

## CI pipeline (§5): deps → unit+coverage → property(+counterexamples) → API fuzz(+seeds) → concurrency →
Solana localnet(+signing-abuse) → static → observability-hooks → rollback/compensation → economic-exhaustion →
time-based → test-integrity(order/no-shared-state) → threat→test mapping completeness.

## Banned test patterns (§7.2/§8.4.1) → CI FAIL
assert-success-only · pass-if-logic-deleted · rely-on-no-crash · snapshot-only · no-threat-mapping ·
no-invariant-assertion · non-deterministic-fuzz · order-dependent · global-mutation · mocks-for-money-logic.
Required test properties (§7.1/§8.4.2): asserts rejection (not success) · asserts exact failure mode ·
reproducible (seed logged) · fails if protection removed · maps to a threat · asserts an invariant.

## Mechanical enforcement rules (§8.6–8.10)
Signing: backend must NOT sign client-raw / non-reconstructable / non-verifiable messages; signed messages
canonically serialized, reproducible from request context, logged with correlation IDs.
Time: client time never trusted; server time only; explicit clock-skew tolerance; expiry fails-closed.
Economic: attacker-cost ≥ system-cost; no free retries with non-zero backend cost; dust ops bounded;
micro-reward farming explicitly tested.
Rollback/compensation: idempotent, replay-safe, atomic, exactly-once; untested rollback = merge forbidden.
Internal boundary: internal APIs hostile; internal headers untrusted; topology gives ZERO security;
unauthed internal endpoint = CI fail.
Kill-switch: halt all money-critical ops, atomic across services, authenticated, audited, replay-safe;
untested kill-switch = merge forbidden.

## Tooling (§2)
coverage c8/nyc + vitest/jest · API fuzz Schemathesis + RESTler (OpenAPI) · property fast-check ·
concurrency k6/Artillery · Solana solana-test-validator + Anchor + Bankrun · static Semgrep/CodeQL/Trivy
(signal-only, don't block unless mapped to exploitable threat) · observability OpenTelemetry + correlation IDs.

## Solana/Anchor coverage (OPTIONAL crypto instance — §1, §2.5)
instruction execution paths · replay · slot-timing edges · CPI boundaries · simulation-vs-execution mismatch ·
key-lifecycle abuse · signing abuse (blind-sign, wrong-message) · nonce/blockhash reuse · signer order/
injection · PDA security · Anchor account validation · program state transitions.

## Enforcement meta (§8.1, §8.12, §8.14)
This spec overrides convenience/AI-suggestions/deadlines. AI is NOT an authority — cannot waive/simplify/
remove tests, cannot replace invariants with mocks, cannot downgrade security tests to unit tests; AI output
is untrusted input. Silence ≠ permission. "If a change cannot be proven safe under adversarial conditions,
it is unsafe."
