# Python Agent Persona

<!-- agent-capsule -->
> Agent Capsule
> Kind: T3 human-canonical prose (d09), advisory only.
> Read when: onboarding to Python/FastAPI-stack conventions in this
> workspace.
> Stop rule: every must/never bullet below is checked by
> `crates/enforcer-validator/src/doc_rule_parity.rs`; do not add a bullet
> without a real `[ruleId]` citation.
<!-- /agent-capsule -->

Guidance for an AI agent (or human) writing Python/FastAPI code in this
workspace. Advisory prose; the mechanized rule is the actual gate.

## Layering

- A router must never reference a repository symbol directly
  [PYFA-1.1]
- A service must never take a `Session`/`AsyncSession` parameter
  [PYFA-2.1]
- A service must never call `commit`/`begin`/`rollback` itself
  [PYFA-3.1]
- `domain/**` must always stay framework-pure — never import FastAPI/HTTP
  [PYFA-9.1]

## Security

- Passwords must never be stored or compared in plaintext [PYFA-12.1]
- Token generation must never use insecure `random.*` [PYFA-12.2]
- CORS configuration must never use a wildcard origin [PYFA-12.3]

Free-text note: as with the other stack personas, "must"/"never" bullets
here are what the doc-rule-parity gate checks; explanatory prose is
unchecked.
