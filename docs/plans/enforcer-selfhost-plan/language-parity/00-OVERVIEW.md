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

**Tier 0 (45):** Assembly, Astro, Beancount, BibTeX, Blade, CSS, CSV, DeviceTree, Diff, Dockerfile, DotEnv, gitattributes, gitignore, GN, Go Mod, GraphQL, HTML, Hyprlang, INI, Janet, Jinja2, JSDoc, JSON, JSON5, K8s†, KDL, Kustomize†, Linker Script, Liquid, Markdown, Mermaid, PO, Properties, Regex, Requirements, RON, reStructuredText, SOQL, SOSL, SSH Config, Svelte, TOML, Vue, XML, YAML — all landed via G2.5 except † (deliberately deferred, see above).

## Where we stand — two separate finish lines, don't conflate them

**Finish line 1 — onboarding (recognize the language, extract defs/calls/imports): 156/158
done.** Only K8s and Kustomize remain, both *deliberately deferred* (need a filename-gated
semantic pass layered on YAML — `cbm_extract_k8s()` in the baseline, `cbm.h:614` — that this
crate's YAML pipeline doesn't have yet; documented at the deferral site in `parsers/mod.rs`).
Every other registered language, all 34 Tier-3 + 49 Tier-2 + 30 Tier-1 + 43 of 45 Tier-0, is
landed on `rust-build`. TSX has its own spec row (distinct from TS, matching the baseline).

**Finish line 2 — full parity (the actual bar: same extraction *depth* as the C baseline, not
just presence): NOT done.** See wave G3 (`03-WAVE-G3-rich-and-parity.md`) — inheritance and
route edges are close to greenfield for ~20+ of the 34 Tier-3 languages (onboarded with those
explicitly deferred, no generic engine fallback exists yet for either), decorators are only
partially verified, and none of this has been live-verified against the actual C binary yet.
Scoped, not started.

Landed via: G1/G1b (engine + original 10), G2.1 (11 langs), G2.2 (24 langs), G2.3 (39 langs,
completing Tier-2 as then understood), G2.4 (29 langs, completing Tier-1 — see the
lost-update-collision lesson in `refs/orchestration-lessons.md`: 20 of 29 G2.4 languages were
silently wiped mid-wave by a concurrent-write race and had to be recovered via systematic
post-hoc grep verification), G2.5 (42 Tier-0 langs, closing a dispatch-wiring gap G2.5d
deliberately left open for 11 of its languages — `LangSpec`+`generic.rs` existed but were
never reachable via `Language`/`classify()`/`parse_file()`/`LanguageTag`; wired by the
orchestrator directly), G2.6 (Agda + FORM — found genuinely missing during G2.5's closeout
audit against the C baseline's `CBM_LANG_*` identifiers, despite being listed as "done" in an
earlier version of this doc's Tier-2 roster; the roster was wrong, not the code — onboarded
from the baseline's own vendored grammars, since neither has a maintained crates.io crate).

**Remaining onboarding gap: K8s and Kustomize only** — deferred, not missed, for the reason
above.

Note: `CBM_LANG_NIM` does not appear in the baseline's live `lang_specs.c` identifiers at all
(prior sessions treated it as a dead stub to subtract from 158 — it isn't present to subtract
in the first place under the current baseline checkout, so the working total is 158, not 157).

## The real long pole: grammar sourcing

Not extractor logic — tree-sitter grammar availability for Rust. Mainstream ~40 have maintained `tree-sitter-*` crates. The long tail the baseline vendored from raw C. Our options per grammar: (a) crates.io crate, (b) vendor the C `parser.c` + bind via `tree-sitter` crate's language FFI, (c) defer. Every wave must `log()` which languages it dropped and why — no silent truncation (lesson L45 discipline).

## Wave plan (see sibling files)

- **G1** — build the generic spec-table extractor engine in Rust; prove it reproduces our existing 10 on their current fixtures (no regression). `01-WAVE-G1-engine.md`
- **G2** — port the 158 spec rows (data copy from lang_specs.c) + onboard grammars in staged batches (mainstream deep-tier → bulk → exotic). **Done through G2.6** — 156/158, only K8s/Kustomize deferred. `02-WAVE-G2-spec-and-grammars.md`
- **G3** — rich-tier passes (inherits/deep-type-refs/decorators/routes) for Tier-3 langs + full live parity re-verification vs the C binary across the language set; regenerate proof. `03-WAVE-G3-rich-and-parity.md`

## Non-negotiables (carry from prior waves)
No inline tests (tests/ only); no unwrap/expect/panic incl. tests; no `#[allow(clippy::…)]`; new fixture extensions LF-pinned in `.gitattributes` the same commit they land; workers claim/release/mail on hub `enforcer-rust-build` lane `primary`, never spawn sub-agents, never run git; orchestrator runs all wave gates FOREGROUND and commits. Do not touch `ocentra-enforcer.config.json`.
