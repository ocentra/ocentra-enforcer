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

- owns: `adapters/cyberskills/**, src/harness/cyberskills-adapter-*.ts, tests/fixtures/cyberskills-adapters/**, tests/cyberskills-adapters/**`
- deps: `d01, f05, h11`
- tier: `P2/P4`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md), [vendor analysis](../../../../vendor/anthropic-cybersecurity-skills/RUST_CONVERSION_ANALYSIS.md).

## Where We Are
`h11` reimplements the FUNDAMENTAL-LOGIC cyberskills (regex/predicate/manifest) as native Rust and drops their CLI dependency. But ~15-20% of skill cores are genuinely PYTHON/TOOL-BOUND: the engine has NO Rust equivalent — symbolic execution / formal analysis (mythril, slither, foundry/forge), network scanners (nmap, nessus, openvas, nikto), fuzzers / exploit frameworks (sqlmap), binary & memory forensics (volatility, ghidra, apktool, MobSF, chipsec, peepdf, autopsy), and cloud-SDK live-inventory fetchers (boto3, azure-mgmt-*, google-cloud-*). These cannot be reimplemented in Rust and MUST NOT be forced into our dogfood. There is today no adapter seam that lets these run as OPTIONAL, out-of-dogfood tools whose findings still feed a thin gate.

## Where We Want To Be
A small set of OPTIONAL python/CLI run-adapters, each wrapping one irreplaceable engine, that: (1) live under `adapters/cyberskills/**` and are EXCLUDED from the enforcer's own dogfood (covered by the `vendor/**` + `adapters/cyberskills/**` ignore globs), (2) GRACEFUL-SKIP honestly (a09-style, honest ran-count) when the binary/lib is absent — never a hard failure, never a silent pass, (3) emit findings in the same finding shape `f05`/`g02` consume so the RESULT (CVE list, CIS findings, benchmark score, contract-weakness taxonomy) feeds a thin T1/T2 severity gate, and (4) are wired behind `f05`'s security-audit scope as OPTIONAL native ties (run only when the tool is present and the scope opts in). The SDK-fetcher half-bound case is handled by keeping ONLY the fetch in the adapter and feeding the fetched JSON into an h11 Rust predicate (offline-capable) rather than duplicating the predicate here. GENERIC over engines; no specific tool is assumed installed. Build this pack ONLY as the (d) engine-bound skills are actually needed — it is the deferred, opt-in complement to h11.

## Requirement Checklist
Each adapter is a thin wrapper; the GATE over its output is scaffolded via `d01` so the finding-to-gate mapping carries doc + fixtures + detection test. No adapter is on the dogfood path.

- [ ] **Dogfood exclusion (fail-closed).** `adapters/cyberskills/**` is covered by `ocentra-enforcer.config.json` `ignoreFileGlobs` (alongside `vendor/**` from h11); a test asserts a self-host scan yields zero findings from this tree.
- [ ] **Graceful-skip (honest, a09-style).** Each adapter, when its binary/lib is missing, returns a skip with an honest ran-count (skipped != passed, skipped != failed); a present-but-erroring tool surfaces the error, never a silent pass. Test both the absent-tool and error-tool paths.
- [ ] **Finding-shape parity.** Adapter output maps to the same finding shape `f05`/`g02` consume (ruleId/severity/location/threat-citation where the engine provides one), so a severity gate can act on it identically to a native rule.
- [ ] **Thin severity gate (scaffolded).** For each adapter, a d01-scaffolded T1/T2 gate turns the tool's result into a pass/fail or scored finding (e.g. `adapter.sca.cve-severity-threshold`, `adapter.k8s.cis-benchmark-fail`, `adapter.contract.high-severity-weakness`). The engine stays external; only the gate is ours.
- [ ] **f05 optional native-tie.** Adapters attach to `f05`'s security-audit scope as OPTIONAL ties (run native AND ours only when present); absence narrows the plan, it does not block. Consumes f05's route-plan shape; does not reimplement the router.
- [ ] **SDK half-bound handling.** For cloud-SDK fetchers, the adapter fetches state only; the PREDICATE over that state is an h11 Rust rule fed generic JSON — no predicate is duplicated in Python here.
- [ ] **No fundamental logic here.** Any skill whose core is a regex/predicate/manifest check belongs in h11, not this pack; this pack contains only genuinely engine-bound wrappers.

## Acceptance And Proof
Tier P2 (scored gates over tool output) / P4 (harness/CI adapter behavior). Fixtures under `tests/fixtures/cyberskills-adapters/<adapter>/` use RECORDED tool output (no live engine required in CI): a captured findings JSON plus its expected gate verdict.

Representative triples:
- graceful-skip: fail `tests/fixtures/cyberskills-adapters/slither/fail_tool_absent_reported_pass.json` (an adapter that silently passes when the binary is missing -> flagged as dishonest), pass `.../pass_tool_absent_skipped.json` (honest skip with ran-count) plus `.../pass_tool_present_findings.json` (present -> real findings), test `cyberskills-adapter-graceful-skip.test`.
- severity gate: fail `.../sca/fail_high_cve_over_threshold.json` (recorded snyk/grype output with a HIGH CVE -> gate fails), pass `.../sca/pass_below_threshold.json`, test `cyberskills-adapter-severity-gate.test`.
- finding-shape parity: a detection test asserts every adapter finding carries the fields `g02` renders (ruleId/severity/location) and, where the engine supplies it, a valid ATT&CK/NIST citation (h03-validated).
- dogfood exclusion: `cyberskills-adapters-not-dogfooded` asserts a self-host scan of `adapters/cyberskills/**` returns zero findings.

Detection tests run every recorded-output fixture through the gate (fail-flagged / pass-clean/skip) and run the d01 parity oracle over the adapter-gate ids. Named proof rows in TEST_PROOF_EXPECTATIONS.md: `cyberskills-adapter-graceful-skip`, `cyberskills-adapter-severity-gate`, and `cyberskills-adapters-dogfood-exclusion`.

## Parallel Ownership Notes
`owns:` is disjoint: `adapters/cyberskills/**`, `src/harness/cyberskills-adapter-*.ts`, `tests/fixtures/cyberskills-adapters/**`, `tests/cyberskills-adapters/**` are new paths; the only shared edit is extending the `ignoreFileGlobs` array in `ocentra-enforcer.config.json` (additive `adapters/cyberskills/**` entry — coordinate with h11 which adds `vendor/**`). Depends on `d01` (scaffold the gates), `f05` (optional native-tie into the security-audit scope — consumed, not redefined), and `h11` (this pack is the deferred complement: h11 owns the native Rust rules and the vendor dogfood-exclusion + vocab; h12 owns only the optional python/CLI wrappers for the irreplaceable engines). This pack introduces the ONLY subprocess/python touch points in the cyberskills conversion, and they are all off the dogfood path and graceful-skipping. If the analysis for a given engine later shows a Rust equivalent exists, that skill moves to h11 and its adapter is removed here.
