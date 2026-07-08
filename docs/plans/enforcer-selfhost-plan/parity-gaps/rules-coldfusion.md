# Parity gaps — rules/coldfusion

CFML/ColdFusion is not a target: no `coldfusion` language in `rules.json` (only common/python/rust/typescript), and no literal-scan registry entry. Every normative rule below is a GAP. Security concepts (SQLi/XSS/swallow/typed-error/secrets/size) exist ONLY as python/rust/ts family rules that do not scan `.cfc`/`.cfm` — so they provide NO backing for the CFML target. Proposed ruleIds use family `coldfusion`, language `coldfusion`, appliesTo `**/*.cfc` (or `**/*.cfm` for view rules).

| ADBP point | ADBP source | Backed? (family or NO/PARTIAL) | Tier | Proposed ruleId | Fail-fixture | Pass-fixture |
|---|---|---|---|---|---|---|
| Handlers must never run `<cfquery>`/`queryExecute()` | architecture.md | NO | T1 | CF-ARCH-1.1 | handlers/Jobs.cfc with `queryExecute()` in an action | handler delegates to service, no query |
| Handlers must not contain business rules (thin HTTP layer) | architecture.md | NO | T2 | CF-ARCH-1.2 | handler with `if(status==...)` domain branch | handler that only reads rc/prc + delegates |
| Handlers inject via WireBox `property`; no `createObject`/`new` for collaborators | architecture.md, patterns.md | NO | T1 | CF-ARCH-1.3 | `new models.jobs.JobGateway()` inside a method | `property name="x" inject="..."` |
| Services must not run a query directly (call gateway) | architecture.md | NO | T1 | CF-ARCH-2.1 | `*Service.cfc` containing `queryExecute(`/`<cfquery` | service calls `gateway.find()` |
| Services (singletons) must be stateless — no per-request data in `variables` scope | architecture.md, patterns.md | NO | T2 | CF-ARCH-2.2 | singleton writing `variables.currentJob=` in a method | data passed via args/returns only |
| Gateways are the ONLY layer that runs queries | architecture.md | NO | T2 | CF-ARCH-3.1 | query in a non-`*Gateway.cfc` model | query only in `*Gateway.cfc` |
| Every dynamic SQL value bound via `cfqueryparam`/params struct (CWE-89) | architecture.md, typing-style.md, tooling.md (QUERYPARAM_REQ) | NO (rust/py SQLi rules don't scan .cfc) | T1 | CF-SEC-1.1 | `queryExecute("... WHERE id=#rc.id#")` interpolated | `queryExecute(sql, {id:rc.id})` / cfqueryparam |
| Views (`.cfm`) must contain no queries/business logic/`createObject()` | architecture.md | NO | T1 | CF-ARCH-4.1 | view `.cfm` with `<cfquery>` or `createObject()` | view outputs only from `prc` |
| Views must encode all output (`encodeForHTML()` etc., CWE-79) | architecture.md, typing-style.md | NO | T1 | CF-SEC-2.1 | `<cfoutput>#userData#</cfoutput>` raw | `<cfoutput>#encodeForHTML(userData)#</cfoutput>` |
| Services must not read `rc`/`form`/`url` (HTTP scopes belong to handlers) | architecture.md | NO | T1 | CF-ARCH-2.3 | service method referencing `form.`/`url.`/`rc.` | service takes typed args |
| Layer dependency is downward-only (gateway must not inject a service; view must not touch gateway) | architecture.md | NO | T2 | CF-ARCH-5.1 | `*Gateway.cfc` with `inject="...Service@app"` | gateway injects only lower deps |
| No `createObject("component")`/`new path.To.Cfc()` for app collaborators (`new` ok for DTOs) | architecture.md, patterns.md | NO | T1 | CF-DI-1.1 | `createObject("component","models.x")` for a service | property injection |
| Services/gateways/FSMs = `singleton`; domain objects/DTOs = `transient` | patterns.md | NO | T2 | CF-DI-1.2 | `*Service.cfc` without `component singleton` | `component singleton {` on services |
| No reaching into `application` scope / global registry to find a collaborator | patterns.md | NO | T2 | CF-DI-1.3 | method reading `application.jobService` | injected property |
| Throw typed, namespaced errors via `type=` (`app.validation.*` etc.) | error-handling.md | NO | T2 | CF-ERR-1.1 | `throw(message="...")` with no `type` | `throw(type="app.validation.MissingField", ...)` |
| Never swallow: empty/`return true` catch forbidden | error-handling.md | NO (rust/py swallow rules don't scan .cfc) | T1 | CF-ERR-2.1 | `catch(any e){}` empty body | catch that logs+rethrows/recovers |
| Bare `catch(any e)` only at a true boundary; must log+safe-return | error-handling.md | NO | T2 | CF-ERR-2.2 | `catch(any e)` inside a service method | narrow `catch(app.payment.Declined e)` |
| No information disclosure — never render `cfcatch.detail`/`tagContext`/stack to user (CWE-209) | error-handling.md | NO | T1 | CF-SEC-3.1 | `<cfoutput>#cfcatch.tagContext#</cfoutput>` | generic view + server-side log |
| No debug output shipped: `this.robustEnabled=false` in prod; no `<cfdump>`/`cfsetting showDebugOutput` in shipped code | error-handling.md, patterns.md | NO | T1 | CF-SEC-3.2 | `<cfdump>` in a handler/view | no cfdump; robustEnabled false |
| `Application.cfc` must implement `onError()` last-resort net | error-handling.md | NO | T2 | CF-ERR-3.1 | `Application.cfc` with no `onError(` | `function onError(exception,eventName)` present |
| Validate boundary input before any service call (fail fast, 4xx) | error-handling.md | NO | T3 (intent hard to mechanize) | CF-ERR-4.1 | — (labeled T3) | — |
| No `writeOutput`/`writeDump` for diagnostics — use LogBox `log` | patterns.md | NO | T2 | CF-LOG-1.1 | `writeDump(var)` in a `.cfc` | `log.error(...)` via injected logbox logger |
| File size ≤ 200 lines (`.cfc`) | patterns.md, tooling.md | NO (no CF file-size rule) | T1 | CF-SIZE-1.1 | `.cfc` of 250 lines | `.cfc` ≤ 200 lines |
| Function/method ≤ 30 lines | patterns.md | NO | T1 | CF-SIZE-1.2 | 40-line method | method ≤ 30 lines |
| Parameters per method ≤ 5 | patterns.md | NO | T1 | CF-SIZE-1.3 | method with 6 args | method with struct/domain arg |
| No catch-all `Utils.cfc` grab-bag (cohesion) | patterns.md | NO | T3 (labeled) | CF-ARCH-6.1 | — (labeled T3) | — |
| Fixed value sets defined as a constants component under `models/enums/`; no scattered raw status strings | state-machines.md | NO (existing `enum` hits are rust/ts) | T2 | CF-FSM-1.1 | `if(status=="in_progress")` literal | reference `jobStatus.IN_PROGRESS` |
| Store enum VALUE not display label in DB | state-machines.md | NO | T3 (labeled) | CF-FSM-1.2 | — (labeled T3) | — |
| ANY stateful entity must route status changes through a formal FSM (no direct `job.setStatus` w/o assertTransition) | state-machines.md | NO | T1 | CF-FSM-2.1 | `job.setStatus("closed")` without `assertTransition` | `setStatus(fsm.assertTransition(from,to))` |
| FSM must validate before mutating/persisting; illegal transition throws `app.state.InvalidTransition` | state-machines.md | NO | T2 | CF-FSM-2.2 | `canTransition` returns bool, mutation regardless | `assertTransition` throws on illegal |
| Allowed transitions declared explicitly; no arbitrary state write | state-machines.md | NO | T2 | CF-FSM-2.3 | direct status assignment path | transitions() map defines allowed set |
| Terminal states (CLOSED/CANCELLED) have no outgoing transitions | state-machines.md | NO | T2 | CF-FSM-2.4 | transitions map gives CLOSED outgoing states | terminal states map to `[]` |
| FSMs singleton + stateless | state-machines.md | NO | T2 | CF-FSM-2.5 | FSM writing `variables.x` in a method | pure from/to in, decision out |
| Every FSM has a TestBox spec (legal + illegal transitions) | state-machines.md, testing.md | NO | T2 | CF-TEST-1.1 | `*StateMachine.cfc` with no matching spec | `*StateMachineTest.cfc` present |
| Every feature: ≥1 happy + 1 error/edge path per public method, same task | testing.md | NO | T3 (labeled) | CF-TEST-1.2 | — (labeled T3) | — |
| One spec per component, mirror source tree; specs extend `testbox.system.BaseSpec` | testing.md | NO | T2 | CF-TEST-1.3 | spec not extending BaseSpec | spec extends BaseSpec |
| Assert on error `type`, not message strings | testing.md | NO | T2 | CF-TEST-1.4 | `.toThrow(message="...")` | `.toThrow(type="app.validation.MissingField")` |
| No assertion-free tests (`it` with no `expect`) | testing.md | NO | T1 | CF-TEST-1.5 | `it("x", function(){ svc.do(); })` no expect | `it` with an `expect(...)` |
| Coverage floor (start 70%) enforced in CI | testing.md, tooling.md | NO | T2 | CF-CI-1.1 | CI config w/o coverage floor | testbox coverage gate ≥70% |
| CommandBox mandatory: `box.json` committed as single dependency/script manifest | tooling.md | NO | T1 | CF-TOOL-1.1 | CFML repo missing `box.json` | `box.json` present at root |
| `.cflintrc` committed (CFLint native gate configured) | tooling.md | NO | T1 | CF-TOOL-1.2 | repo missing `.cflintrc` | `.cflintrc` present |
| No floating dependency ranges in prod; pin versions in `box.json` | tooling.md | NO | T1 | CF-DEP-1.1 | `box.json` dep `"*"`/`"^5"` in deps | pinned exact versions |
| Secret scan (gitleaks) run — no hardcoded secrets (CWE-798) | tooling.md | NO (existing secret rules not CF-scoped) | T1 | CF-SEC-4.1 | secret literal in `.cfc` | no secrets; gitleaks in gate |
| cfformat is source of truth; CI runs `format:check` | tooling.md, typing-style.md | NO | T1 | CF-TOOL-2.1 | CI without cfformat check | `format:check` step in CI |
| CFLint hard-gate rules treated as errors (MISSING_VAR, QUERYPARAM_REQ, GLOBAL_VAR, UNUSED_*, COMPLEX_BOOLEAN_CHECK, NESTED_CFOUTPUT, ARG_TYPE_MISSING, COMPONENT_HINT_MISSING) | tooling.md | NO | T1 | CF-TOOL-1.3 | `.cflintrc` marking these as warning/ignore | these rules set to ERROR |
| Git hooks: pre-commit (format+lint), pre-push (arch task + testbox + gitleaks) | tooling.md | NO | T2 | CF-CI-2.1 | no hook wiring | pre-commit/pre-push scripts present |
| `var`/`local`-scope EVERY function-local variable (singleton race) | typing-style.md, tooling.md (MISSING_VAR) | NO | T1 | CF-STYLE-1.1 | `total=0;` unscoped in a function | `var total=0;` |
| Read arguments via `arguments` scope, never bare name | typing-style.md | NO | T2 | CF-STYLE-1.2 | `for(line in lines)` bare | `arguments.lines` |
| `returntype` declared on every public/remote method | typing-style.md | NO | T1 | CF-STYLE-2.1 | `public function create(...)` no returntype | `public Job function create(...)` |
| Type every argument; `any` needs `// reason:` comment | typing-style.md | NO | T2 | CF-STYLE-2.2 | `function f(customerID)` untyped arg | `required string customerID` |
| `access="private"` by default; `remote` only for real API/AJAX (validate hard) | typing-style.md | NO | T2 | CF-STYLE-2.3 | helper method left `public` w/o caller | `access="private"` helper |
| Script-first: new `.cfc` in script syntax, not `<cffunction>` tags | typing-style.md | NO | T1 | CF-STYLE-3.1 | new `.cfc` using `<cffunction>` | `component { function ... {} }` |
| Banned: `evaluate(expr)` (CWE-94) | typing-style.md | NO | T1 | CF-STYLE-4.1 | `evaluate(userExpr)` | direct expression/`structGet` |
| Banned: `iif(c, de(...), de(...))` | typing-style.md | NO | T1 | CF-STYLE-4.2 | `iif(c, de("a"), de("b"))` | `c ? "a" : "b"` |
| Banned: bare `#dateFormat(now())#` for storage (use parameterized/UTC) | typing-style.md | NO | T2 | CF-STYLE-4.3 | `#dateFormat(now())#` stored | UTC/parameterized `now()` |
| Naming: components PascalCase matching filename; methods/vars camelCase; enum members UPPER_SNAKE; `*Gateway`/`*Service` suffixes | typing-style.md | NO | T2 | CF-STYLE-5.1 | `JobService.cfc` declaring `component jobservice` | filename-matching PascalCase + suffix |
