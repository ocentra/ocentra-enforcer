# Source Shape Rules

## Covered Rules

- `SRC-1.1`: Source files must stay within configured shape limits: file length, function length, export count, class count, Rust function count, and Rust type count.
- `SRC-2.1`: File line budgets are hard limits for handwritten source.
- `SRC-2.2`: Function line budgets are hard limits.
- `SRC-2.3`: Export count budgets are hard limits.
- `SRC-2.4`: Type count budgets are hard limits.
- `SRC-2.5`: Class or struct count budgets are hard limits.
- `SRC-2.6`: Nesting depth budgets are hard limits.
- `SRC-2.7`: Branch count budgets are hard limits.
- `SRC-2.8`: Dumping-ground filenames such as `utils`, `helpers`, `common`, `misc`, `shared`, and `stuff` are forbidden.
- `SRC-2.9`: Comments containing `temporary`, `for now`, `hack`, or `quick fix` are forbidden in production source.
- `SRC-2.10`: Placeholder implementation markers are forbidden.
- `SRC-2.11`: Copied huge blocks are forbidden.
- `SRC-2.12`: Duplicate function names in one module are forbidden.
- `SRC-2.13`: Files importing many responsibility layers are forbidden.
- `SRC-2.14`: Public API exported from `internal/` modules is forbidden.
- `SRC-2.15`: Domain/core modules cannot import app, UI, adapter, platform, or infra layers.

## Enforcement

Run:

```bash
ocentra-enforcer check source-shape --root <repo>
```

Projects can tune `sourceShapePolicies` in `ocentra-enforcer.config.json`.

## Fails

- Source files exceed configured line, export, function, or complexity limits.
- Generated outputs are routed as handwritten source instead of generated artifacts.

## Passes

- Handwritten source stays small, owned, and reviewable.
- Generated or template-heavy files use explicit generated-file policy.

## Fix Recipe

1. Split handwritten source by ownership boundary.
2. Move tests out of production source and into test roots.
3. Configure generated-file routing for deterministic outputs.

## Validator

- scanner: `common/source-shape`
- command: `ocentra-enforcer check source-shape --root <repo> --files <changed-files>`
