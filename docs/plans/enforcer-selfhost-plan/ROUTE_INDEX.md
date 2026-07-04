# ROUTE_INDEX

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Route Index`
> Kind: routing hub. This is the token-efficient map of *which doc to open for which question*. Read this FIRST when you arrive at this plan and are not already routed to a specific workpack.
> Read when: You just entered the plan and need to know where to go, OR you are unsure which index answers your question.
> Stop rule: This doc routes; it does not contain the answers. Follow exactly one route, then stop reading indexes. Do NOT open workpack files or the "Do not default-read" set unless a route sends you there.
> Proves: nothing. It is navigation only.
> Does not prove: any status, proof, or completion.
<!-- /agent-capsule -->

Sources: this is the entry hub. Everything else is reachable from here.

---

## Default agent path (follow in order; stop as soon as your question is answered)

1. **Orienting / "what is this plan?"** -> read [PLAN_STATE.md](./PLAN_STATE.md) (scope, resume-route, what's present, open gaps). Stop.
2. **"What do I work on next / how do tracks sequence / how does parallel execution work?"** -> read [PLAN_EXECUTION_BLUEPRINT.md](./PLAN_EXECUTION_BLUEPRINT.md) (tracks A/B/C/D/E/F/G/H + cross-cutting, recommended sequence, frontier/lane/claim model). Stop.
3. **"Which workpack, what's its status, is it parallel-safe?"** -> read [WORKPACK_INDEX.md](./WORKPACK_INDEX.md) (full status table by track). Pick ONE workpack row. Stop.
4. **You have a workpack selected** -> open exactly that one workpack file under [workpacks/](./workpacks/). Read its capsule + body. Do NOT open sibling workpacks.
5. **You are about to close a workpack (DONE)** -> read [TEST_PROOF_EXPECTATIONS.md](./TEST_PROOF_EXPECTATIONS.md), select your proof tier via the decision tree, make the named proof row GREEN. Stop.

That is the whole loop: **PLAN_STATE -> BLUEPRINT -> WORKPACK_INDEX -> one workpack -> TEST_PROOF_EXPECTATIONS -> close.**

---

## Route by question (jump table)

| Your question | Go to |
|---------------|-------|
| What is the plan's scope and where do I resume? | [PLAN_STATE.md](./PLAN_STATE.md) |
| What order do tracks/workpacks run in? How is parallelism organized? | [PLAN_EXECUTION_BLUEPRINT.md](./PLAN_EXECUTION_BLUEPRINT.md) |
| Which workpack exists, what's its status/owns/tier, what's it parallel-safe with? | [WORKPACK_INDEX.md](./WORKPACK_INDEX.md) |
| What proof does a workpack need before DONE? What do P0-P5 mean? | [TEST_PROOF_EXPECTATIONS.md](./TEST_PROOF_EXPECTATIONS.md) |
| Where's the running proof-row status / how do proof tiers map to workpacks? | [PROOF_INDEX.md](./PROOF_INDEX.md) |
| Which docs exist in this plan and what is each for? | [DOC_INDEX.md](./DOC_INDEX.md) |
| What must I check off before claiming / before closing a workpack? | [CHECKLIST_INDEX.md](./CHECKLIST_INDEX.md) |
| Where do superseded / historical docs go? | [ARCHIVE_INDEX.md](./ARCHIVE_INDEX.md) |
| The actual work item detail (Where We Are / Want To Be / Requirement Checklist) | the specific file in [workpacks/](./workpacks/) — only once selected |

---

## Route by role

- **Hub / orchestrator**: BLUEPRINT (frontier + lane model) -> WORKPACK_INDEX (disjoint-owns check) -> assign lanes. Do not read workpack bodies.
- **Lane worker (executing one workpack)**: your one workpack file -> TEST_PROOF_EXPECTATIONS (your proof row) -> CHECKLIST_INDEX (closeout). Do not read sibling workpacks.
- **Reviewer / auditor**: PROOF_INDEX -> the workpack's proof row -> the named test. Confirm GREEN + seeded-violation-fails before allowing a DONE move.
- **Plan author (adding a workpack)**: DOC_INDEX (doc contract) -> CHECKLIST_INDEX (authoring checklist) -> b02/b03 templates once they exist.

---

## Do NOT default-read (open only when a route above sends you there)

These are large or scoped; opening them speculatively burns context and violates the stop rules:

- **Any file under [workpacks/](./workpacks/)** — 131 workpack files. Open the ONE selected via WORKPACK_INDEX. Never batch-read siblings.
- **The full conversion swarm** (`workpacks/a-conv-01..50`) — 50 near-identical P1 conversion packs. Read only the one you claim.
- **[TEST_PROOF_EXPECTATIONS.md](./TEST_PROOF_EXPECTATIONS.md) proof-row tables (section 4)** — long. Read only your workpack's row + the P0-P5 definitions + the decision tree; skip the other tracks' rows.
- **[ARCHIVE_INDEX.md](./ARCHIVE_INDEX.md) contents** — historical only; never needed to execute current work.
- **Source under `C:/Projects/ocentra-enforcer/src`, `scripts`, `mcp`, `tests`** — you are authoring/executing a PLAN; touch source only when a workpack you have claimed tells you to, and only within its `owns:` globs.

---

## One-line invariants (so you never need to re-derive them)

- A workpack is DONE only when its TEST_PROOF_EXPECTATIONS row is GREEN (named test passes AND, for T1/P4/P5, the seeded-violation case fails).
- Two workpacks with no dep edge MUST have disjoint `owns:` globs; check WORKPACK_INDEX before any concurrent claim.
- Doctrine: enforcement is mechanical. If a doc claims a rule, there is a validator behind it, or it is labeled T3 "advisory, no mechanization possible: <reason>".
