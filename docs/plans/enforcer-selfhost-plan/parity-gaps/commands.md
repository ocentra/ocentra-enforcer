# Parity gaps — ADBP `commands/`

Delta of normative gates enforced by ADBP commands that our `rules.json` does **not** back for a **target project**.
Note: our `CI-*`, `TEST-*`, `DEP-1.1`, `SBOM-1.1`, `SEC-*` families enforce the **Enforcer's own** self-host CI; they do not run the gate against an arbitrary audited project the way these commands do. Where a family only self-checks the enforcer, it is marked PARTIAL.

## deploy-check.md (Simpro Cloud — no backing at all)

| ADBP point | ADBP source | Backed? | Tier | Proposed ruleId | Fail-fixture | Pass-fixture |
|---|---|---|---|---|---|---|
| `claude-cli`/local-only LLM provider must NOT ship to a deploy target (repo has score.yaml) | deploy-check 4.4.1 | NO | T1 | DEPLOY-1.1 | score.yaml + design.json `llm_provider:"claude-cli"` | design.json `llm_provider:"bedrock"` |
| `score.yaml` required fields present (service.name/namespace/team, api.basePath, healthCheckPath, images.{env}.imageTag) | deploy-check 4.1 | NO | T1 | DEPLOY-1.2 | score.yaml missing service.namespace | complete score.yaml |
| `api.basePath` != "/" without domains (catch-all blocks other services) | deploy-check 4.1 | NO | T1 | DEPLOY-1.3 | basePath:"/" no domains | basePath:"/svc" |
| `service.team` in {addons,platform,premium,mobile,pfs} | deploy-check 4.1 | NO | T1 | DEPLOY-1.4 | team:"random" | team:"platform" |
| `imageTag` follows `v{semver}` | deploy-check 4.1 / Validation Ref | PARTIAL (2 prose "semver" hits, no rule) | T1 | DEPLOY-1.5 | imageTag:"latest" | imageTag:"v1.2.3" |
| `productionReady:true` required for prod envs (AU/US/UK) | deploy-check 4.1 | NO | T1 | DEPLOY-1.6 | AU deploy, productionReady absent | productionReady:true |
| database enabled → secrets.paths includes database vault path | deploy-check 4.2 | NO | T1 | DEPLOY-1.7 | database.enabled, no secrets.path | path present |
| sqs enabled → config has queue URL AND worker module exists | deploy-check 4.2/4.4 | NO | T1 | DEPLOY-1.8 | sqs.enabled, no worker.py | worker.py present |
| sidecar.frontend → backend port 3001 (port conflict) | deploy-check 4.2 | NO | T1 | DEPLOY-1.9 | sidecar+backend port 3000 | backend 3001 |
| IRSA annotation present if s3/sqs enabled (charts) | deploy-check 4.3 | NO | T2 | DEPLOY-1.10 | values.yaml no IRSA, sqs on | IRSA annotation set |
| charts values image.tag/repository in sync with score.yaml | deploy-check 4.3 | NO | T1 | DEPLOY-1.11 | tag mismatch | tags equal |
| health endpoint handler exists for healthCheckPath | deploy-check 4.4 | NO | T1 | DEPLOY-1.12 | no handler for /health | handler present |

## deferred.md / deferred-tracker (no backing)

| ADBP point | ADBP source | Backed? | Tier | Proposed ruleId | Fail-fixture | Pass-fixture |
|---|---|---|---|---|---|---|
| No unmarked HARD stub (`raise NotImplemented(Error)`, generic "not implemented", `UnimplementedError()`, unconditional skip, deferral `pytest.skip/skipif`) without `DEFERRED(#ref)` marker | deferred Step 3 | NO (0 hits: deferred/NotImplemented) | T1 | DEFER-1.1 | `raise NotImplementedError()` no marker | `raise NotImplementedError()  # DEFERRED(#42): reason` |
| `DEFERRED(...)` marker must be well-formed (tracker ref + reason present) | deferred Step 3/4 | NO | T1 | DEFER-1.2 | `# DEFERRED: soon` (no #ref) | `# DEFERRED(#42): reason [revisit: v2]` |
| SOFT deferral signals (TODO/FIXME + "for now"/"vN stub") flagged for review | deferred Step 3 (--strict) | PARTIAL (TODO has 9 prose hits, no dedicated project-scan rule) | T2 | DEFER-1.3 | `# TODO: for now` | tracked TODO(#n) |

## audit-ci.md — project gate surface (self-host only ⇒ PARTIAL/NO for target project)

| ADBP point | ADBP source | Backed? | Tier | Proposed ruleId | Fail-fixture | Pass-fixture |
|---|---|---|---|---|---|---|
| Coverage gate must set a **failing** threshold (`fail_under`/vitest thresholds/`--cov-fail-under`), not just collect | audit-ci Step 2 trap 2 / Step 6 (≥70) | NO (0 hits fail_under/cov-fail-under) | T1 | CIGATE-1.1 | pyproject `[tool.coverage]` no fail_under | `fail_under=70` |
| Test step must invoke coverage flag (config threshold w/o `--coverage`/`--cov-fail-under` = drift) | audit-ci Step 2 trap 2 | NO | T1 | CIGATE-1.2 | `vitest run` no --coverage | `vitest run --coverage` |
| CI parity: each sub-project path covered by a workflow whose `paths:` trigger includes it | audit-ci Step 2 trap 1 | PARTIAL (CI-1.15 self only; no path-filter parity, 0 `paths:` hits) | T1 | CIGATE-1.3 | frontend-next/ scripts, no workflow matching `frontend-next/**` | workflow with matching paths |
| Project-level secret scan (gitleaks) wired local+CI | audit-ci Step 2 / check 1.5 | PARTIAL (CI-1.8 gitleaks = enforcer's own CI, not audited project) | T2 | CIGATE-1.4 | project no gitleaks hook | gitleaks in hooks+CI |
| Project-level dep/CVE audit (pip-audit/pnpm audit/cargo audit) wired | audit-ci Step 2 / check 1.5 | PARTIAL (DEP-1.1/cargo audit = self only) | T2 | CIGATE-1.5 | project no dep-audit | dep-audit in CI |
| Local hook runner present (pre-commit / lefthook / xtask+git hook) | audit-ci Step 2 | NO (0 xtask hits) | T2 | CIGATE-1.6 | no hook runner file | .pre-commit-config.yaml present |
| Deployed linter freshness: `tools/pre_commit_checks/*.py` must match recipe (version+hash) | audit-ci Step 2.7 | NO (0 recipe-version hits) | T2 | CIGATE-1.7 | stale copy, version behind | matching `# recipe-version:` + hash |
| Mandated `ARCHITECTURE.md` present WITH required H2s (Overview, Tech Stack, Project Structure, Layer Responsibilities, Data Flow, Key Domain Concepts) | audit-ci 2.8 / check Step 4 / review Agent 5 | PARTIAL (ARCHITECTURE.md 25 prose hits; no presence+header validator) | T2 | DOCGATE-1.1 | ARCHITECTURE.md missing headers | all H2s present |
| Mandated `decisions.md` ADR log present at root | audit-ci 2.8 / review Agent 5 | NO (0 decisions.md hits) | T2 | DOCGATE-1.2 | no decisions.md | decisions.md exists |

## check.md — pre-merge delta gate

| ADBP point | ADBP source | Backed? | Tier | Proposed ruleId | Fail-fixture | Pass-fixture |
|---|---|---|---|---|---|---|
| Diff scope = merge-base(base,HEAD)..HEAD, filter to source ext, ignore generated (.g.dart/.freezed.dart/target/) | check Step 0 | PARTIAL (SCAN-1.15/1.16 diff scope exists; no merge-base base-branch semantics) | T2 | SCOPE-1.1 | scan includes committed-before-base files | only ACMR delta scanned |
| New endpoint/service method → must have companion test file | check Step 4 / review Agent 4 | PARTIAL (TEST-2.1 scaffold self; no per-new-symbol companion) | T1 | COMP-1.1 | new router, no test file | test file present |
| New entity with `status` field → must have FSM definition | check Step 4 | PARTIAL (ARCH prose; no cross-file FSM-coverage rule) | T2 | COMP-1.2 | model has status, no state_machine | FSM present |
| New enum value → FSM transition map updated | check Step 4 | NO | T2 | COMP-1.3 | enum extended, FSM stale | FSM updated |
| New module/dir added → ARCHITECTURE.md updated | check Step 4 | NO | T2 | COMP-1.4 | new dir, ARCHITECTURE.md untouched | doc updated |
| TODO/FIXME/HACK must carry tracker ref `// TODO(#1234):` | check Step 2 (all files) | PARTIAL (LIT/TODO prose; no tracker-ref-required rule) | T1 | LITTODO-1.1 | `// TODO: fix` | `// TODO(#1234): fix` |
| New structural surface (new layer dir / new vertical slice) must be covered by an import-linter/mechanical guard | check Step 6b | NO | T2 | GUARD-1.1 | new `routes/` dir, no import-linter contract naming it | contract present |
| Recurring pattern (5+ files) should graduate to mechanical linter | check Step 6a | T3 (advisory graduation heuristic; not a pass/fail gate) | T3 | — | — | — |

## plan-validate.md / plan-fix-validate.md — plan gates

| ADBP point | ADBP source | Backed? | Tier | Proposed ruleId | Fail-fixture | Pass-fixture |
|---|---|---|---|---|---|---|
| Multi-phase plan (≥2 phases / ≥30 fix units / ≥3 themes) MUST be split into per-phase files | plan-validate 3a / plan-fix-validate 3a | NO | T1 | PLAN-1.1 | monolithic IMPLEMENTATION_PLAN.md, 3 phases | per-phase files exist |
| Phase file must not exceed 300 lines | plan-validate 3a | NO | T1 | PLAN-1.2 | phase file 400 lines | <300 |
| Fix unit file list must be complete — no "e.g."/"such as"/"and similar" | plan-fix-validate 3c/Step4 | NO | T1 | PLAN-1.3 | "e.g. services/foo.py" | explicit list |
| Fix unit grep pattern must return >0 matches now (no stale patterns) | plan-fix-validate 1.5 | NO (0 grep-pattern hits) | T1 | PLAN-1.4 | pattern matches 0 files | pattern >0 |
| Plan `## CI Gate Surface` section present + names phase activating a failing coverage floor | plan-validate 4.5 | NO (0 CI Gate Surface hits) | T1 | PLAN-1.5 | no CI Gate Surface section | section names coverage-floor phase |
| Static hook-runner present OR Phase-1 scaffold task planned | plan-validate 4.5 | NO | T1 | PLAN-1.6 | no hook runner, no scaffold task | scaffold task in Phase 1 |
| Reference example must demonstrate END state (not contain the violation pattern) | plan-fix-validate Step2/Step4 | NO | T2 | PLAN-1.7 | ref file contains violation pattern | ref shows fixed pattern |
| Every actionable REVIEW.md 🔴/🟡 finding maps to a fix unit or Deferred Items | plan-fix-validate Step0/Step4 | NO (0 REVIEW.md/FIX_PLAN hits) | T2 | PLAN-1.8 | finding absent from plan+deferred | mapped/deferred |

## review-architecture.md / validate-review.md / heal-review.md

Most Agent 1–5 rules map to existing `ARCH-*`, `PY-*`, `TS-*`, `SEC-*`, `TEST-*` families (fully backed) — omitted. Command-orchestration constraints below lack backing:

| ADBP point | ADBP source | Backed? | Tier | Proposed ruleId | Fail-fixture | Pass-fixture |
|---|---|---|---|---|---|---|
| Finding must be proven by a grep returning ≥1 match; quote matched line (no invented findings) | review Agent verification discipline | T3 (LLM-output discipline; no mechanical gate possible on the model's reasoning) | T3 | — | — | — |
| Heal orchestrator must NOT edit source itself (only REVIEW*.md); code via subagent | heal-review Delegation Rule | T3 (agent-runtime behavior, not artifact-checkable) | T3 | — | — | — |

## scaffold-linter.md

Scaffolder workflow (detect stack → pick recipe → adapt → write one check + fixtures). Its output *product* — a mechanical check with fail+pass fixtures — is the doctrine target, not a rule to enforce. Gap it implies:

| ADBP point | ADBP source | Backed? | Tier | Proposed ruleId | Fail-fixture | Pass-fixture |
|---|---|---|---|---|---|---|
| A scaffolded linter must ship with both a fail fixture and a pass fixture (test proves it detects) | scaffold-linter (doctrine) / DOCENF-1.2 (self only) | PARTIAL (DOCENF-1.2 requires fail+pass in *rule docs*, not scaffolded project linters) | T1 | LINT-1.1 | new check, only pass fixture | check + fail + pass fixtures |
