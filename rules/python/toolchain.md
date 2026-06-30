# Python Toolchain Rules

## Covered Rules

- `PY-3.1`: Ruff diagnostics should be captured in structured output when available.
- `PY-3.2`: Pyright or mypy type diagnostics should be captured through the harness.

## Enforcement

Run:

```bash
ocentra-enforcer run --root <repo> --tool ruff -- ruff check . --output-format json
ocentra-enforcer run --root <repo> --tool pyright -- pyright --outputjson
ocentra-enforcer run --root <repo> --tool mypy -- mypy .
```

