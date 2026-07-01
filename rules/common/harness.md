# Harness Rules

## Covered Rules

- `HAR-1.1`: Commands run through the Enforcer harness must produce a compact run summary and diagnostics. Failed commands are represented as structured diagnostics even when the tool emits unstructured text.
- `HAR-2.1`: Every run records run ID, command, cwd, start/end time, and exit code.
- `HAR-2.2`: Raw stdout/stderr artifacts are redacted and bounded before consumers read them.
- `HAR-2.3`: Diagnostics are sorted deterministically.
- `HAR-2.4`: Malformed tool output becomes a parser diagnostic instead of a crash.
- `HAR-2.5`: Cargo/rustc JSON diagnostics normalize to rule, file, line, and severity.
- `HAR-2.6`: ESLint JSON diagnostics normalize.
- `HAR-2.7`: Python tool diagnostics normalize.
- `HAR-2.8`: SARIF diagnostics normalize.
- `HAR-2.9`: Last-failure queries return compact diagnostics without terminal dumps.
- `HAR-2.10`: Artifact reads cannot escape `.enforce`.
- `HAR-2.11`: Pinned proof or PR-ready runs survive pruning.
- `HAR-2.12`: Failed commands become failing gates.
- `HAR-2.13`: Harness JSON output has generated schema artifacts.
- `HAR-2.14`: Human-visible output redacts secrets.
- `HAR-2.15`: Harness command execution uses argv arrays with shell disabled by default.

## Storage

Each target repo stores harness output under:

```text
.enforce/runs/<runId>/
.enforce/db/
```

Raw stdout/stderr are retained as artifacts. Agents must first query compact summaries:

```bash
ocentra-enforcer runs last-failure --root <repo> --json
ocentra-enforcer runs diagnostics --root <repo> --limit 20
```

## Fails

- Agents rely on raw terminal dumps when compact diagnostics are available.
- Run reports omit rule IDs, severity, file, line, source, or artifact links.
- Parser errors throw out of the harness instead of producing `HAR-2.4`.
- `readArtifact` returns a path outside the target repo storage root.
- A pruned run deletes a pinned proof bundle.

## Passes

- Raw stdout/stderr are saved, while MCP/CLI returns bounded structured summaries by default.
- Every validation report is complete enough for an agent to repair without rereading a terminal wall.
- SARIF, Cargo, ESLint, Pyright, Ruff, and pytest-like output becomes compact diagnostics.
- `last-failure` returns the newest failed run plus bounded diagnostics.
- Secret-like strings are redacted before artifact text is returned.

## Fix Recipe

1. Run tools through `ocentra-enforcer run` unless the tool is already structured.
2. Query `runs last-failure` or `runs diagnostics` before opening raw artifacts.
3. Fix missing report metadata in the producing check, not in the consumer.

## Validator

- scanner: `common/harness`
- command: `ocentra-enforcer check harness-contracts --root <repo>`
