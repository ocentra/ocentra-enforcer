# d01 Rule Mechanization Engine

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Rule Mechanization Engine`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `src/rule-new.ts, src/rule-scaffold-parity.ts, scripts/rule-new.mjs, tests/rule-new.test.mjs, tests/rule-scaffold-parity.test.mjs`
- deps: `none`
- tier: `P1 unit`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md).

## Where We Are
Adding a rule today means hand-editing `rules/rules.json`, `rules/rule-id-lock.json`, a validator under `src/`, a doc under `rules/<lang>/`, and fixtures under `tests/fixtures/` with nothing checking they agree. ADBP describes "rule packs" only as prose. There is no scaffolder and no single parity oracle.

## Where We Want To Be
A `ocentra rule new <ID>` command scaffolds all five artifacts in lockstep, plus a hard parity validator that fails closed on any ruleId<->validator<->doc<->fixtures<->registry-row mismatch. This is the keystone every other Track D borrow rides.

## Requirement Checklist
- [ ] `rule new` scaffolds: registry row in `rules/rules.json`, id in `rule-id-lock.json`, validator stub, doc section, pass+fail fixtures.
- [ ] Parity validator asserts every registry `id` has a matching validator export, a `doc#anchor` that resolves, and required pass/fail fixtures per `requiresPassFixture`/`requiresFailFixture`.
- [ ] Parity is fail-closed: unknown validator, dangling doc anchor, or missing fixture is an error, not a warning.
- [ ] Reverse direction checked: no orphan validator/doc/fixture without a registry row.
- [ ] Scaffolder output re-validates green under the parity validator.

## Acceptance And Proof
Tier T1 (P1 unit). Prove via `tests/rule-scaffold-parity.test.mjs` (parity across the live registry) and `tests/rule-new.test.mjs` (scaffold a temp rule, assert five artifacts exist and re-pass parity). Named oracle: `rule-scaffold-parity` validator, invocable as an npm script alongside `enforcer:coverage`.

## Parallel Ownership Notes
Keystone with `deps: none`; d06/d07/d08/d12/d13 depend on this engine and its parity oracle. `owns:` set is disjoint (new `rule-new`/`rule-scaffold-parity` files) so it can start immediately while siblings scaffold their specs.
