# e-pack-dart Dart And Flutter Language Pack

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Dart And Flutter Language Pack`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling completion, product status, PR readiness, or broad DONE.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `rules/dart/**.md, src/validators/dart-*.ts, tests/fixtures/dart/**`
- deps: `d01-rule-mechanization-engine, d16-fsm-transition-validity, d22-size-shape-caps`
- tier: `P0/P1`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md), [ADBP_GAPS](../ADBP_GAPS.md).

## Where We Are

The rule registry (`rules/rules.json`) has language entries only for `common`, `python`, `rust`, and `typescript`. There is **zero Dart**: no `dart` language key, no `.dart` extension in any rule's `appliesTo`, no Dart fixtures, and no `src/validators/dart-*.ts` validators. Separately, the Rust literal-scan model in `Tools/ocentra-literal-scan` has a language registry that likewise has **no Dart entry** — `.dart` sources are invisible to the literal-scan lane. Every Dart/Flutter rule from the ADBP `rules-flutter` gap rows (ADBP_GAPS.md Group 2, rows for `DART-ARCH-*`, `DART-TYPE-*`, `DART-SEC-*`, `DART-STATE-*`, `DART-COMP-*`/`DART-PERF-*`, `DART-ERR-*`/`DART-NAV-*`/`DART-L10N-*`, `DART-FALLBACK-*`, `DART-TOOL-*`/`DART-DEP-*`/`DART-GEN-*`/`DART-CI-*`, `DART-IMP-*`/`DART-NAME-*`) is therefore a greenfield gap.

## Where We Want To Be

Dart is a first-class language in the enforcer. Concretely:

1. `dart` is registered as a language (`.dart` `appliesTo`) in `rules/rules.json` via the d01 mechanization engine, so every rule below mints a branded ruleId, a doc anchor, a fail-fixture, a pass-fixture, and a detection test, and passes the d01 5-way parity oracle (ruleId <-> doc <-> validator <-> {fail-fixture + pass-fixture} <-> detection-test).
2. Dart is added to the `Tools/ocentra-literal-scan` language registry so `.dart` files feed the T2 literal-scan scored lane. **Cross-ref e-pack-cfml**, which carries the same registry-add note for CFML — both new languages must be added to that registry, and the two packs must not both edit the same registry file without coordinating (see Parallel Ownership Notes).
3. Dart validators live in `src/validators/dart-*.ts`, one file per rule family, each an AST/analyzer-shaped or regex-shaped visitor that emits registry-backed findings.
4. Size caps for Dart (file <=200, build() <=80, method <=30, <=5 params, line <=120) are provided by **d22** with a Dart per-language limit row; FSM/enum semantics (enum-mandatory, transition map, validate-before-mutate, no silent enum fallback, transition-coverage tests) are provided by **d16**. This pack consumes those engines and only adds the Dart appliesTo + Dart fixtures for them — it does not re-implement the size or FSM mechanism.

## Requirement Checklist

Each row names: fail-fixture (must be flagged) + pass-fixture (must stay clean) + detection test. All fixtures live under `tests/fixtures/dart/<family>/{fail,pass}.dart`; detection tests are the d01-generated parity/detection tests per ruleId unless noted.

T1 deterministic (blocking):

- [ ] `dart` language + `.dart` appliesTo registered in `rules/rules.json` (via d01); Dart added to `Tools/ocentra-literal-scan` language registry.
- [ ] **DART-ARCH-1.1..1.4 / DART-DOMAIN-1.1** layer/import boundaries — feature<->feature import ban; data never imports presentation; presentation never imports data (go via domain); domain is pure Dart (no `package:flutter`). fail: `data/order_repo.dart` with `import '../presentation/order_page.dart';` and a domain file with `import 'package:flutter/material.dart';`. pass: data imports only `domain/`/`core/`; domain file with no Flutter import.
- [ ] **DART-BANG-1.1** no unchecked null-assertion `!` / unguarded `as`. fail: `int.parse(state.pathParameters['id']!)` and `final o = x as Order;` with no preceding `is`. pass: `int.tryParse(...)` with guard; `if (x is Order) { final o = x; }`.
- [ ] **DART-TYPE-1.1..1.6** typed DTOs — no `dynamic`, no `Map<String,dynamic>` DTO/return, typed public signatures. fail: `Map<String,dynamic> parse(dynamic json)`. pass: typed nested DTO class with typed fields.
- [ ] **DART-FREEZED-1.1** immutable entities via `@freezed`. fail: entity class with mutable non-final fields and setters. pass: `@freezed`-annotated class.
- [ ] **DART-SEC-1.1..1.4** security — no hardcoded API key/token literal; tokens/PII to `flutter_secure_storage`, not `SharedPreferences`; HTTPS only; never disable SSL (`badCertificateCallback=(_,__,___)=>true`). fail fixtures: `prefs.setString('auth_token', token)`, `Uri.parse('http://api...')`, `badCertificateCallback = (c,h,p) => true`, `const apiKey = 'sk-live-...'`. pass: `FlutterSecureStorage().write(...)`, `https://`, no override, `String.fromEnvironment('API_KEY')`.
- [ ] **DART-STATE-1.1 (no ChangeNotifier in new code) / DART-RIVERPOD-1.1 (Riverpod 2.x NotifierProvider, ban legacy 1.x StateNotifierProvider) / DART-STATE-1.2 (no `ref.read` in `build()`, use `ref.watch`)**. fail: `class Foo extends ChangeNotifier`; `StateNotifierProvider<...>`; `ref.read(p)` inside `build()`. pass: `NotifierProvider`; `ref.watch(p)` in `build()`.
- [ ] **DART-TYPE-1.7 / DART-ENUMPARSE-1.1** enum parse — no silent `orElse`/`?? default` fallback to a variant. fail: `values.firstWhere(..., orElse: () => Status.pending)`. pass: nullable `tryFromString` / throw. (Mechanism provided by **d16**; this pack supplies the Dart fixtures + appliesTo.)
- [ ] **DART-COMP-1.1 / DART-COMP-1.2** one public widget per file; `{super.key}` first constructor param. fail: two public widgets in one file; `const OrderCard(this.order)` with no `super.key`. pass: single widget; `const OrderCard({super.key, required this.order})`.
- [ ] **DART-PERF-1.1** `ListView.builder` for long/dynamic lists (ban `ListView(children: [...spread])` over a mapped collection). fail: `ListView(children: items.map(...).toList())`. pass: `ListView.builder(itemCount: ..., itemBuilder: ...)`.
- [ ] **DART-PERF-2.1** no `setState(` call inside `build()`. fail: `setState` invoked in `build`. pass: `setState` only in event handlers.
- [ ] **DART-NAME-1.1** snake_case filename matches widget (`order_card.dart` for `OrderCard`). fail: `OrderCard.dart`. pass: `order_card.dart`.
- [ ] **DART-TOOL-1.1..1.3** toolchain gates — `analysis_options.yaml` present + strict; CI runs `dart analyze --fatal-infos` and `dart format --set-exit-if-changed`. fail-fixture: project fixture missing `analysis_options.yaml` / CI without analyze+format step. pass: strict config + CI steps present.
- [ ] **DART-DEP-1.1 / DART-GEN-1.1** `pubspec.lock` committed + pinned `^` deps (ban `foo: any`); never hand-edit generated `.g.dart`/`.freezed.dart` (diff to a generated file with no regen marker). fail: `foo: any`; a hand-edited `.g.dart`. pass: `foo: ^1.2.0`; untouched generated file.
- [ ] **DART-SIZE-2.* (via d22)** file <=200, build() <=80, method <=30, <=5 params, line <=120 for Dart. fail: 210-line file / 90-line build() / 6-param widget ctor. pass: within caps. (Mechanism provided by **d22** Dart per-language limit row; this pack supplies Dart fixtures.)

T2 scored / advisory (non-blocking; fixture asserts score crosses vs stays under threshold, the literal-scan model):

- [ ] **DART-STATE-1.3** detail page mutates a list provider (scored). fail: detail widget calling `ref.read(listProvider.notifier).update(...)`. pass: emits an event / navigates back with result.
- [ ] **DART-FALLBACK-1.1 / DART-STYLE-2.2** no `?? 0` / `?? ''` on a required field; `Dio response.data!` needs a justifying comment (scored). fail: `quantity: json['qty'] ?? 0`; `response.data!` bare. pass: validate-then-construct; `// data guaranteed non-null by ...` above `response.data!`.
- [ ] **DART-FORMMAP-1.1** wizard/form state as `Map<String,Object?>` (scored). fail: `Map<String,Object?> formData`. pass: typed form-state class.
- [ ] **DART-ERR-1.1 / DART-ERR-2.1** typed sealed `Failure` hierarchy, not raw `throw Exception('msg')`; never render exception to the user (scored). fail: `throw Exception('boom')`; `Text('$error')`. pass: sealed `ServerFailure`; generic user message.
- [ ] **DART-SEC-1.5 (print) / DART-SEC-1.6 (kDebugMode-guarded debug)** — bare `print(x)` for diagnostics; unguarded debug output (scored). fail: `print(payload)`; debug block with no `kDebugMode` guard. pass: monitoring logger; `if (kDebugMode) { ... }`.
- [ ] **DART-COLOR-1.1** hardcoded color literal `Color(0xFF...)` in widget (scored). fail: `Color(0xFF00FF00)` in build. pass: `Theme.of(context).colorScheme.primary`.
- [ ] **DART-NAV-2.*** imperative `Navigator.push` instead of declarative GoRouter; hardcoded path string (scored). fail: `Navigator.push(context, MaterialPageRoute(...))`. pass: `context.go('/orders')` with named route.
- [ ] **DART-L10N-2.1** hardcoded user-facing string literal instead of l10n (scored). fail: `Text('Submit order')`. pass: `Text(l10n.submitOrder)`.
- [ ] **DART-STYLE-2.1** string interpolation not concatenation (scored). fail: `'Hello ' + name`. pass: `'Hello $name'`.
- [ ] **DART-INITSTATE-1.1** data fetch in `initState` (scored). fail: `initState(){ fetch().then((d)=>setState(...)); }`. pass: provider/`FutureBuilder`.
- [ ] **DART-IMP-1.1** ungrouped imports (dart -> package -> relative) (scored). fail: interleaved import groups. pass: grouped ordering.
- [ ] **DART-SIZE-CX (via d22)** cyclomatic complexity < 10 / nesting <= 3 for Dart methods (scored). fail: 12-branch method. pass: guard clauses. (Mechanism from **d22**; Dart fixtures here.)
- [ ] **DART-TEST-3.1 (via d16)** every FSM transition tested, valid AND invalid (scored transition coverage). fail: enum-backed FSM with no invalid-transition test. pass: test asserts the illegal transition throws. (Mechanism from **d16**.)

T3 advisory (no mechanization possible — label required; label presence itself is T1-enforced):

- [ ] **DART-NAME-3.1** boolean identifier prefix `is/has/can/should` — `advisory, no mechanization possible + a correct boolean can carry a non-prefixed domain name (e.g. `active`) and prefix intent is not decidable from the token`. Doc row must carry the label; d01 enforces the label's presence.
- [ ] **DART-IMP-2.1** `part`/`part of` reserved for codegen only — `advisory, no mechanization possible + distinguishing an intentional hand-authored part from a codegen part requires design intent not present in source`. Labeled; label presence T1-enforced.
- [ ] **DART-STATE-2.* Bloc conventions** (`@freezed` states, past-tense events, no Bloc->Bloc) — `advisory, no mechanization possible + past-tense event naming and cross-Bloc call intent are design-judgment, not statically decidable`. Labeled; label presence T1-enforced.

## Acceptance And Proof

Tier P0/P1. DONE only when, per TEST_PROOF_EXPECTATIONS.md, every T1 ruleId above has a green d01 parity + detection test (fail-fixture trips, pass-fixture stays clean), every T2 ruleId has a green score-threshold test (fail crosses, pass stays under), and every T3 row is present with its `advisory, no mechanization possible + <reason>` label whose presence is verified by the d01 label check. Additionally: a test proves `dart`/`.dart` is registered in `rules/rules.json` and that `.dart` files are picked up by the `Tools/ocentra-literal-scan` registry. Roughly ~30 T1 + ~30 T2 ruleIds + the 3 labeled T3 rows above.

## Parallel Ownership Notes

Owns `rules/dart/**.md`, `src/validators/dart-*.ts`, and `tests/fixtures/dart/**` exclusively — disjoint from all siblings. Depends on d01 (registry rows + 5-way parity), d16 (FSM/enum semantics — this pack only adds Dart appliesTo + Dart fixtures, never the FSM mechanism), and d22 (size/complexity caps — Dart per-language limits + fixtures only). **Shared edit hazard:** both this pack and e-pack-cfml must add a new language to the single `Tools/ocentra-literal-scan` language registry. Treat that registry file as an append-only additive edit (add the `dart` entry only), coordinate ordering with e-pack-cfml, and keep each pack's fixtures/validators otherwise disjoint. This pack never edits Python/Rust/TypeScript rules, docs, or fixtures.
