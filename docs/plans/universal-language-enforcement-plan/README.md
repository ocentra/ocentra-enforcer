# Universal Language Enforcement Plan

<!-- agent-capsule -->
> Plan: `universal-language-enforcement-plan`
> Purpose: turn the existing broad grammar substrate into honest capability-driven enforcement without making one rule pack, schema library, or language the universal doctrine.
> Authority: this plan refines `p01`, `p03`, and the existing language-parity campaign; it does not silently complete or replace their historical proof.
> Completion rule: every recognized language has a mechanically derived capability row, rules declare the facts they require, unsupported facts cannot become clean passes, and terminal proof is green on one exact integration SHA.
<!-- /agent-capsule -->

## Outcome

Enforcer is shape-driven, not Effect-driven, Zod-driven, TypeScript-driven, or regex-driven. The universal doctrine is about validated boundaries and explicit domain shapes. A project profile may prefer or require a library family, but the engine does not confuse that preference with the underlying requirement.

Language support is a capability ladder, not a boolean:

| Level | Capability | Honest meaning |
|---|---|---|
| L0 | discover | classify the path and retain the language identity |
| L1 | lexical | inspect literals/comments/paths with a declared lexer profile |
| L2 | structural | parse once and expose normalized, provenance-bearing single-file facts |
| L3 | graph | resolve supported cross-file relationships |
| L4 | ecosystem | recognize schema/framework/toolchain shapes through named adapters |
| L5 | rules | run rules whose declared fact requirements are satisfied |

No row may claim L2-L5 merely because a Tree-sitter grammar compiles.

## Confirmed Baseline

These are source-derived values at plan creation, not completion claims:

| Measure | Baseline |
|---|---:|
| `enforcer-memory::parsers::Language` variants | 160 |
| Structural `parse_file` variants | 156 |
| Deliberate non-structural variants | 4 |
| Tree-sitter grammar bindings | 145, plus Tree-sitter core |
| Vendored grammar crates | 51 |
| Literal-language registry rows | 65 named languages, plus `unknown` fallback |
| Native scan language families | 5 |
| Route-plan detected identities | 7, including `Other` |
| Validator crates | 9 |
| Validator families wired into scan | 7; Dart and CFML are not wired |
| Normalized `ParsedFile` collections | 9 |
| Validators receiving syntax facts | 0 |

The exact counts are re-derived by UL00 and must not remain hand-maintained truth.

## Read Order

1. [AGENTS.md](./AGENTS.md)
2. [PLAN_STATE.md](./PLAN_STATE.md)
3. [ARCHITECTURE.md](./ARCHITECTURE.md)
4. [CAPABILITY_MODEL.md](./CAPABILITY_MODEL.md)
5. [WORKPACK_INDEX.md](./WORKPACK_INDEX.md)
6. [MANAGER_RUNBOOK.md](./MANAGER_RUNBOOK.md)
7. The one assigned workpack
8. The matching row in [TEST_PROOF_EXPECTATIONS.md](./TEST_PROOF_EXPECTATIONS.md)

## Non-negotiable Boundaries

- One task owns the grammar/parser migration surface at a time. UL03 cannot start without UL02's recorded ownership transfer.
- Language/security rule crates never depend on memory persistence, retrieval, SQLite, model runtime, or raw Tree-sitter nodes.
- A parser failure, absent capability, unsupported framework, or unavailable tool is not a clean pass.
- Existing text validators remain explicitly L0/L1 until migrated; no bulk relabeling.
- Effect may remain a shipped preferred profile. Zod, Valibot, Pydantic, attrs validators, serde/newtypes, and future families may also satisfy a requirement when the selected profile permits them.
- Workers never merge to `rust-build` or `main`; the boss integrates accepted checkpoints.
