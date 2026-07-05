# Lessons ships-via audit — 2026-07-05 (fresh-context, zero-trust)

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `lessons-audit-2026-07-05` (refs)
> Kind: point-in-time audit evidence. A fresh-context agent verified every orchestration-lessons row's `ships-via` claim against actual landed code.
> Read when: executing x05 (mechanizing the ledger), closing the lessons-mechanization backlog, or auditing fake-green on learning claims.
> Stop rule: evidence snapshot — do not edit rows; a NEW audit gets a NEW dated file.
<!-- /agent-capsule -->

Method: four parallel read-only agents traced each ledger row's named mechanism to code/config, with file:line evidence. Verdict = LANDED (code exists and matches the claim) | PROSE (claim has no landed artifact yet) | PARTIAL.

**Summary: 35 LANDED, 13 PROSE.** (Split-verdict rows count toward PROSE if any named mechanism is unverified.)

## Recurring PROSE gaps (the mechanization backlog seeds)

| Gap | Affected rows | Status |
|---|---|---|
| x02 "how this was built" story artifact | L16-adjacent fills, L25, L29 | EXPECTED-PENDING — x02 pack not yet claimed; the story artifact is its deliverable |
| z01 dogfood-completeness code | L23, L36 | EXPECTED-PENDING — z01 is the terminal gate, lands last by design |
| b05 `/plan` skill file | L16 | EXPECTED-PENDING — b05 capstone not yet claimed |
| L21 hub TTL/repair-stale enforcement | L21 | DEFERRED IN CODE — `api.rs:471` explicitly defers; ttl fields exist unused |
| L37 independent `.chain` sidecar | L37 | **DISCREPANCY** — journal.rs has a real inline hash-chain + tamper tests, but the ledger's landed-at claims an independent sidecar. Either the d04 sidecar exists elsewhere (audit may have checked arc-17's journal, not d04 telemetry) or the ledger over-claims. VERIFY before z01. Note: X06.1 store is being built with the sidecar requirement explicitly, so the mechanism ships regardless. |
| L15 boundary-allowlist rule data | L15 | GAP — RR-6.1 exists generically; no dedicated boundary-allowlist json |
| L26 mod-registration claim handling | L26 (sub-claim) | DOCTRINE-ONLY — orchestrator behavior landed; arc-16 claim design does not special-case mod files |
| L27 scout guidance | L27 (sub-claim) | DOCTRINE-ONLY — lives in worker prompts, not a shipped surface |

## Full row-by-row verdicts

| id | ships-via summary | verdict | evidence |
|---|---|---|---|
| L1 | fixed MCP tool behavior (arc-16) | LANDED | enforcer-coordination/src/api.rs:99 init idempotent, test init_is_idempotent_l1:650 |
| L2 | fixed MCP tool behavior (arc-16) | LANDED | api.rs:54 CallerContext struct, required param in claim/release |
| L3 | c01 doctrine payload (worker-protocol) | LANDED | EXECUTION_MODEL.md + scaffolder.rs capsule renderer |
| L4 | c01 doctrine payload + b06 decision forest | LANDED | agents_forest.rs (b06); EXECUTION_MODEL.md §2d |
| L5 | c01 doctrine payload | LANDED | EXECUTION_MODEL.md; three-role gate doctrine text present |
| L6 | c01 doctrine payload | LANDED | EXECUTION_MODEL.md §2e content |
| L7 | c01 doctrine payload (worker-protocol) | LANDED | EXECUTION_MODEL.md + doctrine payload in hooks/mod.rs |
| L8 | b04 orchestrator binding + c02 capability manifest | LANDED | orchestrator.rs tick/liveness; detect.rs capability_manifest_for() |
| L9 | x03 rename/cutover migration | LANDED | enforcer-install/src/migrate_legacy_name.rs; enforcer-mcp/src/aliases.rs |
| L10 | arc-18 harness + d-track dependency-policy rules | LANDED | enforcer-harness 2151 loc; DEP-1.1 in lang-common/src/families/dep_1.rs |
| L11 | b04 orchestrator binding | LANDED | orchestrator.rs tick/respawn logic |
| L12 | c01 doctrine payload (worker-protocol) | LANDED | EXECUTION_MODEL.md doctrine text |
| L13 | fixed MCP tool behavior (arc-16) + c01 doctrine | LANDED | api.rs:186 normalize_owns_paths, MAX_CLAIM_PATHS=10 batching |
| L14 | b04 orchestrator binding + c01 doctrine | LANDED | orchestrator.rs liveness/watchdog (staleness_threshold_ticks) |
| L15 | rules-as-data (arc-04/arc-06) | PROSE | RR-6.1 exists as generic RuleId, no dedicated boundary-allowlist json found |
| L16 | b04 + b05 /plan skill + c02 capability mapping | LANDED (b04/c02); PROSE (b05) | orchestrator.rs tick loop, detect.rs manifest; no /plan skill file found anywhere |
| L17 | a01 toolchain contract + c01 doctrine | LANDED | .gitattributes eol=lf rules present, matches described fix |
| L18 | x05 import + x06 graph + d22 size/shape rules | LANDED | enforcer-memory graph.rs/lesson.rs; size_shape.rs 740+ loc |
| L19 | b04 orchestrator loop + c02 capability manifest + c01 doctrine | LANDED | orchestrator.rs; detect.rs manifest |
| L20 | rules-as-data (d01/d-track) | LANDED | enforcer-mechanization engine + DEP-1.1 family code |
| L21 | fixed MCP tool behavior (arc-16) + c01 doctrine | PROSE | no repair_stale/heartbeat/TTL-enforcement code; ttl fields unused; api.rs:471 explicitly defers this |
| L22 | b04 orchestrator loop (integrate step) | LANDED | orchestrator.rs verify-before-integrate logic |
| L23 | b04 orchestrator loop + z01 dogfood completeness | LANDED (b04); PROSE (z01) | orchestrator.rs integrate; no z01/dogfood-completeness code |
| L24 | b01 plan-scaffolder + b02 plan-validator | LANDED | scaffolder.rs render_requirement_checklist; validator.rs check_checklist_drift |
| L25 | rules-as-data (arc-04) + x02 story | LANDED (arc-04); PROSE (x02) | enforcer-rules registry.rs/loader.rs; x02 only a workpack doc |
| L26 | b04 orchestrator loop + arc-16 claim design (mod-registration) | LANDED (b04); PROSE (mod-registration) | orchestrator.rs integrate; no lib.rs/mod special-casing in coordination |
| L27 | b04 orchestrator loop (verify) + worker template scout guidance | LANDED (b04); PROSE (scout guidance) | orchestrator.rs verify step; scout guidance doctrine-only |
| L28 | x05 doctor + b02 plan-validator | LANDED | lessons.rs:1759 real_seed_corpus_imports_and_doctor_reports_honest_verdict; validator.rs id-grammar check |
| L29 | a10 self-CI + c10 CI templates + x02 story | LANDED (c10); PROSE (a10 label, x02) | ci/installer_scripts.rs; workflow real but "a10" absent from git log; x02 no artifact |
| L30 | b04 orchestrator integrate step | LANDED | orchestrator.rs integrate logic |
| L31 | b04 orchestrator verify step + a10/c10 CI | LANDED | orchestrator.rs verify; ci/installer_scripts.rs; ocentra-enforcer.yml |
| L32 | b04 orchestrator liveness + c01 worker-protocol doctrine | LANDED | orchestrator.rs liveness detection |
| L33 | b04 orchestrator verify + a10/c10 CI | LANDED | orchestrator.rs verify; ci.yml workspace build+test |
| L34 | c10 CI + a10 self-CI (EOL policy) | LANDED | .gitattributes install.ps1/sh eol=lf override lines |
| L35 | a10 self-CI (gitignore-completeness) + c01 doctrine | LANDED | .gitignore `/coverage/` root-anchored |
| L36 | b04 orchestrator gate + a10/z01 full-suite ownership | LANDED (b04); PROSE (z01) | orchestrator.rs gate scaling; no z01 code |
| L37 | arc-17 proof + a10 self-CI (integrity-verify) | PARTIAL/PROSE | journal.rs inline hash-chain + tamper tests; no independent `.chain` sidecar as literally claimed |
| L38 | b04 orchestrator salvage protocol + c01 worker-protocol | LANDED | orchestrator.rs respawn():875, triage logic |
| L39 | d21 change-discipline rule + x05 self-heal corpus | LANDED | lang-common/src/rules/change_discipline.rs; lessons.rs real_seed_corpus test |
