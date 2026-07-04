# f05-detect-and-route Detect And Route

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Detect And Route`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling completion, product status, PR readiness, or broad DONE.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `src/router/{detect,route-plan,native-tie,scope}.ts, tests/router/**, tests/fixtures/router/**`
- deps: `a01, d01, f03`
- tier: `P1/P3 T1`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md).

## Where We Are
Language, structure, and tool selection are hardcoded per call today: each per-language rule family and each native-tool bridge is invoked directly, and the MCP surface has grown one tool per language/tool (surface bloat). A human or AI must already know which language pack and which native checker to run before invoking anything. There is no single mechanical detector that reads the repo and decides what to run.

## Where We Want To Be
One clever router behind `enforcer check`/`enforcer scan` that, given NOTHING, does the right thing. It MECHANICALLY produces a **route plan** in three stages:
1. **detect languages** — reuse `Tools/ocentra-literal-scan`'s ext->language registry (~65 langs) plus manifest sniffing: `Cargo.toml`, `package.json`/`tsconfig.json`, `pyproject.toml`/`setup.py`, `pubspec.yaml`, `go.mod`, `box.json`, `*.cfc`/`*.cfm`.
2. **detect structure** — workspace members, crates, packages, domain sub-packages, monorepo sub-projects.
3. **resolve scope** — one of `repo|workspace|crate|package|folder|domain|diff`; default `repo`, narrowable by an AI that knows.

It then ROUTES each detected language to that language's enforcer rule packs AND to the available NATIVE tools per f03's tie config (`cargo check`, `tsc`, `eslint`, `ruff`/`pyright`, `dart analyze`, `CFLint`, `go vet`, ...), running native AND ours, scoped. This CONSOLIDATES the per-language/per-tool MCP tools into one routed call. f01 scan-modes, the check/scan/run MCP tools, and the c04 deny-hook all CONSUME this router rather than hardcoding a language.

## Requirement Checklist
- [ ] `detect.ts` reuses the literal-scan ext->language registry + manifest sniff; emits detected language set. Deterministic — no network.
- [ ] `scope.ts` resolves `repo|workspace|crate|package|folder|domain|diff`; default `repo`; explicit scope narrows the plan.
- [ ] `route-plan.ts` emits a serializable ROUTE PLAN: `{ scope, languages[], rulePacks[], nativeTools[] }` — the tested surface.
- [ ] `native-tie.ts` reads f03 tie config to attach native tools per language (run native AND ours).
- [ ] Unknown ext with no manifest routes to literal-scan T2 only (universal floor), never blocks.
- [ ] Scaffolded via d01 so router-emitted route-plan ids carry doc + fixtures + detection test (5-way parity).

## Acceptance And Proof
T1 deterministic. Fixtures assert on the emitted ROUTE PLAN, not side effects. Fail/pass fixture pairs under `tests/fixtures/router/<case>/`:
- mixed `Cargo.toml`+`package.json` repo -> plan includes rust+ts packs AND their native tools (fail fixture: a plan missing ts when package.json present); pass: both present.
- python-only folder -> plan routes python only (fail: leaks rust pack; pass: python-only).
- crate scope -> plan narrows to that crate (fail: repo-wide; pass: single crate).
- unknown ext -> plan carries literal-scan T2 only, no T1 blocker (fail: emits a bogus T1 pack; pass: T2-only).
Detection test `tests/router/route-plan.test.mjs` runs every fixture through detect+scope+route-plan and asserts the plan matches, then runs the d01 parity oracle over router ids. Named proof rows in TEST_PROOF_EXPECTATIONS.md: `router-detect-route-plan` and `router-scope-narrowing`.

## Parallel Ownership Notes
`owns:` is disjoint: `src/router/**`, `tests/router/**`, `tests/fixtures/router/**` are new paths. Consumes (does not own) the literal-scan ext registry (read-only), f03 tie config, d01 scaffolder, a01 toolchain. Does NOT reimplement any language pack or native bridge — it only selects and orders them. f01, the check/scan/run MCP tools, and c04 depend on this router's plan shape but their files are out of scope here.
