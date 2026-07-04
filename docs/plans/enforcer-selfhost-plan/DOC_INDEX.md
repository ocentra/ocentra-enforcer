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
| [RUST_ARCHITECTURE.md](./RUST_ARCHITECTURE.md) | Governing architecture | The enforcer is a **RUST Cargo workspace** (one binary = MCP + CLI; rules structured-data; native Rust dogfood; TS only for Tauri UI; Node dropped). Crate map (**28 crates** = 25 arc + 3 E-built lang crates), distribution (codebase-memory model), and the track re-cast. **Supersedes the `.mjs` -> TypeScript decision.** | Anyone orienting or building Track A | Read after PLAN_STATE |
| [EXECUTION_MODEL.md](./EXECUTION_MODEL.md) | Execution model | HOW the finalized plan is executed: bootstrap-safe rebuild (separate worktree+branch, keep `.mjs` MCP live until Rust green then swap), vendoring (arc-25 `enforcer-events` from OcentraParent, logging-core primitives), and the Fable-5 orchestrator + Sonnet/Haiku/Opus worker swarm coordinated via the hub. Companion to RUST_ARCHITECTURE (WHAT) + WORKPACK_INDEX. | Hub / orchestrator | Before an orchestration run |
| [PLAN_STATE.md](./PLAN_STATE.md) | State | Scope / Resume-route / What-is-present / Open-gaps / Workpack-summary. The orientation doc. | Anyone orienting or resuming | 1st content read |
| [PLAN_EXECUTION_BLUEPRINT.md](./PLAN_EXECUTION_BLUEPRINT.md) | Execution model | Groups the 109 workpacks into tracks A/B/C/D/E/F/G/H + cross-cutting; recommended sequence (x01 rename -> a01 Rust toolchain -> arc crate swarm -> A domains -> d01 -> rest of D+C+E+F+G+H parallel [G: g01 serve surface first; H: h01 classifier first] -> B -> z01 dogfood gate last); the frontier/hub-lane/claim-guard-closeout parallel model + intent-queue for overlap. All tracks are Rust-framed ([RUST_ARCHITECTURE.md](./RUST_ARCHITECTURE.md) for WHAT, [refs/RUST_REFRAME_SPEC.md](./refs/RUST_REFRAME_SPEC.md) for the TS->Rust transformation contract applied to B/C/D/E/F/G/H/x). | Hub / orchestrator | 2nd content read |
| [WORKPACK_INDEX.md](./WORKPACK_INDEX.md) | Status table | Every workpack: Status \| Workpack(link) \| Track \| owns \| owns-disjoint? \| tier \| parallel-safe-with. Grouped A/C/D/E/B/F/G/H + cross-cutting. | Anyone selecting/tracking work | 3rd content read |
| [TEST_PROOF_EXPECTATIONS.md](./TEST_PROOF_EXPECTATIONS.md) | Proof contract | Defines proof tiers P0-P5, the workpack-type -> proof decision tree, and the per-workpack proof rows. The authority for what DONE requires. | Closing agent / reviewer | Before any DONE |
| [PROOF_INDEX.md](./PROOF_INDEX.md) | Proof routing | Maps tiers -> workpacks, points at the proof rows, and states the GREEN-before-DONE rule. | Reviewer / auditor | On review |
| [CHECKLIST_INDEX.md](./CHECKLIST_INDEX.md) | Checklists | Claim checklist, execution checklist, closeout checklist, plan-author checklist — the repeatable gates. | Every worker at claim/close | At claim + close |
| [DOC_INDEX.md](./DOC_INDEX.md) | Catalog (this file) | Inventory of all plan docs + the plan-doc contract. | Plan author / navigator | As needed |
| [ARCHIVE_INDEX.md](./ARCHIVE_INDEX.md) | Archive | Where superseded/historical docs live and the rule for archiving. | Anyone chasing history | Rarely |

---

## Workpack docs (`workpacks/` directory)

109 assigned-workpack files. Each is a **scoped work item**, not an index — open only the one you have claimed (see the stop rule in every workpack capsule). Grouped:

| Group | Files | Count | What they are |
|-------|-------|-------|---------------|
| A.0 Rust toolchain | `a01-cargo-workspace-and-toolchain.md` | 1 | The Rust toolchain gate (Cargo + clippy/rustfmt/deny/audit + `rust-toolchain.toml`); blocks all of Track A. |
| A crate-build swarm (Rust) | `arc-01-enforcer-core.md` .. `arc-25-enforcer-events.md` | 25 | Dependency-ordered Cargo crates standing up the workspace (core, domain, config, rules, validator, per-language validators enforcer-lang-{rust,ts,py,common,security,iac,k8s}, literal-scan, mechanization, scan, coordination, proof, harness, security, plan, mcp, cli, install, ui, events), plus `arc-25` `enforcer-events` — the lean in-process typed event spine VENDORED from OcentraParent's ocentra-eventing (renamed) and consumed by arc-15/arc-16/arc-17. Replaces the removed 50-pack `.mjs -> TS` conversion swarm; see [RUST_ARCHITECTURE.md](./RUST_ARCHITECTURE.md). |
| A domain packs (Rust) | `a02-*.md` .. `a10-*.md` | 9 | `enforcer-domain` newtype brands (RuleId/Path/Sha256/Coord-ids), serde parse-at-boundary, waiver honesty, anti-silent-skip, real self-enforcement+CI (enforcer's own Rust rules on its own crates). With `a01` (A.0 toolchain) this is the 10-pack `a01..a10` Rust-hardening set; the A track totals **35** (25 arc crates + 10 a0x). |
| C install/enforce | `c01-*.md` .. `c11-*.md` | 11 | Harness-neutral install core + CLI, autodetect, Claude/Codex/generic adapters, deny + sessionstart hooks, stub adapters, and `c09` the remaining six adapters (Antigravity/Windsurf/OpenCode/Aider/KiloCode/Kiro) — all 11 harnesses covered — plus `c10` CI integration and `c11` onboarding skill. |
| D ADBP borrows + mechanized families | `d01-*.md` .. `d15-*.md`, `d16`/`d17`/`d18`/`d21`/`d22`/`d23`/`d25`/`d26`/`d27`/`d28` | 25 | Rule-mechanization engine (keystone) + ratchet, deferred-gate, telemetry, context-budget, lifecycle, fix-loop, feedback, doc-rule parity, resilience auditor, CI parity, layered eslint rules, version drift, ideation T3, README grounding; then FSM validity, Rust error-handling, security STOP watchlist, change discipline, size/shape caps, test companion/quality, orchestrator verify gates, dispatch prompt assembly, loop resilience/telemetry, target-repo CI parity. |
| E new languages + universal scanning | `e01-*.md`, `e-pack-dart.md`, `e-pack-cfml.md`, `e-pack-frontend-react.md`, `e-pack-python.md`, `e-pack-crypto-blockchain.md` | 6 | Always-on universal literal-scan T2 floor + first-class Dart, CFML (CFLint shell-out), React/Next (Effect-only), and Python FastAPI (layered/clean-arch + Python security) language packs, plus the OPTIONAL opt-in `e-pack-crypto-blockchain` (Solana/Anchor on-chain, OFF by default, deps `d01`/`d17`/`d18`/`h01`; consumes h06 signing + h07 localnet adapter). **Three of these packs BUILD NEW lang crates** (no separate arc pack): `e-pack-dart` -> `crates/enforcer-lang-dart`, `e-pack-cfml` -> `crates/enforcer-lang-cfml`, and the OPT-IN `e-pack-crypto-blockchain` -> `crates/enforcer-lang-crypto` (OFF by default) — these are the 3 E-built lang crates that bring the crate map to **28** (25 arc + 3). |
| F scan surface, onboarding & agent-shaping | `f01-*.md` .. `f05-*.md` | 5 | Agent-selectable scan MODES (`enforcer_scan`, scoped-not-whole-repo), index-on-ask `.enforce/` onboarding, per-project native-tie config schema, the AGENT-INLINE (silent) vs HUMAN-REVIEW run-context split that Track G's UI honors, and `f05` the foundational detect-and-route router (emits the ROUTE PLAN that check/scan/run + c04 consume). |
| G UI layer (vendored hub dashboard/server) | `g01-*.md` .. `g08-*.md` | 8 | Human-invoked local UI built on the vendored `src/coordination/vendor/{server,dashboard}.js`: `g01` serve surface (lands first), then scan-report, per-violation actions, Run-dispatch into the coordination ledger, settings, and the hub coordination dashboard all mount into it, plus `g07` the UI-security layer (loopback/CSRF/dispatch-authorization guards) reused by every g0x endpoint, and `g08` the rules-&-skills explorer (renders every rule/skill as browsable UI — meaning, fail/pass, tier, framework map; where `.md` lives for humans while the AI reads the structured rule). |
| H money-critical & security-testing mandate | `h01-*.md` .. `h08-*.md`, `h11-*.md`, `h12-*.md` | 10 | Generic mechanization of the ingested `refs/security-testing-source.md` spec into T1/T2 rules (GENERIC across any value system, never crypto/game-specific): `h01` money-critical classifier (keystone, emits the manifest h02/h03/h05/h06 consume), `h02` required-test-categories gate, `h03` threat↔invariant↔test mapping, `h04` security-test-quality banned patterns, `h05` economic-invariant property suite, `h06` money-critical mechanics (signing/time/boundary/kill-switch + economic/rollback), `h07` security-tooling CI + observability, and `h08` the testing-mandate SKILL + neutral profile `profiles/money-critical-security.json` + policy-spec ingestion; plus the cyber-skills mechanization pair `h11` (vendored `anthropic-cybersecurity-skills` corpus -> native Rust rules + h03 vocab seed + f05 security-audit scope + `vendor/**` dogfood exclusion) and `h12` (the OPTIONAL out-of-dogfood python/CLI adapter complement for the irreplaceable engines). |
| B planning skill | `b01-*.md` .. `b05-*.md` | 5 | Scaffolder, structure validator, capsule/index templates, parallel-orchestrator binding, plan skill + self-validate. |
| Cross-cutting | `x01-neutral-rename.md`, `x02-docs-refresh.md`, `x03-rename-migration.md`, `z01-dogfood-proof-gate.md` | 4 | `x01` renames the product to `enforcer` (early); `x02` refreshes product docs to `enforcer` + adds a section per new capability (after x01); `x03` is a transitional migrate that rewrites already-installed `ocentra-enforcer` regs/tool-names to `enforcer` (after x01); `z01` is the terminal dogfood-proof-gate (deps ALL tracks) that runs the enforcer on its own self and gates plan-DONE. |

For the exact owns/deps/tier/status of any workpack, see [WORKPACK_INDEX.md](./WORKPACK_INDEX.md). Do not read the swarm en masse (see ROUTE_INDEX "Do not default-read").

---

## Ingested reference / source docs (`refs/` + this directory)

Raw inputs the plan's workpacks were derived from. They are analysis/reference docs, not proof-bearing plan surfaces — cite them for provenance, do not treat them as status.

The ADBP (Agent-Driven Best Practices) comparison corpus feeds Track D and Track E; the ingested security-testing spec feeds Track H (and the optional crypto pack); the vendored `anthropic-cybersecurity-skills` corpus + its Rust-conversion analysis feed the `h11`/`h12` cyber-skills mechanization pair.

| Doc | Kind | Purpose | Primary reader |
|-----|------|---------|----------------|
| [refs/RUST_REFRAME_SPEC.md](./refs/RUST_REFRAME_SPEC.md) | Transformation contract | The authoritative TS->Rust transformation contract used to re-frame every non-Track-A workpack (C/D/E/F/G/H/B/x): universal TS->Rust rules (validators->`Validator` trait, schemas->branded newtypes, tests->`cargo test` fixtures), the disjoint-owns model (arc crate = skeleton, feature packs = specific files), and the per-track pack->crate mapping table. | Anyone re-framing or auditing a workpack's crate assignment |
| [refs/security-testing-source.md](./refs/security-testing-source.md) | Ingested reference spec | The money-critical & security-testing mandate spec (GENERIC — any system handling money/payments/value behind untrusted infra; NOT game- or crypto-specific; crypto/blockchain is ONE OPTIONAL instance). Provenance kept neutral (no product branding). This is the "HOW to test security" spec that today is PROSE; **Track H (`h01`–`h08`) mechanizes it into T1/T2 rules** per the tested-enforcement doctrine, and the OPTIONAL `e-pack-crypto-blockchain` mechanizes its §2.5 on-chain instance. | Track H pack authors + `e-pack-crypto-blockchain` |
| [vendor/anthropic-cybersecurity-skills/RUST_CONVERSION_ANALYSIS.md](../../../vendor/anthropic-cybersecurity-skills/RUST_CONVERSION_ANALYSIS.md) | Vendored corpus analysis | The disposition of the vendored `anthropic-cybersecurity-skills` corpus: Rust-convertible (~55-65% of skill cores) vs python-bound (~15-20%), the T1/T2/T3/adapter breakdown, the harvest targets, and the mapping plan. The authority for WHICH cyberskills `h11` reimplements as native Rust and which `h12` wraps as optional out-of-dogfood adapters. | `h11`/`h12` pack authors |
| [vendor/anthropic-cybersecurity-skills/README.md](../../../vendor/anthropic-cybersecurity-skills/README.md) | Vendored corpus README | Provenance + orientation for the vendored corpus (817 skills, Apache-2.0). Referenced by `h11` for corpus structure (SKILL.md frontmatter, MITRE/NIST mappings) and dogfood-exclusion scope; the corpus tree itself stays out of the enforcer's own dogfood via the `vendor/**` ignore-glob h11 adds. | `h11`/`h12` pack authors |
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
