# h07 Security Tooling CI And Observability

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Security Tooling CI And Observability`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling completion, product status, PR readiness, or broad DONE.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `adapters/ci/security-pipeline.*`, `src/harness/security-tooling-*.ts`, `tests/security-tooling/**`
- deps: `d01`, `a10`, `c01`
- tier: `P2/P4`

Sources: [PLAN_STATE](../PLAN_STATE.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md), [security-testing source](../refs/security-testing-source.md).

## Where We Are
The spec's §2 tooling, §5 pipeline, §2.7 observability and §4 coverage-fail live only as prose. No mechanized adapter wires coverage floors, API fuzz, property, concurrency, static or observability tools into CI, and nothing fails a run when a money path lacks a security log.

## Where We Want To Be
A generic, tool-agnostic security pipeline adapter that runs each stage via the run-harness + CI adapters, every tool graceful-skipping (honest, a09-style) when absent: coverage (c8/nyc + vitest/jest, floors >=90% line / >=80% branch, fail-CI-on-drop); API fuzz (Schemathesis/RESTler over OpenAPI); property (fast-check); concurrency (k6/Artillery); static (Semgrep/CodeQL/Trivy — signal-only/non-blocking unless mapped to an exploitable threat); observability (OpenTelemetry + correlation IDs on money paths, no-sampling on security events, property/fuzz failures logging counterexamples/seeds). Crypto localnet tooling (solana-test-validator/Anchor/Bankrun) is an OPTIONAL adapter enabled only when the optional crypto pack is on.

## Requirement Checklist
- [ ] T1: coverage below floor (line<90 / branch<80) or a drop fails CI.
- [ ] T2: money-critical path emitting no security log / correlation ID is scored and flagged.
- [ ] T1: a security event emitted under sampling (dropped) fails.
- [ ] Static findings stay signal-only unless threat-mapped (then blocking).
- [ ] Property/fuzz failures persist counterexample/seed; absent seed fails.
- [ ] Every tool graceful-skips (honest ran-count) when the binary is missing.

## Acceptance And Proof
Tier P2/P4. Fixtures per stage: `fail/coverage-below-floor` (fails), `pass/coverage-at-floor`; `fail/money-path-no-security-log` (T2), `pass/money-path-logged`; `fail/security-event-sampled`, `pass/security-event-unsampled`; `fail/fuzz-no-seed`, `pass/fuzz-with-seed`. Detection test `security-tooling-pipeline.test` asserts each fail fixture blocks/scores and each pass fixture is clean, plus graceful-skip ran-counts. 5-way parity oracle. Rows in TEST_PROOF_EXPECTATIONS.md.

## Parallel Ownership Notes
Depends on d01 (mechanization), a10 (self-CI gates), c01 (install/CLI contract). Consumes h01 classifier output read-only via the harness. Owns the CI security-pipeline adapter + harness tooling files + tests exclusively; the crypto localnet adapter hook is a disjoint opt-in seam consumed by e-pack-crypto-blockchain.
