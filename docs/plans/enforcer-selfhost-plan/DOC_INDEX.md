# DOC_INDEX

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Doc Index`
> Kind: catalog. Lists every document in this plan directory, what it is for, who reads it, and its capsule contract. Use it to answer "does a doc for X exist, and what does it claim to prove?"
> Read when: You need the map of all plan docs (not the routing decision — that's ROUTE_INDEX), OR you are adding a new doc and need to know the contract every plan doc follows.
> Stop rule: This is a catalog. It does not contain the docs' contents. It proves nothing about status.
> Proves: nothing. Inventory + contract only.
> Does not prove: workpack status, proof status, or completion.
<!-- /agent-capsule -->

Sources: [ROUTE_INDEX](./ROUTE_INDEX.md), [WORKPACK_INDEX](./WORKPACK_INDEX.md).

---

## The plan-doc contract (every doc in this plan follows it)

1. Opens with an `<!-- agent-capsule -->` ... `<!-- /agent-capsule -->` block (Plan / Doc / Kind / Read when / Stop rule / Proves / Does not prove [/ Proof rule for proof-bearing docs]).
2. Index/routing docs are **token-efficient routes**: a "Default agent path" and an explicit "Do not default-read" list where relevant.
3. Proof-bearing statements cite a mechanical backing (a validator, test, or artifact) — never bare prose. Doctrine: prose without a backing check is hope, not proof.
4. Cross-links use real relative filenames that resolve.

---

## Root-level docs (this directory)

| Doc | Kind | Purpose | Primary reader | Read-order role |
|-----|------|---------|----------------|-----------------|
| [ROUTE_INDEX.md](./ROUTE_INDEX.md) | Routing hub | The token-efficient "where do I go for X" entry point; Default agent path + Do-not-default-read. | Anyone arriving at the plan | **Entry point** |
| [PLAN_STATE.md](./PLAN_STATE.md) | State | Scope / Resume-route / What-is-present / Open-gaps / Workpack-summary. The orientation doc. | Anyone orienting or resuming | 1st content read |
| [PLAN_EXECUTION_BLUEPRINT.md](./PLAN_EXECUTION_BLUEPRINT.md) | Execution model | Groups the 120 workpacks into tracks A/B/C/D/E/F/G + cross-cutting; recommended sequence (x01 rename -> a01 -> conv swarm -> A domains -> d01 -> rest of D+C+E+F+G parallel [G: g01 serve surface first] -> B -> z01 dogfood gate last); the frontier/hub-lane/claim-guard-closeout parallel model + intent-queue for overlap. | Hub / orchestrator | 2nd content read |
| [WORKPACK_INDEX.md](./WORKPACK_INDEX.md) | Status table | Every workpack: Status \| Workpack(link) \| Track \| owns \| owns-disjoint? \| tier \| parallel-safe-with. Grouped A/C/D/E/B/F/G + cross-cutting. | Anyone selecting/tracking work | 3rd content read |
| [TEST_PROOF_EXPECTATIONS.md](./TEST_PROOF_EXPECTATIONS.md) | Proof contract | Defines proof tiers P0-P5, the workpack-type -> proof decision tree, and the per-workpack proof rows. The authority for what DONE requires. | Closing agent / reviewer | Before any DONE |
| [PROOF_INDEX.md](./PROOF_INDEX.md) | Proof routing | Maps tiers -> workpacks, points at the proof rows, and states the GREEN-before-DONE rule. | Reviewer / auditor | On review |
| [CHECKLIST_INDEX.md](./CHECKLIST_INDEX.md) | Checklists | Claim checklist, execution checklist, closeout checklist, plan-author checklist — the repeatable gates. | Every worker at claim/close | At claim + close |
| [DOC_INDEX.md](./DOC_INDEX.md) | Catalog (this file) | Inventory of all plan docs + the plan-doc contract. | Plan author / navigator | As needed |
| [ARCHIVE_INDEX.md](./ARCHIVE_INDEX.md) | Archive | Where superseded/historical docs live and the rule for archiving. | Anyone chasing history | Rarely |

---

## Workpack docs (`workpacks/` directory)

120 assigned-workpack files. Each is a **scoped work item**, not an index — open only the one you have claimed (see the stop rule in every workpack capsule). Grouped:

| Group | Files | Count | What they are |
|-------|-------|-------|---------------|
| A.0 toolchain | `a01-ts-toolchain-and-build.md` | 1 | The compiler-contract gate; blocks all of Track A. |
| A.1 conversion swarm | `a-conv-01-*.md` .. `a-conv-50-*.md` | 50 | Uniform P1 `.mjs -> TS` conversions + module splits; per-cluster owns. |
| A.2 domain packs | `a02-*.md` .. `a10-*.md` | 9 | Brands (RuleId/Path/Sha256/Coord-ids), parse-at-boundary, waiver honesty, anti-silent-skip, real self-enforcement+CI. |
| C install/enforce | `c01-*.md` .. `c09-*.md` | 9 | Harness-neutral install core + CLI, autodetect, Claude/Codex/generic adapters, deny + sessionstart hooks, stub adapters, and `c09` the remaining six adapters (Antigravity/Windsurf/OpenCode/Aider/KiloCode/Kiro) — all 11 harnesses covered. |
| D ADBP borrows + mechanized families | `d01-*.md` .. `d15-*.md`, `d16`/`d17`/`d18`/`d21`/`d22`/`d23`/`d25`/`d26`/`d27`/`d28` | 25 | Rule-mechanization engine (keystone) + ratchet, deferred-gate, telemetry, context-budget, lifecycle, fix-loop, feedback, doc-rule parity, resilience auditor, CI parity, layered eslint rules, version drift, ideation T3, README grounding; then FSM validity, Rust error-handling, security STOP watchlist, change discipline, size/shape caps, test companion/quality, orchestrator verify gates, dispatch prompt assembly, loop resilience/telemetry, target-repo CI parity. |
| E new languages + universal scanning | `e01-*.md`, `e-pack-dart.md`, `e-pack-cfml.md`, `e-pack-frontend-react.md`, `e-pack-python.md` | 5 | Always-on universal literal-scan T2 floor + first-class Dart, CFML (CFLint shell-out), React/Next (Effect-only), and Python FastAPI (layered/clean-arch + Python security) language packs. |
| F scan surface, onboarding & agent-shaping | `f01-*.md` .. `f05-*.md` | 5 | Agent-selectable scan MODES (`enforcer_scan`, scoped-not-whole-repo), index-on-ask `.enforce/` onboarding, per-project native-tie config schema, the AGENT-INLINE (silent) vs HUMAN-REVIEW run-context split that Track G's UI honors, and `f05` the foundational detect-and-route router (emits the ROUTE PLAN that check/scan/run + c04 consume). |
| G UI layer (vendored hub dashboard/server) | `g01-*.md` .. `g07-*.md` | 7 | Human-invoked local UI built on the vendored `src/coordination/vendor/{server,dashboard}.js`: `g01` serve surface (lands first), then scan-report, per-violation actions, Run-dispatch into the coordination ledger, settings, and the hub coordination dashboard all mount into it, plus `g07` the UI-security layer (loopback/CSRF/dispatch-authorization guards) reused by every g0x endpoint. |
| B planning skill | `b01-*.md` .. `b05-*.md` | 5 | Scaffolder, structure validator, capsule/index templates, parallel-orchestrator binding, plan skill + self-validate. |
| Cross-cutting | `x01-neutral-rename.md`, `x02-docs-refresh.md`, `x03-rename-migration.md`, `z01-dogfood-proof-gate.md` | 4 | `x01` renames the product to `enforcer` (early); `x02` refreshes product docs to `enforcer` + adds a section per new capability (after x01); `x03` is a transitional migrate that rewrites already-installed `ocentra-enforcer` regs/tool-names to `enforcer` (after x01); `z01` is the terminal dogfood-proof-gate (deps ALL tracks) that runs the enforcer on its own self and gates plan-DONE. |

For the exact owns/deps/tier/status of any workpack, see [WORKPACK_INDEX.md](./WORKPACK_INDEX.md). Do not read the swarm en masse (see ROUTE_INDEX "Do not default-read").

---

## ADBP gap-analysis source docs (this directory)

The ADBP (Agent-Driven Best Practices) comparison corpus. These are the raw "what does ADBP enforce that we lack, and how should we borrow it" inputs the Track D and Track E workpacks were derived from. They are analysis/reference docs, not proof-bearing plan surfaces — cite them for provenance, do not treat them as status.

| Doc | Kind | Purpose | Primary reader |
|-----|------|---------|----------------|
| [ADBP_GAPS.md](./ADBP_GAPS.md) | Gap delta | The 99-gap "what we lack" delta between ADBP and the current enforcer registry, grouped by area (rules, linters, commands/ergonomics gates). Workpacks reference its row ranges (e.g. FSM rows 41-50, Rust error rows 51-67, security rows 68-81) as the source of each borrowed rule. | Track D / Track E pack authors |
| [ADBP_PARITY_MATRIX.md](./ADBP_PARITY_MATRIX.md) | Borrow classification | The tiered classification of every ADBP idea into borrow / already-covered / diverge, mapped onto the T1/T2/T3 mechanization ladder. The authority for how each gap should be dragged up the ladder (or explicitly not borrowed, e.g. the Zod->Effect divergence). | Anyone deciding how to mechanize a borrow |
| [parity-gaps/](./parity-gaps/) | Per-area raw gap tables | Directory of per-area raw gap tables backing the delta: `rules-python.md`, `rules-rust-shared.md`, `rules-flutter.md`, `rules-frontend.md`, `rules-coldfusion.md`, `linters-python.md`, `linters-frontend-rust-cfml.md`, `commands.md`, `ergonomics.md`, `agents-skills.md`. Each is the granular evidence for one slice of ADBP_GAPS. | Pack authors needing the raw per-rule detail |

---

## Docs this plan references but does not own

- **`C:/Projects/ocentra-enforcer/README.md`** — gains a `## Research Grounding` section via workpack **d15** (that anchor is d15's owns; the rest of README is out of scope).
- **`C:/Projects/ocentra-enforcer/AGENTS.md`** and per-language rule docs under `rules/` — referenced by **d09** (doc-rule parity) as the source of must/never bullets that must cite ruleIds.
- **`docs/agents/**`, `docs/research-grounding.md`** — new product docs authored by workpacks d09 / d15 respectively (they live in the repo, not in this plan dir).

---

## Adding a new doc to this plan

1. Give it an `<!-- agent-capsule -->` block matching the contract above.
2. If it is an index/routing doc, include a Default agent path and a Do-not-default-read list.
3. Register it in this DOC_INDEX table and add a route to it in [ROUTE_INDEX.md](./ROUTE_INDEX.md).
4. If it makes any provable claim, name the validator/test that backs it (or label it T3 with a reason). No prose-only proof.
