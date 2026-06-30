# Python Test Rules

## Covered Rules

- `PY-2.1`: Skipped or focused Python tests are forbidden. This catches `pytest.mark.skip`, `pytest.skip`, `pytest.mark.xfail`, focus markers, and `unittest.skip`.

## Enforcement

Run:

```bash
ocentra-enforcer scan --root <repo> --languages python,common --files <test-files>
ocentra-enforcer run --root <repo> --tool pytest -- pytest --junitxml=.ocentra-enforcer/pytest.xml
```

