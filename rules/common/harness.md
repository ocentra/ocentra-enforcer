# Harness Rules

## Covered Rules

- `HAR-1.1`: Commands run through the Enforcer harness must produce a compact run summary and diagnostics. Failed commands are represented as structured diagnostics even when the tool emits unstructured text.

## Storage

Each target repo stores harness output under:

```text
.ocentra-enforcer/runs/<runId>/
.ocentra-enforcer/db/
```

Raw stdout/stderr are retained as artifacts. Agents should first query compact summaries:

```bash
ocentra-enforcer runs last-failure --root <repo> --json
ocentra-enforcer runs diagnostics --root <repo> --limit 20
```

