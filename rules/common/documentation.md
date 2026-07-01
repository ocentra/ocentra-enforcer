# Documentation Advisory Rules

## Covered Rules

- `DOC-1.1`: Exported or public API should have a short rustdoc, JSDoc, or Python docstring.

## Default Severity

Documentation rules are advisory by default. They should guide humans and agents
without blocking work unless a project profile explicitly upgrades them to
`error` or adds `warning` to `failOn`.

Projects that prefer minimal comments can disable this rule:

```json
{
  "rules": {
    "DOC-1.1": { "enabled": false }
  }
}
```

Projects that require public API docs can make it a hard gate:

```json
{
  "rules": {
    "DOC-1.1": { "severity": "error" }
  }
}
```

## Fails

- Public APIs, modules, or exported types are left unexplained when the profile requires documentation.
- Documentation becomes a replacement for executable policy instead of pointing to validators.

## Passes

- Docs explain ownership, boundaries, expected behavior, and validation without becoming a giant rulebook.
- Documentation rules remain advisory unless a project explicitly promotes them.

## Fix Recipe

1. Add concise ownership and behavior docs for public surfaces.
2. Route detailed docs through indexes instead of broad root documents.
3. Keep hard enforcement in validators and link docs to the matching rule IDs.

## Validator

- scanner: `common/documentation`
- command: `ocentra-enforcer scan --root <repo> --languages common --files <changed-docs-or-source>`
