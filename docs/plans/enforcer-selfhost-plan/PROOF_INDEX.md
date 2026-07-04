# PROOF_INDEX

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Proof Index`
> Kind: proof routing surface. Maps proof tiers P0-P5 to the workpacks that carry them and points at the concrete proof rows. The reviewer's map.
> Read when: You are reviewing/closing a workpack and need to find its proof row, OR you want to see which workpacks carry which proof burden, OR you are auditing whether the plan's proof coverage is honest.
> Stop rule: The tier *definitions* and the per-workpack proof *rows* live in TEST_PROOF_EXPECTATIONS.md. This index routes to them; it does not restate them and does not itself certify anything.
> Proves: nothing directly. It is a routing + coverage view over the proof contract.
> Does not prove: that any test passes. GREEN status lives only in the TEST_PROOF_EXPECTATIONS proof rows.
<!-- /agent-capsule -->

Sources: [TEST_PROOF_EXPECTATIONS](./TEST_PROOF_EXPECTATIONS.md) (authority), [WORKPACK_INDEX](./WORKPACK_INDEX.md), [CHECKLIST_INDEX](./CHECKLIST_INDEX.md).

---

## The one rule this index exists to enforce

**A workpack is DONE only when its proof row in [TEST_PROOF_EXPECTATIONS.md](./TEST_PROOF_EXPECTATIONS.md) is GREEN.** GREEN means: the named test passes on the migrated tree AND (for T1 / P4 / P5) the seeded-violation case is demonstrated to fail. A green test that never trips is a hollow scan — a doctrine violation, not a proof.

To review a workpack: open [TEST_PROOF_EXPECTATIONS.md](./TEST_PROOF_EXPECTATIONS.md) section 4, find the row, run the named test + the seeded-violation case, then authorize (or reject) the WORKPACK_INDEX status move.

---

## Proof tiers at a glance (definitions in TEST_PROOF_EXPECTATIONS section 2)

| Tier | One-liner | Fail-closed evidence |
|------|-----------|----------------------|
| **P0** | Contract / schema | tsc negative fixture OR schema decode test OR frozen snapshot |
| **P1** | Unit | pass+fail+edge test; conversions: scoped `tsc --noEmit`==0 + no `import *` |
| **P2** | CI / cross-platform | CI job runs it; determinism + seeded-skew-fails |
| **P3** | Live MCP-tool | invoke the tool path; mutate input; re-observe |
| **P4** | Self-enforce green | real gate vs live tree, honestly green + seeded self-violation fails |
| **P5** | Install / integration | temp-home install->verify green; corrupt->fail; uninstall restores; hooks: exact deny+ruleId+fix |

Rule-mechanization ladder (orthogonal): **T1** hard/deterministic (fail-closed), **T2** scored+confidence (non-blocking), **T3** justified prose, label-gated. Every ADBP borrow is dragged UP this ladder.

---

## Coverage: which workpacks carry which proof tier

Counts are primary-tier assignments; several workpacks carry a secondary row (noted).

### P0 - Contract / schema
- **Track A**: a03, a04, a05, a06, a07 (all T1 brands/boundaries, proven by tsc negative fixtures + decode tests).
- **Track C**: c08 (adapter stubs; T3-labeled `status:"deferred"`, contract-verified).
- **Track D**: d12 (T1 AST rules), d14 (T1 label gate over T3 content).
- **Track B**: b03 (frozen-snapshot templates).

### P1 - Unit
- **Track A**: a01, a08, and the **entire conversion swarm a-conv-01..50** (uniform scoped-typecheck proof).
- **Track C**: c01, c02, c07.
- **Track D**: d01, d02, d03, d04, d06, d07, d08, d09, d10, d13.
- **Track B**: b01, b04.

### P2 - CI / cross-platform
- **Track D**: d05 (T1 ratchet + T2 score), d11 (T1 CI==local parity).

### P3 - Live MCP-tool
- **Track A**: a02 (fingerprint-over-`dist/` against live `mcp_status`/freshness).

### P4 - Self-enforce green
- **Track A**: a09 (honest skips), a10 (real self-enforcement + CI hard-fail).
- **Track B**: b02 (validator vs THIS plan dir), b05 (skill self-validate vs THIS plan dir).

### P5 - Install / integration proof
- **Track C**: c03, c04 (T1 deny-hook mechanical bridge), c05, c06.

### Doc-only (no runtime tier)
- **Track D**: d15 (README research grounding; proof = artifact exists + cross-link integrity; explicitly gates nothing — the honesty guardrail is the doc-only label itself).

---

## Secondary / dual-tier rows (T1 + T2 together)

These emit a hard gate **and** a scored signal; both rows must be satisfied:

| Workpack | T1 (hard, fail-closed) | T2 (scored, non-blocking) |
|----------|------------------------|---------------------------|
| d05 Context Budget | surface-growth ratchet vs committed baseline | surface-per-tool efficiency score + confidence |
| d10 Resilience Auditor | required-test obligation rows | failure-mode "smell" scores + confidence |

For each, the T2 proof asserts `score in [0,1]`, a confidence value, and **zero effect on exit code**.

---

## T3 items and their mechanical label gates

T3 = justified prose. Per doctrine it must be labeled `advisory, no mechanization possible: <reason>` and the **label** is enforced mechanically (T1):

| T3 content | Where | Label enforced by |
|------------|-------|-------------------|
| Ideation skills (devil, think-with-me) | d14 `skills/ideation/**` | `ideation-skills-labeling` (P0/T1): every file must carry the exact T3 label; unlabeled fails closed; skills excluded from any rule registry |
| Per-stack agent personas | d09 `docs/agents/**` | persona free-text is ungated, but every must/never bullet must cite a real `[ruleId]` (`doc-rule-parity`, P1/T1) |
| Stub adapters "not yet implemented" | c08 | `adapter-stub-contract` (P0): each returns `status:"deferred"` + reason, writes nothing |
| README research grounding | d15 | doc-only; scoped as gating nothing (the scoping label is the guardrail) |

---

## Reviewer quick path

1. Get the workpack id from [WORKPACK_INDEX.md](./WORKPACK_INDEX.md).
2. Open its row in [TEST_PROOF_EXPECTATIONS.md](./TEST_PROOF_EXPECTATIONS.md) section 4.
3. Run the **Named test / oracle** — it must pass.
4. Run the **Seeded-violation case** — for T1/P4/P5 it MUST fail (proves the gate is real).
5. For T2 rows: confirm `score in [0,1]`, confidence present, exit code unchanged.
6. For T3: confirm the label gate passes (never trust the prose).
7. Flip the row Status to `GREEN`, then authorize the WORKPACK_INDEX move to DONE.
