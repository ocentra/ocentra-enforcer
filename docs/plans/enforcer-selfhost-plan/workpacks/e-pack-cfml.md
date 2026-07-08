# e-pack-cfml CFML And ColdFusion Language Pack

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `CFML And ColdFusion Language Pack`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling completion, product status, PR readiness, or broad DONE.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `crates/enforcer-lang-cfml/**`
- deps: `arc-05-validator, arc-04-rules, arc-18-harness, d01-rule-mechanization-engine, d16-fsm-transition-validity, d22-size-shape-caps`
- tier: `P0/P1`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md), [ADBP_GAPS](../ADBP_GAPS.md), [RUST_ARCHITECTURE](../RUST_ARCHITECTURE.md).

## Where We Are

The Cargo workspace ships per-family validator crates for `enforcer-lang-{rust,ts,py,common,security,iac,k8s}`, and `enforcer-rules` carries structured rule records only for `common`, `python`, `rust`, and `typescript`. There is **zero CFML/ColdFusion**: no `enforcer-lang-cfml` crate, no `coldfusion` language key, no `.cfc`/`.cfm` extension in any rule record's `appliesTo`, no CFML fixtures, and no CFML `Validator` impls. CFML is likewise **absent from the `enforcer-literal-scan` (arc-13) language registry** — `.cfc`/`.cfm` sources are invisible to the universal literal-scan floor. Critically, the enforcer has **no in-process CFML AST/parser**: unlike Rust (`syn`) or TS/Dart (`tree-sitter`), there is no bundled grammar to build a syntax tree for CFML. Every CFML rule from the ADBP gap rows (ADBP_GAPS.md Group 2 CFML clusters `CF-ARCH-*`, `CF-DI-*`/`CFML-DI-*`, `CF-SEC-*`/`CFML-SQL-*`/`CFML-BAN-*`, `CF-ERR-*`/`CF-LOG-*`, `CF-FSM-*`, `CF-STYLE-*`/`CFML-VAR-*`/`CFML-TYPE-*`, `CF-SIZE-*`/`CFML-SIZE-*`/`CFML-CPLX-*`/`CFML-DEAD-*`, `CF-TOOL-*`/`CF-CI-*`, `CF-TEST-*`, and the `linters-frontend-rust-cfml` CFLint rows) is a greenfield gap.

## Where We Want To Be

CFML is a first-class language in the enforcer, implemented as a **NEW workspace crate `enforcer-lang-cfml`** that this pack stands up itself (no arc-* pack pre-builds it), enforced through a **CFLint / CommandBox shell-out adapter running via `enforcer-harness` (arc-18)** because there is no native AST. Concretely:

1. **This pack stands up the crate skeleton itself.** `crates/enforcer-lang-cfml/Cargo.toml` (`[lints] workspace=true`, deps on `enforcer-domain`, `enforcer-rules` (arc-04), `enforcer-validator` (arc-05), and `enforcer-harness` (arc-18) for the run-adapter), `src/lib.rs` (crate root + module tree), and a `register()` fn that adds every CFML `Validator` to the shared rule set. Each validator impls the `Validator` trait (arc-05) and emits `enforcer-domain::Finding`s with a `Fix:` hint; obeys `[workspace.lints]` (no `unwrap/expect/panic/print_*`, no `pub use` barrels).
2. **Mechanism (stated explicitly): T1 shell-out adapter via arc-18.** CFML `Validator`s call an `enforcer-harness` (arc-18) run-adapter that shells out to CFLint (and/or a CommandBox `box task run` / cfformat / TestBox invocation) over the target `.cfc`/`.cfm` files, parses the CFLint JSON result (deserialized into a `serde` newtype in the crate, not a bare string), and maps each CFLint rule code (e.g. `MISSING_VAR`, `QUERYPARAM_REQ`, `GLOBAL_VAR`, `UNUSED_LOCAL_VARIABLE`, `COMPLEX_BOOLEAN_CHECK`, `NESTED_CFOUTPUT`, `ARG_TYPE_MISSING`, `ARG_HINT_MISSING`, `COMPONENT_HINT_MISSING`) onto a branded enforcer `RuleId`. Rules with no CFLint code (layer boundaries, DI shape, FSM routing) are backed by enforcer-side regex/structural `Validator`s over the CFML source. The adapter is deterministic (T1) where the CFLint code is a hard gate; scored (T2) where CFLint emits it as a warning or where the enforcer heuristic is a scored literal-scan signal. The `.cflintrc` fixture must set the hard-gate codes to `ERROR` severity so the adapter treats them as blocking. Per arc-18 discipline the adapter **graceful-skips with a clear diagnostic (not a false pass)** when the CFLint/CommandBox binary is absent.
3. Every CFML rule is registered as a typed rule RECORD in `enforcer-rules` (`coldfusion` language, `.cfc`/`.cfm` `appliesTo`) via the d01 engine, so each rule mints a branded `RuleId`, doc anchor, fail-fixture, pass-fixture, and `cargo test` detection test, passing the d01 5-way parity oracle.
4. **`.cfc`/`.cfm` are ADDED to the `enforcer-literal-scan` (arc-13) language registry** (currently missing) so they feed the always-on T2 literal-scan scored floor. In the Rust workspace this crate declares its own extensions and the arc-13/e01 owner records them — it is NOT a shared mutable JS registry file (cross-ref `e01-literal-scan-universal` and `e-pack-dart`; see Parallel Ownership Notes).
5. FSM/enum semantics (constants component in `models/enums/`, `assertTransition` routing, explicit transitions map, terminal states empty, singleton+stateless FSM, transition-coverage) come from **d16** (`enforcer-lang-common` FSM validator); size/complexity caps (method <=30, component <=200, <=5 args, complexity) come from **d22** (`enforcer-lang-common` size-shape validators). This pack adds the CFML `appliesTo` + CFML fixtures for those validators, not the mechanisms.

## Requirement Checklist

Each row names: fail-fixture (must be flagged) + pass-fixture (must stay clean) + detection test. Fixtures live under `crates/enforcer-lang-cfml/tests/fixtures/<family>/{bad,good}/*.{cfc,cfm}`; detection tests are `cargo test -p enforcer-lang-cfml` (`#[test]`) parity/detection tests (d01-generated), run against the CFLint/CommandBox shell-out adapter.

T1 deterministic (blocking; via CFLint hard-gate code or enforcer structural validator):

- [ ] `enforcer-lang-cfml` crate skeleton stood up (Cargo.toml + lib.rs + `register()`); `coldfusion` language + `.cfc`/`.cfm` appliesTo registered as rule records in `enforcer-rules` (via d01); `.cfc`/`.cfm` added to the `enforcer-literal-scan` registry; CFLint/CommandBox shell-out adapter wired through `enforcer-harness` (arc-18).
- [ ] **CF-ARCH-1.1..5.1 / CFML-LAYER-1.1** layered architecture — handlers run no `<cfquery>`/`queryExecute` (delegate to service); `*Service.cfc` never queries (calls gateway); queries only in `*Gateway.cfc`; views (`.cfm`) contain no query/logic/`createObject`; downward-only deps. fail: handler `.cfc` with `queryExecute(...)`; `<cfquery>` in `OrderService.cfc`; `.cfm` view with `<cfquery>`. pass: handler delegates; query only in `OrderGateway.cfc`.
- [ ] **CF-DI-1.1 / CFML-DI-1.1** WireBox DI — no `createObject("component",...)` / `new FooService()` for collaborators; use `property name="x" inject="...";`. fail: `var svc = new OrderService();` / `createObject("component","models.OrderService")`. pass: `property name="orderService" inject="OrderService";`.
- [ ] **CF-SEC-1.1 / CFML-SQL-1.1 (SQLi, CWE-89)** — every dynamic SQL value uses `cfqueryparam` / a param struct (CFLint `QUERYPARAM_REQ` as ERROR). fail: `queryExecute("... WHERE id = #rc.id#")`. pass: `queryExecute("... WHERE id = :id", { id: rc.id })`.
- [ ] **CF-SEC-2.1 (XSS, CWE-79)** raw `<cfoutput>` of untrusted value — encode via `encodeForHTML`. fail: `<cfoutput>#user.name#</cfoutput>`. pass: `<cfoutput>#encodeForHTML(user.name)#</cfoutput>`.
- [ ] **CF-STYLE-4.1 / CF-STYLE-4.2 / CFML-BAN-1.1** ban `evaluate()` and `iif()`. fail: `evaluate(form.expr)`; `iif(x, ...)`. pass: direct expression / `x ? a : b`.
- [ ] **CF-SEC-4.1** hardcoded secret literal (CWE-798). fail: `variables.apiKey = "sk-live-...";`. pass: read from env/config store.
- [ ] **CF-STYLE-1.1 / CFML-VAR-1.1 (MISSING_VAR)** every function-local variable `var`/`local`-scoped (singleton race). CFLint `MISSING_VAR`/`GLOBAL_VAR` as ERROR. fail: `total = 0;` unscoped inside a function. pass: `var total = 0;`.
- [ ] **CFML-VAR-1.2** use `arguments` scope, not the bare argument name. fail: bare `id` reference where an `arguments.id` exists. pass: `arguments.id`.
- [ ] **CF-STYLE-2.1 / CFML-TYPE-1.1** `returntype` on every public/remote method; type every arg (`any` needs a reason). CFLint `ARG_TYPE_MISSING`/`COMPONENT_HINT_MISSING` as ERROR. fail: `public function create(id) {}` (no returntype, untyped arg). pass: `public Order function create(required numeric id) {}`.
- [ ] **CF-ERR-1.1 (typed throw)** throw typed/namespaced `type=` errors, not bare message. fail: `throw(message="bad");`. pass: `throw(type="app.validation.invalidOrder", message="...");`.
- [ ] **CF-ERR-2.1 (empty catch swallow)** never swallow — empty catch or `return true` catch banned. fail: `catch(any e) {}`. pass: `catch(any e) { log.error(...); rethrow; }` at a boundary.
- [ ] **CF-SEC-3.1 (info disclosure, CWE-209)** no `cfcatch.detail`/`tagContext`/stack to the client; no `<cfdump>` / `robustEnabled=true` in prod. fail: returning `cfcatch.tagContext` to caller; `<cfdump var="#e#">` in a view. pass: generic client message, detail logged.
- [ ] **CF-FSM-1.1 / CF-FSM-2.* (via d16)** constants component under `models/enums/` (no scattered status strings); stateful entity routes through `assertTransition`; explicit transitions map; terminal states empty; FSM singleton+stateless. fail: `job.setStatus("closed")` with no `assertTransition`. pass: `job.setStatus(orderFsm.assertTransition(from, to))`. (Mechanism from **d16**; CFML fixtures + appliesTo here.)
- [ ] **CF-SIZE-1.1..1.3 (via d22)** method <=30 lines, component <=200 lines, <=5 args. fail: 40-line method / 6-arg function. pass: within caps. (Mechanism from **d22**; CFML fixtures here.)
- [ ] **CF-TOOL-1.1 / CF-CI-1.1** `box.json` (CommandBox) committed; `.cflintrc` committed with the hard-gate rules set to `ERROR`. fail: repo fixture missing `box.json` / `.cflintrc`, or `.cflintrc` with a hard-gate code at WARNING. pass: both manifests present, hard-gate codes ERROR.
- [ ] **CF-DEP-1.1** pinned deps in `box.json` — no `"*"`/`"^"` wildcard. fail: `"foo": "*"`. pass: `"foo": "1.2.0"`.
- [ ] **CF-TEST-1.1** one TestBox spec per component, extending `testbox.system.BaseSpec`. fail: component with no matching spec, or a spec not extending BaseSpec. pass: mirroring spec extending BaseSpec.

T2 scored / advisory (non-blocking; fixture asserts score crosses vs stays under threshold, the `enforcer-literal-scan` model / CFLint-as-warning):

- [ ] **CF-ARCH-3.1** services read `rc`/`form`/`url` scopes directly (scored). fail: `rc.customerId` inside `OrderService.cfc`. pass: value passed in as a typed argument.
- [ ] **CF-DI-1.2 / CF-DI-1.3** lifecycle scope — services/gateways/FSMs singleton, DTOs transient; no `application`-scope service lookup (scored). fail: `application.orderService.create(...)`. pass: injected singleton.
- [ ] **CF-LOG-1.1** use LogBox, not `writeDump`/`writeOutput` for diagnostics (scored). fail: `writeDump(var=order)`. pass: `log.error("...", order)`.
- [ ] **CFML-CPLX-2.1 (via d22)** cyclomatic complexity / `COMPLEX_BOOLEAN_CHECK` (>3 terms) / `NESTED_CFOUTPUT` (scored). fail: 4-term boolean; nested `<cfoutput>`. pass: extracted predicate; single output.
- [ ] **CFML-DEAD-1.1** unused local/arg (`UNUSED_LOCAL_VARIABLE`/`UNUSED_METHOD_ARGUMENT`) (scored). fail: declared-unused `var`. pass: no unused locals.
- [ ] **CF-STYLE-3.1** script-first `.cfc` (tag-based component body) (scored). fail: `<cffunction>`-tag component. pass: `component { function ... }` script syntax.
- [ ] **CF-STYLE-5.1** PascalCase filename + `*Service`/`*Gateway` suffix convention (scored). fail: `orderservice.cfc`. pass: `OrderService.cfc`.
- [ ] **CF-STYLE-2.2** `access=private` default for non-API methods (scored). fail: public method used only internally. pass: `access="private"`.
- [ ] **CF-TOOL-2.1** cfformat `format:check` step present in CI (scored). fail: CI fixture without a format:check step. pass: step present.
- [ ] **CF-CI-2.1** TestBox coverage floor >=70% wired as a failing threshold + pre-commit/pre-push hooks (scored). fail: coverage config with no fail floor. pass: fail floor set.
- [ ] **CF-TEST-1.4 (via d23 semantics) / CF-TEST-1.5** assert on error `type` not message; no assertion-free `it` (scored). fail: `it("x", function(){ svc.create(); })` with no `expect`; `expect(function(){...}).toThrow(message="...")`. pass: an `expect(...)` present; `.toThrow(type="app.validation....")`.
- [ ] **CF-FSM transition-coverage (via d16)** every FSM transition tested, valid AND invalid (scored). fail: FSM with no invalid-transition spec. pass: spec asserts the illegal transition throws. (Mechanism from **d16**.)

T3 advisory (no mechanization possible — label required; label presence itself is T1-enforced):

- [ ] **CF-ARCH-6.1** no catch-all `Utils.cfc` dumping-ground beyond a size threshold — `advisory, no mechanization possible + whether a cohesively-named component is a genuine dumping-ground vs a legitimate small helper is design judgment the size gate alone cannot decide` (the >50-line/name heuristic lives in d17/d22 as its scored proxy; the intent call is T3). Rule record carries the label; d01 enforces the label's presence.

## Acceptance And Proof

Tier P0/P1. DONE only when, per TEST_PROOF_EXPECTATIONS.md, `cargo test -p enforcer-lang-cfml` is green: every T1 ruleId has a green d01 parity + detection test exercised through the CFLint/CommandBox shell-out adapter (fail-fixture trips, pass-fixture stays clean), every T2 ruleId has a green score-threshold test, and the T3 row is present with its `advisory, no mechanization possible + <reason>` label whose presence the d01 label check verifies. Additionally: a test proves the `enforcer-lang-cfml` crate registers `coldfusion`/`.cfc`/`.cfm` as rule records in `enforcer-rules`, that `.cfc`/`.cfm` files are picked up by the `enforcer-literal-scan` registry, and that the arc-18 adapter degrades to a clear diagnostic (not a false pass) when the CFLint/CommandBox binary is absent. (Crate-map delta: this pack ADDS the `enforcer-lang-cfml` crate to the workspace — the reconciliation pass records the crate note; do not edit shared index files here.)

## Parallel Ownership Notes

Owns `crates/enforcer-lang-cfml/**` exclusively (the whole new crate: `Cargo.toml`, `src/**`, `tests/fixtures/cfml/**`) — disjoint from all siblings by file. This pack builds its OWN crate skeleton since no arc-* pack pre-builds it. Depends on arc-05 (the `Validator` trait + fixture/parity harness), arc-04 (`enforcer-rules` record load), arc-18 (`enforcer-harness` run-adapter that hosts the CFLint/CommandBox shell-out), d01 (registry records + 5-way parity), d16 (FSM/enum semantics — CFML appliesTo + fixtures only), and d22 (size/complexity caps — CFML fixtures only). The CFLint/CommandBox binary is an external tool dependency run through arc-18, not a repo edit. **No shared-file hazard in the Rust workspace:** unlike the old shared JS literal-scan registry, each new-language crate (this one, `enforcer-lang-dart`) declares its own extensions within its own crate; the `.cfc`/`.cfm` registration in `enforcer-literal-scan` (arc-13, owned by `e01-literal-scan-universal`) is an additive record the arc-13/e01 owner records — coordinate ordering with e01 but do not co-edit a single mutable registry file. This pack never edits Python/Rust/TypeScript rule crates, records, or fixtures.
