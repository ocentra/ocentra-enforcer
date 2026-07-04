# a-conv-31 Rust Rules Source Patterns And Leaves

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Rust Rules Source Patterns And Leaves`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `scripts/rust-rules-source-patterns.mjs, scripts/rust-rules-source-helpers.mjs, scripts/rust-rules-source-names.mjs, scripts/rust-rules-source-signature-text.mjs, scripts/rust-rules-source-classification.mjs, scripts/rust-rules-source-signatures.mjs`
- deps: `a-conv-29`
- tier: `P1 unit`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md).

## Where We Are
The rust-rules source pattern tables, helpers, names, signature-text, classification, and signatures leaves implement the T2 literal-scan pattern set for Rust source, atop the path-core leaf.

## Where We Want To Be
All six rust source leaves are strict TS with typed pattern tables and signature descriptors preserving T2 scored output.

## Requirement Checklist
- [ ] Convert every owned file to strict TS with explicit exported types; no implicit `any`.
- [ ] Drop all wildcard imports (`import * as`); replace with named imports.
- [ ] Scoped `tsc --noEmit` over only the owned files passes under strict mode.
- [ ] Keep T2 scored output for pattern matches (typed score + confidence).

## Acceptance And Proof
Tier P1. Scoped typecheck (tsconfig include limited to the owned files) exits 0 under strict mode. `grep` for `import *` across owned files returns empty. Record the scoped-typecheck artifact path in TEST_PROOF_EXPECTATIONS.md.

## Parallel Ownership Notes
Blocks a-conv-28, a-conv-32, a-conv-33, a-conv-35. Deps on a-conv-29; owns the rust source pattern/leaf files, disjoint from the late-parts cluster.
