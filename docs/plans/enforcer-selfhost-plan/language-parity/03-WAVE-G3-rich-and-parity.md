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

## Done when
Every onboarded language verified equal-or-better vs the baseline; deferred-grammar list is
explicit and owner-visible; proof regenerated; final commit pushed. THEN update
`docs/plans/enforcer-selfhost-plan/MEMORY_RETRIEVAL_STATE_BOARD.md` and report.
