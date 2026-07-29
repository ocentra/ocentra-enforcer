# TypeScript Agent Persona

<!-- agent-capsule -->
> Agent Capsule
> Kind: T3 human-canonical prose (d09), advisory only.
> Read when: onboarding to TypeScript/frontend-stack conventions in this
> workspace.
> Stop rule: every must/never bullet below is checked by
> `crates/enforcer-validator/src/doc_rule_parity.rs`; do not add a bullet
> without a real `[ruleId]` citation.
<!-- /agent-capsule -->

Guidance for an AI agent (or human) writing TypeScript/frontend code in this
workspace. Advisory prose; the mechanized rule is the actual gate.

## Architecture

- A component must never import across a feature boundary
  [FE-ARCH-1.3]
- Server data must never be stored in the client store [FE-STATE-1.1]
- Data-loading must never happen via `fetch`/`axios` inside a bare
  `useEffect` [FE-STATE-1.2]

## Types and errors

- Services must never expose an untyped error; typed errors are required
  [FE-PAT-1.4]
- A type-only import must always use `import type` [FE-TS-1.14]
- `any` must never appear without an explicit waiver justifying it
  [FE-TS-1.5]

## Accessibility

- `next/image` must always be used, and it must always carry `alt` text
  [FE-CMP-1.12]
- An `<input>` must never omit a `label`/`aria-label` association
  [FE-A11Y-1.3]

Free-text note: as with the Rust persona, "must"/"never" bullets here are
the ones the doc-rule-parity gate checks; explanatory prose is unchecked.
