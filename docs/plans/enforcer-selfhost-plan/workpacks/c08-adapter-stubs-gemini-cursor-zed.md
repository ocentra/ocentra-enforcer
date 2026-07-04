# c08 Adapter Stubs Gemini Cursor Zed

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Adapter Stubs Gemini Cursor Zed`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `src/install/adapters/gemini.*, src/install/adapters/cursor.*, src/install/adapters/zed.*`
- deps: `c01-install-core-and-cli-contract`
- tier: `P0 contract/schema`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md).

## Where We Are
Only Codex has a real adapter, and c03/c06/c07 add Claude/generic. Gemini, Cursor, and Zed each have differing config surfaces we are not building fully yet, but the c01 adapter registry must still resolve them by id rather than silently no-op or crash.

## Where We Want To Be
Contract-only stub adapters for `gemini`, `cursor`, and `zed` that satisfy the c01 interface, declare themselves not-yet-implemented explicitly, and note that ADBP-style config converters are deferred.

## Requirement Checklist
- [ ] Each stub implements `plan/apply/verify` returning a well-formed report with `status:"deferred"` and a reason.
- [ ] `install`/`apply` on a stub is a safe no-op that writes nothing and does not throw.
- [ ] `verify` returns a single advisory check labeled `deferred: no mechanization yet` (T3-labeled, reason stated).
- [ ] A source comment records that ADBP-style converters for these harnesses are deferred (link Track B once numbered).
- [ ] Stubs are registered in the c01 adapter registry so autodetect can surface them.

## Acceptance And Proof
P0 contract (`adapter-stub-contract` in TEST_PROOF_EXPECTATIONS.md): a schema test iterates all three stubs and asserts each conforms to the adapter interface, returns `status:"deferred"`, and performs zero filesystem writes when applied against a temp fixture. Registry lookup for each id must resolve (no throw).

## Parallel Ownership Notes
Owns only the three stub adapter files — disjoint from generic (c07 `generic.*`), codex (c06), and claude (c03). Depends only on c01. Deliberately scoped as contract-only so Track B ADBP converter work can land later without touching these files' interface.
