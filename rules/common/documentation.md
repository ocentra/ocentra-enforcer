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
