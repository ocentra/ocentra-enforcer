# Wave G3 — Rich-tier passes + full parity re-verification

**Outcome:** the 34 Tier-3 languages get their richer edges through the generic engine, and
the whole language set is live-verified against the C binary.

## Rich-tier generalization (mirror baseline's layered passes)
- **Inheritance/implements** — generalize `extract_base_classes` (baseline extract_defs.c:2373):
  generic heritage-field fallback (`superclass`, `superinterfaces`, `class_heritage`,
  `implements_clause`, `base_list`, `base_class_clause`) + dedicated quirk walkers for the
  ones the baseline special-cases (TS/PHP/Kotlin/Squirrel/Julia/F#/D/PowerShell/Pascal/C++/C#).
- **Deep type-refs** — extend beyond base param/return types for the baseline's deep set
  (Go/TS/TSX/Java/Python/Rust); wire the generic base-type extraction for all others.
- **Decorators** — populate `decorator_types` rows (TS/TSX and any lang with decorator nodes).
- **Routes** — port `service_patterns.c:315-377` library-name matching so route edges appear
  for any onboarded language whose imports name a known web framework.

## Full parity re-verification
Extend `tests/feature_parity` to index a multi-language corpus in BOTH the C binary and our
crate, comparing per-language: defs found, call edges, imports, complexity query, and (Tier-3)
inherits/type-refs/routes. Regenerate `proof/memory/x06-kg-parity.json` with per-language rows.
`worse`/`unrunnable` = failures to fix unless the baseline itself errors (document with evidence).
Deferred grammars (G2 step B.3) are recorded as `pending`, not `worse`.

## Scoping (done 2026-07-09, before spawning any workers)

**First pass (superseded below): wrong.** An initial grep-based estimate ("33
`out.inherits.push` call sites across ~10 hand-migrated languages, ~20+ Tier-3 languages are
greenfield") undercounted badly — it missed that most Tier-3 languages already picked up
heritage quirks organically during the G2.2-G2.4 onboarding waves (worker discretion went
beyond the strict tier requirement in a lot of cases), not just the original 10.

**Corrected pass: cross-referenced every language with non-empty `class_types` in `spec.rs`
against every function in `generic.rs` that actually calls `out.inherits.push`, by name, not
by grep count.** Real result: **24 of the 34 Tier-3 languages already have inherits** (Ada,
Apex, C++, Crystal, C#, D, Dart, F#, Groovy, Java, Julia, Kotlin, Objective-C, Odin, Pascal,
PHP, PowerShell, Python, Rust, Scala, Squirrel, Swift, TypeScript, Go). Of the remaining 10
(C, CUDA, GDScript, Gleam, GLSL, QML, ReScript, Solidity, TSX, Zig):
- **TSX** shares TypeScript's quirk — already covered.
- **C, CUDA, GLSL, Zig** have no class-based inheritance concept in the language at all — zero
  INHERITS edges is *correct* baseline behavior, not a gap.
- **GDScript** treats `extends` as an IMPORT by deliberate design (the whole file is an
  implicit single class; there's no `class_declaration`-shaped node to attach a `sub_name`
  to) — also correct as-is, not a gap.
- **Gleam, ReScript, QML** are functional/component-composition languages without
  `class X : Y`-shaped heritage syntax — very likely also N/A, not independently re-verified
  line-by-line here (low priority given the pattern above).
- **Solidity** was the one real, confirmed gap: `contract Foo is Bar, Baz(args)` produced zero
  INHERITS edges (`solidity_quirk` didn't claim `contract_declaration`/`interface_declaration`
  at all, and no other quirk did either) — **fixed as part of Stage 1** (see below).

**Revised conclusion: this is a much smaller build-out than first estimated for inherits
specifically** — a generic engine fallback still has value (future-proofing for languages
onboarded later with no quirk, matching the baseline's own design faithfully), but it will not
newly light up "~20+ languages." **Decorators and routes have not been re-audited with this
same rigor yet** — treat their counts below as the original (possibly also-overstated)
estimate until Stage 3/4 actually measure them the same way Stage 1 did for inherits.

- **Decorators: partially generic, not yet re-verified.** 18 languages have non-empty
  `decorator_types` rows in `spec.rs` (Rust, TypeScript, Python, Java, C#, PHP, Kotlin, Swift,
  GDScript, Dart, Scala, Groovy, Apex, Crystal, QML, ReScript, Cairo, CFScript). Whether
  `walk()` actually surfaces every one of these into a graph-visible decorator edge (vs. some
  being inert metadata) is Stage 3's first task, not an assumption to carry forward.
- **Routes: 4 quirk wirings only** (`go_quirks`, `typescript_quirks`, `csharp_quirks`,
  `qml_quirks`, via `route_from_call: Some(...)`). This count came from the same shallow grep
  method that undercounted inherits — re-audit properly (by function name, not raw count)
  before assuming the gap size in Stage 4.

Staging (Stage 1 done, see below; Stages 2-5 unchanged in shape, revised in expected size):

1. **Stage 1 — DONE (2026-07-09).** Added `generic_base_class_names()` to `generic.rs`, a
   direct Rust port of the baseline's own generic fallback tier in `extract_base_classes`
   (`extract_defs.c:2540-2576`, ground-truth field list: `superclass`, `superclasses`,
   `superinterfaces`, `interfaces`, `bases`, `type_inheritance_clause`,
   `delegation_specifiers`, plus the `extends_interfaces`/`super_interfaces` named-child
   fallback for Java-shaped interface heritage) — wired into `walk()`'s `class_types` branch,
   firing only when no `on_unmatched_node` quirk already claimed the node (every language with
   an existing heritage quirk short-circuits before reaching it, zero double-counting, zero
   regression risk by construction). Also fixed the one confirmed real gap found during
   scoping: extended `solidity_quirk` to claim `contract_declaration`/`interface_declaration`
   directly, extracting `inheritance_specifier` children's `ancestor` field (verified against
   `tree-sitter-solidity`'s own `node-types.json`, not guessed) — `contract Foo is Bar, Baz(42)`
   now produces real INHERITS edges. Net effect on the generic *field-name* fallback itself:
   likely zero-to-few additional languages get new edges from it today (most of Tier-3 that
   has real heritage syntax already had a quirk), but it's still correct architecture and
   directly benefits any future language onboarded without one.
2. **Stage 2 — DONE (2026-07-09).** Spot-checked the 3 unconfirmed Tier-3 languages: **no real
   gap found.** Gleam (`type_definition`/`type_alias` class_types — functional, no class-based
   inheritance concept) and ReScript (`module_declaration`/`type_declaration` class_types —
   same, OCaml-family module/type system) are legitimately N/A. QML already gets full heritage
   coverage via `qml_quirk`'s `_ => ts_quirk(...)` delegation for class-shaped nodes — missed by
   the earlier by-function-name grep since the `inherits.push` call physically lives inside
   `ts_quirk`, not `qml_quirk` itself.
3. **Stage 3 — DONE (2026-07-10).** Audited the 18 languages with populated `decorator_types`
   the same rigorous way Stage 1 was corrected (cross-referencing which functions actually call
   `out.decorates.push`, not a raw count). Real result: only 9 (Rust, TypeScript, Python, Java,
   C#, PHP, GDScript, Apex, ReScript) plus QML (via the same `ts_quirk` delegation as Stage 2)
   had real extraction — the other 8 (Kotlin, Swift, Dart, Scala, Groovy, Crystal, Cairo,
   CFScript) had decorator/annotation syntax declared but zero extraction code. Fixed 7 of them
   via 8 parallel grammar-research passes, each independently verified against the grammar's own
   `node-types.json`/`grammar.js`, not guessed from cross-language convention — every language's
   annotation shape turned out genuinely different (fields vs. fields-less, modifiers-wrapper
   vs. direct-positional-child vs. prev-sibling; see the `40ce6c9` commit message for the full
   per-language breakdown). The 8th, CFScript, was a confirmed **negative** finding: its
   `decorator_types: &["decorator"]` was a vestigial copy-paste artifact from a shared JS/TS
   grammar-generation script — the `decorator` node is TC39 `@Name` syntax unreachable from any
   real ColdFusion construct — corrected to `&[]` rather than forcing a quirk onto nothing.
   7 new tests, one per fixed language. Confirms `decorator_types` itself remains globally
   unconsulted by the generic engine outside per-language quirks — same architectural shape as
   inherits before Stage 1, and as routes below: there is no generic field-driven fallback for
   decorators, only quirks/`on_method_defined` hooks per language.
4. **Stage 4 — DONE (2026-07-10).** The original plan ("port `service_patterns.c:315-377`'s
   library-name matching") was based on a table that turned out to be **dead code in the
   baseline itself** — `route_reg_libraries` and every function in `service_patterns.c` that
   consults it (`cbm_service_pattern_match`, `cbm_service_pattern_route_method`, etc.) has
   **zero callers anywhere else in the C codebase**, confirmed by grepping the whole
   `internal/cbm/` tree. The baseline's REAL, actually-wired route mechanism is
   `extract_route_from_decorators`/`annotation_route_method` (`extract_defs.c:1275,1303`),
   called unconditionally for every function/method regardless of language, purely by matching
   the DECORATOR/ANNOTATION's own name against a small fixed list (`GetMapping`→GET,
   `PostMapping`→POST, `PutMapping`→PUT, `PatchMapping`→PATCH, `DeleteMapping`→DELETE,
   `RequestMapping`→ANY, plus bare JAX-RS `GET`/`POST`/`PUT`/`DELETE`/`PATCH`) or a Python-style
   `@x.route(...)`/`@x.get(...)` call-shaped decorator. No per-language whitelist at all except
   one special case: `extract_defs.c:4137` explicitly joins a class-level route prefix for
   `CBM_LANG_JAVA`/`CBM_LANG_KOTLIN` identically (Spring MVC's `@RequestMapping` on the
   controller class), confirming Kotlin is meant to get the exact same treatment as Java.

   Re-auditing our own port by function name (not the original shallow `route_from_call: Some(`
   grep) found real coverage already exists for **8 languages**, not 4: Go/TypeScript/C#
   (call-shape, `route_from_call` hook), QML (via `ts_quirk` delegation), Python
   (`py_route_from_decorated`, decorator-call-shape), Java (`java_route_from_mapping`,
   Spring-annotation-shape), PHP (`php_routes_from_attributes`/`php_route_from_symfony_attribute`/
   `php_route_from_scoped_call`). The one real, confirmed gap: **Kotlin** had annotation
   extraction (from Stage 3) but no route detection reusing it — fixed by adding
   `kotlin_route_from_mapping`/`kotlin_annotation_path_argument`, reusing
   `JAVA_MAPPING_ANNOTATIONS` verbatim (verified against `tree-sitter-kotlin-ng`'s own
   `node-types.json` for the `value_arguments`/`value_argument`/`string_literal` path-argument
   shape), wired into `kotlin_function_like`'s existing per-function decorator scan.

   Rust (actix/axum/rocket), Ruby (Sinatra/ActionDispatch), Elixir (Phoenix), and Scala
   (akka-http/play) remain without route detection — **not a gap**, because none of their
   idiomatic routing styles match the baseline's actual (narrow) decorator/annotation-name
   mechanism: Sinatra/Phoenix/akka-http are call-DSL or macro-based, not decorator-based; Rocket
   IS attribute-based (`#[get("/path")]`) but uses lowercase `get`/`post`, and
   `annotation_route_method`'s own `strcmp` checks are case-sensitive against uppercase
   `GET`/`POST` — so the baseline itself would not recognize Rocket's routes either. Extracting
   these anyway would mean implementing behavior the baseline doesn't have, which is not what
   parity means. 1 new test (`spring_mapping_annotation_records_a_route`).
5. **Stage 5:** full parity re-verification per the "Full parity re-verification" section below,
   once Stages 1-4 land.

Also carrying forward: **Agda and FORM**, found missing during G2.5's audit, were onboarded in
wave G2.6 (see `00-OVERVIEW.md` — 156/158 total, only K8s/Kustomize deliberately deferred) —
done before Stage 1 started, per the recommendation below.

## Done when
Every onboarded language verified equal-or-better vs the baseline; deferred-grammar list is
explicit and owner-visible; proof regenerated; final commit pushed. THEN update
`docs/plans/enforcer-selfhost-plan/MEMORY_RETRIEVAL_STATE_BOARD.md` and report.
