# a-conv-15 Proof CLI Storage Legacy

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Proof CLI Storage Legacy`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `src/proof-cli-format.mjs, src/proof-cli-parse.mjs, src/proof-cli.mjs, src/proof-storage.mjs, src/proof-legacy.mjs`
- deps: `a-conv-01`
- tier: `P1 unit`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md).

## Where We Are
The proof CLI (format, parse, entry), proof storage, and `proof-legacy.mjs` handle proof formatting, parsing, persistence, and legacy import. `src/proof-legacy.mjs` mixes several legacy-format adapters and must be split before typing.

## Where We Want To Be
The proof CLI/storage/legacy files are strict TS with typed proof records and a typed storage interface.

## Requirement Checklist
- [ ] Convert every owned file to strict TS with explicit exported types; no implicit `any`.
- [ ] Drop all wildcard imports (`import * as`); replace with named imports.
- [ ] SPLIT `src/proof-legacy.mjs`: divide into cohesive TS modules by responsibility; no barrel wildcard re-exports.
- [ ] Scoped `tsc --noEmit` over only the owned files passes under strict mode.
- [ ] Type the proof record and storage read/write surface explicitly.

## Acceptance And Proof
Tier P1. Scoped typecheck (tsconfig include limited to the owned files) exits 0 under strict mode. `grep` for `import *` across owned files returns empty. Each SPLIT target (`src/proof-legacy.mjs`) is replaced by named modules whose combined exports match the original public surface, re-checked by dependent clusters. Record the scoped-typecheck artifact path in TEST_PROOF_EXPECTATIONS.md.

## Parallel Ownership Notes
Blocks a-conv-16. Deps only on a-conv-01; owns the proof-cli/storage/legacy files, disjoint from the proof entry cluster.
