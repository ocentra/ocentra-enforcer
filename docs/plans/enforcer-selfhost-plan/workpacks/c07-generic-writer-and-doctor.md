# c07 Generic Writer And Doctor

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Generic Writer And Doctor`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `src/install/adapters/generic.*, src/install/doctor.*`
- deps: `c01-install-core-and-cli-contract, c02-harness-autodetect`
- tier: `P1 unit`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md).

## Where We Are
The Codex doctor logic is tangled into `codex-install.mjs` and is Codex-specific. Harnesses that only need a plain `.mcp.json` server entry have no path today, and there is no shared, mechanical doctor that verifies an install regardless of adapter.

## Where We Want To Be
A generic adapter that writes a standard `.mcp.json` server entry for harnesses with no bespoke needs, plus a shared `src/install/doctor.*` that mechanically re-reads disk and reports per-check pass/fail across all adapters.

## Requirement Checklist
- [ ] Generic adapter writes/upserts `mcpServers["ocentra-enforcer"]` into a target `.mcp.json` given a home path.
- [ ] Shared doctor aggregates per-adapter `verify` checks into one report with severities.
- [ ] Doctor is mechanical: every check re-reads the actual file and resolves the server path, never trusts the plan.
- [ ] Doctor exit is fail-closed on any `error`-severity check; `warning` checks do not fail.
- [ ] Generic adapter and doctor are pure over injected `fs` for fixture testing.

## Acceptance And Proof
T1: unit tests (`generic-writer` and `install-doctor` in TEST_PROOF_EXPECTATIONS.md) assert the generic adapter's `.mcp.json` output against a golden file, and that doctor returns green on a good fixture, red (with the failing check name) on a fixture with a missing/renamed server file.

## Parallel Ownership Notes
Owns `src/install/adapters/generic.*` and `src/install/doctor.*` only — disjoint from codex (c06), claude (c03), and stub (c08) adapters. Depends on c01/c02. Runs concurrently with all other adapter workpacks.
