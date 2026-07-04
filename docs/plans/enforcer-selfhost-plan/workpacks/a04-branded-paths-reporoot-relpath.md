# a04 Branded Paths RepoRoot RelPath

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Branded Paths RepoRoot RelPath`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `src/path-utils.*`
- deps: `a01`
- tier: `P0`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md).

## Where We Are
`src/path-utils.mjs` returns and accepts raw `string` paths with no type-level distinction between an absolute repo root and a repo-relative path. Callers routinely `path.join` a relative onto another relative, or leak an absolute path into a report field expected to be relative, with no compiler signal.

## Where We Want To Be
`RepoRoot` (validated absolute base) and `RelPath` (validated repo-relative) branded types minted only inside `src/path-utils.*`, so mixing them is a compile error and cross-platform normalization happens once at the boundary.

## Requirement Checklist
- [ ] Define `RepoRoot` and `RelPath` brands with decoders in `src/path-utils.*`.
- [ ] Only `path-utils` mints the brands; consumers import the types, never construct.
- [ ] `toRelPath(root, abs)` and `resolve(root, rel)` typed so root+rel is checked, rel+rel is rejected.
- [ ] Windows/POSIX separator normalization applied at mint time; `RelPath` never contains `\\` or a drive prefix.
- [ ] Decode fails-closed on absolute-passed-as-relative and on `..` escapes above root.

## Acceptance And Proof
Tier P0. Unit tests: valid mint, rejection of absolute-as-relative and `..` escape, separator normalization on Windows and POSIX. A `tsc --noEmit` negative fixture proves `RelPath` cannot substitute for `RepoRoot`. Rows in TEST_PROOF_EXPECTATIONS.md.

## Parallel Ownership Notes
Depends on a01. Owns `src/path-utils.*` exclusively; disjoint from a03/a05/a06 brand domains. Downstream consumers migrate in their own workpacks — this pack ships the types + boundary only.
