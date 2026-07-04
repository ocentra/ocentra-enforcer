# d15 Readme Research Grounding

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Readme Research Grounding`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `docs/research-grounding.md, README.md#research-grounding`
- deps: `none`
- tier: `P0 contract/schema`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md), [RUST_ARCHITECTURE](../RUST_ARCHITECTURE.md).

## Where We Are
The current `README.md` asserts the enforcer's value without cited evidence. ADBP references external research (context budgets, AST-over-prose enforcement, ratchets) that grounds the design choices behind the pure-Rust engine (a single self-contained Cargo-built binary that is both the MCP stdio server and the CLI). That grounding is not adopted here.

## Where We Want To Be
Adopt the cited evidence in the README as a "Research Grounding" section backed by `docs/research-grounding.md`, so design claims trace to sources. Doc-only workpack — no crate, no `Validator`, no gate.

## Requirement Checklist
- [ ] Create `docs/research-grounding.md` listing each borrowed idea (context budget, mechanical AST-over-prose enforcement, grandfather ratchet, deferred-work gate, rules-as-structured-data) with a citation.
- [ ] Add a `## Research Grounding` section to `README.md` linking to it.
- [ ] Each design claim in the new README section maps to a numbered source in the grounding doc.
- [ ] Clearly scope this as documentation: it enforces nothing, defines no crate, and ships no `Validator`.

## Acceptance And Proof
Tier: documentation, P0 contract/schema (content deliverable, no runtime tier). Proof is artifact existence and cross-link integrity: `docs/research-grounding.md` exists, the README section links to it, and every claim references a numbered source. No `cargo test` gate is claimed — this is explicitly a doc-only borrow, and that scoping is the honesty guardrail per doctrine (no prose masquerading as a check).

## Parallel Ownership Notes
`deps: none`. Owns `docs/research-grounding.md` and a README anchor section, disjoint from d09 (agent docs) and d14 (skills); fully concurrent. Does not gate any build and touches no crate.
