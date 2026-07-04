# Parity Gaps — rules-flutter (Dart/Flutter)

Registry has NO Dart language entry (`languages: rust, typescript, python, common`) and **zero `.dart` in any `appliesTo`**. Every Flutter rule below is therefore a gap. "PARTIAL" = a concept-analog family exists for other languages (`source-shape`, `security`, `tests`, `source`) but has no Dart validator/appliesTo; "NO" = Dart-specific with no analog.

| ADBP point | ADBP source | Backed? (family or NO/PARTIAL) | Tier | Proposed ruleId | Fail-fixture | Pass-fixture |
|---|---|---|---|---|---|---|
| Feature MUST NOT import from another feature's dir | architecture.md#feature-module-rules | PARTIAL (source-shape) | T1 | DART-ARCH-1.1 | `features/a/x.dart` imports `package:app/features/b/...` | cross-feature type moved to `shared/`/`core/` |
| Data layer MUST NEVER import from presentation | architecture.md#circular-import-prevention | PARTIAL (source-shape circular) | T1 | DART-ARCH-1.2 | `data/auth_service.dart` imports `../presentation/...` | data imports only `domain/`/`core/` |
| Presentation never imports `data/` directly (go via domain) | architecture.md#presentation-layer | PARTIAL (source-shape) | T1 | DART-ARCH-1.3 | page imports `../data/...` | page imports `../domain/...` |
| Domain layer = pure Dart, no Flutter/package imports | architecture.md#domain-layer; typing-style#domain-enums | PARTIAL (source-shape domain-cannot-import-UI) | T1 | DART-ARCH-1.4 | `domain/enums/x.dart` imports `package:flutter/material.dart` | domain file imports only `dart:`/pure dart |
| Every navigation target MUST have a route definition | architecture.md#route-completeness | NO | T2 | DART-ARCH-2.1 | `context.push('/x/$id')` with no matching `GoRoute` | route defined for pushed path |
| Never unguarded `!` / `int.parse(...!)` on path params | architecture.md#path-parameter-safety | NO | T1 | DART-ARCH-1.5 | `int.parse(state.pathParameters['id']!)` | `int.tryParse` with null-guard fallback |
| Widget file SHOULD NOT exceed 200 lines | components.md#widget-structure; tooling#module-size | PARTIAL (source-shape shape-limits) | T2 | DART-SIZE-2.1 | `.dart` widget file > 200 lines | file <= 200 lines |
| `build()` SHOULD NOT exceed 80 lines | components.md#widget-structure | NO | T2 | DART-SIZE-2.2 | `build()` body > 80 lines | build() <= 80 lines |
| One public widget per file | components.md#widget-structure | PARTIAL (source-shape) | T1 | DART-COMP-1.1 | two public `extends *Widget` classes in one file | single public widget |
| File name matches widget (snake_case) | components.md; typing-style#naming | NO | T1 | DART-NAME-1.1 | `OrderCard.dart` / mismatched name | `order_card.dart` exports `OrderCard` |
| Widget nesting SHOULD NOT exceed 5 levels | components.md#composition | NO | T2 | DART-COMP-2.1 | widget tree nested > 5 | nesting <= 5 |
| Never use `StatefulWidget` for server data | components.md#widget-types; state-patterns#rules | NO | T2 | DART-STATE-2.1 | `StatefulWidget` with `http.get`/repository in `setState` | server data via Riverpod/Bloc |
| Always use `const` constructors when possible | components.md#constructor; typing-style#immutability | NO | T2 | DART-STYLE-2.1 | constructable-const widget without `const` | `const` constructor used |
| Always include `{super.key}` first param | components.md#constructor | NO | T1 | DART-COMP-1.2 | widget ctor missing `super.key` | `const X({super.key, ...})` |
| Never `UniqueKey` unless forcing rebuild | components.md#keys | NO | T2 | DART-COMP-2.2 | `UniqueKey()` in list item builder | `ValueKey(id)` used |
| `ListView.builder` for long/dynamic lists (not `ListView(children:[])`) | components.md#performance | NO | T2 | DART-PERF-2.1 | `ListView(children: list.map(...))` | `ListView.builder` |
| Never call `setState`/rebuild from `build()` | components.md#performance | NO | T1 | DART-PERF-1.1 | `setState(` inside `build()` | no setState in build |
| Typed `Failure` hierarchy; never throw raw `Exception('msg')` | patterns.md#error-handling; typing-style#error-handling | PARTIAL (source-shape typed-errors) | T1 | DART-ERR-1.1 | `throw Exception('...')` | `throw ServerFailure(...)` sealed type |
| Never show raw exceptions/stack traces to users | patterns.md#error-handling | NO | T2 | DART-ERR-2.1 | exception `.toString()` rendered in `Text(...)` | mapped user-friendly message |
| Log errors to monitoring, never just `print` | patterns.md#error-handling | PARTIAL (analog print rules other langs) | T2 | DART-ERR-2.2 | `catch(e){ print(e); }` | Sentry/Logger call |
| Use declarative routing, not imperative `Navigator.push` | patterns.md#navigation | NO | T2 | DART-NAV-2.1 | `Navigator.push(context, ...)` | GoRouter `context.go/push` |
| Never hardcode path strings in multiple places | patterns.md#navigation | NO | T2 | DART-NAV-2.2 | literal `'/orders/...'` duplicated | route name constant referenced |
| Always paginate list endpoints (no unbounded loads) | patterns.md#pagination | NO | T3 (labeled: intent) | DART-PAG-3.1 | data source returns full list no page params | paginated fetch |
| Never hardcode user-visible strings (use l10n) | patterns.md#localization | NO | T2 | DART-L10N-2.1 | string literal in `Text('Submit')` | `context.l10n.submit` |
| Never hardcode API URLs/keys/env values | patterns.md#config; security.md#sensitive-data | PARTIAL (security inline-secrets, no Dart) | T1 | DART-SEC-1.1 | `const apiKey='sk-...'` / `Uri.parse('https://api...')` literal | `String.fromEnvironment(...)` |
| Tokens/PII MUST use `flutter_secure_storage`, not `SharedPreferences` | patterns.md#security; security.md#secure-storage; state-patterns#persistence | NO | T1 | DART-SEC-1.2 | `prefs.setString('auth_token', token)` | `FlutterSecureStorage().write(...)` |
| Always HTTPS, never HTTP for API calls | patterns.md#security; security.md#network | NO | T1 | DART-SEC-1.3 | `http.get(Uri.parse('http://...'))` | `https://` uri |
| Never disable SSL cert validation | security.md#network | NO | T1 | DART-SEC-1.4 | `badCertificateCallback = (c,h,p)=>true` | no override / proper pinning |
| Validate data received from platform channels | security.md#platform-channels | NO | T3 (labeled: intent) | DART-SEC-3.1 | platform channel result used unvalidated | validated before use |
| Debug code guarded with `kDebugMode`; never `print()` in prod | security.md#debug-code; tooling#debug-logging | NO | T1 | DART-SEC-1.5 | bare `print(...)`/`debugPrint(...)` outside `kDebugMode` | wrapped in `if (kDebugMode)` |
| Never log sensitive data (tokens/passwords/PII) | security.md; tooling#sensitive-data-in-logs | NO | T1 | DART-SEC-1.6 | `debugPrint('token: $token')` | no sensitive interpolation in log |
| Release build obfuscation enabled (`--obfuscate --split-debug-info`) | security.md#obfuscation | NO | T3 (labeled: build-flag) | DART-SEC-3.2 | CI build cmd lacks `--obfuscate` | build with obfuscate flags |
| `.env` excluded from version control | security.md#sensitive-data | PARTIAL (sensitive-files-forbidden, no Dart) | T1 | DART-SEC-1.7 | `.env` tracked in git | `.env` in `.gitignore` |
| Never store server data in `setState` | state-patterns.md#rules | NO | T2 | DART-STATE-2.2 | server fetch stored via `setState` | provider/Bloc holds it |
| Never fetch in `initState` with manual `setState` | state-patterns.md#rules | NO | T1 | DART-STATE-1.1 | `initState(){ fetch().then((d)=>setState()); }` | provider/FutureBuilder |
| Never use `ChangeNotifier` for new code | state-patterns.md#rules; provider-types | NO | T2 | DART-STATE-2.3 | `extends ChangeNotifier` | Notifier/Bloc |
| Never legacy `StateNotifierProvider`/`StateProvider`(complex)/`ChangeNotifierProvider` | state-patterns.md#riverpod-2x | NO | T1 | DART-STATE-1.2 | `StateNotifierProvider<...>` | `NotifierProvider`/`AsyncNotifierProvider` |
| `ref.watch` in `build()`, never `ref.read` there | state-patterns.md#conventions | NO | T1 | DART-STATE-1.3 | `ref.read(...)` in `build()` | `ref.watch(...)` in build |
| Detail pages MUST NOT mutate list providers directly | state-patterns.md#provider-mutation-boundaries | NO | T2 | DART-STATE-2.4 | detail page calls `ordersListProvider.notifier).removeOrder` | own provider + invalidate |
| ANY flow with 3+ states & constrained transitions MUST use FSM | state-patterns.md#finite-state-machines | NO | T3 (labeled: design-intent) | DART-FSM-3.1 | multi-step flow tracked by bare bool flags | freezed sealed union FSM |
| Bloc states use `freezed` sealed classes | state-patterns.md#bloc; typing-style#immutability | NO | T2 | DART-STATE-2.5 | Bloc state plain class, no `@freezed`/sealed | `@freezed sealed class` state |
| Bloc events past-tense, not imperative | state-patterns.md#bloc-conventions | NO | T3 (labeled: naming intent) | DART-STATE-3.1 | event `CreateOrder` (imperative) | `OrderCreated` |
| Blocs never call other Blocs | state-patterns.md#bloc-conventions | NO | T2 | DART-STATE-2.6 | Bloc referencing another Bloc instance | shared use case/repo |
| Never persist sensitive data unencrypted | state-patterns.md#persistence | NO | T1 | DART-SEC-1.8 | hydrated_bloc persisting token to prefs | secure_storage for sensitive |
| Test names read as `should ... when ...` specs | testing.md#test-naming | NO | T2 | DART-TEST-2.1 | `test('test 1', ...)` | `test('should ... when ...')` |
| FSM: every valid AND invalid transition tested | testing.md#coverage; tests-you-must-not-skip | NO | T3 (labeled: coverage-intent) | DART-TEST-3.1 | FSM with untested invalid transition | invalid-transition test present |
| Widget tests find by text/type/semantics, not `Key` | testing.md#widget-testing-principles | NO | T2 | DART-TEST-2.2 | `find.byKey(...)` as primary finder | `find.text`/`find.byType` |
| Never assert widget tree structure (Column contains Text) | testing.md#widget-testing-principles | NO | T2 | DART-TEST-2.3 | assert `Column` contains `Text` | assert visible text/state |
| Every use case: >=1 happy + >=1 error path test | testing.md#coverage | PARTIAL (tests family other langs) | T2 | DART-TEST-2.4 | use case with only happy test | happy + error tests |
| Test dir mirrors `lib/` structure | testing.md#organization | NO | T2 | DART-TEST-2.5 | `lib/features/orders/domain/x.dart` w/ test elsewhere | `test/features/orders/domain/x_test.dart` |
| `dart analyze --fatal-infos` must pass | tooling.md#code-quality | PARTIAL (toolchain other langs) | T1 | DART-TOOL-1.1 | analyze emits info/warning | clean analyze |
| `dart format --set-exit-if-changed` must pass | tooling.md#code-quality | PARTIAL (toolchain fmt other langs) | T1 | DART-TOOL-1.2 | unformatted `.dart` | formatted |
| `analysis_options.yaml` present w/ strict-casts/inference/raw-types + lint set | tooling.md#analysis-options | NO | T1 | DART-TOOL-1.3 | missing/weak `analysis_options.yaml` | strict analyzer config present |
| `pubspec.lock` committed | tooling.md#package-manager | PARTIAL (dependencies lockfile other langs) | T1 | DART-DEP-1.1 | `pubspec.lock` gitignored/absent | lockfile committed |
| Dependency versions pinned with `^`, no unconstrained ranges | tooling.md#package-manager | PARTIAL (dependencies) | T1 | DART-DEP-1.2 | `foo: any` in pubspec | `foo: ^1.2.0` |
| NEVER edit generated `.g.dart`/`.freezed.dart` files | tooling.md#code-generation | PARTIAL (generated-artifacts other langs) | T1 | DART-GEN-1.1 | hand-edited `.g.dart` (no generator header) | generated file untouched |
| CI must run analyze + format + test on every PR | tooling.md#build-deployment | PARTIAL (ci family) | T1 | DART-CI-1.1 | CI yaml lacks `dart analyze`/`flutter test` | CI runs all three |
| Dart file SHOULD NOT exceed 200 lines | tooling.md#module-size | PARTIAL (source-shape) | T2 | DART-SIZE-2.3 | `.dart` file > 200 lines | <= 200 |
| Function SHOULD NOT exceed 30 lines | tooling.md#module-size | NO | T2 | DART-SIZE-2.4 | function > 30 lines | <= 30 |
| Class SHOULD NOT exceed 150 lines | tooling.md#module-size | NO | T2 | DART-SIZE-2.5 | class > 150 lines | <= 150 |
| Max function parameters: 5 | tooling.md#module-size | NO | T1 | DART-SIZE-1.1 | function with 6+ params | <= 5 params / params class |
| Cyclomatic complexity per function < 10 | tooling.md#module-size | NO | T2 | DART-SIZE-2.6 | function complexity >= 10 | < 10 |
| Dio `response.data!` requires one-time justification comment | tooling.md#dio-response-handling | NO | T2 | DART-STYLE-2.2 | `response.data!` with no justifying comment | comment above `!` usage |
| All params/return types explicitly typed (public APIs) | typing-style.md#type-annotations | PARTIAL (source typed-signatures other langs) | T1 | DART-TYPE-1.1 | `getOrders({filter}) async {}` | fully typed signature |
| Never use `dynamic` (unless commented) | typing-style.md#type-annotations | PARTIAL (source no-any/dynamic other langs) | T1 | DART-TYPE-1.2 | uncommented `dynamic x` | `Object`/generics |
| Never `Map<String,dynamic>` for nested DTO objects | typing-style.md#dto-nested-objects | NO | T1 | DART-TYPE-1.3 | freezed DTO field `Map<String,dynamic>? location` | typed nested `LocationDto?` |
| Never `!` without preceding null check/guarantee | typing-style.md#null-safety | NO | T1 | DART-TYPE-1.4 | `user!.name` no prior guard | `user?.name ?? default` |
| Never `as` downcast without `is` check first | typing-style.md#null-safety | PARTIAL (source unsafe-cast other langs) | T1 | DART-TYPE-1.5 | `value as String` unchecked | `if (value is String)` |
| Domain entities/value objects MUST be immutable (`@freezed`/final+const) | typing-style.md#immutability | PARTIAL (source-shape/immutability other langs) | T1 | DART-TYPE-1.6 | domain entity with mutable fields | `@freezed`/final+const ctor |
| Enum parsing: no silent `orElse` fallback to default | typing-style.md#enum-parsing | NO | T1 | DART-TYPE-1.7 | `firstWhere(..., orElse: ()=>X.item)` | throw / nullable `tryFromString` |
| Never raw strings for statuses/roles/types (use enums) | typing-style.md#enums | NO | T2 | DART-TYPE-2.1 | status compared as `'pending'` string | enum `OrderStatus.pending` |
| Naming: snake_case files, PascalCase types, etc. | typing-style.md#naming | NO | T1 | DART-NAME-1.2 | `OrderCard.dart` file | `order_card.dart` |
| Boolean vars/params use `is/has/can/should` prefix | typing-style.md#naming | T3 (labeled: naming intent) | T3 | DART-NAME-3.1 | `bool compact` param | `bool isCompact` |
| Imports grouped dart -> package -> relative | typing-style.md#imports; tooling `directives_ordering` | PARTIAL (imports-modules other langs) | T1 | DART-IMP-1.1 | ungrouped/unordered imports | grouped & ordered |
| `part`/`part of` ONLY for codegen, never manual splitting | typing-style.md#imports | NO | T2 | DART-IMP-2.1 | manual `part 'helpers.dart'` (non-generated) | part only for `.g/.freezed` |
| String interpolation, never concatenation for display | typing-style.md#string-formatting | NO | T2 | DART-STYLE-2.3 | `'Order ' + id` | `'Order $id'` |
