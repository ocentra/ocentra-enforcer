# a-conv-33 Rust Rules Source Scan And Engine

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Rust Rules Source Scan And Engine`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `scripts/rust-rules-source-scan.mjs, scripts/rust-rules-scan-engine.mjs, scripts/rust-rules-cargo-scan.mjs`
- deps: `a-conv-01, a-conv-29, a-conv-31, a-conv-32`
- tier: `P1 unit`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md).

## Where We Are
`rust-rules-source-scan.mjs`, the scan engine, and `rust-rules-cargo-scan.mjs` drive the full Rust source and Cargo scans over the pattern (a-conv-31) and late (a-conv-32) rule sets. Both scan files are oversized and must be split.

## Where We Want To Be
The source-scan, engine, and cargo-scan are strict TS with a typed scan result; the two oversized scan files become thin typed entries over cohesive modules.

## Requirement Checklist
- [ ] Convert every owned file to strict TS with explicit exported types; no implicit `any`.
- [ ] Drop all wildcard imports (`import * as`); replace with named imports.
- [ ] SPLIT `scripts/rust-rules-source-scan.mjs`: divide into cohesive TS modules by responsibility; no barrel wildcard re-exports.
- [ ] SPLIT `scripts/rust-rules-cargo-scan.mjs`: divide into cohesive TS modules by responsibility; no barrel wildcard re-exports.
- [ ] Scoped `tsc --noEmit` over only the owned files passes under strict mode.
- [ ] Type the scan-engine finding aggregation result explicitly.

## Acceptance And Proof
Tier P1. Scoped typecheck (tsconfig include limited to the owned files) exits 0 under strict mode. `grep` for `import *` across owned files returns empty. Each SPLIT target (`scripts/rust-rules-source-scan.mjs`, `scripts/rust-rules-cargo-scan.mjs`) is replaced by named modules whose combined exports match the original public surface, re-checked by dependent clusters. Record the scoped-typecheck artifact path in TEST_PROOF_EXPECTATIONS.md.

## Parallel Ownership Notes
Blocks a-conv-35. Deps span a-conv-01/29/31/32; owns the source-scan/engine/cargo-scan files, disjoint from the scan-core clusters.
