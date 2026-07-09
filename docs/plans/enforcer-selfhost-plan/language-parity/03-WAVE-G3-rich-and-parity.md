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

Measured actual current coverage rather than assuming the onboarding waves left room for this:

- **Inherits: 100% quirk-only, no generic fallback exists.** `walk()` has zero `LangSpec`
  field for heritage/base-class node kinds — `InheritsRef` is populated at exactly 33 call
  sites in `generic.rs`, all inside per-language `Quirks` hooks (Go, TypeScript/JS, Python,
  Java, C#, and a handful more of the original hand-migrated set). None of the ~24 other
  Tier-3 languages (PHP/Kotlin/Squirrel/Julia/F#/D/PowerShell/Pascal/C++/Swift/Dart/Scala/
  Groovy/Apex/Crystal/QML/ReScript/Cairo/CFScript/GDScript/Ruby/Rust/R/…) get any inherits
  edges at all today, despite being Tier-3.
- **Decorators: partially generic.** 18 languages have non-empty `decorator_types` rows in
  `spec.rs` (Rust, TypeScript, Python, Java, C#, PHP, Kotlin, Swift, GDScript, Dart, Scala,
  Groovy, Apex, Crystal, QML, ReScript, Cairo, CFScript) — the spec field itself is consulted
  generically, unlike inherits. Not yet independently verified end-to-end that `walk()`
  actually surfaces every one of these into a graph-visible decorator edge (vs. some being
  inert metadata) — first task of this wave, not an assumption to carry forward.
- **Routes: 4 quirk wirings only** (`go_quirks`, `typescript_quirks`, `csharp_quirks`,
  `qml_quirks`, via `route_from_call: Some(...)`). Every other Tier-3 language with
  web-framework imports (Python/Flask-Django, PHP/Laravel, Kotlin/Ktor, Java/Spring, Ruby/
  Rails, Swift/Vapor, …) currently produces zero route edges. `service_patterns.c:315-377`
  (baseline's library-name matching) has not been ported at all yet.

**Conclusion: this is not a polish pass, it's a build-out.** Inherits and routes are
essentially greenfield for ~20+ of the 34 Tier-3 languages. Staging:

1. **Stage 1 (engine change, orchestrator, no coordination risk):** add a generic
   heritage-field fallback to `walk()` — mirrors how `call_types`/`decorator_types` are
   already consulted directly from `LangSpec` — driven by a new `inherit_types`-style field
   checking the baseline's common heritage field/node names (`superclass`, `superinterfaces`,
   `class_heritage`, `implements_clause`, `base_list`, `base_class_clause`). One engine change,
   benefits every Tier-3 language at once without per-language worker fan-out.
2. **Stage 2 (quirk walkers, worker-sized):** dedicated heritage quirks only for the languages
   the baseline itself special-cases beyond the generic fallback (TS/PHP/Kotlin/Squirrel/
   Julia/F#/D/PowerShell/Pascal/C++/C# per the baseline's `extract_base_classes`,
   `extract_defs.c:2373`) — most of these already have quirks from earlier hand-migration; the
   gap is the *other* ~15 Tier-3 languages that need a check against whether the Stage-1
   generic fallback alone is sufficient for them.
3. **Stage 3 (decorators):** verify the 18 languages with populated `decorator_types` actually
   surface decorator edges end-to-end; wire the field for any Tier-3 language missing it.
4. **Stage 4 (routes):** port `service_patterns.c:315-377`'s library-name matching as a
   generic layer over the import graph (not per-language quirks) so route edges appear for any
   onboarded language whose imports name a known web framework — this is the highest-leverage
   single change for routes, same reasoning as Stage 1 for inherits.
5. **Stage 5:** full parity re-verification per the "Full parity re-verification" section below,
   once Stages 1-4 land.

Also carrying forward from the G2.5 closeout: **Agda and FORM are not onboarded at all**
(found during G2.5's audit against the C baseline, documented in `00-OVERVIEW.md`) — decide
whether to fold their onboarding into this wave (idle worker headroom) or run a short G2.6
first. Recommendation: G2.6 first (it's pure onboarding, same playbook as G2.1-G2.5, no reason
to block it on G3's harder engine work), then G3 Stages 1-5.

## Done when
Every onboarded language verified equal-or-better vs the baseline; deferred-grammar list is
explicit and owner-visible; proof regenerated; final commit pushed. THEN update
`docs/plans/enforcer-selfhost-plan/MEMORY_RETRIEVAL_STATE_BOARD.md` and report.
