# d03 Deferred Work Gate

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Deferred Work Gate`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `src/deferred-work-gate.ts, tests/deferred-work-gate.test.mjs, tests/fixtures/deferred/**`
- deps: `d01-rule-mechanization-engine`
- tier: `P1 unit`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md).

## Where We Are
Stubs and TODO/FIXME/`unimplemented!`/`throw new Error("not implemented")` markers leak into merged code with no gate. ADBP treats "no silent deferral" as prose advice. The enforcer's own scanners in `src/generic-*-scanner.mjs` detect literals but do not hard-fail unmarked deferral.

## Where We Want To Be
A diff-scoped gate that hard-fails any newly introduced stub/deferral marker unless it carries an explicit, structured `DEFERRED(#ref)[revisit:<date-or-milestone>]` annotation.

## Requirement Checklist
- [ ] Detect a fixed vocabulary of deferral markers per language (TODO, FIXME, stub throws, `unimplemented!`, `todo!`, `pass  # TODO`).
- [ ] Exempt only markers matching the exact `DEFERRED(#<ref>)[revisit:<value>]` grammar; malformed annotations still fail.
- [ ] Diff-scoped: only lines added/changed in the working diff are gated, so legacy stubs do not block (paired with d02 baseline).
- [ ] `#<ref>` must be non-empty; `revisit:` value must be non-empty.
- [ ] Emitted as a first-class registry ruleId via d01 (fixtures + doc + validator parity).

## Acceptance And Proof
Tier T1 (P1 unit). Prove via `tests/deferred-work-gate.test.mjs` over `tests/fixtures/deferred/**`: unmarked stub in added lines fails; correctly annotated stub passes; malformed annotation fails; legacy stub outside the diff passes. Mechanism: regex/marker scan intersected with diff hunks, structured-annotation parser as the only escape hatch.

## Parallel Ownership Notes
Depends on d01 for its ruleId/fixture parity. Composes with d02 (diff-scoping vs baseline) but owns disjoint files, so it runs concurrently with d02/d04.
