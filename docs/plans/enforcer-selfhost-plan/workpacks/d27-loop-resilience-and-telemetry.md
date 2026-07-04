# d27 Loop Resilience And Telemetry

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Loop Resilience And Telemetry`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling completion, product status, PR readiness, or broad DONE.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `src/loop/resilience-hooks.ts, src/loop/resilience-meter.ts, src/loop/resilience-hydrate.ts, hooks/statusline-meter.sh, hooks/loop-compaction-flag.sh, tests/loop-resilience-hooks.test.mjs, tests/loop-resilience-meter.test.mjs`
- deps: `d01-rule-mechanization-engine, d04-run-telemetry-ndjson`
- tier: `P1`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md), [ADBP_GAPS](../ADBP_GAPS.md).

## Where We Are
ADBP's `ergonomics/loop-resilience` (rows LOOP-1.1..1.5 in [ADBP_GAPS](../ADBP_GAPS.md#group-3--command--ergonomics-gates)) requires the self-improving loop to survive compaction and context exhaustion: a PreCompact breadcrumb, a context-fill meter, a `.harness/` re-hydration guard, and consent-gated per-project install. The enforcer has none of these. The deterministic per-run telemetry RECORD itself is owned by existing **d04-run-telemetry-ndjson** and the always-on context ceiling is owned by existing **d05-context-budget-brake**; this pack is the loop-resilience half and REFERENCES those, it does not duplicate them.

## Where We Want To Be
- `hooks/statusline-meter.sh` writes `.harness/context-meter.json` (per-tier + total token breakdown) — LOOP-1.2.
- `hooks/loop-compaction-flag.sh` drops `.harness/compaction-pending` on PreCompact, NON-BLOCKING (never fails compaction) — LOOP-1.1.
- Both hooks are inert unless `.harness/` exists (re-hydration guard) — LOOP-1.3.
- Per-iteration re-hydrate from `state.json` — LOOP-1.5 (T3-labeled: agent-runtime behavior).
- Install is consent-gated via `/init-component`, never `deploy.sh` — LOOP-1.4.
- Telemetry linkage: the per-run RECORD (TEL-1.1..1.5) is proven by **d04** — this pack only asserts the resilience hooks feed the same `.harness`/`.harness-archive` surface; the context-ceiling (CTX-1.1..1.4) is proven by **d05**. No duplicate ruleIds authored here.

## Requirement Checklist
- [ ] `loop-compaction-flag.sh` writes `.harness/compaction-pending` on PreCompact and exits 0 even on write failure (non-blocking) — LOOP-1.1.
- [ ] `statusline-meter.sh` emits `.harness/context-meter.json` with per-tier breakdown + total — LOOP-1.2.
- [ ] Both hooks no-op (exit 0, write nothing) when `.harness/` is absent — LOOP-1.3 guard.
- [ ] Consent-gated install path (`/init-component`), and a check that `deploy.sh` does NOT install these hooks — LOOP-1.4.
- [ ] Re-hydration reads `state.json` per iteration — LOOP-1.5, carried as `advisory, no mechanization possible + per-iteration agent-runtime behavior, only the guard file is observable`.
- [ ] Validators are deterministic over hook output files and `.harness/` presence; no duplication of d04 (telemetry record) or d05 (context ceiling).

## Acceptance And Proof
Tier P1. LOOP-1.1/1.2/1.3/1.4 are mechanizable (T1/T2) over hook-produced files and install manifests; LOOP-1.5 is T3-labeled. Select detection tests in TEST_PROOF_EXPECTATIONS.md before DONE.

Per-rule 5-way parity (ruleId <-> doc <-> validator <-> {fail,pass} <-> test):
- **LOOP-1.1 (non-blocking PreCompact breadcrumb):** fail-fixture `tests/fixtures/loop/loop-1.1/fail-precompact-blocks/` (PreCompact hook missing, or a hook that returns non-zero on write failure) flagged; pass-fixture `.../pass-nonblocking-breadcrumb/` (drops `compaction-pending`, exits 0 regardless) clean. Test: `tests/loop-resilience-hooks.test.mjs`.
- **LOOP-1.2 (context meter):** fail-fixture `.../loop-1.2/fail-meter-no-breakdown/` (`context-meter.json` missing per-tier breakdown or total); pass-fixture `.../pass-meter-full/`. Test: `tests/loop-resilience-meter.test.mjs`.
- **LOOP-1.3 (`.harness/` guard):** fail-fixture `.../loop-1.3/fail-runs-without-harness/` (hook writes even when `.harness/` absent); pass-fixture `.../pass-guarded-noop/`. Test: `tests/loop-resilience-hooks.test.mjs`.
- **LOOP-1.4 (consent-gated install):** fail-fixture `.../loop-1.4/fail-deploy-installs-hooks/` (`deploy.sh` installs the hooks); pass-fixture `.../pass-init-component-consent/`. Test: `tests/loop-resilience-hooks.test.mjs`.
- **LOOP-1.5:** advisory, no mechanization possible + per-iteration agent-runtime behavior; only the presence of the `state.json` hydration guard file is asserted (label presence is T1).

## Parallel Ownership Notes
Depends on d01 (harness) and d04 (telemetry record it feeds — referenced, not re-implemented). Owns a disjoint `src/loop/resilience-*` tree plus the two `hooks/*.sh` files and their tests. Does NOT touch d04's `.harness-archive/metrics.jsonl` writer or d05's context-ceiling validator — those ruleIds stay in their home packs; this pack only reads/asserts the shared `.harness` surface. Concurrent-safe with d04/d05 (disjoint files).
