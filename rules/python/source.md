# Python Source Rules

## Covered Rules

- `PY-1.1`: Broad lint suppressions are forbidden. Do not use `# noqa` or `pylint: disable` as a bypass.
- `PY-1.2`: `# type: ignore` is forbidden unless a future profile explicitly allows a reviewed exception.
- `PY-1.3`: Naked domain string aliases are forbidden. Do not model identifiers, paths, keys, names, slugs, routes, or status values as `Alias = str`; use `typing.NewType`, a validated dataclass/value object, or a project-owned schema boundary.
- `PY-4.1`: `Any` is forbidden. Use precise types, protocols, or validated boundary models.
- `PY-4.2`: Production functions must have parameter and return annotations.
- `PY-4.3`: Production functions must have explicit return annotations.
- `PY-4.4`: `dict[str, Any]` domain APIs are forbidden.
- `PY-4.5`: raw `str` ID aliases are forbidden.
- `PY-4.6`: raw `str`/`int`/`bool` domain parameters are forbidden for ID/count/state names.
- `PY-4.7`: `TypedDict` domain models are forbidden outside boundaries.
- `PY-4.8`: Pydantic `BaseModel` domain models are forbidden by default.
- `PY-4.9`: Optional field soup is forbidden in domain models.
- `PY-4.10`: Mutable default arguments are forbidden.
- `PY-4.11`: `except Exception` is forbidden. Catch specific exceptions.
- `PY-4.12`: Bare `except:` is forbidden.
- `PY-4.13`: `pass` inside exception handlers is forbidden.
- `PY-4.14`: `print(...)` debugging is forbidden in source.
- `PY-4.15`: Runtime `assert` statements are forbidden in production Python source.
- `PY-4.16`: `eval`, `exec`, and `compile` are forbidden.
- `PY-4.17`: `subprocess(..., shell=True)` is forbidden.
- `PY-4.18`: `os.system` is forbidden.
- `PY-4.19`: `pickle.loads` is forbidden.
- `PY-4.20`: `yaml.load` must use `SafeLoader` or `safe_load`.
- `PY-4.21`: global mutable state is forbidden.
- `PY-4.22`: dynamic imports are forbidden in domain code.
- `PY-4.23`: Naive `datetime.now()` and `datetime.utcnow()` calls are forbidden.
- `PY-4.24`: `time.sleep` is forbidden in async code.
- `PY-4.25`: `requests.*(...)` calls must pass an explicit `timeout=`.
- `PY-4.26`: `asyncio.create_task` results must be tracked.
- `PY-4.27`: coroutine-like calls must be awaited or returned.
- `PY-4.28`: parent-relative imports are forbidden.
- `PY-4.29`: Wildcard imports are forbidden.
- `PY-4.30`: `from module import *` is forbidden.
- `PY-4.31`: dumping-ground module names like `utils.py`, `helpers.py`, and `common.py` are forbidden.
- `PY-4.32`: dataclass value objects must use `frozen=True` and `slots=True`.
- `PY-4.33`: `NamedTuple`/tuple domain records are forbidden.
- `PY-4.34`: raw JSON `dict` domain inputs are forbidden.
- `PY-4.35`: environment reads must stay in config boundaries.

## Examples

Fails:

```python
from typing import Any
from module import *

def fetch(url, headers={}):
    try:
        print(url)
        return requests.get(url).json()
    except Exception:
        return {}
```

Passes:

```python
def fetch_json(url: Url, headers: Mapping[str, str] | None = None) -> JsonObject:
    response = requests.get(str(url), headers=headers, timeout=5)
    return JsonObject.parse(response.json())
```

## Fix Recipe

1. Add exact annotations to every production function.
2. Replace `Any` with protocol, dataclass, TypedDict, or schema-owned types.
3. Replace mutable defaults with `None` and construct inside the function.
4. Catch exact exception types and return modeled errors.
5. Use structured logging, shell-free subprocess argv, explicit HTTP timeouts, and explicit imports.

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

## Fails

- Python source uses broad ignores, `Any`, untyped definitions, mutable defaults, broad exceptions, print debugging, or unsafe subprocess calls.
- Domain values are represented as raw strings where schema brands are required.

## Passes

- External data is decoded at boundaries and domain code uses precise typed values.
- Ruff, Pyright, mypy, or equivalent structured outputs are ingested through the harness.

## Fix Recipe

1. Replace ignores with real types or narrow local fixes.
2. Move parsing to a boundary and return typed domain values.
3. Run Python scanner and configured native tools through the harness.

## Validator

- scanner: `python/source`
- command: `ocentra-enforcer scan --root <repo> --languages python,common --files <changed-files>`
