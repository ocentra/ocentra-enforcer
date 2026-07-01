# Rule Registry Integrity Rules

## Covered Rules

- `ENF-1.1`: Every routed rule doc ID must exist in `rules/rules.json`, and each registry row must include required metadata.
- `ENF-1.2`: Every registry rule must point to an existing routed doc anchor.
- `ENF-1.3`: Every scanner-emitted or check-emitted rule ID must be registered.
- `ENF-1.4`: Every validator-backed rule marked as requiring fixture evidence must have test evidence.
- `ENF-1.5`: Rule IDs must stay in `rules/rule-id-lock.json`; removing or renumbering locked IDs fails.
- `ENF-1.6`: Rule IDs must be unique.
- `ENF-1.7`: Rule title/snippet metadata must not drift between registry and validator metadata.
- `ENF-1.8`: Every violation report must include `ruleId`, `title`, `file`, `line`, `detail`, `doc`, `snippet`, and `source`.
- `ENF-1.9`: JSON output and registry ordering must be deterministic.
- `ENF-1.10`: Human-readable output must be deterministic by normalized file, line, and rule ID.
- `DOCENF-1.1`: Routed rule docs must include `Covered Rules`, `Fails`, `Passes`, `Fix Recipe`, and `Validator`.
- `DOCENF-1.2`: Source rule docs must include fail and pass examples.
- `DOCENF-1.3`: Tagged code blocks in routed docs must stay parseable.
- `DOCENF-1.4`: Registry fix snippets must stay compact.
- `DOCENF-1.5`: Registry doc anchors must be stable lowercase markdown anchors.
- `DOCENF-1.6`: Immutable rule docs must use mandatory language.
- `DOCENF-1.7`: Docs must not make legacy aliases canonical.
- `DOCENF-1.8`: Docs must not describe the pack as Rust-only when multiple languages are registered.
- `DOCENF-1.9`: Advisory rule docs must explain profile promotion to error.
- `DOCENF-1.10`: Review and proof rules must name expected evidence.

## Fails

```json
{
  "rules": [
    {
      "id": "ENF-FAKE.1",
      "doc": "rules/common/missing.md#covered-rules"
    }
  ]
}
```

## Passes

```json
{
  "rules": [
    {
      "id": "ENF-1.2",
      "language": "common",
      "family": "harness",
      "severity": "error",
      "title": "Registry docs must point to stable anchors",
      "snippet": "Point each registry rule at an existing routed doc anchor.",
      "lockLevel": "immutable",
      "canDisable": false,
      "canDowngrade": false,
      "requiresFailFixture": false,
      "requiresPassFixture": false,
      "appliesTo": ["rules/**"],
      "triggers": ["registry", "doc"],
      "validator": "common/rule-coverage",
      "doc": "rules/common/registry.md#covered-rules"
    }
  ]
}
```

## Fix Recipe

1. Keep exactly one registry row per rule ID.
2. Add `title`, `snippet`, `lockLevel`, `canDisable`, `canDowngrade`, `requiresFailFixture`, and `requiresPassFixture` to every row.
3. Point `doc` to an existing markdown heading.
4. Register every rule emitted by scanner, check, CLI, or MCP code before using the ID.
5. Add fixture or test evidence when a validator-backed rule requires it.
6. Update `rules/rule-id-lock.json` intentionally when adding a rule.
7. Keep routed rule docs complete enough for an agent to fix failures without reading a monolithic rulebook.

## Validator

- scanner: `common/rule-coverage`
- implemented in: `src/checks.mjs`
- command: `ocentra-enforcer check rule-coverage --root <repo>`
