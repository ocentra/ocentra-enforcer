# x05 Lesson Capture Self-Heal

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Lesson Capture Self-Heal`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `crates/enforcer-plan/src/lessons.rs, crates/enforcer-plan/templates/lesson-*.tpl, crates/enforcer-plan/tests/fixtures/lessons/**`
- deps: `arc-20`, `b06`
- tier: `P1 unit / P4 golden`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md), [RUST_ARCHITECTURE](../RUST_ARCHITECTURE.md), [orchestration-lessons](../refs/orchestration-lessons.md).

## Where We Are
Live orchestration of this very plan keeps producing lessons (mail lifecycle, stale-base resets, read-scope discipline, three-role gating — see the seed corpus [refs/orchestration-lessons.md](../refs/orchestration-lessons.md)) — but the capture is MANUAL: the orchestrator hand-edits EXECUTION_MODEL/prompt templates, and nothing guarantees a lesson ever reaches the HARNESS surfaces future sessions actually load (skills, AGENTS/CLAUDE managed blocks, rules, the b06 decision forest). A lesson that lives only in a plan doc dies with the plan; a lesson without a landed artifact is a TODO wearing a hat. The owner requirement (2026-07-04): "whatever we learn somehow needs to go into the harness — lesson learnt is captured, turned into a skill or rule — memory self-healing."

## Where We Want To Be
A mechanized lesson-capture loop in `enforcer-plan` (`src/lessons.rs`): lessons are STRUCTURED RECORDS (serde, branded ids) captured via `enforcer lesson add|list|route` (CLI via arc-22, MCP via arc-21), each carrying `observed` (live evidence), `lesson`, and one or more ROUTES declaring the harness surface it ships through — `doctrineBlock` (the c01 install payload / AGENTS/CLAUDE managed blocks), `skill` (a skill-file section), `ruleCandidate` (a d01 scaffold input), `forestNode` (a b06 decision-forest entry), or `planDoc` (EXECUTION_MODEL §-ref, transitional only). Emitters render each route from `templates/lesson-*.tpl`; a fail-closed doctor check verifies EVERY captured lesson has ≥1 LANDED artifact (the emitted block/section/scaffold exists and contains the lesson id), so lessons cannot silently rot as prose. The seed corpus in refs/ imports as the initial ledger. This is the self-healing memory: fault observed → lesson recorded → routed into the surface every future agent loads → the system that made the mistake cannot make it again silently.

## Requirement Checklist
- [ ] `src/lessons.rs`: `LessonRecord { id: LessonId, date, domain: LessonDomain, observed, lesson, routes: Vec<LessonRoute>, landedAt: Vec<ArtifactRef> }` — serde camelCase, `LessonId`/`ArtifactRef` branded `enforcer-domain` newtypes, parse-at-boundary, no bare `String` ids. `LessonDomain` = enum `{ Harness, Code }` (the learning thesis is DUAL-DOMAIN — see RUST_ARCHITECTURE "The learning thesis"); `LessonRoute` = enum `{ DoctrineBlock, Skill, RuleCandidate, ForestNode, PlanDoc }`. A `Code`-domain lesson routed `RuleCandidate` REQUIRES fail/pass fixtures at landing (the d01 parity oracle applies) — a coding lesson without fixtures cannot land.
- [ ] Ledger storage: append-only NDJSON under `.enforce/lessons.ndjson` (same append-only + verify-on-open discipline as `enforcer-proof`); never rewrite prior rows; a fill-in of a pending `landedAt` appends a supersede record.
- [ ] `enforcer lesson add` (CLI seam for arc-22, MCP tool seam for arc-21): captures a record; `lesson list` filters by route/pending; `lesson route <id>` runs the emitters for that record.
- [ ] Emitters render from `templates/lesson-*.tpl` per route: doctrine-block emitter feeds the c01 shared install payload (managed-block section keyed by lesson id); skill emitter appends a keyed section to the enforcer skill; rule-candidate emitter writes a d01 scaffolder input stub; forest-node emitter writes a b06 decision-forest node fragment. Emitters are pure over injected fs (temp-dir testable), honor `--dry-run` (zero writes), and preserve unrelated content (managed markers, c01 helpers).
- [ ] Fail-closed doctor check (contributed to the c07 shared doctor): every non-`PlanDoc` lesson must have ≥1 landed artifact whose content contains the lesson id; a pending/unrouted lesson is `Severity::Error`, not a warning. `PlanDoc`-only routes are TRANSITIONAL and flagged `Severity::Warning` (prose is not a landing).
- [ ] Import the seed corpus: a one-shot `lesson import` reads the refs lesson ledger — [refs/orchestration-lessons.md](../refs/orchestration-lessons.md) AND all `refs/lessons/*.md` domain shards, per the ledger's L18 split policy — table rows into the ledger (L1..L18+), mapping their `ships-via` column to routes; import is idempotent (re-import adds nothing). Post-import, the .md ledger stops being source-of-truth (graph destiny: rows become x06 lesson nodes; the .md becomes a generated view).
- [ ] Capture hooks at the coordination seams (cross-ref, not owned here): arc-16 closeout and b04 orchestrator binding SHOULD prompt for lessons at lane closeout — this pack ships the record/emitter/doctor core they call; obey `[workspace.lints]`; no `pub use` barrels.

## Acceptance And Proof
Tier P1/P4 (`lesson-capture-loop` in TEST_PROOF_EXPECTATIONS.md), proved by `cargo test -p enforcer-plan`:
- **Pass fixture:** add a lesson with `DoctrineBlock`+`Skill` routes → emitters produce golden artifacts (pinned under `tests/fixtures/lessons/golden/**`, keyed by lesson id) → doctor reports the lesson landed (green).
- **Fail fixture:** a captured lesson with zero landed artifacts (or an artifact missing its lesson id) → doctor returns `Severity::Error` naming the lesson id, non-zero exit through arc-22 — not a skip, not a warning.
- **Append-only:** rewriting a prior ledger row is detected (verify-on-open fails); supersede-append passes.
- **Import idempotence:** seed-corpus import twice → identical ledger; `--dry-run` on all emitters → zero writes (fs-diff empty).
Clean `cargo clippy` / `cargo fmt --check`.

## Parallel Ownership Notes
Owns only `crates/enforcer-plan/src/lessons.rs` + `templates/lesson-*.tpl` + `tests/fixtures/lessons/**` — disjoint from b01–b06 files inside `enforcer-plan` (b06 owns `src/agents_forest.rs` + `templates/agents-*.tpl`; this pack only EMITS a forest-node fragment b06's validator accepts, coordinated by fragment-schema not shared files). The refs/ seed ledger is orchestrator-maintained (appended live), read-only here except via `lesson import`. Consumers (c01 payload, c07 doctor, d01 scaffold input, arc-16/b04 capture hooks) integrate via their own packs. deps `arc-20` (crate), `b06` (fragment schema). owns disjoint? = Y.
