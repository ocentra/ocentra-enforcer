# Python Test Rules

## Covered Rules

- `PY-2.1`: Skipped or focused Python tests are forbidden. This catches `pytest.mark.skip`, `pytest.skip`, `pytest.mark.xfail`, focus markers, and `unittest.skip`.
- `PY-6.1`: `pytest.mark.skip`, `skipif`, and `xfail` require an explicit waiver.
- `PY-6.2`: Weak Python assertions are forbidden. Assert concrete values, errors, and state transitions.
- `PY-6.3`: empty Python tests are forbidden.
- `PY-6.4`: Python tests without assertions are forbidden.
- `PY-6.5`: monkeypatch and mocks are forbidden by default.
- `PY-6.6`: network access is forbidden in unit tests.
- `PY-6.7`: sleep-based Python tests are forbidden.
- `PY-6.8`: Validator, parser, decoder, and normalizer tests must include invalid-input coverage.
- `PY-6.9`: Exception-path tests must use `pytest.raises` or equivalent explicit error assertions.
- `PY-6.10`: Parser and normalizer tests must include property-based Hypothesis coverage.

## Enforcement

Run:

```bash
ocentra-enforcer scan --root <repo> --languages python,common --files <test-files>
ocentra-enforcer run --root <repo> --tool pytest -- pytest --junitxml=.ocentra-enforcer/pytest.xml
```

## Fails

- Python tests are skipped, focused, empty, inline in production modules, or only assert truthiness.
- Pytest output reports zero collected tests for a claimed proof.

## Passes

- Tests live under explicit test roots and produce structured pytest/JUnit evidence.
- Claims include fresh test artifacts for the current commit and scope.

## Fix Recipe

1. Move inline tests into `tests/` or the configured test root.
2. Replace weak assertions with exact expected values or exceptions.
3. Run pytest through the harness and query compact diagnostics.

## Validator

- scanner: `python/tests`
- command: `ocentra-enforcer scan --root <repo> --languages python,common --files <test-files>`
