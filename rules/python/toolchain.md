# Python Toolchain Rules

## Covered Rules

- `PY-3.1`: Ruff diagnostics must be captured in structured output when available.
- `PY-3.2`: Pyright or mypy type diagnostics must be captured through the harness.
- `PY-5.1`: Python projects require `pyproject.toml` as the tool policy owner.
- `PY-5.2`: Ruff configuration is required in `pyproject.toml`.
- `PY-5.3`: Pyright or mypy configuration is required.
- `PY-5.4`: Python type checker strict mode is required.
- `PY-5.5`: Ruff diagnostics must be emitted as JSON through the harness.
- `PY-5.6`: Pyright or mypy diagnostics must be emitted as structured output through the harness.
- `PY-5.7`: `uv.lock`, `poetry.lock`, `pdm.lock`, or configured equivalent is required.
- `PY-5.8`: Unpinned `requirements.txt` entries are forbidden.
- `PY-5.9`: Direct git dependencies are forbidden.
- `PY-5.10`: Local path dependencies require waiver.

## Enforcement

Run:

```bash
ocentra-enforcer run --root <repo> --tool ruff -- ruff check . --output-format json
ocentra-enforcer run --root <repo> --tool pyright -- pyright --outputjson
ocentra-enforcer run --root <repo> --tool mypy -- mypy .
```

## Fails

- Ruff, Pyright, mypy, pytest, Bandit, or pip-audit findings are ignored or left as raw terminal dumps.
- Tool outputs cannot be associated with files, severities, and rule IDs.

## Passes

- Native Python tool output is captured as structured artifacts and compact diagnostics.
- Tool availability and profile expectations are declared in project config.

## Fix Recipe

1. Run native Python tools through `ocentra-enforcer run`.
2. Prefer JSON or JUnit output formats where available.
3. Fix the first compact diagnostic group before opening raw logs.

## Validator

- scanner: `python/toolchain`
- command: `ocentra-enforcer run --root <repo> --tool pyright -- pyright --outputjson`
