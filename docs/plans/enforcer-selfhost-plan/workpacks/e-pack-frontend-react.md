# e-pack-frontend-react Frontend React And Next Rule Family

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Frontend React And Next Rule Family`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling completion, product status, PR readiness, or broad DONE.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `rules/frontend/**.md, src/validators/frontend-*.ts, tests/fixtures/frontend/**`
- deps: `d01, d16, d22`
- tier: `P0/P1`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md), [ADBP_GAPS](../ADBP_GAPS.md).

## Where We Are
The registry has `common/python/rust/typescript` languages but **no frontend (`FE-*`) rule family at all** (ADBP_GAPS Group 2 row "Frontend/React-Next greenfield" and the whole `rules-frontend.md` gap file). React/Next/Vite conventions — feature-layer boundaries, Server/Client discipline, TanStack-Query data-fetching, hooks discipline, component shape, typed errors, a11y, env centralization, TS strictness — are entirely unenforced. There is nothing under `rules/frontend/`, no `frontend-*` validator, and no `tests/fixtures/frontend/` tree.

## Where We Want To Be
A greenfield `FE-*` family, scaffolded through the d01 mechanization engine so every rule ships in 5-way parity (ruleId <-> doc <-> validator <-> {fail+pass fixture} <-> detection test). T1 rules block; T2 rules are scored/advisory over the same fixtures.

CRITICAL DOCTRINE DIVERGENCE FROM ADBP (do not silently copy ADBP here): ADBP mandates **Zod** as the frontend validation source-of-truth (its `FE-TS-1.11`). Our house rule **TS-1.2 FORBIDS Zod and mandates Effect Schema**, and this plan keeps **Effect-only everywhere**. Therefore this pack **explicitly does NOT borrow ADBP's Zod mandate**. Instead of backing `FE-TS-1.11` (Zod-as-SoT), we invert it: we ship `FE-EFFECT-1.1` which **flags any Zod usage as a violation** (`import ... from "zod"`, `z.object(`, `zodResolver`) and mandates Effect Schema (`import { Schema } from "@effect/schema"` / `Schema.Struct`) as the boundary-validation source-of-truth. The ADBP-form form rule (React Hook Form + Zod resolver) is likewise re-expressed against Effect (`@effect/schema` resolver), never Zod. This divergence is deliberate and load-bearing: any future reader who "restores parity with ADBP" by adding a Zod mandate would break our Effect-only doctrine.

FSM/enum semantics for frontend stateful entities are borrowed from **d16** (explicit transition map, no ad-hoc `setStatus` string mutation — `FE-FSM-1.2`). Cross-cutting size/shape caps (component-file/function-length) are borrowed from **d22** rather than re-implemented here; this pack only ships the FE-specific structural rules.

## Requirement Checklist
Each rule below is scaffolded via d01 and ships fail-fixture + pass-fixture + detection test. Grouped by concern:

- [ ] **Feature boundaries** (`FE-ARCH-1.3`, T1): a `features/<a>/**` file importing `@/features/<b>/...` is flagged; importing only `@/lib`/`@/shared`/`@/components` stays clean.
- [ ] **Components->features layer inversion** (`FE-ARCH-1.4`, T2): `components/**` importing from `@/features/**` (or calling `useQuery`/`fetch`) is flagged as a layer inversion; presentational component taking data via props stays clean.
- [ ] **No server-data-in-client-store** (`FE-STATE-1.1`, T1): a Zustand/`useState` store field populated from an API response (server data) is flagged; store holding only UI flags stays clean.
- [ ] **No fetch/axios in useEffect** (`FE-STATE-1.2`, T1): `useEffect(()=>{ fetch(...) / axios. ... },[])` for data-loading is flagged; a query hook (`useQuery({queryKey,queryFn})`) stays clean.
- [ ] **useEffect has WHY comment** (`FE-HOOK-1.2`, T1): a `useEffect(` with no preceding `// why:` comment is flagged; one carrying `// why:` stays clean.
- [ ] **Typed errors** (`FE-PAT-1.4`, T1): `throw new Error(...)` inside `services/**` is flagged; `throw new ApiError(...)` (a named typed error class) stays clean.
- [ ] **next/image + alt / a11y** (`FE-CMP-1.12` + `FE-A11Y-1.2`, T1): a raw `<img src>` or an `<Image>`/`<img>` missing `alt` is flagged; `<Image width height alt=...>` stays clean. Companion a11y rule `FE-A11Y-1.3` (input needs label/aria-label) rides the same validator.
- [ ] **import.meta.env / process.env centralization** (`FE-CFG-1.1`, T1): reading `import.meta.env.*` or `process.env.*` anywhere except `lib/env.ts` is flagged; importing the typed `env` from `lib/env` stays clean.
- [ ] **no-explicit-any** (`FE-TS-1.5`, T1): `: any` without a justifying `eslint-disable`+reason is flagged; `unknown`+guard (or a justified disable) stays clean.
- [ ] **type-only import** (`FE-TS-1.14`, T1): `import { User }` where `User` is used only as a type is flagged; `import type { User }` stays clean.
- [ ] **Explicit FSM transitions** (`FE-FSM-1.2`, via d16, T1): an ad-hoc `setStatus("shipped")` string mutation with no transition table is flagged; an explicit `as const` transition map routed through `assertTransition(from,to)` stays clean.
- [ ] **Effect-not-Zod** (`FE-EFFECT-1.1`, T1 — the divergence rule): any Zod usage (`from "zod"`, `z.object(`, `zodResolver`) is flagged as a violation mandating Effect Schema; boundary validation via `@effect/schema` (`Schema.Struct`) stays clean.

## Acceptance And Proof
All rules are T1 (blocking) except `FE-ARCH-1.4` (T2 scored layer-inversion advisory), which asserts its score crosses the threshold on the fail fixture and stays under on the pass fixture. Per-rule fixtures live under `tests/fixtures/frontend/<ruleId>/{fail,pass}.*`; validators under `src/validators/frontend-<concern>.ts`; docs under `rules/frontend/<concern>.md`. Detection tests iterate every `FE-*` fixture pair through the engine and assert fail-flagged / pass-clean, then run the d01 parity oracle over the new family (every `FE-*` registry id resolves to a validator export + doc anchor + both fixtures). Named proof rows to be added/updated in TEST_PROOF_EXPECTATIONS.md: `frontend-family-detection` and `frontend-family-parity`. Explicitly assert `FE-EFFECT-1.1` fail fixture (a `z.object` schema) is flagged and its pass fixture (`Schema.Struct`) is clean — this proof pins the Effect-only divergence mechanically.

## Parallel Ownership Notes
`owns:` is disjoint from every sibling: `rules/frontend/**`, `src/validators/frontend-*.ts`, and `tests/fixtures/frontend/**` are new paths owned exclusively here. FSM semantics are consumed from d16 (do not redefine the transition engine here — only the FE-facing rule doc/fixtures). Size caps are consumed from d22. This pack must not touch `rules/rules.json` routing beyond the rows d01's scaffolder writes for its own `FE-*` ids.
