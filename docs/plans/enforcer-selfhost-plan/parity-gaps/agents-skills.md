# Parity gaps — agents-skills

Registry backs only `common`/`python`/`rust`/`typescript`. There is **no Dart/Flutter and no ColdFusion/CFML language coverage at all** — every rule from those two agents is a gap. Below are ADBP normative rules with NO or PARTIAL backing.

| ADBP point | ADBP source | Backed? (family or NO/PARTIAL) | Tier | Proposed ruleId | Fail-fixture | Pass-fixture |
|---|---|---|---|---|---|---|
| Files ≤ 200 lines (400 for Rust) | CLAUDE.TEMPLATE HR#2; rust-cli GR#8; python-fastapi; flutter; coldfusion GR#10 | NO (no length rule family) | T1 | SIZE-FILE-1.1 | 250-line .py / 450-line .rs source | 199-line file |
| Functions ≤ 30 lines | CLAUDE.TEMPLATE HR#3; rust-cli GR#8; python-fastapi; flutter; coldfusion GR#10 | NO | T1 | SIZE-FUNC-1.1 | 40-line function body | 29-line function |
| Classes ≤ 150 lines, ≤10-12 public methods | python-fastapi Module Size; flutter | NO | T1 | SIZE-CLASS-1.1 | class with 25 methods | class with 8 methods |
| ≤ 5 function parameters | rust-cli GR#8; python-fastapi; flutter; coldfusion GR#10 | NO | T1 | SIZE-PARAMS-1.1 | fn with 7 params | fn with 4 params |
| Cyclomatic complexity < 10, nesting ≤ 3 | python-fastapi; flutter | NO (COMPLEXITY family thin) | T2 | SIZE-CX-1.1 | fn scoring cc=12 | fn scoring cc=6 |
| Line length ≤ 120 incl. trailing pragma | python-fastapi Module Size | NO | T1 | SIZE-LINE-1.1 | 130-char line w/ trailing `# type: ignore` | 118-char line |
| Test files ≤ 300 lines | python-fastapi; react-nextjs; flutter | NO | T1 | SIZE-TESTFILE-1.1 | 350-line test file | 280-line test file |
| No logic in `main.rs` (thin binary) | rust-cli GR#1 | NO | T2 | RUST-ARCH-1.1 | `main.rs` with business fn | `main.rs` only parse+run() |
| stdout=data / stderr=logs; no `println!`/log to stdout in stdio MCP | rust-cli GR#5; rust-mcp GR#1 | PARTIAL (stdout mentioned re: child-proc parsing only) | T1 | RUST-IO-1.1 | `println!` in server/domain path | `tracing` to stderr only |
| `thiserror` for lib, `anyhow` for bin; preserve cause via `#[from]`/`.context()` | rust-cli GR#3 | PARTIAL (thiserror derive rule; no lib-vs-bin split / cause-preservation check) | T2 | RUST-ERR-1.1 | error enum w/o `#[from]`, lost cause | `#[from]`/`.context()` present |
| Inline captured format args (`format!("{x}")`) | rust-cli / rust-mcp Do/Don't | NO (clippy uninlined_format_args) | T1 | RUST-FMT-1.1 | `format!("{}", x)` | `format!("{x}")` |
| No sentinel returns (`-1`/`""`); return `Result`/`Option` | rust-cli Decision Flow | NO | T2 | RUST-SENTINEL-1.1 | fn returns `-1` on failure | returns `Result`/`Option` |
| Borrow params (`&str`/`&Path`/`&[T]`) not owned | rust-cli Decision Flow | NO | T2 | RUST-BORROW-1.1 | `fn f(s: String)` for read-only | `fn f(s: &str)` |
| Every `#[allow(...)]` carries `reason = "..."` | rust-cli Tooling; CLAUDE HR#5 | PARTIAL (suppression justification generic; no allow+reason mechanical check) | T1 | RUST-ALLOW-1.1 | `#[allow(dead_code)]` no reason | `#[allow(dead_code, reason="…")]` |
| MCP modern macro API (`#[tool_router]`/`Parameters<T>`; not legacy `tool_box`) | rust-mcp GR#2 | NO | T1 | MCP-MACRO-1.1 | `#[tool(tool_box)]` legacy | `#[tool_router]` + `#[tool_handler]` |
| MCP tool method never `.unwrap()`/`panic!`; returns `Result<_,McpError>` | rust-mcp GR#5 | PARTIAL (unwrap family exists; not scoped to tool handlers) | T2 | MCP-HANDLER-1.1 | `.unwrap()` in tool fn | returns `Result`, maps err |
| Every tool-arg field has `///` doc comment | rust-mcp checklist | NO | T2 | MCP-DOC-1.1 | JsonSchema field w/o doc | each field documented |
| No blocking calls on tokio executor (`std::fs`/`time::sleep`) | rust-mcp checklist | NO | T2 | MCP-ASYNC-1.1 | `std::thread::sleep` in async tool | `tokio::time::sleep` |
| Checked arithmetic on untrusted numbers | rust-cli/mcp Security | PARTIAL (get_unchecked/transmute rules; no checked-arith-on-input) | T2 | SEC-ARITH-1.1 | `a + b` on parsed input | `a.checked_add(b)` |
| Path traversal: canonicalize + base-dir check | rust-cli/mcp/python/flutter Security | NO | T1 | SEC-PATH-1.1 | open user path w/o canonicalize | `resolve()`+`is_relative_to()` check |
| No shell-string exec of untrusted input | rust-mcp Security; python-fastapi | PARTIAL (py eval/exec covered?; no arg-vector-vs-shell check) | T1 | SEC-SHELL-1.1 | `Command::new("sh").arg("-c", user)` | arg-vector, no shell |
| Services must not import `Session`/ORM; no `commit/flush/add` in service | python-fastapi Layer Rules; CLAUDE constraints | PARTIAL (layer/commit keywords hit; verify service-scoped) | T1 | PYFA-LAYER-1.1 | `db: Session` in service | repo-mediated access |
| Services return Pydantic, never ORM/`dict` | python-fastapi | NO | T2 | PYFA-DTO-1.1 | service returns ORM model | returns `*Out` schema |
| Repos accept typed schemas/params, never `data: dict` | python-fastapi; CLAUDE | NO | T2 | PYFA-REPO-1.1 | `def save(data: dict)` | typed schema param |
| Router declares explicit `response_model`+`status_code` | python-fastapi Routers | NO | T2 | PYFA-ROUTE-1.1 | route missing `response_model` | both declared |
| Services raise domain exceptions, not `HTTPException` | python-fastapi | NO | T2 | PYFA-EXC-1.1 | `raise HTTPException` in service | domain exception |
| No eager multi-repo load; dispatch by type | python-fastapi Selective Loading | NO | T3 (design pattern, hard to mechanize) | PYFA-EAGER-1.3 | 7-repo `_load_entity` | dict-of-callables dispatch |
| `uv` mandatory; never pip/poetry/pipenv | python-fastapi Tooling | NO | T1 | PYFA-UV-1.1 | `poetry.lock`/`Pipfile` present | `uv.lock` present |
| `yaml.safe_load` never `yaml.load`; no `eval/exec/shell=True` | python-fastapi Security | PARTIAL (verify eval coverage) | T1 | PYSEC-INJECT-1.1 | `yaml.load(x)` | `yaml.safe_load(x)` |
| Any status/role/type is StrEnum (no raw strings) | python-fastapi; CLAUDE; flutter | PARTIAL (magic-string→enum rule exists) | T2 | ENUM-STATE-1.2 | `if status == "open"` | `if status == Status.OPEN` |
| Stateful entity has formal FSM; validate transition before mutate | python-fastapi; flutter; coldfusion GR#6; CLAUDE | NO | T2 | FSM-1.1 | raw `entity.status = x` write | routed through FSM assert |
| No `dangerouslySetInnerHTML` without sanitization | react-nextjs Security | NO | T1 | TSX-XSS-1.1 | `dangerouslySetInnerHTML={{__html:user}}` | DOMPurify-sanitized |
| Never store tokens in `localStorage` | react-nextjs Security | NO | T1 | TSX-TOKEN-1.1 | `localStorage.setItem('token',…)` | httpOnly cookie |
| No `useEffect`+`useState` for data fetching; use TanStack | react-nextjs Data Fetching; CLAUDE | NO | T2 | TSX-FETCH-1.1 | `useEffect(()=>{fetch()})` | `useQuery` |
| Every `useEffect` has `// WHY:` comment | react-nextjs; CLAUDE | NO | T1 | TSX-EFFECT-1.1 | `useEffect` w/o WHY comment | `// WHY:` present |
| No secrets under `NEXT_PUBLIC_`; no server secret to client | react-nextjs Security | NO | T1 | TSX-ENVPUB-1.1 | `NEXT_PUBLIC_API_SECRET` | server-only env |
| No untyped `Record<string,unknown>`/`{[k]:any}` between components/steps | react-nextjs TS hard rule | PARTIAL (TS Any family; verify Record<string,unknown>) | T2 | TSX-UNTYPED-1.1 | `onSubmit(data: Record<string,unknown>)` | Zod-inferred type |
| Use const objects/unions not TS `enum` | react-nextjs TS | NO | T3 (stylistic) | TSX-ENUM-1.3 | `enum Foo{}` | `as const` object |
| `app/` pages < 50 lines, no business logic | react-nextjs Layer | NO | T2 | TSX-PAGE-1.1 | page.tsx 120 lines w/ logic | thin orchestrator |
| Component file ≤150, JSX ≤80 lines | react-nextjs Component Patterns | NO | T1 | TSX-COMP-1.1 | 200-line component | 140-line component |
| **All Flutter/Dart rules — no `dart` language in registry** | flutter.md (entire) | NO | — | see rows below | — | — |
| Domain layer pure Dart — no `package:flutter` imports | flutter Layer Rules; CLAUDE | NO | T1 | DART-DOMAIN-1.1 | `import 'package:flutter/…'` in domain | no flutter import |
| No `!` bang operator without justification | flutter Typing | NO | T2 | DART-BANG-1.1 | `value!` unjustified | null-checked access |
| `@freezed` for entities; no mutable classes / hand-rolled copyWith | flutter Typing; CLAUDE | NO | T2 | DART-FREEZED-1.1 | mutable entity class | `@freezed` model |
| No legacy Riverpod 1.x (`StateNotifier`/`ChangeNotifierProvider`) | flutter Riverpod | NO | T1 | DART-RIVERPOD-1.1 | `StateNotifierProvider` | `NotifierProvider` |
| No `setState` for server data; no `ChangeNotifier` new code | flutter State Mgmt; CLAUDE | NO | T2 | DART-STATE-1.1 | `setState` storing fetch result | Riverpod/Bloc |
| No data fetch in `initState` | flutter State Mgmt | NO | T2 | DART-INITSTATE-1.1 | `initState(){fetch();}` | provider-backed |
| No raw-string state comparisons anywhere | flutter Enums; CLAUDE | PARTIAL (magic-string rule, not dart) | T2 | DART-ENUMCMP-1.1 | `if(status=='received')` | enum comparison |
| Enum parsing no silent fallback (`orElse:()=>default`) | flutter Enum Parsing | NO | T1 | DART-ENUMPARSE-1.1 | `orElse:()=>Type.item` | throw / nullable |
| No silent `?? 0`/`?? ''` on required fields | flutter No Silent Fallbacks | NO | T2 | DART-FALLBACK-1.1 | `quantity ?? 0` on required | validate-then-construct |
| No `Map<String,Object?>` form/wizard data | flutter Form Typing | NO | T2 | DART-FORMMAP-1.1 | `onSubmit(Map<String,Object?>)` | typed `@freezed` class |
| No hardcoded color literals in widgets | flutter Accessibility | NO | T1 | DART-COLOR-1.1 | `Color(0xFF2E7D32)` in widget | `Theme.of(context)` token |
| No `print`/`debugPrint` unguarded in production | flutter Debug Logging; CLAUDE | PARTIAL (print keyword hits; not dart-scoped/kDebugMode) | T1 | DART-PRINT-1.1 | bare `print(x)` | `if(kDebugMode)` guard |
| Tokens in `flutter_secure_storage`, not `SharedPreferences` | flutter Security | NO | T1 | DART-SECSTORE-1.1 | `prefs.setString('token',…)` | `FlutterSecureStorage().write` |
| No `!` on `state.pathParameters['id']`; 404 fallback route required | flutter Navigation | NO | T2 | DART-ROUTE-1.1 | `pathParameters['id']!` | `tryParse(...)??` + errorBuilder |
| Never edit generated files (`.g.dart`/`.freezed.dart`) | flutter Tooling | NO | T1 | DART-GEN-1.1 | edit to `.freezed.dart` | untouched |
| **All ColdFusion/CFML rules — no `coldfusion` language in registry** | coldfusion.md (entire) | NO | — | see rows below | — | — |
| `var`/`local`-scope every function-local var (CFLint MISSING_VAR) | coldfusion GR#1 | NO | T1 | CF-VARSCOPE-1.1 | unscoped `x = …` in function | `var x = …` |
| Every dynamic SQL value via `cfqueryparam`/bound param | coldfusion GR#2; Security | PARTIAL (sql-fragment rule; not CFML cfqueryparam) | T1 | CF-QPARAM-1.1 | `WHERE id=#url.id#` | `cfqueryparam`/`:id` bind |
| Layered one-way deps; handlers/services run no queries | coldfusion GR#3 | NO | T2 | CF-LAYER-1.1 | `<cfquery>` in handler/service | query only in gateway |
| Inject via WireBox; no `createObject`/`new` for services | coldfusion GR#4 | NO | T2 | CF-DI-1.1 | `createObject("JobService")` | `inject="JobService@app"` |
| Singletons stateless — no request data in `variables` | coldfusion GR#5 | NO | T3 (semantic, hard to mechanize) | CF-STATELESS-1.3 | singleton stores req data in `variables` | flows via args |
| Typed boundaries: `returntype`+typed `required` args, `access=private` default | coldfusion GR#7 | NO | T2 | CF-TYPES-1.1 | public fn no returntype | typed signature |
| No silent swallow (`catch(any e){}`); throw typed errors | coldfusion GR#8 | NO | T1 | CF-CATCH-1.1 | empty `catch(any e){}` | log/rethrow typed |
| Encode all output (`encodeForHTML` etc.); no raw `<cfoutput>#raw#</cfoutput>` | coldfusion Security | NO | T1 | CF-XSS-1.1 | `<cfoutput>#user#</cfoutput>` | `encodeForHTML(user)` |
| No `evaluate()`/`iif()` on input (code injection) | coldfusion Security | NO | T1 | CF-EVAL-1.1 | `evaluate(form.x)` | direct expr |
| Production `robustEnabled=false`; no `<cfdump>`/debug to users | coldfusion Security | NO | T1 | CF-INFO-1.1 | `robustEnabled=true` in prod Application.cfc | `false` |
| **Cross-cutting (all agents):** every public fn/method gets 1 happy + 1 error test in SAME task; companion test file must exist | rust-cli GR#7; rust-mcp; python-fastapi (check_test_companion_exists); flutter; coldfusion; CLAUDE HR#7 | NO (TEST family exists but no companion-existence / happy+error-per-method check) | T1 | TEST-COMPANION-1.1 | new `services/foo.py` w/o `test_foo.py` | companion present w/ happy+error |
| Assert on error variant/type, not message strings | rust-cli; coldfusion Testing | NO | T2 | TEST-VARIANT-1.1 | `assert err.message == "…"` | `matches!(e, Err(X))` |
| Test behavior not implementation; query by role/text not test-id/class | react-nextjs; flutter Testing | NO | T3 (heuristic) | TEST-BEHAVIOR-1.3 | `getByTestId` / assert on `Column` | `getByRole`/`find.text` |
| Every FSM: test all valid AND invalid transitions | python-fastapi; flutter; coldfusion Testing | NO | T2 | TEST-FSM-1.1 | FSM w/o invalid-transition test | invalid transitions asserted |
| `THREAT_MODEL.md` read/honored before implementing sensitive features | all agents Security; THREAT_MODEL.template | NO | T3 (process, not code-mechanical) | THREAT-READ-1.3 | new sensitive endpoint, no threat-model ref | listed in THREAT_MODEL |
| Stale `///`/doc comment updated in same edit as behavior change | rust-cli/mcp Do/Don't | PARTIAL (stale-doc keyword exists, MCP/registry-scoped only) | T3 | DOC-STALE-1.3 | doc says old behavior post-change | doc matches code |
