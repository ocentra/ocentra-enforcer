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

- owns: `crates/enforcer-harness/src/security_pipeline.rs, crates/enforcer-harness/src/security_pipeline/**, crates/enforcer-harness/tests/security_pipeline.rs, crates/enforcer-harness/tests/fixtures/security_pipeline/**`
- deps: `d01, arc-18, a10, c01`
- tier: `P2/P4`

Sources: [PLAN_STATE](../PLAN_STATE.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md), [security-testing source](../refs/security-testing-source.md), [RUST_ARCHITECTURE](../RUST_ARCHITECTURE.md).

## Where We Are
The spec's §2 tooling, §5 pipeline, §2.7 observability and §4 coverage-fail live only as prose. `enforcer-harness` (arc-18) already provides Rust run-adapters for native tools (cargo/tsc/ruff/dart/CFLint...) + compact diagnostics + a graceful-skip seam, but no `security_pipeline` module wires the security-testing stages (coverage floors, API fuzz, property, concurrency, static, observability) into CI, and nothing fails a run when a money path lacks a security log. The retired Node engine had no such pipeline.

## Where We Want To Be
A generic, tool-agnostic security pipeline built as Rust run-adapters in `crates/enforcer-harness/src/security_pipeline.rs` (+ per-stage modules under `src/security_pipeline/`), each stage invoked through the arc-18 run-adapter/compact-diagnostic machinery and each tool GRACEFUL-SKIPPING honestly (a09-style honest ran-count, `skipped != passed != failed`) when its binary/lib is absent. Stages, each parsing tool output into `enforcer-domain` `Finding`s/diagnostics:
- coverage (c8/nyc + vitest/jest for TS targets; `cargo llvm-cov`/`tarpaulin` for Rust targets): floors >=90% line / >=80% branch, fail-CI-on-drop.
- API fuzz (Schemathesis/RESTler over OpenAPI); property (fast-check / proptest); concurrency (k6/Artillery).
- static (Semgrep/CodeQL/Trivy) — signal-only/non-blocking unless mapped to an exploitable `ThreatId`.
- observability (OpenTelemetry + correlation IDs on money paths, no-sampling on security events; property/fuzz failures logging counterexamples/seeds).
Crypto localnet tooling (solana-test-validator/Anchor/Bankrun) is an OPTIONAL adapter under `src/security_pipeline/crypto_localnet.rs`, enabled only when the optional crypto pack (e-pack-crypto-blockchain) is on and consumed by it read-only. The pack scaffolds each stage's pass/fail GATE through d01 so the finding-to-gate mapping carries doc + fixtures + a `cargo test` detection test. Obeys `[workspace.lints]` (no `unwrap/expect/panic/print_*`; no `pub use` barrels).

## Requirement Checklist
- [ ] T1: coverage below floor (line<90 / branch<80) or a drop emits a `Finding` that fails CI.
- [ ] T2: a money-critical path emitting no security log / correlation ID is scored (score + confidence) and flagged.
- [ ] T1: a security event emitted under sampling (dropped) emits a `Finding`.
- [ ] Static findings stay signal-only (diagnostic, non-blocking) unless threat-mapped to an exploitable `ThreatId` (then blocking).
- [ ] Property/fuzz failures persist counterexample/seed; an absent seed emits a `Finding`.
- [ ] Every stage graceful-skips honestly (honest ran-count via the arc-18 skip seam) when the binary/lib is missing — never a hard failure, never a silent pass; a present-but-erroring tool surfaces the error.
- [ ] The crypto-localnet adapter is a disjoint opt-in seam (`src/security_pipeline/crypto_localnet.rs`), off unless e-pack-crypto-blockchain enables it; its absence narrows the plan, never blocks.
- [ ] Runs both as a local `enforcer` check (through the arc-18 run-adapters) and as a CI job (self-referential). Clean `cargo clippy` / `cargo fmt --check`.

## Acceptance And Proof
Tier P2/P4. Prove via `cargo test -p enforcer-harness` (`crates/enforcer-harness/tests/security_pipeline.rs`) over `crates/enforcer-harness/tests/fixtures/security_pipeline/**` using RECORDED tool output (no live engine required in CI): a captured stage-output sample plus its expected gate verdict.
- coverage: fail `security_pipeline/coverage/bad/below_floor.json` (line<90 -> gate fails); pass `.../good/at_floor.json`.
- observability money-path log (T2): fail `security_pipeline/observability/bad/money_path_no_security_log.*` (scored + flagged); pass `.../good/money_path_logged.*`.
- security event sampled: fail `security_pipeline/observability/bad/security_event_sampled.*`; pass `.../good/security_event_unsampled.*`.
- fuzz seed: fail `security_pipeline/fuzz/bad/no_seed.json`; pass `.../good/with_seed.json`.
- graceful-skip: a missing-tool fixture yields an honest skip with a ran-count (`skipped != passed`), and a silently-passing adapter is itself flagged as dishonest.
Detection test `#[test] security_pipeline` asserts each fail fixture blocks/scores and each pass fixture is clean, plus graceful-skip ran-counts. 5-way parity oracle over every gate `RuleId`. Record artifact paths in TEST_PROOF_EXPECTATIONS.md.

## Parallel Ownership Notes
`owns:` is disjoint BY FILE: `crates/enforcer-harness/src/security_pipeline.rs`, its `src/security_pipeline/**` stage modules (including `crypto_localnet.rs`), `crates/enforcer-harness/tests/security_pipeline.rs`, and `crates/enforcer-harness/tests/fixtures/security_pipeline/**` are new paths inside the `enforcer-harness` crate whose SKELETON arc-18 owns (run-adapters + compact diagnostics) — must NOT edit that skeleton or the sibling harness feature modules d11 (`ci_parity.rs`) / d28 (`target_ci_parity.rs`). Depends on `d01` (mechanization — scaffolds each stage gate), `arc-18` (crate skeleton — sequences this after the run-adapter base exists), `a10` (self-CI gates — the CI job that runs this stage), and `c01` (install/CLI contract — the `enforcer` binary surface that invokes it). Consumes the h01 `enforcer-security` classifier output read-only via the harness to know which paths are money-critical for the observability stage. The crypto-localnet adapter seam is a disjoint opt-in consumed by e-pack-crypto-blockchain, which is OFF by default. `owns disjoint? = Y` (deps arc-18 sequences it after the crate skeleton exists).
