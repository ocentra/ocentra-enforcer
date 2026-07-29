# z94 Consistent Sample

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Consistent Sample`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `crates/sample/src/lib.rs`
- deps: `a01`
- tier: `P1`

## Where We Are

No such k8s `.mjs` logic exists in this crate; the surface is greenfield.

## Where We Want To Be

Something exists.

## Requirement Checklist

- [ ] Implement the greenfield RuleSpec pattern from scratch.

## Acceptance And Proof

`cargo test -p sample`.

## Parallel Ownership Notes

None.
