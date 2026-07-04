# a-conv-32 Rust Rules Source Late Parts

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Rust Rules Source Late Parts`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `scripts/rust-rules-source-late-boundaries.mjs, scripts/rust-rules-source-late-domain-debug.mjs, scripts/rust-rules-source-late-test-evidence.mjs, scripts/rust-rules-source-late-test-structure.mjs, scripts/rust-rules-source-late-unsafe.mjs, scripts/rust-rules-source-signature-rules.mjs, scripts/rust-rules-source-late-rules.mjs`
- deps: `a-conv-29, a-conv-31`
- tier: `P1 unit`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md).

## Where We Are
The rust-rules late-phase rules (boundaries, domain-debug, test-evidence, test-structure, unsafe, signature-rules, late-rules rollup) run the second pass over Rust source using the pattern leaves from a-conv-31.

## Where We Want To Be
All seven late-part modules are strict TS with typed late-rule descriptors composing into a typed late-rule set.

## Requirement Checklist
- [ ] Convert every owned file to strict TS with explicit exported types; no implicit `any`.
- [ ] Drop all wildcard imports (`import * as`); replace with named imports.
- [ ] Scoped `tsc --noEmit` over only the owned files passes under strict mode.

## Acceptance And Proof
Tier P1. Scoped typecheck (tsconfig include limited to the owned files) exits 0 under strict mode. `grep` for `import *` across owned files returns empty. Record the scoped-typecheck artifact path in TEST_PROOF_EXPECTATIONS.md.

## Parallel Ownership Notes
Blocks a-conv-33. Deps on a-conv-29, a-conv-31; owns the rust late-* and signature-rules files exclusively.
