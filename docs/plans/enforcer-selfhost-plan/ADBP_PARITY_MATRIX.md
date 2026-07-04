# ADBP Parity Matrix (Master)

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `ADBP Parity Matrix (Master)`
> Kind: authoritative parity matrix. The single deduped classification of every ADBP borrow point into a tested-enforcement tier.
> Read when: You need the canonical list of ADBP points, their tier, and the concrete fail/pass fixture + detection test that backs each one. Supersedes the empty `parity/` directory and the tier hints scattered across the D/C workpacks.
> Stop rule: This file classifies and points at backing tests; it does not itself prove they are green. A row is only real when its named validator/test passes. Workpack proof rows in TEST_PROOF_EXPECTATIONS.md remain the DONE authority.
> Proves: the tier assignment and the existence of a concrete fail-fixture + pass-fixture + detection test per T1/T2 point.
> Does not prove: that those tests currently pass, or workpack DONE status.
<!-- /agent-capsule -->

Sources merged: `docs/plans/enforcer-selfhost-plan/parity/**` (empty — no per-stack files present), the Track D ADBP-borrow workpacks (`d01`–`d15`), the ADBP-derived Track C harness gates (`c04`, `c05`, `c06`), `README_FULL_ORIGINAL.md` (the DOCTRINE + borrow narrative), and the mechanized per-stack rule registry `rules/rules.json` (569 rows) that these points classify against.

---

## The tested-enforcement doctrine (the spine)

Rules are conditions; **enforcement MUST be mechanical and TESTED**. Prose without a backing check is hope, not proof. Every ADBP point is dragged UP a three-rung ladder — never copied down as prose:

- **T1 — deterministic validator.** Fail-closed. Full parity: every `ruleId` has a validator, a resolving doc anchor, and pass/fail fixtures that agree. Anything that *blocks* must be T1, and must ship a **fail-fixture (must be flagged)** + **pass-fixture (must stay clean)** + a **detection test** that exercises both.
- **T2 — scored/advisory scanner.** A regex/AST/heuristic that emits a `score` in `[0,1]` and a `confidence`, and does **not** block — the Rust literal-scan model. Still mechanized, still tested: its fixtures assert the score crosses (fail-fixture) or stays under (pass-fixture) the threshold. T2 is mechanization that chooses not to be fatal, not an excuse to skip mechanization.
- **T3 — justified prose.** Reserved for genuine judgment where a *scored* test is impossible. **Does NOT count as enforcement.** Every T3 item MUST carry the label `advisory, no mechanization possible + <reason>`; the *presence of that label* is itself enforced at T1.

**Column contract (9 columns, every section):**
`ID` | `ADBP point` | `Stacks` | `Tier` | `Mechanism` | `Fail-fixture (must be flagged)` | `Pass-fixture (must stay clean)` | `Detection test` | `Backing (workpack · ruleId/family)`

---

## Summary

- **Total unique points: 45** (after collapsing per-stack repeats to one row noting the stacks). 42 rows in the tiered sections below + 3 in the T3 residue section.
- **By tier:** **T1 = 39** · **T2 = 3** · **T3 = 3**.
  - Per-section T1: Backend 7, Frontend/Mobile 5, Mechanical linters 11, Command gates 9, Agent golden-rules 4, Ergonomics 3.
  - The 3 standalone **T2** rows are `BACK-08` (string-literal-risk score), `AGENT-02` (reverse doc-rule advisory), and `ERG-04` (context-surface efficiency score).
  - Two points are **split-tier** (a blocking T1 lane + an advisory T2 lane): `MECH-05` (context-budget brake) and `AGENT-04` (resilience auditor). Each is counted **once, in T1** (its blocking lane), with the T2 score noted inline in the row — it is not double-counted in the T2 total.
- **T3 residue: 3** genuinely untestable points (see the final section). Their *labeling* is T1-enforced (`AGENT-05`); the judgment itself is not enforcement and gates nothing.

---

## Backend rules

Language-agnostic source-integrity and boundary rules that repeat across the Rust / TypeScript / JavaScript / Python stacks. Per-stack duplicates are collapsed into one row; the `Stacks` column records where the rule fires.

| ID | ADBP point | Stacks | Tier | Mechanism | Fail-fixture (must be flagged) | Pass-fixture (must stay clean) | Detection test | Backing (workpack · ruleId/family) |
|----|-----------|--------|------|-----------|-------------------------------|-------------------------------|----------------|-----------------------------------|
| BACK-01 | Deferred-work gate: no unmarked stub/deferral (`TODO`/`FIXME`/`unimplemented!`/`todo!`/`throw new Error("not implemented")`/`pass # TODO`) in added lines unless annotated `DEFERRED(#ref)[revisit:<v>]` | Rust, TS/JS, Python | T1 | Diff-scoped marker scan intersected with diff hunks; structured-annotation parser is the only escape hatch | Added line with a bare `TODO` / `unimplemented!()` stub, or a malformed `DEFERRED()` annotation | A correctly annotated `DEFERRED(#123)[revisit:2026-Q4]` stub, or a legacy stub outside the diff | `tests/deferred-work-gate.test.mjs` over `tests/fixtures/deferred/**` | d03 · new registry ruleId (via d01); overlaps `SRC-1.2`,`RR-4.2`,`RR-4.3` |
| BACK-02 | No suppression / validation-bypass comments (prettier-ignore, `eslint-disable`, `@ts-ignore`/`@ts-expect-error`, Python `# type: ignore`/`# noqa`, Rust `#[allow]`/`#[expect]`, rustfmt-skip, clippy suppressions) | Rust, TS/JS, Python | T1 | Deterministic bypass-comment scanner | File with an unexplained `@ts-ignore` / `#[allow(dead_code)]` / `# noqa` | File with no suppressions (or one downgraded only by explicit target policy) | `ocentra-enforcer check validation-bypass` fixtures | RR-2.\* · TS-2.1 · PY-1.\* |
| BACK-03 | No placeholder implementation: source `TODO`/`FIXME`/`TBD`/`placeholder` comments and not-implemented/debug-print code paths | Rust, TS/JS, Python | T1 | Placeholder-implementation scanner | Function body that is `// placeholder` / `console.log("debug")` return-nothing stub | Real implementation with no placeholder markers | `ocentra-enforcer check placeholder-implementation` fixtures | SRC-1.2 · RR-4.2 · RR-4.3 |
| BACK-04 | No naked domain strings / naked domain type aliases / manual brands (parse-at-boundary) | Rust, TS/JS, Python | T1 | AST/scanner for naked domain aliases and manual brand shapes | `type UserId = string` used as a domain id; a naked string passed where a branded id is required | A properly branded `UserId` type used at the boundary | `check no-naked-domain-strings` fixtures | RR-6.1 · RR-6.5 · RR-18.16 · TS-1.3 · PY-1.3 |
| BACK-05 | No re-exports / barrel files (architecture boundary) | Rust, TS/JS | T1 | Re-export architecture scanner (public wildcard + barrel forms) | `pub use crate::*;` wildcard re-export; a TS barrel `export * from './x'` | Direct imports; no re-export surface | `ocentra-enforcer check reexports` fixtures | TS-1.1 · RR-7.2 · RR-7.3 |
| BACK-06 | No test doubles in source (mocks/stubs/fakes leaking into non-test code) | Rust, TS/JS, Python | T1 | Test-double scanner over source paths | A `MockFoo` / `FakeRepo` defined in a `src/` file | Test doubles confined to test trees | `ocentra-enforcer check` (TEST-1.1) fixtures | TEST-1.1 |
| BACK-07 | Source-shape limits (file/function/export/type size caps) | Rust, TS/JS, Python | T1 | Config-driven source-shape validator | A file/function exceeding the configured cap | A file within all configured caps | `ocentra-enforcer check source-shape` fixtures | SRC-1.1 |
| BACK-08 | String-literal risk score (app string literals that should be constants/branded) | Rust, TS/JS | T2 | Scored literal scanner (the Rust literal-scan model) — score + confidence, non-blocking | A hot file whose literal density scores above the advisory threshold | A file below the literal-risk threshold | Rust literal-scan fixtures (`advise literals` / `check literal-risk`) | literal-risk family · `no-app-string-literals` |

## Frontend/Mobile rules

Layered / frontend AST linters borrowed from ADBP, mechanized as first-class registry ruleIds (T1 deterministic, not text-heuristic).

| ID | ADBP point | Stacks | Tier | Mechanism | Fail-fixture (must be flagged) | Pass-fixture (must stay clean) | Detection test | Backing (workpack · ruleId/family) |
|----|-----------|--------|------|-----------|-------------------------------|-------------------------------|----------------|-----------------------------------|
| FRONT-01 | No repository/data access inside the router layer (`no-repo-in-router`) | TS/JS (frontend) | T1 | AST visitor over router-layer files | A router module importing/calling a repository directly | A router that delegates to a service, no repo import | `tests/fixtures/layered-frontend/no-repo-in-router/{fail,pass}` via eslint-rule tester + d01 oracle | d12 · new registry ruleId |
| FRONT-02 | No data fetching inside `useEffect` (`no-fetch-in-useEffect`) | TS/JS (frontend) | T1 | AST visitor detecting fetch/axios calls in a `useEffect` body | A `useEffect(() => { fetch(...) })` | Fetch moved to a query hook / loader | `tests/fixtures/layered-frontend/no-fetch-in-use-effect/{fail,pass}` | d12 · new registry ruleId |
| FRONT-03 | Feature-boundary imports (`feature-boundaries`) — no cross-feature deep imports | TS/JS (frontend) | T1 | AST import-path boundary visitor | `import x from '../otherFeature/internal/x'` crossing a feature boundary | Import via the feature's public entry only | `tests/fixtures/layered-frontend/feature-boundaries/{fail,pass}` | d12 · new registry ruleId |
| FRONT-04 | String-enum-only enums (`str-enum-only`) — no numeric/implicit enums | TS/JS (frontend/mobile) | T1 | AST enum-shape visitor | A numeric or implicit-value `enum Color { Red }` | A string-valued `enum Color { Red = 'red' }` | `tests/fixtures/layered-frontend/str-enum-only/{fail,pass}` | d12 · new registry ruleId |
| FRONT-05 | Symbol-level dependency injection (`symbol-level-DI`) — no concrete-class DI | TS/JS (frontend/mobile) | T1 | AST visitor asserting DI tokens are symbols/interfaces, not concrete classes | Constructor injecting a concrete class directly | Injection via a symbol/interface token | `tests/fixtures/layered-frontend/symbol-level-DI/{fail,pass}` | d12 · new registry ruleId |

## Mechanical linters

The engine and the language-agnostic mechanical checks that make every other point testable, plus cross-cutting artifact/security/portability linters that repeat across stacks.

| ID | ADBP point | Stacks | Tier | Mechanism | Fail-fixture (must be flagged) | Pass-fixture (must stay clean) | Detection test | Backing (workpack · ruleId/family) |
|----|-----------|--------|------|-----------|-------------------------------|-------------------------------|----------------|-----------------------------------|
| MECH-01 | Rule-mechanization engine + fail-closed parity oracle (ruleId ↔ validator ↔ doc-anchor ↔ fixtures ↔ registry-row, both directions) — **the keystone** | all (engine) | T1 | Scaffolder + parity validator (`rule-scaffold-parity`) | A registry row with a dangling doc anchor / missing fixture / no validator export; an orphan validator with no row | The live registry re-validating green; a freshly scaffolded temp rule passing parity | `tests/rule-scaffold-parity.test.mjs`, `tests/rule-new.test.mjs` | d01 · engine (deps: none) |
| MECH-02 | Grandfather ratchet: baseline findings warn; any new finding or grown count/severity fails; ratchet only tightens | all | T1 | Set-diff of normalized finding keys vs persisted hashed baseline, fail-closed on delta | An added finding not in baseline, or a grandfathered finding whose count grew | An unchanged run (in-baseline findings warn, not fail) | `tests/baseline-ratchet.test.mjs` over `tests/fixtures/baseline/**` | d02 · ruleIds via d01 |
| MECH-03 | Run-telemetry NDJSON: exactly one schema-valid line per run (observer, never a gate) | all | T1 | Schema-decode-then-append writer; every emitted line re-parsed | A forced schema-violating telemetry line (rejected); a half-written line | Two runs → two independently-parseable valid NDJSON lines | `tests/run-telemetry.test.mjs` | d04 · Effect schema `schemas/effect/run-telemetry-schema.ts` |
| MECH-04 | CI parity: local hook/step set == CI job set; pinned tool versions agree across both sources of truth | all | T1 | Normalized set/version diff between local step manifest and CI manifest, fail-closed | An injected local-only step, or a node/rust-toolchain version skew | Matched step sets and matched pinned versions | `tests/ci-parity.test.mjs`; CI runs `scripts/ci-parity-verify.mjs` | d11 |
| MECH-05 | Context-budget brake: measured MCP tool-description surface ratcheted vs committed baseline (T1) + surface-per-tool efficiency score (T2 advisory) | Enforcer (self) | T1 (+T2 inline) | Static enumeration of the tool registry + byte/token count diffed vs baseline; T2 emits a `[0,1]` efficiency score | Simulated surface growth beyond tolerance (T1 fails) | Unchanged surface at/under baseline (T1 passes); T2 score is informational only | `tests/context-budget.test.mjs`; CI `scripts/context-budget-scan.mjs` | d05 · baseline `proof/context-budget-baseline.json` |
| MECH-06 | Rule-version + config-drift: content hash over the rule-config set must match the pinned manifest unless a version bump accompanies it | all (vendored) | T1 | Deterministic hash over `rules/rules.json` + `rule-id-lock.json` + config files compared to `rule-version-manifest.json`, fail-closed | Config content change with no version bump; or a version bump with no content change | Unchanged config; or a matched version+hash bump (both together) | `tests/rule-version-drift.test.mjs` | d13 · `rules/rule-version-manifest.json` |
| MECH-07 | No tracked generated artifacts (marker / generated-output-path / tracked-only modes) | all | T1 | Generated-artifact scanner across three modes | A checked-in file bearing a generated marker or under a generated-output path | Generated outputs untracked / gitignored | `check generated-artifacts --tracked` fixtures | GEN-1.1 · GEN-1.2 |
| MECH-08 | Staged-secret / sensitive-path scan (inline secrets, sensitive paths, staged-only mode) | all | T1 | Secret + sensitive-path scanner | A staged file with an inline API key / a sensitive path | No secrets; sensitive paths excluded per policy | `check secrets --staged` fixtures | SEC-1.1 · SEC-1.2 |
| MECH-09 | Cross-platform script portability (no unguarded Windows-only npm invocations in scripts) | TS/JS (scripts) | T1 | Portability scanner over `scripts/**` | A `scripts/*.mjs` calling a Windows-only command unguarded | A cross-platform-guarded invocation | `check` (PORT-1.1) fixtures | PORT-1.1 |
| MECH-10 | Anti-silent-skip: a scanner that ran zero checks fails instead of silently passing | all (engine) | T1 | Coverage assertion — zero-executed-checks is an error, not a green | A scan whose selector matched nothing yet reported pass | A scan that executed ≥1 check and reported honestly | `enforcer:coverage` / a09 anti-silent-skip test | a09 |
| MECH-11 | Waiver honesty: permissive "overrides" renamed to named, explicit waivers | all (engine) | T1 | Waiver-shape validator (named waiver required; anonymous override rejected) | An anonymous `overrides` block silently relaxing a rule | A named, justified waiver entry | a08 waiver-honesty test | a08 · WAIVER-\* family |

## Command gates

Harness-boundary and lifecycle gates borrowed from ADBP — the mechanical bridges that force the checks to run and bind phases to validators rather than model self-report.

| ID | ADBP point | Stacks | Tier | Mechanism | Fail-fixture (must be flagged) | Pass-fixture (must stay clean) | Detection test | Backing (workpack · ruleId/family) |
|----|-----------|--------|------|-----------|-------------------------------|-------------------------------|----------------|-----------------------------------|
| CMD-01 | PreToolUse deny-hook: on `Edit\|Write\|MultiEdit`, run scan/check + coordination guard and BLOCK T1 violations before the write lands (T2 warns, T3 never blocks, fail-closed on error) | all (Claude harness) | T1 | Hook reads PreToolUse payload from stdin; T1 finding → exit deny with `ruleId`+`fix` in reason | A seeded violating edit payload → must exit **deny** with the exact `ruleId` and its `fix` string | A conforming edit → exit **allow**; a T2-only finding → allow-with-warning, never deny | `claude-deny-hook-blocks` (P5) in TEST_PROOF_EXPECTATIONS.md | c04 · `src/install/hooks/pretooluse-*`,`guard-*` |
| CMD-02 | SessionStart hook: inject the enforcer-first reminder + T1/T2/T3 doctrine deterministically each session | all (Claude harness) | T1 | Deterministic reminder emitter from a single source-of-truth constant; snapshot-pinned | Reminder body drifting from the pinned snapshot (missing enforcer-first marker or doctrine tokens) | Byte-identical reminder containing the marker + T1/T2/T3 tokens | `claude-sessionstart-injects` (P5) snapshot test | c05 · `src/install/hooks/sessionstart-*` |
| CMD-03 | Lifecycle command family (`plan\|implement\|check\|fix\|review`): every phase's verdict decided by a named validator/oracle, no prose-only pass path; `review` blocks on missing proof rows | all | T1 | Command-dispatch table mapping phase → validator function; exit-code semantics asserted | A failing oracle that still reported phase success; `review` passing with missing proof rows | Each phase routing to its correct oracle; failing oracle → non-zero exit | `tests/lifecycle-commands.test.mjs` | d06 |
| CMD-04 | Self-correct fix loop: bounded fix→re-check→keep-or-revert; accept only if findings strictly decrease and no new ruleId appears; hard iteration cap; always terminates | all | T1 | Re-scan-and-compare gate wrapping deterministic snapshot/restore; before/after finding counts | A neutral/regressing "fix" that was kept, or a loop that exceeded the iteration cap | An improving fix kept; a regressing fix reverted; final findings ≤ start | `tests/fix-loop.test.mjs` over `tests/fixtures/fix-loop/**` | d07 |
| CMD-05 | Harness-feedback pipeline: classify each harness failure `prevent` vs `detect`; `prevent` auto-scaffolds a PROPOSED (non-blocking) validator via d01 | all | T1 | Classifier over parsed failure fields feeding the d01 scaffolder; resulting registry state asserted | A preventable failure that produced no PROPOSED row/fixtures; a PROPOSED rule that gated a build | A preventable failure → PROPOSED row + fixtures passing d01 parity; a detect-only failure → none | `tests/harness-feedback.test.mjs` over `tests/fixtures/harness-feedback/**` | d08 |
| CMD-06 | Required-tests gate: packages/apps with `src/` and Rust crates must carry test scaffolds | Rust, TS/JS, Python | T1 | Required-test-presence validator (`.gitkeep`-only vs truly-empty distinguished) | A package with `src/` and no test scaffold | A package/crate with the required test scaffold present | `ocentra-enforcer check required-tests` fixtures | TEST-2.1 |
| CMD-07 | Dependency-policy gate (npm high audit + license policy + cargo-audit when lockfiles exist) | Rust, TS/JS | T1 | Dependency-policy runner over present lockfiles | A high-severity advisory or a disallowed license | Clean audit, all licenses within policy | `ocentra-enforcer check dependency-policy` fixtures | DEP-1.\* |
| CMD-08 | Single-source contracts gate | all | T1 | Contract-uniqueness validator (accepts migrated contract config shape) | Two divergent definitions of the same contract | One canonical contract source | `ocentra-enforcer check single-source-contracts` fixtures | CONTRACT-1.1 |
| CMD-09 | SBOM emission gate | all | T1 | SBOM writer to target-root artifact path (supports `--dry-run`) | A run that failed to emit the required SBOM artifact | A run that wrote a valid SBOM to the requested path | `ocentra-enforcer check sbom` fixtures | SBOM-1.1 |

## Agent golden-rules

Rules about the agent/rule-doc layer itself — keeping persona/agent prose honest by tying every imperative to a real mechanized rule.

| ID | ADBP point | Stacks | Tier | Mechanism | Fail-fixture (must be flagged) | Pass-fixture (must stay clean) | Detection test | Backing (workpack · ruleId/family) |
|----|-----------|--------|------|-----------|-------------------------------|-------------------------------|----------------|-----------------------------------|
| AGENT-01 | Doc-rule citation parity: every must/never bullet in agent/rule docs cites an existing registry `ruleId`; uncited bullets fail closed (persona free-text ignored) | all | T1 | Markdown bullet parser extracting `[ruleId]` tokens, checked against the d01 registry map | A must/never bullet with no `[ruleId]`, or one citing a dangling id | A bullet citing a real registry id; persona prose with no imperative | `tests/doc-rule-parity.test.mjs` | d09 · `src/doc-rule-parity.ts` |
| AGENT-02 | Reverse doc-rule advisory: flag high-value rules that no agent doc mentions | all | T2 | Reverse index scan, advisory score, non-blocking | A high-value ruleId with zero agent-doc mentions (advisory flag) | Every high-value rule mentioned somewhere; low-value rules unflagged | `tests/doc-rule-parity.test.mjs` (advisory lane) | d09 (optional T2 lane) |
| AGENT-03 | AI-rule-index routing: AGENTS/rule docs routed through a small index; oversized rule files flagged | all | T1 | AI-rule-index validator | AGENTS.md not routed through the index, or an oversized rule doc | Docs routed through the index; rule files within size | `ocentra-enforcer check ai-rule-index` fixtures | AI-1.1 |
| AGENT-04 | Resilience auditor: adversarial pass emits required-test obligations (T1) + failure-mode smell scores (T2) | all | T1 (+T2 inline) | Obligation table (T1) forcing a matching test per accepted failure mode; regex/AST heuristic scorer (T2, `[0,1]`+confidence, non-blocking) | An accepted failure-mode obligation with no matching test (T1 fails) | Every obligation met by a test (T1 passes); smell scores never gate | `tests/resilience-auditor.test.mjs` over `tests/fixtures/resilience/**` | d10 |
| AGENT-05 | Ideation-skill T3 labeling: every file under `skills/ideation/**` carries the exact `Tier: T3 advisory — no mechanization possible: <reason>` label and appears in no rule registry | all | T1 | Label-presence validator over `skills/ideation/**` (mechanization is on the *label*, not the judgment) | An ideation skill file missing the T3 label, or one registered in a rule registry | Every ideation file labeled; none in any enforcement registry | `tests/ideation-skills-labeling.test.mjs` | d14 |

## Ergonomics

Meta / self-hosting ergonomics: parity of installed adapters, byte-stable emitters, and context-surface efficiency — the points that keep the tool honest about its own footprint.

| ID | ADBP point | Stacks | Tier | Mechanism | Fail-fixture (must be flagged) | Pass-fixture (must stay clean) | Detection test | Backing (workpack · ruleId/family) |
|----|-----------|--------|------|-----------|-------------------------------|-------------------------------|----------------|-----------------------------------|
| ERG-01 | Adapter parity (Codex): the adapter's generated TOML `mcp_servers` block + `AGENTS.md` managed block + doctor check names/severities equal the pinned golden snapshot | install (Codex) | T1 | Golden-file test vs `codexMcpTomlBlock`/`globalAgentsInstructionBlock`; doctor snapshot | Any byte diff in the generated TOML/AGENTS block or a changed doctor check name/severity | Adapter output byte-identical to the golden files | `codex-adapter-parity` (P5) in TEST_PROOF_EXPECTATIONS.md | c06 · `src/install/adapters/codex.*` |
| ERG-02 | Plan scaffolder byte-stability: `plan new` emits byte-stable output, refuses to overwrite | plan tooling | T1 | Golden-fixture emitter comparison + overwrite-refusal check | A non-deterministic / drifted `plan new` output, or an overwrite that clobbered an existing file | Byte-stable output; overwrite refused | b01 plan-scaffolder golden test | b01 |
| ERG-03 | Plan-structure validator self-gate: `PLAN-*` structure rules (capsule/frontmatter/owns-disjoint/deps/xlink) fail closed, and the `/plan` skill passes its own validator against this plan dir with zero findings | plan tooling | T1 | `PLAN-*` structure validator run against the plan directory (self-referential) | A plan doc missing a capsule / with overlapping `owns:` / a dangling xlink | This plan directory validating with zero `PLAN-*` findings | b02/b05 plan-structure + self-validate tests | b02 · b05 |
| ERG-04 | Context-surface efficiency score (surface bytes per tool) | Enforcer (self) | T2 | Advisory `[0,1]` score with confidence, non-blocking, recorded to telemetry | A tool whose description bloats the per-tool efficiency score below threshold (advisory) | Tool descriptions within the efficiency band | `tests/context-budget.test.mjs` (T2 lane) | d05 (T2 lane) |

---

## T3 residue (genuinely untestable — advisory only, does NOT count as enforcement)

These three points cannot be reduced even to a *scored* test, because they require open-ended human/model judgment with no ground-truth signal to score against. Per doctrine they ship only as labeled prose; **their labeling is the sole mechanized part (T1, see `AGENT-05`).** They gate nothing and produce no findings.

| ID | ADBP point | Why no T1/T2 is possible (justification) | The only mechanized guardrail |
|----|-----------|------------------------------------------|-------------------------------|
| T3-01 | Devil's-advocate ideation pass (`skills/ideation/devil.md`) | The value is adversarial reasoning about a *specific* design under discussion; there is no fixture whose "correct" set of objections is knowable in advance, so neither a deterministic verdict nor a meaningful score exists. | Its file must carry the exact `Tier: T3 advisory — no mechanization possible: <reason>` label and appear in no rule registry (enforced at T1 by `AGENT-05` / d14). |
| T3-02 | Think-with-me collaborative ideation pass (`skills/ideation/think-with-me.md`) | Open-ended co-reasoning has no defined pass condition and no scoreable target output; any "score" would be arbitrary, violating the T2 requirement that the score mean something testable. | Same T1 labeling gate as above (`AGENT-05` / d14); `skills/ideation/README.md` states it produces no findings and gates nothing. |
| T3-03 | README research-grounding narrative (`docs/research-grounding.md` + `README.md#research-grounding`) | It is a documentation deliverable adopting cited evidence for design choices; there is no code path to validate and no finding to emit. Doctrine explicitly scopes it as ships-no-validator. | Not a runtime rule; the honesty guardrail is that it is *labeled documentation-only* and its cross-links resolve (artifact-existence check in d15), never presented as a check. |
