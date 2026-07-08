# Language-Parity Campaign — Overview

<!-- agent-capsule -->
> Plan: `enforcer-selfhost-plan` / workpack dir: `language-parity`
> Goal: bring `crates/enforcer-memory` from 10 hand-written language extractors to
> full parity with the codebase-memory-mcp C baseline's **158 registered languages**.
> Decision (owner, 2026-07-06): "Generic engine + all ~166". Pivot to the baseline's
> table-driven architecture; do NOT hand-write per-language extractors.
> Ground truth (READ-ONLY): `C:\Projects\codebase-memory-mcp`.
> Prereq DONE: core memory-graph parity, branch rust-build tip `0443ea7`.
<!-- /agent-capsule -->

## The finding that sets the architecture

The baseline is **table-driven, not 160 bespoke extractors**. `internal/cbm/lang_specs.c`
(`lang_specs[CBM_LANG_COUNT]` table at :1601-2589) holds one `CBMLangSpec` per language:
arrays of tree-sitter node-type NAMES for func / class / field / module / call / import /
branch / decorator. The shared walkers consume those arrays generically:

- `extract_defs.c` — definitions + complexity (`set_def_complexity` :3030 off `branching_node_types`) + inheritance (`extract_base_classes` :2373, dedicated walkers for TS/PHP/Kotlin/Squirrel/Julia/F#/D/PowerShell/Pascal + generic fallback)
- `extract_calls.c` — call edges (data-driven :72-1196 with `if (lang==CBM_LANG_X)` one-off quirks)
- `extract_imports.c` — import edges
- `extract_type_refs.c` — base param/return types for ALL langs (:349); deep body-level type refs gated to Go/TS/TSX/Java/Python/Rust only (:212-242)
- `service_patterns.c:315-377` — routes via library-NAME pattern match (express/flask/gin/axum/Spring…), works for any lang with imports

So "adding a language" = declare its node-type name arrays + register extension + (only if the grammar is weird) a quirk branch. That is the design we must mirror in Rust.

## Registered surface (excl. dead `CBM_LANG_NIM` — no ext/grammar/spec, skip it)

**158 languages**, tiered by extraction depth:

| Tier | Count | Meaning |
|---|---|---|
| 3 (deep) | 34 | defs+calls+imports + inherits walker OR deep type-refs OR decorators |
| 2 (mid)  | 49 | defs+calls+imports |
| 1 (shallow) | 30 | partial (missing one of defs/calls/imports) |
| 0 (nominal) | 45 | structure/markup/config only (mostly `empty_types` calls) |

**Tier 3 (34):** Ada, Apex, C, C++, C#, Crystal, CUDA, Dart, D, F#, GDScript, Gleam, GLSL, Go, Groovy, Java, Julia, Kotlin, Objective-C, Odin, Pascal, PHP, PowerShell, Python, QML, ReScript, Rust, Scala, Solidity, Squirrel, Swift, TSX, TypeScript, Zig

**Tier 2 (49):** Agda, Bash, Bicep, BitBake, Cairo, CFScript, CMake, COBOL, Common Lisp, Elixir, Elm, Erlang, FORM, Fortran, FunC, Hare, Haskell, HLSL, ISPC, JavaScript, Jsonnet, Just, Lean, Lua, Magma, Makefile, Move, NASM, Nickel, OCaml, Perl, Pony, Puppet, PureScript, R, Ruby, SCSS, Slang, Starlark, Sway, SystemVerilog, Templ, TLA+, Typst, Verilog, VHDL, VimScript, WGSL, Wolfram

**Tier 1 (30):** AWK, Cap'n Proto, CFML, Clojure, Emacs Lisp, Fennel, Fish, Go Template, HCL, Kconfig, LLVM IR, Luau, MATLAB, Meson, Nix, Pine, Pkl, Prisma, Protobuf, Racket, Scheme, Smali, Smithy, SQL, TableGen, Tcl, Teal, Thrift, WIT, Zsh

**Tier 0 (45):** Assembly, Astro, Beancount, BibTeX, Blade, CSS, CSV, DeviceTree, Diff, Dockerfile, DotEnv, gitattributes, gitignore, GN, Go Mod, GraphQL, HTML, Hyprlang, INI, Janet, Jinja2, JSDoc, JSON, JSON5, K8s, KDL, Kustomize, Linker Script, Liquid, Markdown, Mermaid, PO, Properties, Regex, Requirements, RON, reStructuredText, SOQL, SOSL, SSH Config, Svelte, TOML, Vue, XML, YAML

## Where we stand (our 10)

Rust, TypeScript, Python, Go, Java, C, C++, C#, PHP at Tier 3; JavaScript at Tier 2 (matches baseline — JS gets no decorators/inherits walker there either). **Missing: 25 Tier-3 + ~47 Tier-2 + all Tier-1/0.** TSX is a distinct grammar/spec from TS in the baseline — we currently fold it into TS; it needs its own spec row.

## The real long pole: grammar sourcing

Not extractor logic — tree-sitter grammar availability for Rust. Mainstream ~40 have maintained `tree-sitter-*` crates. The long tail the baseline vendored from raw C. Our options per grammar: (a) crates.io crate, (b) vendor the C `parser.c` + bind via `tree-sitter` crate's language FFI, (c) defer. Every wave must `log()` which languages it dropped and why — no silent truncation (lesson L45 discipline).

## Wave plan (see sibling files)

- **G1** — build the generic spec-table extractor engine in Rust; prove it reproduces our existing 10 on their current fixtures (no regression). `01-WAVE-G1-engine.md`
- **G2** — port the 158 spec rows (data copy from lang_specs.c) + onboard grammars in staged batches (mainstream deep-tier → bulk → exotic). `02-WAVE-G2-spec-and-grammars.md`
- **G3** — rich-tier passes (inherits/deep-type-refs/decorators/routes) for Tier-3 langs + full live parity re-verification vs the C binary across the language set; regenerate proof. `03-WAVE-G3-rich-and-parity.md`

## Non-negotiables (carry from prior waves)
No inline tests (tests/ only); no unwrap/expect/panic incl. tests; no `#[allow(clippy::…)]`; new fixture extensions LF-pinned in `.gitattributes` the same commit they land; workers claim/release/mail on hub `enforcer-rust-build` lane `primary`, never spawn sub-agents, never run git; orchestrator runs all wave gates FOREGROUND and commits. Do not touch `ocentra-enforcer.config.json`.
