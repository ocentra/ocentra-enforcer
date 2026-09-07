# UL02 - Grammar Ownership Transfer

<!-- agent-capsule -->
> Agent Capsule
> Plan: `universal-language-enforcement-plan`
> Doc: `UL02 Grammar Ownership Transfer`
> Kind: boss decision workpack.
> Read when: before any parser, grammar, vendor, or `enforcer-memory` language move.
> Stop rule: no product source edit is legal until the ownership decision is committed.
> Proves: one migration base, freeze interval, sequence, and post-transfer owner.
> Does not prove: parser parity or successful extraction.
> Proof rule: decision names exact SHAs and every active/stale claim on the surface.
<!-- /agent-capsule -->

- owns: `docs/plans/universal-language-enforcement-plan/decisions/UL02-GRAMMAR-OWNERSHIP.md`
- deps: `UL00`
- tier: `P0 architecture decision`

> Owner class: boss plus current language-parity owner; not a small-model implementation packet.
> Batch limit: one committed decision record.

## Where We Are

The existing language-parity campaign owns `enforcer-memory` parser/language/grammar surfaces and still documents later rich-parity work. CyberSkills CP02 proposed overlapping extraction without an explicit transfer.

## Where We Want To Be

One recorded authority determines whether remaining rich-parity work lands before extraction, during a freeze, or after migration in the new owner. Every task knows the sole legal grammar surface and base SHA.

## Owns

- one decision record containing current owner, new owner, exact base/tree SHA, active claim audit, freeze start/end criteria, move order, rollback, and post-transfer directory map;
- no parser, grammar, manifest, or vendor file.

## Objective

Prevent two campaigns from moving or extending the same 145 grammar-binding substrate concurrently. Make UL03 dependency-legal and update CyberSkills to depend on the shared result.

## Requirement Checklist

- [ ] Record live worktree/branch/claim state and current language-parity plan status.
- [ ] Choose and justify G3-before, G3-during, or G3-after extraction.
- [ ] Name the exact source and destination ownership patterns.
- [ ] Define a no-new-grammar freeze interval and emergency rule.
- [ ] Define how commits already in flight are accepted/rejected.
- [ ] Define rollback without destructive reset or vendor loss.
- [ ] Obtain explicit boss/current-owner acceptance.

## Acceptance And Proof

Plan validators, link checks, coordination guard, and a read-only source/claim inventory must pass. UL03 stays blocked until the committed decision is referenced from both plans.

## Stop conditions

Stop on an unresolved claim, unknown active branch, uncommitted grammar work, or disagreement about the migration base.

## Parallel Ownership Notes

Evidence gathering is parallel-safe and read-only. The decision record has one writer.
