# Per-Stack Agent Persona Docs

<!-- agent-capsule -->
> Agent Capsule
> Kind: T3 human-canonical prose (d09). NOT the machine-consumed source.
> Read when: onboarding a human reviewer to a stack's must/never posture, or
> authoring/reviewing one of the sibling persona files in this directory.
> Stop rule: do not treat this prose as the enforcement source of truth —
> the STRUCTURED rule (`enforcer-rules::registry::RuleRecord`, keyed by
> `RuleId`) is what the engine and every AI consumer read.
<!-- /agent-capsule -->

These files are the T3 persona layer: advisory, human-readable "how to work
in this stack" guidance for `rust.md`, `typescript.md`, `python.md`, and
`common.md`. Per `RUST_ARCHITECTURE.md`, prose here is allowed to exist only
because it hangs off a REAL, mechanized rule — every `must`/`never` bullet
below carries an explicit `[ruleId]` citation, and
`crates/enforcer-validator/src/doc_rule_parity.rs` (d09) is the T1 oracle
that asserts each cited id resolves to a real record in the `enforcer-rules`
registry. A bullet with no citation, or a citation to an id that is not
registered, is prose pretending to be enforcement — the oracle fails it
closed.

Free-text explanatory prose (this paragraph, headers, examples) is
deliberately NOT gated: only imperative `must`/`never` bullets are checked.

- [`rust.md`](rust.md) — Rust stack persona.
- [`typescript.md`](typescript.md) — TypeScript/frontend stack persona.
- [`python.md`](python.md) — Python/FastAPI stack persona.
- [`common.md`](common.md) — cross-stack (language-agnostic) persona.
