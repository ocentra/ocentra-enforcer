# Wave G2 — Port the 158 spec rows + onboard grammars (staged)

**Outcome:** every language whose grammar we can source is classified, parsed, and extracted
at its baseline tier (defs+calls+imports+complexity) through the G1 engine.

## Two independent work streams

### A. Spec-row port (pure data, cheap, parallelizable)
Copy each language's node-type arrays from `internal/cbm/lang_specs.c` into Rust `LangSpec`
rows, plus its extension/filename mapping from `src/discover/language.c` (`EXT_TABLE` :30-621,
`FILENAME_TABLE` :632-672, and the `.m` content-sniff `cbm_disambiguate_m` :997). This is
mechanical transcription — split across workers by tier/alphabet chunk. One worker owns a
disjoint slice of languages; each adds spec rows + extension entries + a tiny fixture.

### B. Grammar onboarding (the long pole — gate the campaign's breadth)
Per language, resolve the tree-sitter grammar for Rust, in preference order:
1. Maintained crates.io `tree-sitter-<lang>` crate → add to Cargo, bind.
2. No crate but grammar exists → vendor the C `parser.c`(+`scanner.c`) from the baseline's
   `internal/cbm/vendored/grammars/<lang>/` and bind via the `tree-sitter` crate FFI.
3. Unavailable/broken ABI → DEFER; `log()` it explicitly in the wave report and mark the
   spec row `grammar: pending`. NEVER silently drop (L45).

**Staged batches** (so we ship value continuously, not one giant unmergeable wave):
- G2.1 — mainstream Tier-3 missing: Kotlin, Swift, Dart, Scala, TSX, Ruby(Tier-2), Zig,
  Solidity, GDScript, Groovy, Objective-C. (highest real-world value)
- G2.2 — remaining Tier-3 + Tier-2 with ready crates: Lua, Elixir, Bash, Haskell, OCaml,
  Erlang, R, Perl, F#, Julia, Crystal, Clojure, SQL, HCL, Nix, CMake, Make, …
- G2.3 — Tier-0/1 config & markup (mostly `empty_types` calls, trivial specs): YAML, JSON,
  TOML, Dockerfile, HTML, CSS/SCSS, Vue, Svelte, GraphQL, Protobuf, Markdown, XML, INI, …
- G2.4 — exotic long tail as grammars allow: COBOL, Wolfram, Hare, Pony, Pine, Ada, Agda, …

## Fixtures & tests
Per language: one small source file under `tests/fixtures/memory/lang_<x>/` exercising a
function, a call, an import (and a branch for complexity). A table-driven test asserts the
generic engine finds the expected def/call/import for each onboarded language. New fixture
extensions LF-pinned in `.gitattributes` the same commit (extend the existing `lang_*/**` glob).

## Gates & cadence
Crate-scoped gates per batch; orchestrator runs the workspace bar + commits per batch. Each
batch is its own commit so breadth lands incrementally. Report per batch: languages onboarded,
grammars vendored vs crate vs deferred, test counts.
