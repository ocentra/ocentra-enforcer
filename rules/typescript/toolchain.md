# TypeScript Toolchain Rules

## Covered Rules

- `TS-5.1`: TypeScript compiler checks must run with `tsc --noEmit` for validation gates.
- `TS-5.2`: ESLint output should be consumed through JSON-formatted diagnostics when available.

## Enforcement

Prefer harnessed commands so Codex can query compact diagnostics instead of terminal dumps:

```bash
ocentra-enforcer run --root <repo> --tool tsc -- npx tsc --noEmit --pretty false
ocentra-enforcer run --root <repo> --tool eslint -- npx eslint . --format json
ocentra-enforcer runs diagnostics --root <repo> --limit 20
```

