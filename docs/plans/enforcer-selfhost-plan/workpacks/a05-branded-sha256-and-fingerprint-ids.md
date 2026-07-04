# a05 Branded Sha256 And Fingerprint Ids

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Branded Sha256 And Fingerprint Ids`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `mcp/rust-rules-mcp-fingerprint.*` (type/brand surface only)
- deps: `a01`
- tier: `P0`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md).

## Where We Are
`mcp/rust-rules-mcp-fingerprint.mjs` computes `createHash("sha256").update(...).digest("hex")` and stores results in fields typed as raw `string` (`sha256`, `digest`). Startup and current digests are compared as plain strings; a truncated, uppercased, or non-hex value would compare and store without complaint.

## Where We Want To Be
A `Sha256` branded type minted only by the hashing helper (and its decoder), so every fingerprint `sha256`/`digest` field and comparison is typed `Sha256`, and any raw string entering the digest set must pass a `^[0-9a-f]{64}$` decode.

## Requirement Checklist
- [ ] Define `Sha256` brand + decoder (`^[0-9a-f]{64}$`, lowercase) alongside the fingerprint module.
- [ ] Hash helper returns `Sha256`; `fingerprintFile`, `buildMcpFingerprint`, `fingerprintChange` typed on it.
- [ ] `digest` field and comparisons use `Sha256` equality, not bare `string`.
- [ ] Decode fails-closed on wrong length / uppercase / non-hex.

## Acceptance And Proof
Tier P0. Unit tests: valid 64-hex mints, rejection of length/case/charset violations, digest round-trip. A `tsc --noEmit` negative fixture proves a bare `string` cannot populate a `Sha256` field. Rows in TEST_PROOF_EXPECTATIONS.md.

## Parallel Ownership Notes
Depends on a01. Shares the fingerprint module file glob with a02 by design: a05 owns the `Sha256` type/brand surface, a02 owns the `dist/`-tracking runtime behavior. Sequence a05 before a02 (a02 consumes `Sha256`), or coordinate on the single file. Disjoint from a03/a04/a06 domains.
