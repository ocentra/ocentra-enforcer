# Contributing

Ocentra Enforcer changes are accepted only when rules, validators, registry
metadata, docs, schemas, fixtures, and tests move together.

Rule changes must include:

- A `rules/rules.json` entry with title, snippet, lock policy, severity, doc,
  validator, triggers, and applies-to metadata.
- A routed rule doc section with fails, passes, fix recipe, validator, and
  fixture evidence.
- Validator logic that emits the exact rule ID.
- Pass and fail fixtures or an explicit review-only rule classification.
- Tests proving the diagnostic includes rule ID, file, line, detail, doc,
  snippet, and source.

Do not weaken immutable rules, bypass self-checks, or add config overrides
without waiver governance.
