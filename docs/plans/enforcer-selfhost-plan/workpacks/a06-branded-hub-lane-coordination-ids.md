# a06 Branded Hub Lane Coordination Ids

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Branded Hub Lane Coordination Ids`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `src/coordination/context.*`
- deps: `a01`
- tier: `P0`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md).

## Where We Are
Coordination code (`src/coordination/`, driven by `api.mjs`/`runner.mjs`) threads hub names and lane ids as raw `string`. A hub name passed where a lane id is expected, or an unsanitized id used to build a filesystem path under the coordination root, is invisible to the compiler and only surfaces as a mislocated or colliding coordination artifact.

## Where We Want To Be
`HubName` and `LaneId` branded types minted only in `src/coordination/context.*`, so the two cannot be swapped and every id is validated (safe charset for on-disk use) before it reaches path construction or presence/claim APIs.

## Requirement Checklist
- [ ] Define `HubName` and `LaneId` brands + decoders in `src/coordination/context.*`.
- [ ] Decoders enforce a filesystem-safe charset (no separators, no `..`, bounded length).
- [ ] Context/claim/presence signatures accept `HubName`/`LaneId`, never bare `string`.
- [ ] Swapping a `HubName` for a `LaneId` (or vice versa) is a compile error.
- [ ] Decode fails-closed on empty / unsafe / oversized ids.

## Acceptance And Proof
Tier P0. Unit tests: valid mint for both brands, rejection of unsafe charset / empty / oversize, and confirmation that path construction consumes only branded ids. A `tsc --noEmit` negative fixture proves `HubName` != `LaneId`. Rows in TEST_PROOF_EXPECTATIONS.md.

## Parallel Ownership Notes
Depends on a01. Owns `src/coordination/context.*` exclusively; disjoint from a03/a04/a05 brand domains and from a07's routing/env boundary. Broader coordination modules migrate in later packs.
