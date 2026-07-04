# a07 Parse At Boundary Json And Env

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Parse At Boundary Json And Env`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `src/routing.*`, `src/env-boundary.*`
- deps: `a01`, `a03`
- tier: `P0`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md).

## Where We Are
`src/routing.mjs` calls `JSON.parse(fs.readFileSync(...))` in at least five places (rules.json, config, profile, target config) and feeds the raw `any` result straight into logic. `process.env` is read ad hoc across the codebase with no single validated boundary. Untyped JSON and undeclared env vars are the two largest holes in the type story.

## Where We Want To Be
Every `JSON.parse` in `src/routing.*` is wrapped so the parsed value is immediately Effect-decoded to a known schema (never `any`), and a single `src/env-boundary.*` module is the only place `process.env` is read, exposing a decoded, typed config object.

## Requirement Checklist
- [ ] Replace each raw `JSON.parse` in `src/routing.*` with parse-then-`decode` returning a typed schema value.
- [ ] Decode failure is fail-closed (throws/Left with the file path), never a silent `{}`.
- [ ] Create `src/env-boundary.*` as the sole reader of `process.env`; all other reads route through it.
- [ ] Env decoder declares each consumed var, its type, and required/default; unknown/missing-required fails-closed.
- [ ] Consumed rule ids from parsed JSON use the a03 `RuleId` decoder.

## Acceptance And Proof
Tier P0. Unit tests: malformed JSON and schema-invalid JSON each produce a decode error naming the source; a grep/AST check asserts zero `process.env` reads outside `src/env-boundary.*` and zero bare `JSON.parse` results typed `any` in `src/routing.*`. Rows in TEST_PROOF_EXPECTATIONS.md.

## Parallel Ownership Notes
Depends on a01 (compiler) and a03 (`RuleId` for parsed rule ids). Owns `src/routing.*` and the new `src/env-boundary.*` exclusively; disjoint from all brand-domain packs. The env-boundary single-owner invariant is what a09/a10 later assert mechanically.
