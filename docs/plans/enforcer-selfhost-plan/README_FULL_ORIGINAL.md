# enforcer-selfhost-plan — Full Narrative (background)

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Full Narrative (background)`
> Kind: narrative / background. NOT a task list and NOT proof of anything.
> Read when: You want the "why" behind the plan. Never read this to find work — use README + PLAN_STATE.
> Stop rule: Reading this authorizes nothing. Do not derive status or scope from prose here.
> Proves: nothing.
> Does not prove: any workpack DONE. Proof rows do.
> Proof rule: The mechanical claims below are only real when their named validators are green.
<!-- /agent-capsule -->

> This is the long-form companion to [`README.md`](./README.md). The short README is the route; this is the story. If the two disagree on facts, the short README and `PLAN_STATE.md` are authoritative.

## The thesis

The Ocentra Enforcer's own README opens with: *"Do not rely on AI or human discipline. Make bad code mechanically impossible to land."* The uncomfortable truth is that the enforcer did not fully live by that. It is written in `.mjs` with no TypeScript compiler contract, it brands none of its own domain types, its `enforcer:self` runs only `check source-shape` and does not hard-fail on real findings, and several of its own guarantees exist as prose rather than checks. A tool that tells everyone else to mechanize its rules should be the first codebase to obey.

This plan makes the enforcer eat its own dog food and, in doing so, generalizes the machinery so it can enforce anywhere.

## The DOCTRINE (the spine of every decision)

Rules are conditions; **enforcement MUST be mechanical**. Prose without a backing check is hope, not proof. Everything in this plan is sorted onto a three-rung ladder:

- **T1 — hard/deterministic validator.** Fail-closed, with full parity: every `ruleId` has a validator, a doc anchor, and pass/fail fixtures, and they must agree. Anything that *blocks* has to be T1. If it can be mechanized deterministically, it must be.
- **T2 — scored/advisory but still mechanical.** A regex/AST/heuristic that emits a `score` and a `confidence` and does not block — the model already proven by the Rust literal-scan. T2 is not an excuse to skip mechanization; it is mechanization that chooses not to be fatal.
- **T3 — justified prose.** Reserved for genuine judgment (a devil's-advocate pass, a think-with-me pass) where mechanization is impossible. T3 items MUST carry the label `advisory, no mechanization possible + <reason>`, and the *presence of that label* is itself enforced at T1.

The rule for borrowing: **every ADBP idea is dragged UP this ladder, never copied down as prose.** A "rule pack" described in prose becomes a scaffolder plus a parity oracle. A "context budget" becomes a measured brake with a baseline. A "grandfather clause" becomes a ratchet with a stored baseline that can only tighten.

## The four tracks, in depth

### Track A — Self-host (dogfood)

The big-bang migration. `a01` establishes the compiler contract (strict `tsconfig`, `typecheck`/`build` scripts) that everything else needs. The **conversion swarm** (`a-conv-01`…`a-conv-50`) then converts the entire `.mjs` surface — `src/`, `mcp/`, and `tests/` — to strict TypeScript, starting from the dependency-free leaf surface (`a-conv-01`: path/metadata/policy helpers, and the split of the 776-line `rule-metadata.mjs`) and fanning out through clusters into rollups and finally the test packs. Fifty conversion packs exist because the tree is large and the `owns:` sets must stay disjoint for parallel execution.

On top of the migration, the **domain-hardening** packs make the types honest: branded `RuleId`/registry (`a03`), branded paths (`a04`), branded sha256/fingerprint ids (`a05`), branded hub/lane coordination ids (`a06`), parse-at-boundary for JSON and env (`a07`). Then the integrity packs: waiver honesty — renaming permissive "overrides" into named, honest waivers (`a08`); anti-silent-skip coverage so a scanner that ran zero checks fails instead of passing (`a09`); and the capstone `a10`, which makes `enforcer:self` run the full TS lane, hard-fail on any real finding, and wires `typecheck` + `enforcer:self` into local CI and a GitHub workflow. `a10` goes last on purpose: you only turn on hard-fail self-enforcement once the tree it guards is green.

### Track B — Planning skill

The OcentraParent plan methodology (capsules, token-efficient READMEs, PLAN_STATE/WORKPACK_INDEX/BLUEPRINT/TEST_PROOF_EXPECTATIONS, the frontier/hub/claim-guard-closeout model) currently lives in people's heads. Track B mechanizes it: `b01` is a deterministic `ocentra plan new` emitter (byte-stable, golden-fixture-checked, refuses to overwrite); `b02` is the `PLAN-*` structure validator (fail-closed on capsule/frontmatter/owns-disjoint/deps/xlink violations); `b03` ships the capsule/index templates; `b04` binds the parallel orchestrator; and `b05` ships the `/plan` skill — which proves itself by running `b02`'s validator against *this very plan directory* and requiring zero findings. The planning tool must survive its own gate.

### Track C — Install + enforce anywhere

Today `src/codex-install.mjs` (417 lines) hardcodes the Codex adapter. `c01` lifts the managed-block/backup/report-apply machinery into a harness-neutral core with a pluggable adapter interface (`plan`/`apply`/`verify`) and a stable CLI contract (`--scope`, `--dry-run`, non-TTY JSON). `c02` autodetects the harness. Adapters follow: `c03` Claude, `c06` Codex (at parity with the old behavior), `c07` generic writer + doctor, `c08` stubs for Gemini/Cursor/Zed. The two hook packs are where enforcement becomes mechanical at the harness boundary: `c04` is a **PreToolUse deny-hook** that, on `Edit|Write|MultiEdit`, runs scan/check + coordination guard and **blocks T1 violations before the write lands** (T2 warns, T3 never blocks, fail-closed on error); `c05` is the SessionStart hook. Guidance-only installs are prose an agent can ignore; the deny-hook is the T1 bridge that makes self-enforcement real for any harness.

### Track D — ADBP borrows, mechanized

The keystone is `d01`, the rule-mechanization engine: `ocentra rule new <ID>` scaffolds all five artifacts (registry row, id-lock entry, validator stub, doc section, pass+fail fixtures) in lockstep, plus a fail-closed parity oracle that checks ruleId <-> validator <-> doc <-> fixtures <-> registry-row agreement in both directions. Every other Track D borrow rides this engine: `d02` grandfather ratchet (baseline that can only tighten), `d03` deferred-work gate, `d04` run-telemetry NDJSON (with an Effect schema), `d05` context-budget brake (measured, with a baseline), `d06` lifecycle commands, `d07` self-correct fix loop, `d08` harness-feedback pipeline, `d09` per-stack agents + doc-rule parity, `d10` resilience auditor, `d11` CI-parity validator, `d12` layered/frontend eslint ruleids, `d13` rule-version + drift. Two packs are deliberately not gates: `d14` ships the ideation skills (devil's-advocate, think-with-me) explicitly labeled **T3** with the mandatory reason, and enforces *the labeling* at T1; `d15` adds a documentation-only research-grounding section, honestly scoped as ships-no-validator.

## Why this order

`a01` first because nothing types without a compiler. Conversion swarm next so the codebase is TS before anything builds on it. Track A domain/integrity packs harden the now-typed tree. `d01` early because Track D fans out from its parity oracle. The rest of D and all of C run in parallel — they share no scope. Track B is a self-contained tool track. `a10` — real hard-fail self-enforcement in CI — goes last, gating a tree that is already green.

## How completion is judged

Not by this narrative. Completion is judged only by the named tests in [`TEST_PROOF_EXPECTATIONS.md`](./TEST_PROOF_EXPECTATIONS.md), at the proof tier (P0–P5) required for each workpack's type, with proof rows updated and `owns:`-disjointness held. See [`AGENTS.md`](./AGENTS.md) for the binding contract and the failure conditions. This document is background; the route is [`README.md`](./README.md).
