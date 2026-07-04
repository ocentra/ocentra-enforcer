# arc-19 Crate enforcer-security

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Crate enforcer-security`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `crates/enforcer-security/**`
- deps: `arc-01`, `arc-02`, `arc-05`
- tier: `P1`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md), [RUST_ARCHITECTURE](../RUST_ARCHITECTURE.md).

## Where We Are
Track H money-critical and security-testing validators (financial-correctness checks, security-testing shapes) are specced but not implemented as a crate; adjacent detection exists only in the retired Node engine. There is no Track H crate, and no crate owns the no-bypass meta-check that guarantees the enforcer itself ships no inline-suppress escape hatch.

## Where We Want To Be
`enforcer-security` is the Track H crate per RUST_ARCHITECTURE.md: it stands up the crate SKELETON (`Cargo.toml`, `src/lib.rs`, the `Validator`-registration/module-root under `src/rules/`, and the no-bypass meta-check) for money-critical & security-testing `Validator` impls (built on `enforcer-validator`), each returning structured `Finding`s tagged with `ThreatId` (MITRE/OWASP) from `enforcer-domain`, plus fail/pass fixtures and a `cargo test` detection test. It hosts the no-bypass meta-check (the doctrine's `enforcer-config` is the SINGLE declarative control-plane; there is never an inline-disable) and is the home crate for the d18 security-stop family and the Track H rule families (h01-h08, h11) which own their own `src/rules/<name>.rs` modules atop this skeleton.

## Requirement Checklist
- [ ] Stand up the `enforcer-security` crate skeleton per RUST_ARCHITECTURE.md: `Cargo.toml` (opting into `[lints] workspace=true`), `src/lib.rs`, the `src/rules/` module-root, and the crate's `Validator` registration seam so feature packs (d18, h01-h08, h11) drop in `src/rules/<name>.rs`.
- [ ] Implement the no-bypass meta-check as a `Validator` in this crate: BAN inline lint-disable / validation-bypass directives across scanned code (e.g. `#[allow(...)]` on enforcer-governed lints, `// eslint-disable`, `# noqa`, `# type: ignore`, `@ts-ignore`, `clippy::allow` on the deny wall, and any ad-hoc suppress comment) — the enforcer ships NO inline-suppress escape hatch; the ONLY legitimate exemption path is a declarative, committed, gated waiver read from `enforcer-config` (owner+reason+ruleId), never an inline comment. Emit a structured `Finding` + terse `Fix:` hint pointing at the declarative waiver path.
- [ ] Implement the Track H money-critical validators per RUST_ARCHITECTURE.md (financial-correctness / money-handling rules), keyed to their `RuleId`s in `enforcer-rules`, as `Validator` impls returning `Finding`s (feature packs own the individual `src/rules/<name>.rs`).
- [ ] Implement the security-testing validators, tagging `ThreatId` (MITRE/OWASP) where applicable.
- [ ] Where an irreplaceable engine is needed (symbolic-exec/fuzz/network-scan) leave a graceful-skip adapter seam to `enforcer-harness` (arc-18) run-adapters, not an ad-hoc shell-out.
- [ ] Provide fail/pass fixtures per rule under `crates/enforcer-security/tests/fixtures/<rule>/{bad,good}/`; wire them through the `enforcer-validator` parity harness.
- [ ] `cargo test -p enforcer-security` passes: every validator (including the no-bypass meta-check) fires on its fail fixture and is silent on its pass fixture.
- [ ] Clean `cargo clippy` / `cargo fmt --check` (obey `[workspace.lints]`; no `unwrap/expect/panic/print_*`; no `pub use` barrels).

## Acceptance And Proof
Tier P1. Proof row asserts `cargo test -p enforcer-security` exits 0 with fail/pass fixture coverage per Track H rule and the no-bypass meta-check (fail fixture = code carrying an inline-suppress directive is flagged; pass fixture = same violation waived only via the declarative `enforcer-config` gated waiver). Record the artifact path in TEST_PROOF_EXPECTATIONS.md.

## Parallel Ownership Notes
Owns the `enforcer-security` crate SKELETON only: `crates/enforcer-security/Cargo.toml`, `crates/enforcer-security/src/lib.rs`, the `src/rules/` module-root + `Validator`-registration, and the no-bypass meta-check (`crates/enforcer-security/src/rules/no_bypass.rs` + its fixtures). Deps arc-01/02/05.

Parallel Ownership Note (disjoint feature packs): the d18 security-stop family and the Track H rule packs (h01 money-critical-classifier, h02-h06, h07, h08 policy-ingest/`src/policy_ingest.rs`, h11 cyberskills-corpus) each own SPECIFIC files under this crate — `crates/enforcer-security/src/rules/<name>.rs` (+ a `src/rules/<name>/` dir if needed) and `crates/enforcer-security/tests/fixtures/<name>/**` — NOT the whole crate; they `deps:` arc-19 and are sequenced after this skeleton exists. owns stay DISJOINT BY FILE. Parallel-safe with the lang crates (arc-06..13) and arc-14 — disjoint crate trees, all on the shared validator base. Distinct from arc-10 (`enforcer-lang-security` source-pattern family; d18 also lands a source-pattern half there per the mapping — this crate owns the money-critical & no-bypass slice, arc-10 owns the source-pattern slice).
