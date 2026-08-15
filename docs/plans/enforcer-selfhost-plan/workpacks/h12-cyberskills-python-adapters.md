# h12 Cyberskills Python Adapters

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Cyberskills Python Adapters`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling completion, product status, PR readiness, or broad DONE.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `crates/enforcer-harness/adapters/cyberskills/**, crates/enforcer-harness/src/adapters/cyberskills/**, crates/enforcer-harness/tests/fixtures/cyberskills_adapters/**, crates/enforcer-harness/tests/cyberskills_adapters.rs`
- deps: `d01, arc-18, f05, h11`
- tier: `P2/P4`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md), [RUST_ARCHITECTURE](../RUST_ARCHITECTURE.md), [vendor analysis](../../../../vendor/anthropic-cybersecurity-skills/RUST_CONVERSION_ANALYSIS.md).

Execution refinement: [CyberSkills Parity Plan](../../cyberskills-parity-plan/README.md) supersedes the one-wrapper-per-skill interpretation. Universal UL07 deepens the shared typed/allowlisted `enforcer-harness` process contract; CP06-CP10 add CyberSkills conformance requirements, one adapter per real engine or stable output protocol, and bounded skill-to-engine mappings. This h12 document remains historical scope context; live external runners are not yet proved.

## Where We Are
`h11` reimplements the FUNDAMENTAL-LOGIC cyberskills (regex/predicate/manifest) as native Rust `Validator`s and drops their CLI dependency. But ~15-20% of skill cores are genuinely PYTHON/TOOL-BOUND: the engine has NO Rust equivalent — symbolic execution / formal analysis (mythril, slither, foundry/forge), network scanners (nmap, nessus, openvas, nikto), fuzzers / exploit frameworks (sqlmap), binary & memory forensics (volatility, ghidra, apktool, MobSF, chipsec, peepdf, autopsy), and cloud-SDK live-inventory fetchers (boto3, azure-mgmt-*, google-cloud-*). These cannot be reimplemented in Rust and MUST NOT be forced into our dogfood. `enforcer-harness` (arc-18) already provides the graceful-skip run-adapter seam, but no adapter seam yet lets these irreplaceable engines run as OPTIONAL, out-of-dogfood tools whose findings still feed a thin gate. This is the ONE place a subprocess touch point is legitimate: the ENGINE is external (validating a user's target via an irreplaceable tool), not the enforcer being Python — the enforcer stays a pure-Rust binary that merely shells out through the harness.

## Where We Want To Be
A small set of OPTIONAL run-adapters wired through `enforcer-harness` (arc-18), each wrapping one irreplaceable external engine, that:
1. Live as external adapter scripts under `crates/enforcer-harness/adapters/cyberskills/**` (out-of-dogfood tool wrappers) driven by thin Rust run-adapter modules under `crates/enforcer-harness/src/adapters/cyberskills/**` (the `enforcer-harness` code that invokes them, parses their output into `enforcer-domain` `Finding`s, and applies the gate). Both trees are EXCLUDED from the enforcer's own dogfood (covered by the `vendor/**` + `adapters/cyberskills/**` exempt globs in the enforcer's committed policy read by `enforcer-config`).
2. GRACEFUL-SKIP honestly (a09-style, honest ran-count via the arc-18 skip seam) when the binary/lib is absent — never a hard failure, never a silent pass; a present-but-erroring tool surfaces the error.
3. Emit findings in the same `enforcer-domain::Finding` shape `f05`/`g02` consume so the RESULT (CVE list, CIS findings, benchmark score, contract-weakness taxonomy) feeds a thin T1/T2 severity gate.
4. Are wired behind `f05`'s security-audit scope (in `enforcer-scan`) as OPTIONAL native ties (run only when the tool is present and the scope opts in).
The SDK-fetcher half-bound case keeps ONLY the fetch in the adapter and feeds the fetched JSON into an h11 Rust `Validator` (offline-capable) rather than duplicating the predicate here. Each gate is scaffolded via `d01` so the finding-to-gate mapping carries doc + fixtures + a `cargo test` detection test. The Rust adapter modules obey `[workspace.lints]` (no `unwrap/expect/panic/print_*`; no `pub use` barrels). GENERIC over engines; no specific tool is assumed installed. Build this pack ONLY as the (d) engine-bound skills are actually needed — it is the deferred, opt-in complement to h11. (Honest graceful-skip note: if no engine adapter is warranted yet, the pack lands only the seam + the graceful-skip test and records the skip honestly, never a fake pass.)

## Requirement Checklist
Each adapter is a thin wrapper; the GATE over its output is a Rust `Validator`/gate scaffolded via `d01` so the finding-to-gate mapping carries doc + fixtures + a `cargo test` detection test. No adapter is on the dogfood path.

- [ ] **Dogfood exclusion (fail-closed).** `adapters/cyberskills/**` is covered by the enforcer's committed exempt-globs policy read by `enforcer-config` (alongside `vendor/**` from h11); a test asserts an `enforcer-scan` self-host scan yields zero findings from this tree.
- [ ] **Graceful-skip (honest, a09-style).** Each adapter, when its binary/lib is missing, returns a skip with an honest ran-count (`skipped != passed != failed`) via the arc-18 skip seam; a present-but-erroring tool surfaces the error, never a silent pass. Test both the absent-tool and error-tool paths.
- [ ] **Finding-shape parity.** Adapter output maps to the same `enforcer-domain::Finding` shape `f05`/`g02` consume (`RuleId`/`Severity`/`RelPath` location/`ThreatId` citation where the engine provides one), so a severity gate can act on it identically to a native rule.
- [ ] **Thin severity gate (scaffolded).** For each adapter, a d01-scaffolded T1/T2 gate turns the tool's result into a pass/fail or scored `Finding` (e.g. `adapter.sca.cve-severity-threshold`, `adapter.k8s.cis-benchmark-fail`, `adapter.contract.high-severity-weakness`). The engine stays external; only the gate is ours (Rust).
- [ ] **f05 optional native-tie.** Adapters attach to `f05`'s security-audit scope (in `enforcer-scan`) as OPTIONAL ties (run native AND ours only when present); absence narrows the plan, it does not block. Consumes f05's route-plan shape; does not reimplement the router.
- [ ] **SDK half-bound handling.** For cloud-SDK fetchers, the adapter fetches state only; the PREDICATE over that state is an h11 Rust `Validator` fed generic JSON — no predicate is duplicated in Python here.
- [ ] **No fundamental logic here.** Any skill whose core is a regex/predicate/manifest check belongs in h11, not this pack; this pack contains only genuinely engine-bound wrappers.
- [ ] Clean `cargo clippy` / `cargo fmt --check` on the Rust adapter modules.

## Acceptance And Proof
Tier P2 (scored gates over tool output) / P4 (harness/CI adapter behavior). Prove via `cargo test -p enforcer-harness` (`crates/enforcer-harness/tests/cyberskills_adapters.rs`). Fixtures under `crates/enforcer-harness/tests/fixtures/cyberskills_adapters/<adapter>/` use RECORDED tool output (no live engine required in CI): a captured findings JSON plus its expected gate verdict.

Representative triples:
- graceful-skip: fail `crates/enforcer-harness/tests/fixtures/cyberskills_adapters/slither/bad/tool_absent_reported_pass.json` (an adapter that silently passes when the binary is missing -> flagged as dishonest), pass `.../good/tool_absent_skipped.json` (honest skip with ran-count) plus `.../good/tool_present_findings.json` (present -> real findings), `#[test] cyberskills_adapter_graceful_skip`.
- severity gate: fail `.../sca/bad/high_cve_over_threshold.json` (recorded snyk/grype output with a HIGH CVE -> gate fails), pass `.../sca/good/below_threshold.json`, `#[test] cyberskills_adapter_severity_gate`.
- finding-shape parity: a detection test asserts every adapter finding carries the fields `g02` renders (`RuleId`/`Severity`/`RelPath` location) and, where the engine supplies it, a valid ATT&CK/NIST `ThreatId` citation (h03-validated).
- dogfood exclusion: `#[test] cyberskills_adapters_not_dogfooded` asserts an `enforcer-scan` self-host scan of `adapters/cyberskills/**` returns zero findings.

Detection tests run every recorded-output fixture through the gate (fail-flagged / pass-clean/skip) and run the d01 parity oracle over the adapter-gate `RuleId`s. Named proof rows in TEST_PROOF_EXPECTATIONS.md: `cyberskills-adapter-graceful-skip`, `cyberskills-adapter-severity-gate`, and `cyberskills-adapters-dogfood-exclusion`.

## Parallel Ownership Notes
`owns:` is disjoint BY FILE: `crates/enforcer-harness/adapters/cyberskills/**` (external tool-wrapper scripts), `crates/enforcer-harness/src/adapters/cyberskills/**` (the Rust run-adapter modules), `crates/enforcer-harness/tests/fixtures/cyberskills_adapters/**`, and `crates/enforcer-harness/tests/cyberskills_adapters.rs` are new paths inside the `enforcer-harness` crate whose SKELETON arc-18 owns — must NOT edit that skeleton or the sibling harness feature modules (h07 `security_pipeline.rs`, d11 `ci_parity.rs`, d28 `target_ci_parity.rs`). The only shared edit is extending the enforcer's committed exempt-globs policy read by `enforcer-config` (additive `adapters/cyberskills/**` entry — coordinate with h11 which adds `vendor/**`). Depends on `d01` (scaffold the gates), `arc-18` (the `enforcer-harness` run-adapter + graceful-skip skeleton — sequences this after it exists), `f05` (optional native-tie into the security-audit scope — consumed, not redefined), and `h11` (this pack is the deferred complement: h11 owns the native Rust rules and the vendor dogfood-exclusion + vocab; h12 owns only the optional python/CLI wrappers for the irreplaceable engines). This pack introduces the ONLY subprocess/python touch points in the cyberskills conversion, and they are all off the dogfood path and graceful-skipping — the enforcer engine itself stays pure Rust. If the analysis for a given engine later shows a Rust equivalent exists, that skill moves to h11 and its adapter is removed here. `owns disjoint? = Y` (deps arc-18 sequences it after the crate skeleton; deps h11 after the native rules + exempt-glob entry exist).
