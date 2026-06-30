# Python Source Rules

## Covered Rules

- `PY-1.1`: Broad lint suppressions are forbidden. Do not use `# noqa` or `pylint: disable` as a bypass.
- `PY-1.2`: `# type: ignore` is forbidden unless a future profile explicitly allows a reviewed exception.
- `PY-1.3`: Naked domain string aliases are forbidden. Do not model identifiers, paths, keys, names, slugs, routes, or status values as `Alias = str`; use `typing.NewType`, a validated dataclass/value object, or a project-owned schema boundary.

## Enforcement

Run:

```bash
ocentra-enforcer scan --root <repo> --languages python,common --files <changed-files>
ocentra-enforcer check no-naked-domain-strings --root <repo> --files <changed-files>
```

Use Ruff, Pyright, or mypy through the harness when available:

```bash
ocentra-enforcer run --root <repo> --tool ruff -- ruff check . --output-format json
ocentra-enforcer runs last-failure --root <repo>
```
