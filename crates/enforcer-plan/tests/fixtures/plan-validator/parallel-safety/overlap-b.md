# z98 Overlap B

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Overlap B`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `crates/sample/src/shared.rs`, `crates/sample/src/b.rs`
- deps: `a02`
- tier: `P1`

## Where We Are

B.

## Where We Want To Be

B done.

## Requirement Checklist

- [ ] Build B.

## Acceptance And Proof

`cargo test -p sample`.

## Parallel Ownership Notes

None.
