# CP11 - Advisory and Manual Retention

<!-- agent-capsule -->
> Agent Capsule
> Plan: `cyberskills-parity-plan`
> Doc: `CP11 Advisory and Manual Retention`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `proof/cyberskills/cp11/<batch-id>/**` and immutable retention proposals; `cyberskills-ledger-integrator` owns ledger application
- deps: `cp08`
- tier: `P1 T3`

> Owner class: Luna-safe.
> Batch limit: exactly 10 skills, except the final remainder.
> Depends on: approved CP08 components.

## Where We Are

Advisory and manual knowledge can be lost or mislabeled as enforcement when a whole skill is forced into one native/adapter/prose bucket.

## Where We Want To Be

Retain every non-mechanical component with stable source evidence, an honest purpose/reason, and a T1 retention check.

## Batch-01 validation packet

The graph-selected first packet is `proof/cyberskills/cp11/batch-01/retention.json`. It covers the deterministic first ten `IF/ai-security` catalog IDs, references immutable CP08 artifact paths and hashes, and records 10 retained advisory plus 10 retained manual components. The packet has no native implementation or external execution authority. Its artifact SHA-256 is `D33B01026D72DAB2B2DAEBD21C3392366EA1C804C4EFAC62CFA3D89F771B1D89` (12291 bytes).

The graph retention gate also validates all 816 non-protected CP08 identities. This is validation evidence in the current worktree; CP11 is not `DONE` until the exact packet and gate evidence are reviewed on a committed tree.

## Objective

Ensure knowledge that cannot honestly decide pass/fail remains discoverable and never disappears behind a false parity count.

## Requirement Checklist

- [ ] Every advisory/manual component retains canonical source path, hash, and section anchors.
- [ ] `advisory` explains what a user or AI learns and why it is not enforcement.
- [ ] `manual` states actor, prerequisites, procedure, expected evidence, and why judgment/environment prevents deterministic enforcement.
- [ ] A mechanical retention gate fails if the component or source anchor disappears.
- [ ] No T1/T2-looking predicate is buried as advisory merely to avoid implementation.
- [ ] No advisory/manual item is described as passed, clean, compliant, or enforced.
- [ ] If a third-party engine could mechanize part of it, split that part into an external component.

## Acceptance And Proof

Run the ledger/retention tests and derived summary. The result may increase retained coverage but never native/engine proof counts.

## Parallel Ownership Notes

Audit batches may be disjoint and read-only. `cyberskills-ledger-integrator` serializes the disposition ledger, and `<batch-id>` is assigned before claim.
