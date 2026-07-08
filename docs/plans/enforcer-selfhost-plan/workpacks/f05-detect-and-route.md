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

- owns: `crates/enforcer-scan/src/router/**`, `crates/enforcer-scan/tests/router.rs`, `crates/enforcer-scan/tests/fixtures/router/**`
- deps: `arc-15-enforcer-scan, arc-13-enforcer-literal-scan, d01-rule-mechanization-engine, f03-project-tie-and-native-augment`
- tier: `P1/P3 T1`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md), [RUST_ARCHITECTURE](../RUST_ARCHITECTURE.md).

## Where We Are
Language, structure, and tool selection are hardcoded per call today: each per-language rule family and each native-tool bridge is invoked directly, and the MCP surface has grown one tool per language/tool (surface bloat). A human or AI must already know which language pack and which native checker to run before invoking anything. There is no single mechanical detector inside `enforcer-scan` that reads the repo and decides what to run.

## Where We Want To Be
One clever router module tree behind `enforcer check`/`enforcer scan` (in `crates/enforcer-scan/src/router/**`) that, given NOTHING, does the right thing. It MECHANICALLY produces a **route plan** in three stages:
1. **detect languages** (`router/detect.rs`) — reuse the `enforcer-literal-scan` (arc-13) ext->language registry (~65 langs) plus manifest sniffing: `Cargo.toml`, `package.json`/`tsconfig.json`, `pyproject.toml`/`setup.py`, `pubspec.yaml`, `go.mod`, `box.json`, `*.cfc`/`*.cfm`.
2. **detect structure** — Cargo workspace members, crates, packages, domain sub-packages, monorepo sub-projects.
3. **resolve scope** (`router/scope.rs`) — one of `repo|workspace|crate|package|folder|domain|diff` (`enforcer-domain` `ScanScope`); default `repo`, narrowable by an AI that knows.

It then ROUTES each detected language to that language's enforcer `Validator` packs (arc-06..13) AND to the available NATIVE tools per f03's tie config (`cargo check`, `tsc`, `eslint`, `ruff`/`pyright`, `dart analyze`, `CFLint`, `go vet`, ...) run through the `enforcer-harness` (arc-18) run-adapters, running native AND ours, scoped. This CONSOLIDATES the per-language/per-tool MCP tools into one routed call. f01 scan-modes, the check/scan/run MCP tools, and the c04 deny-hook all CONSUME this router rather than hardcoding a language.

## Requirement Checklist
- [ ] `router/detect.rs` reuses the arc-13 ext->language registry + manifest sniff; emits a detected language set. Deterministic — no network.
- [ ] `router/scope.rs` resolves `repo|workspace|crate|package|folder|domain|diff` (`ScanScope` newtype); default `repo`; explicit scope narrows the plan.
- [ ] `router/plan.rs` emits a serializable ROUTE PLAN (`serde` struct `{ scope, languages[], rule_packs[], native_tools[] }`) — the tested surface.
- [ ] `router/native_tie.rs` reads the f03 tie config (resolver API) to attach native tools per language, dispatched via the arc-18 harness (run native AND ours).
- [ ] Unknown ext with no manifest routes to the arc-13 literal-scan T2 floor only (universal floor), never blocks.
- [ ] Scaffolded via d01 so router-emitted route-plan ids carry doc + fail/pass fixtures + a `cargo test` detection test (5-way parity, Rust-native).

## Acceptance And Proof
T1 deterministic. Fixtures assert on the emitted ROUTE PLAN, not side effects. Fail/pass fixture pairs under `crates/enforcer-scan/tests/fixtures/router/<case>/`:
- mixed `Cargo.toml`+`package.json` repo -> plan includes rust+ts packs AND their native tools (fail fixture: a plan missing ts when package.json present); pass: both present.
- python-only folder -> plan routes python only (fail: leaks rust pack; pass: python-only).
- crate scope -> plan narrows to that crate (fail: repo-wide; pass: single crate).
- unknown ext -> plan carries literal-scan T2 only, no T1 blocker (fail: emits a bogus T1 pack; pass: T2-only).
Detection test `crates/enforcer-scan/tests/router.rs` runs every fixture through detect+scope+plan and asserts the plan matches (`cargo test -p enforcer-scan --test router` exits 0), then runs the d01 parity oracle over router ids. Named proof rows in TEST_PROOF_EXPECTATIONS.md: `router-detect-route-plan` and `router-scope-narrowing`.

## Parallel Ownership Notes
`owns:` is disjoint: `crates/enforcer-scan/src/router/**`, `crates/enforcer-scan/tests/router.rs`, `crates/enforcer-scan/tests/fixtures/router/**` are new paths inside the arc-15 crate (which owns the crate skeleton + fan-out engine). Disjoint by file from f01 (`modes.rs`) and f02 (`onboard.rs`), which also live in `enforcer-scan`. Consumes (does not own) the arc-13 literal-scan ext registry (read-only), the f03 tie config, the d01 scaffolder, and the arc-18 harness adapters. Does NOT reimplement any language pack or native bridge — it only selects and orders them. f01, the check/scan/run MCP tools, and c04 depend on this router's plan shape but their files are out of scope here. `owns disjoint? = Y`.
