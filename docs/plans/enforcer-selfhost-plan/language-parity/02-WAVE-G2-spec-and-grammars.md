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

## Progress log

- **G2.1 DONE (pushed 80f6237):** Kotlin, Swift, TSX, Dart, Scala, Groovy, Ruby, Zig,
  Objective-C, Solidity, GDScript — 11 languages, 0 deferred, all real crates.io grammars.
  Real bugs caught against actual grammar shapes (not blind baseline transcription):
  Dart's fn-name/body field split, Groovy's baseline array missing `method_declaration`
  entirely, Solidity's wrapped call-function field, GDScript's `base_call` callee. Ruby's
  inheritance depth deliberately matches the baseline's own (limited) support rather than
  over-building. Complexity extraction deferred per-language (`complexity_language() ->
  None`) for this whole batch — richer tier work lands in G3.
- **G2.2 DONE (pushed a6ef2f8):** Ada, Apex, Crystal, CUDA, D, PowerShell, F#, Gleam, GLSL,
  Julia, Odin, Pascal, QML, ReScript, Squirrel — completes ALL 34 baseline Tier-3 languages.
  Plus Lua, Elixir, Bash, Haskell, OCaml, Erlang, R, Perl, Clojure (Tier-2). 24 languages,
  0 deferred. CUDA/GLSL reuse the existing C++/C grammar deps (mirrors baseline's own
  lang_specs.c aliasing). Real bugs caught: Elixir's baseline func-def extractor drops every
  guard-clause def (`def foo(x) when x > 0`) — fixed as a genuine improvement, documented;
  Elixir's baseline imports pass is non-recursive and breaks on any `defmodule`-wrapped file
  (virtually all of them) — this crate's recursive walk doesn't have that bug; Lua's baseline
  `branch_types` names a node (`for_in_statement`) that doesn't exist in the real grammar.
  8 of 8 parallel workers hit a simultaneous session rate-limit wall mid-wave (see
  orchestration-lessons.md) — landed code was verified directly against tree+gates, not
  trusted from self-reports; orchestrator hand-fixed the resulting 12 clippy + 2 test errors.
- **Bookkeeping correction:** G2.2's "9 mainstream Tier-2" framing was wrong — Clojure is
  actually Tier-1 (done anyway, no rework needed, just relabeling). True remaining count
  after G2.1+G2.2: Tier-2 39 remaining (of 49; done: JS pre-existing, Ruby, Lua, Elixir,
  Bash, Haskell, OCaml, Erlang, R, Perl = 10), Tier-1 29 remaining (of 30; done: Clojure),
  Tier-0 45 remaining (of 45; none done). Total remaining: 113, not the 74 first assumed.
- **Next: G2.3** — all 39 remaining Tier-2 languages (highest remaining depth), 5 workers
  per the L47 session-limit-sizing lesson (not 8). Tier-1 (29) + Tier-0 (45) follow in
  G2.4/G2.5.

## Fixtures & tests
Per language: one small source file under `tests/fixtures/memory/lang_<x>/` exercising a
function, a call, an import (and a branch for complexity). A table-driven test asserts the
generic engine finds the expected def/call/import for each onboarded language. New fixture
extensions LF-pinned in `.gitattributes` the same commit (extend the existing `lang_*/**` glob).

## Gates & cadence
Crate-scoped gates per batch; orchestrator runs the workspace bar + commits per batch. Each
batch is its own commit so breadth lands incrementally. Report per batch: languages onboarded,
grammars vendored vs crate vs deferred, test counts.
