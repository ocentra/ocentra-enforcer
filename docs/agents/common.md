# Common (Cross-Stack) Agent Persona

<!-- agent-capsule -->
> Agent Capsule
> Kind: T3 human-canonical prose (d09), advisory only.
> Read when: onboarding to language-agnostic conventions that apply across
> every stack in this workspace.
> Stop rule: every must/never bullet below is checked by
> `crates/enforcer-validator/src/doc_rule_parity.rs`; do not add a bullet
> without a real `[ruleId]` citation.
<!-- /agent-capsule -->

Guidance that applies regardless of language. Advisory prose; the
mechanized rule is the actual gate.

## Size and shape

- A file must never exceed the file-length cap (200 lines; Rust override
  400) [SIZE-FILE-1.1]
- A function must never exceed the function-length cap (30 lines)
  [SIZE-FUNC-1.1]
- A public item must always carry documentation explaining its purpose
  [RUST-DOC-PUBLIC-ITEM]

## Deferred work and ownership

- A deferred-work marker must always carry a structured `DEFERRED`
  annotation, never a bare `TODO` [DEFER-1.1]
- An owner-set marker must never be dropped across an external doc rewrite
  [OWNERSET-1.1]

Free-text note: as with the stack-specific personas, "must"/"never" bullets
here are what the doc-rule-parity gate checks; explanatory prose is
unchecked.
