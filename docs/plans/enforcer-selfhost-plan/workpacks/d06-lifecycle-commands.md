# d06 Lifecycle Commands

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Lifecycle Commands`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `src/lifecycle-commands.ts, src/lifecycle-oracle.ts, tests/lifecycle-commands.test.mjs`
- deps: `d01-rule-mechanization-engine`
- tier: `P1 unit`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md).

## Where We Are
The CLI (`src/cli-main.mjs`, `src/cli-command-dispatch.mjs`) exposes scan/check/proof verbs but no coherent lifecycle. ADBP describes a plan->implement->check->fix->review flow as narrative. There is no single command family binding those phases to our validators.

## Where We Want To Be
A `plan | implement | check | fix | review` command family where every phase's pass/fail is decided by our existing validators (the oracle), not by prose or model self-report.

## Requirement Checklist
- [ ] Five subcommands registered in the CLI dispatch with stable exit-code semantics.
- [ ] Each phase delegates its verdict to a named validator/oracle (e.g. `check` -> registry validators; `review` -> d10 auditor rows).
- [ ] `fix` invokes d07 loop; `review` requires green proof rows before it can pass.
- [ ] No phase can report success unless its oracle returns pass; there is no prose-only pass path.
- [ ] Phase transitions recorded to d04 telemetry.

## Acceptance And Proof
Tier T1 (P1 unit). Prove via `tests/lifecycle-commands.test.mjs`: each subcommand routes to the correct oracle; a failing oracle forces a non-zero exit; `review` blocks on missing proof rows. Mechanism: command dispatch table mapping phase -> validator function, asserted by stubbed-oracle outcomes.

## Parallel Ownership Notes
Depends on d01 for validator registry. Wraps d07 (fix) and d10 (review) by contract, but owns disjoint lifecycle files so it can be built against their interfaces concurrently.
