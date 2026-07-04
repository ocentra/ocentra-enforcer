# b06 Agents Decision Forest

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Agents Decision Forest`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `crates/enforcer-plan/src/agents_forest.rs, crates/enforcer-plan/templates/agents-global.tpl, crates/enforcer-plan/templates/agents-project.tpl, crates/enforcer-plan/templates/agents-plan.tpl, crates/enforcer-plan/tests/fixtures/agents_forest/**`
- deps: `arc-20-enforcer-plan, b02-plan-structure-validator, b03-capsule-index-templates`
- tier: `P1 P4 T1`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [RUST_ARCHITECTURE](../RUST_ARCHITECTURE.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md), [AUDIT_FINDINGS](../AUDIT_FINDINGS.md) (WAVE 5, AGENTS.md decision-forest).

## Where We Are
Owner requirement (2026-07-04): on any stop, crash, or resume, an agent burns tokens re-discovering where it is — re-reading whole plans, scanning scattered prose, guessing which state is current. There is no hierarchical, read-FIRST routing surface. The owner wants a `AGENTS.md` "decision forest": a fixed, layered chain of small routing files — GLOBAL `AGENTS.md` -> MAIN PROJECT `AGENTS.md` -> PER-PLAN `AGENTS.md` -> a decision tree -> the plan's resume-state — where each tier is a minimal, read-first router that says "check this, then that" so an agent locates current state with the fewest tokens. The owner prefers `AGENTS.md` over `CLAUDE.md` (harness-neutral naming). Today nothing scaffolds this chain, nothing declares the read-first routing contract, and nothing proves the chain resolves. b02 provides the structure-validation approach (PLAN-* `Validator` + `Finding`s) and b03 provides the capsule/index templating this reuses.

## Where We Want To Be
A scaffolder + validator module `crates/enforcer-plan/src/agents_forest.rs` that:
1. SCAFFOLDS the 3-tier `AGENTS.md` set from three templates — `templates/agents-global.tpl` (workspace/machine root), `templates/agents-project.tpl` (repo root), `templates/agents-plan.tpl` (per `docs/plans/<name>/`). Each rendered tier declares (a) its read-first routing header, (b) an explicit pointer to the NEXT tier down, and (c) a decision tree ("if resuming -> read X; if starting fresh -> read Y; if blocked -> read Z") whose leaves route to the plan's resume-state (the b05/WAVE-5 Where-We-Are + checklist + tasklist + progress + prev/next records).
2. VALIDATES the forest with a b02-style `Validator` (own `ruleId`s, e.g. `AGENTS-CHAIN-RESOLVES`, `AGENTS-ROUTING-DECLARED`, `AGENTS-TREE-TERMINATES`): asserts the global->project->plan chain resolves (each tier's NEXT pointer names an existing lower tier), each `AGENTS.md` declares its read-first routing block, and every decision-tree leaf terminates at a real resume-state anchor. Emits `Finding`s, never panics.
3. Proves a RESUME SIMULATION: given a plan dir, an agent that reads ONLY the `AGENTS.md` chain (never the full plan body) can locate the current plan state, and the token cost of that chain is bounded (each tier stays under a small line/byte budget).

TRANSITIONAL-TO-TYPED intent (state explicitly in the module doc + templates): these `AGENTS.md` files are a TRANSITIONAL prose surface. They are designed to be dropped for a typed system/db/schema later — the routing chain and decision tree are modeled as data (tier -> next-pointer -> decision-node -> resume-anchor) so the same structure can be served from a typed store and rendered in the Tauri desktop UI for humans. Do NOT hard-couple the validator to prose surviving forever: parse the structured markers (managed blocks / typed front-matter), not free text, so the backing store can swap under a stable contract.

## Requirement Checklist
- [ ] `agents_forest.rs` scaffolds the 3-tier `AGENTS.md` set (global/project/plan) from `templates/agents-{global,project,plan}.tpl` — reusing b03's capsule/index templating approach, not a hand-rolled writer.
- [ ] Each rendered tier declares a READ-FIRST routing block (a structured marker, e.g. a managed block or typed front-matter) stating "read me first, then route to <next tier>".
- [ ] Each tier carries an explicit pointer to the NEXT tier down (global->project->plan) and a decision tree whose leaves route to the plan's resume-state anchors.
- [ ] A `Validator` (b02-style, own `ruleId`s) asserts the chain resolves global->project->plan (every NEXT pointer targets an existing lower tier) and emits `Finding`s on breaks.
- [ ] The `Validator` asserts each `AGENTS.md` declares its read-first routing and that every decision-tree leaf terminates at a real resume-state anchor.
- [ ] A RESUME SIMULATION test asserts an agent can locate current plan state by reading ONLY the `AGENTS.md` chain, and that the chain is token-minimal (per-tier size budget enforced).
- [ ] Module doc + templates STATE the transitional-to-typed-data intent (prose surface replaceable by a typed system/db/schema; Tauri UI for humans; validator parses structured markers, not free prose).
- [ ] `tests/fixtures/agents_forest/**` provides FAIL fixtures (broken chain, missing routing block, dangling decision leaf, oversized tier) and PASS fixtures (resolving chain, declared routing, terminating tree, within budget).
- [ ] Obeys `[workspace.lints]` for the Rust it owns (no `unwrap`/`expect`/`panic`/`print_*`); no `pub use` barrels.

## Acceptance And Proof
Tier P1/P4 (mechanical detection + self-enforce green), T1, Rust-native (`cargo test`). Prove via `cargo test -p enforcer-plan`:
- A scaffold test renders all three tiers from the templates and asserts each output carries the read-first routing marker + NEXT pointer + decision tree (structured markers, not substring prose).
- A chain-resolution test drives the `Validator` over PASS fixtures (`tests/fixtures/agents_forest/pass/**`) and asserts zero `Finding`s; over FAIL fixtures (`tests/fixtures/agents_forest/fail/**` — broken NEXT pointer, missing routing block, dangling decision leaf, oversized tier) and asserts the specific `ruleId` `Finding` fires for each.
- A resume-simulation test walks ONLY the `AGENTS.md` chain (global->project->plan->decision tree) from a fixture and asserts it resolves to the plan's current resume-state anchor without reading the plan body, and that the summed chain size is within the declared token/line budget.
- A doc-intent check asserts the module doc + each template declare the transitional-to-typed-data statement.
Name these detection tests in TEST_PROOF_EXPECTATIONS.md.

## Parallel Ownership Notes
Track B pack. Deps: `arc-20-enforcer-plan` (the crate this module lives in and whose `Validator`/`Finding` types it reuses), `b02-plan-structure-validator` (reuses its structure-validation approach + `ruleId`/`Finding` pattern — read-only, does not modify b02's modules), and `b03-capsule-index-templates` (reuses its templating approach for rendering the three tiers). Owns a disjoint file set: the single module `src/agents_forest.rs`, three new `templates/agents-*.tpl` files, and its own `tests/fixtures/agents_forest/**` subtree — disjoint by file from b01/b02/b03/b04/b05's modules, templates, and fixtures. It consumes sibling entrypoints read-only and blocks nothing.
