# TEST_PROOF_EXPECTATIONS

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Test Proof Expectations`
> Kind: index / contract doc. This is the proof-tier authority every workpack routes into before DONE.
> Read when: You are about to close a workpack (select its proof tier + name its proof rows here), OR you are authoring/reviewing a workpack's "Acceptance And Proof" section, OR you need to know what P0-P5 mean.
> Stop rule: This doc defines proof *tiers and obligations*. It does not itself prove any workpack. A workpack is only DONE when its own proof rows here are green — not when this doc is read.
> Proves: the mapping workpack-type -> required proof tier -> concrete test/artifact. Nothing else.
> Does not prove: that any test currently passes; the runtime status lives in WORKPACK_INDEX.md and the proof rows below.
> Proof rule: Every workpack MUST select a proof tier from P0-P5 via the decision tree, and MUST name its concrete test file(s) / artifact path(s) in a proof row here before it moves to DONE.
<!-- /agent-capsule -->

Sources: [WORKPACK_INDEX](./WORKPACK_INDEX.md), [PROOF_INDEX](./PROOF_INDEX.md), [PLAN_EXECUTION_BLUEPRINT](./PLAN_EXECUTION_BLUEPRINT.md), [PLAN_STATE](./PLAN_STATE.md).

---

## 1. Doctrine this doc enforces

Rules are conditions. **Enforcement MUST be mechanical.** Prose without a backing check is hope, not proof. This doc is where every workpack is forced to name the *mechanical* check that backs its claim of DONE.

The plan's mechanization ladder (the tier a workpack's *rule* sits at) is distinct from the proof tier (the *test shape* that proves it):

- **T1** — hard/deterministic validator. Fail-closed. ruleId<->validator<->doc<->fixtures parity. A T1 claim MUST be proven by a test that trips on a seeded violation and passes on a conforming input.
- **T2** — scored/advisory but still mechanical (regex/AST/heuristic emitting `score`+`confidence`, non-blocking; the Rust literal-scan model). A T2 claim MUST be proven by a test asserting the score is in `[0,1]`, carries a confidence, and never changes exit code.
- **T3** — justified prose, only when mechanization is impossible; MUST be labeled `advisory, no mechanization possible + <reason>`. A T3 claim is proven only by a **labeling gate** (a T1 check that the label is present) — never by trusting the prose.

Every ADBP borrow is dragged UP this ladder, never copied as prose. If a borrow lands as T3, the T3 label itself is mechanically enforced.

### 5-way tested-enforcement parity (the load-bearing invariant)

A rule is only "enforced" when it is **tested**. The doctrine, stated plainly: **an untested validator does not count as enforcement.** A validator with no fail-fixture that trips it and no pass-fixture that stays clean is unproven code, not a gate. Every rule the plan ships (and every rule the d01 mechanization engine scaffolds) MUST satisfy the parity for its T-tier before its proof row can go GREEN:

- **Every T1 rule** requires the full **5-way chain**: `ruleId <-> doc <-> validator <-> {fail-fixture + pass-fixture} <-> detection-test`. All five artifacts must exist and cross-resolve; the detection test must assert the fail-fixture is flagged AND the pass-fixture stays clean. A missing link at any of the five points fails the d01 `rule-scaffold-parity` oracle closed.
- **Every T2 rule** requires fixtures that assert the **score behavior against a threshold**: a fail-fixture whose `score` **crosses** the fail threshold and a pass-fixture whose `score` **stays under** it (the Rust literal-scan scored model), plus a confidence value and zero effect on exit code. The chain is the same 5-way shape; the detection test asserts the threshold crossing rather than a hard block.
- **Every T3 rule** requires only the enforced **LABEL**: the rule doc carries the verbatim `advisory, no mechanization possible + <reason>` label, and the *presence of that label* is itself T1-enforced by the d01 parity oracle. No behavioral fixture — but the label gate is mandatory and mechanical.

This 5-way parity is exactly what the new Track D families (d16-d18, d21-d23, d25-d28), the Track E language/universal packs, and the terminal z01 dogfood gate are built to satisfy: every new ruleId is scaffolded through d01 so its ruleId, doc anchor, validator, fail+pass fixtures, and detection test all exist and cross-resolve, or it does not count.

---

## 2. Proof tiers P0-P5

The proof tier answers: **what shape of evidence closes this workpack?** Higher number = heavier, more integrated proof. A workpack picks exactly one *primary* tier (it may carry secondary rows, e.g. a T1+T2 workpack lists both a hard-gate row and a score row).

| Tier | Name | What it proves | Required evidence | Fail-closed? |
|------|------|----------------|-------------------|--------------|
| **P0** | Contract / schema | A type, schema, brand, template, or interface has the exact declared shape; illegal shapes are rejected at compile/decode time. | A `tsc --noEmit` negative fixture (bad shape must not compile) OR a schema decode test (valid mints, invalid rejects) OR a frozen-snapshot template test. No runtime execution required. | Yes — an illegal shape MUST fail to compile/decode. |
| **P1** | Unit | A single module/function does what it says over inputs incl. edge/adversarial cases. | A unit test (`tests/*.test.mjs` or `test/**/*.test.ts`) with pass + fail + edge cases; for conversions, a **scoped `tsc --noEmit`** over only the owned files exits 0 and `grep 'import *'` is empty. | Yes for T1 rules; observers may be non-gating. |
| **P2** | CI / cross-platform | The check runs identically as a CI job and locally, across the supported Node range and OS matrix (Win/POSIX). | A CI job invokes the script; a test asserts determinism and platform-independence (path normalization, seeded skew fails). | Yes — CI job exits non-zero on violation. |
| **P3** | Live MCP-tool | A live MCP tool / server path behaves correctly against a running (or faithfully stubbed) transport. | A test invokes the MCP tool path (e.g. `mcp_status`/freshness) and asserts observable behavior; mutate an input, re-observe the change. | Yes for T1-scoped tool behavior. |
| **P4** | Self-enforce green | The enforcer enforces *itself*: run the real gate against this repo/plan and it must be honestly green (nonzero ran-count, no hollow pass), and a seeded self-violation must fail it. | Run `enforcer:self` / the PLAN-* validator / the parity oracle against the live tree; assert green with visible ran-counts AND a seeded violation fixture makes it exit non-zero. | Yes — a hollow (zero-ran) or bypassed run MUST fail. |
| **P5** | Install / integration proof | An install/adapter/hook produces the correct on-disk or runtime result end-to-end against a real (temp) fixture home, round-trips, and fails closed on corruption. | Against a temp fixture: `install`->`verify` all-green; corrupt input -> `verify` fails; `install`->`uninstall` restores pre-state; for hooks, a seeded payload yields the exact deny/allow + ruleId+fix. | Yes — corruption or a seeded violation MUST be caught. |

Notes:
- Workpacks in the tree label their tier with the *proof* tier vocabulary (e.g. `P1 unit`, `P0 contract/schema`, `P5 install-proof`) and, where the rule is scored/advisory, also name the T-tier of the rule. The two vocabularies co-exist by design: P-tier = evidence shape, T-tier = mechanization ladder.
- A doc-only workpack (d15) has **no runtime tier**: its proof is artifact existence + cross-link integrity, and the doctrine honesty guardrail is that it is *explicitly labeled doc-only, gating nothing* (no prose masquerading as a check).

---

## 3. Decision tree: workpack type -> required proof

Walk top to bottom; take the first branch that matches.

```
START
 |
 |-- Is the deliverable ONLY documentation/prose (no validator, gates nothing)?
 |      YES -> DOC-ONLY: proof = artifact exists + every claim cross-links a source.
 |             MUST be labeled T3 "advisory, no mechanization possible: <reason>"
 |             or "doc-only, gates nothing". (d15, d14 content layer, d09 persona layer)
 |             The LABEL is enforced at P0/T1 even when the content is T3.
 |      NO  -> continue
 |
 |-- Does it define a type / brand / schema / template / adapter interface (no runtime behavior)?
 |      YES -> P0 contract/schema: tsc negative fixture OR schema decode test OR frozen snapshot.
 |             (a03-a06 brands, c08 stubs, b03 templates, d12 AST-rule contracts, d14 labeling)
 |      NO  -> continue
 |
 |-- Is it a .mjs -> TS conversion / module split (behavior preserved)?
 |      YES -> P1 unit: scoped `tsc --noEmit` over owned files == 0,
 |             `grep 'import *'` empty, splits preserve every test case.
 |             (all a-conv-01 .. a-conv-50)
 |      NO  -> continue
 |
 |-- Is it a single validator / oracle / engine with deterministic pass/fail?
 |      YES -> P1 unit (T1): pass + fail + edge fixtures; seeded violation trips it;
 |             ruleId<->validator<->doc<->fixture parity holds; fails closed on missing input.
 |             (d01-d04, d06-d10, d13, a08)
 |      NO  -> continue
 |
 |-- Does it emit a score/confidence (heuristic, non-blocking)?
 |      YES -> P1 unit (T2): score in [0,1], confidence present, exit code unchanged,
 |             asserted against fixtures. Pair with a T1 row if it also hard-ratchets.
 |             (d05 score, d10 smells)
 |      NO  -> continue
 |
 |-- Must it match between local and CI, or run across the OS/Node matrix?
 |      YES -> P2 CI cross-platform: CI job runs it; determinism + seeded-skew-fails test.
 |             (d05 ratchet, d11)
 |      NO  -> continue
 |
 |-- Does it exercise a live MCP tool / server / transport path?
 |      YES -> P3 live MCP-tool: invoke the tool path, mutate input, re-observe.
 |             (a02 fingerprint-over-dist)
 |      NO  -> continue
 |
 |-- Does it install / write config / register a hook into a harness home?
 |      YES -> P5 install-proof: temp-home install->verify green; corrupt->fail;
 |             install->uninstall restores; hooks: seeded payload -> exact deny/allow+ruleId+fix.
 |             (c03-c06 adapters+hooks)
 |      NO  -> continue
 |
 |-- Does it turn the enforcer/validator on ITSELF (this repo or this plan dir)?
 |      YES -> P4 self-enforce green: real gate vs live tree, honestly green with ran-counts,
 |             seeded self-violation fails. No --no-verify bypass; waivers are the only exception.
 |             (a09, a10, b02, b05)
 |      NO  -> re-read this tree; every workpack MUST land in exactly one primary tier.
END
```

Ambiguity rule: if two branches both match, pick the **higher-numbered** tier as primary (heavier proof) and list the lower as a secondary row. Example: c07 defines an interface (P0) but also writes `.mjs.json` and re-reads disk (would be P5-ish) — it is scoped P1 unit because it is pure over injected `fs`; its adapter siblings that touch a real home are P5.

---

## 4. Proof rows

Every workpack registers its concrete proof here before DONE. Columns:
`Workpack | Proof tier(s) | Named test / oracle | Artifact path | Seeded-violation (fail) case | Status`.
Status starts `PENDING`; the closing agent flips it to `GREEN` only when the named test passes AND (for T1/P4/P5) the seeded-violation case is shown to fail. `GREEN` here is the *only* thing that authorizes a WORKPACK_INDEX status move to DONE.

### Track A - conversion swarm (a-conv-01 .. a-conv-50)

All P1 unit, T-tier N/A (mechanical conversion). Uniform proof: scoped `tsc --noEmit` over owned files exits 0 under strict; `grep 'import *'` over owned files empty; every pre-existing test case preserved across any SPLIT.

| Workpack | Proof tier | Named test / oracle | Artifact path | Seeded-violation case | Status |
|----------|-----------|---------------------|---------------|-----------------------|--------|
| a-conv-01..50 (each) | P1 unit | scoped-typecheck (per-pack tsconfig include) + `import *` grep + case-preservation diff | `proof/typecheck/a-conv-<NN>.txt` | inject a type error in an owned file -> scoped `tsc` non-zero | PENDING |

SPLIT-bearing packs additionally assert combined split exports == original public surface, re-checked by the dependent cluster's typecheck: a-conv-01 (`rule-metadata`), a-conv-50 (`coordination.test`, `rust-rules-mcp.test`), and any pack whose file exceeds shape limits.

### Track A - domain packs (a01 .. a10)

| Workpack | Proof tier(s) | Named test / oracle | Artifact path | Seeded-violation case | Status |
|----------|--------------|---------------------|---------------|-----------------------|--------|
| a01 | P1 unit | `tsc --noEmit` exit-code test + tsconfig-flags schema test | `proof/typecheck/baseline.txt` | seed a type error fixture -> `npm run typecheck` non-zero | PENDING |
| a02 | P3 live MCP-tool | `mcp-fingerprint-over-dist` (live `mcp_status`/freshness) | `proof/mcp/a02-fingerprint.json` | mutate a `dist/` file -> digest changes; unbuilt -> `exists:false` | PENDING |
| a03 | P0 contract/schema (T1) | `ruleid-brand-decode` + ruleId<->registry parity + tsc negative fixture | `proof/schema/a03-ruleid.txt` | bare `string` where `RuleId` required -> tsc fails; unknown id -> decode Left | PENDING |
| a04 | P0 contract/schema (T1) | `path-brand-decode` + tsc negative fixture (Win+POSIX) | `proof/schema/a04-paths.txt` | absolute-as-relative / `..` escape -> decode fails; `RelPath` for `RepoRoot` -> tsc fails | PENDING |
| a05 | P0 contract/schema (T1) | `sha256-brand-decode` round-trip + tsc negative fixture | `proof/schema/a05-sha256.txt` | wrong length/case/charset -> decode fails; bare string in `Sha256` field -> tsc fails | PENDING |
| a06 | P0 contract/schema (T1) | `coordination-id-brand-decode` + tsc negative fixture | `proof/schema/a06-coordids.txt` | unsafe charset/empty/oversize -> fails; `HubName` vs `LaneId` swap -> tsc fails | PENDING |
| a07 | P0 contract/schema (T1) | `parse-at-boundary` decode tests + `no process.env outside env-boundary` grep/AST + `no any JSON.parse in routing` | `proof/schema/a07-boundary.txt` | malformed/schema-invalid JSON -> decode error naming file; env read outside boundary -> gate trips | PENDING |
| a08 | P1 unit (T1) | `waiver-honesty` config test + count parity | `proof/config/a08-waivers.json` | a bare `sourceShapeOverrides` limit-bump or empty-reason waiver -> fails; waived+fixed != 51 -> fails | PENDING |
| a09 | P4 self-enforce green (T1) | `anti-silent-skip` scanner test + `enforcer:self:scan` ran-count | `proof/self/a09-scan.txt` | unmatched-ext / missing-tool / empty-selection -> explicit `skipped:<reason>` not silent pass | PENDING |
| a10 | P4 self-enforce green (T1) | `real-self-enforcement` + `ci-local` + workflow gate | `proof/self/a10-ci.txt` | seed a self-violating fixture -> `enforcer:self`+`ci-local` non-zero; hollow zero-ran -> CI fails | PENDING |

### Track C - install / enforce (c01 .. c09)

| Workpack | Proof tier(s) | Named test / oracle | Artifact path | Seeded-violation case | Status |
|----------|--------------|---------------------|---------------|-----------------------|--------|
| c01 | P1 unit (T1) | `install-core-contract` | `proof/install/c01-contract.json` | `--dry-run` writes any file -> fails; unknown adapter id -> error not silent skip | PENDING |
| c02 | P1 unit (T1) | `harness-autodetect` (temp-home fixtures) | `proof/install/c02-detect.json` | empty home -> no false positive; env-override precedence wrong -> fails | PENDING |
| c03 | P5 install-proof (T1) | `claude-adapter-install` (temp `~/.claude`) | `proof/install/c03-claude.json` | corrupt `.mcp.json` -> `verify` fails; install->uninstall doesn't restore -> fails | PENDING |
| c04 | P5 install-proof (T1 bridge) | `claude-deny-hook-blocks` | `proof/install/c04-denyhook.json` | seeded violating edit -> exit=deny with exact `ruleId`+`fix`; conforming -> allow; T2-only -> allow-with-warning | PENDING |
| c05 | P5 install-proof (T1) | `claude-sessionstart-injects` (snapshot) | `proof/install/c05-sessionstart.txt` | reminder body drift vs snapshot -> fails; missing T1/T2/T3 tokens -> fails | PENDING |
| c06 | P5 install-proof (T1) | `codex-adapter-parity` (golden TOML + `AGENTS.md`) | `proof/install/c06-codex.txt` | generated block != current `codexMcpTomlBlock`/`globalAgentsInstructionBlock` -> fails | PENDING |
| c07 | P1 unit (T1) | `generic-writer` + `install-doctor` (golden + fixtures) | `proof/install/c07-generic.json` | missing/renamed server file -> doctor red naming the check | PENDING |
| c08 | P0 contract/schema (T3-labeled) | `adapter-stub-contract` | `proof/install/c08-stubs.json` | stub applies writes a file / doesn't return `status:"deferred"` / registry miss -> fails | PENDING |
| c09 | P5 install-proof (T1) | `remaining-adapters-detect` (autodetect enumerates all six ids; doctor aggregates per-adapter checks over temp-home fixtures) | `proof/install/c09-remaining-adapters.json` | JSON-config harness (antigravity/windsurf/kilocode/kiro) with server entry missing/renamed on disk -> `verify` reports the named failing check; second apply not byte-identical -> non-idempotent fail; CLI-only harness (opencode/aider) missing the T3 `deferred: no mcp surface` label or writing any file -> fails; absent harness not returning `skipped:not-detected` -> silent-skip fail | PENDING |

### Track D - ADBP borrows rebuilt mechanically (d01 .. d15, then d16-d18, d21-d23, d25-d28)

| Workpack | Proof tier(s) | Named test / oracle | Artifact path | Seeded-violation case | Status |
|----------|--------------|---------------------|---------------|-----------------------|--------|
| d01 | P1 unit (T1) | `rule-scaffold-parity` + `rule-new` | `proof/rules/d01-parity.txt` | unknown validator / dangling doc anchor / missing fixture / orphan -> parity fails closed | PENDING |
| d02 | P1 unit (T1) | `baseline-ratchet` | `proof/rules/d02-baseline.json` | one added finding fails; grown count fails; ratchet can't silently expand | PENDING |
| d03 | P1 unit (T1) | `deferred-work-gate` | `proof/rules/d03-deferred.txt` | unmarked stub in diff fails; malformed `DEFERRED(#ref)[revisit:]` fails; legacy stub passes | PENDING |
| d04 | P1 unit (T1) | `run-telemetry` (re-parse every line) | `proof/telemetry/runs.ndjson` | schema-invalid line rejected; half-line on crash -> not written | PENDING |
| d05 | P2 CI cross-platform (T1 ratchet + T2 score) | `context-budget` + CI `context-budget-scan` | `proof/context-budget-baseline.json` | simulated surface growth beyond tolerance fails ratchet; T2 score out of [0,1] fails | PENDING |
| d06 | P1 unit (T1) | `lifecycle-commands` | `proof/rules/d06-lifecycle.txt` | a phase reports success while its oracle returns fail -> test fails; `review` w/o proof rows -> non-zero | PENDING |
| d07 | P1 unit (T1) | `fix-loop` | `proof/rules/d07-fixloop.txt` | neutral/regressing fix not reverted -> fails; loop exceeds cap -> fails; final findings > start -> fails | PENDING |
| d08 | P1 unit (T1) | `harness-feedback` | `proof/rules/d08-feedback.txt` | preventable failure yields no PROPOSED row -> fails; PROPOSED gates a build -> fails | PENDING |
| d09 | P1 unit (T3 persona + T1 citation) | `doc-rule-parity` | `proof/rules/d09-docparity.txt` | uncited or dangling-`[ruleId]` must/never bullet -> fails; persona free-text gated -> fails | PENDING |
| d10 | P1 unit (T1 obligations + T2 smells) | `resilience-auditor` | `proof/rules/d10-resilience.txt` | unmet required-test row passes review -> fails; smell gates a build -> fails; score out of [0,1] fails | PENDING |
| d11 | P2 CI cross-platform (T1) | `ci-parity` + CI `ci-parity-verify` | `proof/ci/d11-parity.txt` | injected local-only step fails; version skew fails | PENDING |
| d12 | P0 contract/schema (T1 AST) | layered-frontend eslint-tester + d01 parity | `proof/rules/d12-layered.txt` | each rule's fail-fixture must trip and pass-fixture must not; missing registry parity fails | PENDING |
| d13 | P1 unit (T1) | `rule-version-drift` | `proof/rules/d13-drift.json` | content change w/o version bump fails; version bump w/o content change fails | PENDING |
| d14 | P0 contract/schema (T1 label over T3 content) | `ideation-skills-labeling` | `proof/rules/d14-labeling.txt` | any `skills/ideation/**` file missing exact T3 label -> fails; skill in a rule registry -> fails | PENDING |
| d15 | doc-only (no runtime tier) | cross-link integrity check (artifact existence) | `docs/research-grounding.md` | a README claim with no numbered source; a dead cross-link -> review fails | PENDING |
| d16 | P1 unit (T1 blocks + T2 scored) | `fsm-*` detection tests + d01 `rule-scaffold-parity` (5-way per ruleId) | `proof/rules/d16-fsm.txt` | raw `order.status = "shipped"` not routed through a transition -> T1 flag; FSM with no invalid-transition test -> T2 score crosses; pass triple stays clean | PENDING |
| d17 | P1 unit (T1 blocks + T2 scored) | `rust-err-*` detection tests + d01 parity | `proof/rules/d17-rust-err.txt` | `.unwrap()` in non-`#[cfg(test)]` src -> T1 flag; bare `?` w/o `.with_context` -> T2 crosses; unwrap under `#[cfg(test)]` stays clean | PENDING |
| d18 | P1 unit (T1 pattern/AST + 2 T2 audit + 1 labeled T3) | `security-stop-*` detection tests + d01 parity | `proof/rules/d18-security-stop.txt` | `cursor.execute(f"...{x}")` SQLi -> T1 flag; `Md5::new()` -> T1 flag; dispatch prompt missing SECURITY-STOP Block 1 -> flag; STOP-interrupt-honoring row carries the advisory label (label presence T1) | PENDING |
| d21 | P1 unit (T1 marker grammar + T2 dep-add + T3-labeled) | `change-discipline.test.mjs` + d01 parity | `proof/rules/d21-change-discipline.txt` | bare `// TODO: fix later` (no `(#NNNN)`) -> T1 flag; manifest gaining a dep w/o rationale -> T2 score crosses; refactor-isolation/ADR-deviation rows carry advisory label (label presence T1) | PENDING |
| d22 | P1 unit (T1 hard caps + T2 scored complexity) | `size-shape.test.mjs` + d01 parity | `proof/rules/d22-size-shape.txt` | 201-line file / 31-line fn / 6-param sig -> T1 flag; cc=12 5-deep -> T2 crosses; trailing-pragma line >120 still flags; baselined-grown +1 line fails ratchet (composes d02) | PENDING |
| d23 | P1 unit (T1 presence + T2 quality heuristics) | `test-quality.test.mjs` + d01 parity (consumes d16 FSM model) | `proof/rules/d23-test-quality.txt` | source file with no companion test -> T1 flag; assertion-free `it()` -> T1 flag; `test_order_1` name / assert-on-message / FSM w/o invalid-transition test -> T2 crosses | PENDING |
| d25 | P1 unit (T1 gates + labeled T3 residue) | `orchestrator-verify-gate-{a,b,c}.test.mjs` | `proof/rules/d25-orchestrator.txt` | self-reported "789 passed" with no re-run artifact -> Gate A flag; `git diff`-based staging dropping untracked source -> Gate B flag; commits-ahead-of-base unreconciled -> Gate C flag; orchestrator text in dispatch prompt -> ORCH-1.10 flag | PENDING |
| d26 | P1 unit (T1) | `dispatch-quality-blocks.test.mjs` | `proof/rules/d26-dispatch.txt` | assembled prompt missing SECURITY-STOP Block 1 -> flag; blocks out of mandated order (git boundary not dead-last) -> flag; fix-phase prompt lacking zero-match addendum -> flag; full ordered prompt stays clean | PENDING |
| d27 | P1 unit (T1/T2 mechanizable + LOOP-1.5 labeled T3) | `loop-resilience-hooks.test.mjs` + `loop-resilience-meter.test.mjs` | `proof/rules/d27-loop.txt` | PreCompact hook that returns non-zero on write failure -> flag; `context-meter.json` missing per-tier breakdown -> flag; hook writing when `.harness/` absent -> guard flag; `deploy.sh` installing hooks -> flag; LOOP-1.5 carries advisory label (label presence T1) | PENDING |
| d28 | P2 CI cross-platform (T1 structural + T2 DOCGATE header) | `target-ci-parity.test.mjs` + d01 parity | `proof/ci/d28-target-parity.txt` | target repo with a local-only check/version vs CI -> flag; missing coverage `fail_under` -> flag; sub-project with no workflow `paths:` -> flag; missing `decisions.md` -> T1 flag; `ARCHITECTURE.md` missing required H2s -> T2 score below threshold | PENDING |

### Track E - new languages + universal scanning (e01, e-pack-dart, e-pack-cfml, e-pack-frontend-react, e-pack-python)

| Workpack | Proof tier(s) | Named test / oracle | Artifact path | Seeded-violation case | Status |
|----------|--------------|---------------------|---------------|-----------------------|--------|
| e01 | P1 unit (T2 scored/advisory, non-blocking) | `literal-scan-bridge.test.mjs` (named rows `literal-scan-universal-threshold`, `literal-scan-graceful-skip`) + d01 parity | `proof/rules/e01-literal-scan.txt` | high-literal-risk source `score` must cross threshold; clean equivalent stays under; `cargo` stubbed-absent -> graceful skip advisory, run still exits 0; `.dart`/`.cfc`/`.cfm` recognized by registry | PENDING |
| e-pack-dart | P0/P1 (T1 blocks + T2 scored + labeled T3) | `dart-*` d01-generated detection/parity tests | `proof/rules/e-pack-dart.txt` | `data/` importing `presentation/` or Flutter-in-domain -> T1 flag; unchecked `!`/hardcoded API key -> T1 flag; `Color(0xFF..)` in build / `?? 0` on required field -> T2 crosses; boolean-prefix/Bloc-conventions rows carry advisory label; `dart`/`.dart` registered in `rules.json` + literal-scan registry | PENDING |
| e-pack-cfml | P0/P1 (T1 CFLint/structural + T2 scored + labeled T3) | `cfml-*` d01-generated detection/parity tests via CFLint/CommandBox shell-out adapter | `proof/rules/e-pack-cfml.txt` | `queryExecute("...#rc.id#")` SQLi / `<cfquery>` in handler / `new OrderService()` (no WireBox) -> T1 flag; direct `rc` scope in service / `writeDump` diagnostics -> T2 crosses; `Utils.cfc` dumping-ground carries advisory label; `coldfusion`/`.cfc`/`.cfm` registered + adapter degrades cleanly when CFLint absent | PENDING |
| e-pack-frontend-react | P0/P1 (T1 blocks + one T2 layer-inversion) | `frontend-*` d01-generated tests (named rows `frontend-family-detection`, `frontend-family-parity`) | `proof/rules/e-pack-frontend.txt` | cross-feature import / server-data-in-client-store / fetch-in-useEffect / `throw new Error` in services -> T1 flag; **`z.object` Zod usage -> FE-EFFECT-1.1 flag, `Schema.Struct` clean** (pins Effect-only divergence); components->features import -> T2 layer-inversion crosses | PENDING |
| e-pack-python | P0/P1 (T1 layering/DI + Python security blocks) | `python-fastapi-*` d01-generated detection/parity tests (named rows `python-fastapi-layered-detection`, `python-fastapi-family-parity`) | `proof/rules/e-pack-python.txt` | `routers/**` referencing a `*Repository` symbol / `Session` in `services/**` / `commit()` in a service / ORM model imported into `services/**` / `raise HTTPException` outside `routers/**` / `domain/**` importing FastAPI -> T1 flag (symbol-level, so a comment mentioning `Repository` in a pass fixture stays clean); plaintext-password store, `random.*` token, `allow_origins=["*"]` -> T1 security flag; StrEnum/enum-location consumed from d16, size/shape from d22; every `py-fastapi-*` id resolves to validator export + doc anchor + both fixtures or parity fails closed | PENDING |

### Track F - scan surface, onboarding & agent-shaping (f01 .. f05)

| Workpack | Proof tier(s) | Named test / oracle | Artifact path | Seeded-violation case | Status |
|----------|--------------|---------------------|---------------|-----------------------|--------|
| f01 | P1/P3 (T1) | `scan-modes-select` (mode enum + scope resolution over the d01 engine) | `proof/scan/f01-modes.txt` | a `full`-only violation seeded OUTSIDE the scoped path -> `scoped`/`quick` must NOT report it (scope honored), same violation INSIDE scope -> `scoped` reports it and `full` always reports it; an invalid mode string is rejected at the schema boundary (non-zero/error) | PENDING |
| f02 | P1/P5 | `onboard-scaffolds-enforce` (temp-repo fixture) | `proof/scan/f02-onboard.json` | scan a repo with no `.enforce/` -> asserts "not onboarded" error (no baseline to compare); `enforcer onboard` on a fresh repo -> `.enforce/` exists with profile + baseline + registration; a second onboard run not byte-identical / dropping waivers -> non-idempotent fail | PENDING |
| f03 | P1 (T1) | `project-config-native-mode` (Effect-schema decode at boundary) | `proof/scan/f03-project-config.txt` | malformed `.enforce/config` (bad `nativeMode`) -> typed boundary parse error, no silent default; valid config -> resolver returns `augment` scoped (native+enforcer both selected); absent config -> scoped `augment` default (never whole-repo); a mode silently suppressing our checks without a gated waiver -> fails honesty | PENDING |
| f04 | P1 (T1) | `run-context-agent-inline-silent` | `proof/scan/f04-run-context.txt` | force a UI/server open under `agent-inline` -> refused (test fails if a listener binds); `human-review` -> UI/server start path reachable (loopback+token) returns structured HTML; deny-hook + MCP scan with no mode set -> resolved mode is `agent-inline` AND no server socket/UI artifact produced | PENDING |
| f05 | P1/P3 (T1) | `router-detect-route-plan` + `router-scope-narrowing` (`tests/router/route-plan.test.mjs` over fail/pass fixtures + d01 parity oracle over router ids) | `proof/scan/f05-route-plan.txt` | mixed `Cargo.toml`+`package.json` repo plan missing ts when package.json present -> fail (pass: rust+ts packs AND their native tools); python-only folder plan leaking rust pack -> fail (pass: python-only); crate scope resolving repo-wide -> fail (pass: single crate); unknown ext emitting a bogus T1 pack -> fail (pass: literal-scan T2-only floor, never a T1 blocker) | PENDING |

### Track G - UI layer on vendored hub dashboard/server (g01 .. g07)

All G packs honor f04 silent mode: no UI render / no server bind during inline agent runs.

| Workpack | Proof tier(s) | Named test / oracle | Artifact path | Seeded-violation case | Status |
|----------|--------------|---------------------|---------------|-----------------------|--------|
| g01 | P5 | `serve-surface-contract` (CLI aliases resolve, loopback-default, vendored HTTP core reused) | `proof/ui/g01-serve.json` | `serve-remote-no-token` (host bind without token) -> server refuses to start; `serve-loopback-default` -> binds 127.0.0.1, returns shell HTML with the view-mount registry present; the vendored `server.js` HTTP core reimplemented instead of wrapped -> fails | PENDING |
| g02 | P3 | `report-view-contract` (violation-matrix completeness + grouping keys + silent-mode suppression) | `proof/ui/g02-report.json` | `report-silent-mode-suppressed` (f04 silent active) -> zero UI output emitted; `report-matrix-render` (fixture `.enforce/` mixed severities) -> matrix groups correctly and every row exposes `ruleId` + why/doc-anchor + file:line; a row missing anchor/location or an external asset fetched -> fails | PENDING |
| g03 | P1 (T1) | `waiver-honesty-actions` (a08 waiver shape; no silent suppression) | `proof/ui/g03-actions.json` | `ignore-without-reason-refused` (empty reason) -> action rejected, nothing written; `ignore-writes-named-waiver` -> a named `.enforce/` waiver row appears with owner+reason+ruleId (NOT a hidden mute); any code path performing a silent suppression -> fails | PENDING |
| g04 | P5 (T1) | `run-dispatch-intent` (fix-intent schema at boundary; ledger write via a-conv-23 API) | `proof/ui/g04-run-dispatch.json` | a Run payload missing ruleId/files -> rejected (schema error, zero ledger writes); a valid Run click -> one well-formed fix-intent claimable by MCP `coordination_claim`; a duplicate Run not deduping on ruleId+files (forks a lane) -> fails | PENDING |
| g05 | P1/P5 (T1) | `settings-config-writes` (writes route through c-track adapters; golden config diff) | `proof/ui/g05-settings.json` | a waiver save missing owner/reason/ruleId -> rejected, writes nothing; toggling a CI gate -> correct hook/CI config written once (temp-dir, diff matches golden); re-toggling ON twice not byte-identical (dup hook lines) -> fails; the UI touching config files directly instead of via adapters -> fails | PENDING |
| g06 | P3 (T1) | `hub-dashboard-mount` (live materialized state via a-conv-23; read-only) | `proof/ui/g06-hub.json` | a corrupt/missing materialized view not rendering the empty state (throws / non-200) -> fails; a seeded ledger with one lane + one claim -> served HTML contains those exact lane/claim ids; the dashboard issuing ANY mutating call against the coordination API (spy) -> fails | PENDING |
| g07 | P5 (T1) | `ui-security-contract` (loopback-bind assertion + origin/CSRF + dispatch-authorization guards, sourced from `src/ui/security/*`) | `proof/ui/g07-security.json` | `sec-xorigin-waiver-reject` (cross-origin POST to waiver/config endpoint) -> rejected; `sec-remote-bind-no-token` (non-loopback bind without token) -> refused to start; `sec-dispatch-no-token` (dispatch without a valid intent token) -> refused; `sec-same-origin-token-ok` (same-origin + valid token to waiver/config/dispatch) -> succeeds; guards re-inlined per endpoint instead of sourced from `src/ui/security/*` -> fails | PENDING |

### Cross-cutting - rename + docs refresh + rename migration + terminal dogfood gate (x01, x02, x03, z01)

| Workpack | Proof tier(s) | Named test / oracle | Artifact path | Seeded-violation case | Status |
|----------|--------------|---------------------|---------------|-----------------------|--------|
| x01 | P1 (T1 grep-clean + mcp:smoke) | named rows `neutral-rename-grep-clean` + `neutral-rename-mcp-smoke` | `proof/rename/x01.txt` | any residual `ocentra[-_]enforcer` token in shipped/config surfaces (package.json, renamed entry scripts, `enforcer.config.json`, managed-block/global-instruction) -> grep non-empty = fail; mcp:smoke not green under server name `enforcer` -> fail. (Scoped to shipped/config surfaces, NOT `Tools/ocentra-literal-scan/**` nor plan-doc prose.) | PENDING |
| x02 | P1 (T1 grep-clean + sections-present) | `docs-refresh-check` (named rows `docs-refresh-grep-clean` + `docs-refresh-sections-present`) | `proof/docs/x02-docs-refresh.txt` | fail: a docs surface with a stale `ocentra`/`codex install` *product* reference OR a missing capability section -> gate names the offending file/section. pass: `grep -riE "ocentra\|codex install"` over owned docs product surfaces (excluding x01-owned path refs + `Tools/ocentra-literal-scan/**`) returns empty AND every new top-level capability (router f05, scan modes f01, UI g-track, multi-harness c-track incl. c09, onboarding f02, silent-vs-human f04, dart/cfml/frontend/python) has a present non-empty doc section. | PENDING |
| x03 | P1 (T1) | `rename-migration-contract` | `proof/install/x03-rename-migration.txt` | fail (`migrate-legacy-config-present`): a harness config still carrying the old `ocentra-enforcer` server entry after migrate ran -> re-scan finds the old entry. pass (`migrate-legacy-config-rewritten`): migrate detects the legacy entry + legacy `rust_rules_*`/`ocentra_enforcer_*` tool names, rewrites the registration to `enforcer` (`mcp__enforcer__*`), emits one one-time notice, is idempotent on re-run, and a post-migration re-scan finds **zero** `ocentra-enforcer` entries (transitional, no permanent alias). | PENDING |
| z01 | P4 self-enforce green (T1 terminal gate on plan-DONE) | `dogfood-gate.test.mjs` (named row `dogfood-self-zero-violations`) | `proof/dogfood-manifest.json` | a deliberately-planted self-violation (seeded T1 breach in a fixture repo state) makes the gate exit non-zero and refuse the DONE verdict; the clean repo state produces a PASS manifest (zero self-violations, no advisory above the committed T2 ceiling). Runs LAST, after every authoring/validating pack is DONE. | PENDING |

### Track B - planning skill (b01 .. b05)

| Workpack | Proof tier(s) | Named test / oracle | Artifact path | Seeded-violation case | Status |
|----------|--------------|---------------------|---------------|-----------------------|--------|
| b01 | P1 unit (T1) | `scaffolder.emit` + `scaffolder.determinism` + b02 cross-check | `proof/plan/b01-emit.txt` | emitter output != golden tree; two runs differ; refuses overwrite w/o `--force` fails | PENDING |
| b02 | P4 self-enforce green (T1) | `validator.rules` + `validator.selfhost` + PLAN-* parity | `proof/plan/b02-selfhost.txt` | run vs this plan dir with a seeded capsule/owns-overlap violation -> non-zero | PENDING |
| b03 | P0 contract/schema (T1) | `templates.snapshot` + no-inline-capsule grep/AST | `proof/plan/b03-templates.txt` | template drift vs frozen fixture; a capsule literal outside `src/plan/templates/` -> fails | PENDING |
| b04 | P1 unit (T1) | `orchestrator.frontier` + `.lanes` + `.claim-guard` (MCP fake) | `proof/plan/b04-orch.txt` | overlapping-owns concurrent claim not rejected -> fails; claim/guard/closeout out of order -> fails | PENDING |
| b05 | P4 self-enforce green (T1) | `skill.selfvalidate` + `skill.command` + doc-parity | `proof/plan/b05-skill.txt` | `/plan` dispatches a stub not the real validator; a SKILL.md doctrine claim with no ruleId -> fails | PENDING |

---

## 5. Closing checklist (per workpack, before DONE)

1. Confirm the proof tier via the section-3 decision tree; if the workpack file's stated tier disagrees, resolve before proceeding.
2. Ensure the named test(s) exist and pass on the migrated tree.
3. For T1 / P4 / P5: demonstrate the seeded-violation case actually fails (a green test that never trips is a hollow scan — doctrine violation).
4. For T2: assert `score in [0,1]`, a confidence value, and zero effect on exit code.
5. For T3: assert the mechanical **label gate** passes (the prose is never trusted directly).
6. Flip this row's Status to `GREEN`, then and only then move the WORKPACK_INDEX status to DONE.
