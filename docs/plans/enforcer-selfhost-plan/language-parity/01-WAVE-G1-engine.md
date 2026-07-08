# Wave G1 — Generic spec-table extractor engine

**Outcome:** one Rust extractor driven by per-language spec tables, producing the same
node/edge sets our 10 bespoke extractors do today, verified by re-running the existing
fixtures with zero regression. No new languages yet — this wave only proves the engine.

## Scope
- New module `crates/enforcer-memory/src/languages/spec.rs` (or `src/languages/generic.rs`):
  - A `LangSpec` struct mirroring `CBMLangSpec`: `func_types`, `class_types`, `field_types`,
    `module_types`, `call_types`, `import_types`, `branch_types`, `decorator_types`
    (each `&'static [&'static str]` of tree-sitter node kind names).
  - A generic walker: given a parsed tree + a `LangSpec`, emit defs (Function/Method/Class/
    Struct/Interface/Enum/…), call edges (with the receiver/arg capture C1 added), import
    edges, and branch-based cyclomatic complexity — reusing existing `code_graph`/`parsers`/
    `complexity`/`resolution` types. Do NOT fork those; feed them.
  - A quirk hook: `fn quirk(lang, node, ctx)` seam so a handful of languages can override
    (mirrors baseline's `if (lang==CBM_LANG_X)` branches) without polluting the generic path.
- Represent the existing 10 languages as `LangSpec` rows (read our current extractors +
  the baseline's arrays for the node-type names) and route them through the generic walker.

## Migration strategy (no regression is the whole point)
- Keep the bespoke extractors in place; add the generic engine ALONGSIDE behind an internal
  switch. Port ONE language first (Go — smallest arrays), diff its graph output against the
  bespoke Go extractor on `tests/fixtures/memory/lang_go` until identical, then the rest.
- Rich-tier behaviors the generic walker doesn't yet do (routes, inherits, deep type-refs)
  stay on the bespoke path for now; G3 generalizes them. The generic path must at least
  match defs+calls+imports+complexity for all 10.

## Gates
`cargo test -p enforcer-memory` (existing language + resolution + complexity suites must stay
green), `cargo clippy -p enforcer-memory --all-targets -- -D warnings`, `cargo fmt --check`.
The existing `feature_parity` harness rows for the 10 languages must not regress.

## Done when
Generic engine reproduces all 10 languages' current graph output on existing fixtures; bespoke
extractors either removed or reduced to quirk hooks; one wave commit pushed.
