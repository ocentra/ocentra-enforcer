# Contributing

<!-- ai-dense -->
```yaml
rule_change_requires: "typed rule record (enforcer-rules) + Validator impl emitting the same ruleId + fail/pass fixtures + a cargo test detection test + a doc anchor"
banned: "weakening immutable rules, bypassing self-checks, config overrides without waiver governance"
```
<!-- /ai-dense -->

Enforcer changes are accepted only when rules, validators, registry
metadata, docs, fixtures, and tests move together.

Rule changes must include:

- A typed rule record in `enforcer-rules` with `ruleId`, title, severity,
  doc anchor, validator reference, triggers, and applies-to metadata.
- A routed rule doc section with fails, passes, fix recipe, validator, and
  fixture evidence.
- A `Validator` implementation that emits the exact rule ID.
- Pass and fail fixtures, plus a `cargo test` detection test — or an
  explicit review-only rule classification.
- Tests proving the diagnostic includes rule ID, file, line, detail, doc,
  snippet, and source.

Do not weaken immutable rules, bypass self-checks, or add config overrides
without waiver governance.
