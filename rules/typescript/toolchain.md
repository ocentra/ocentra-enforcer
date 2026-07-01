# TypeScript Toolchain Rules

## Covered Rules

- `TS-5.1`: TypeScript compiler checks must run with `tsc --noEmit` for validation gates.
- `TS-5.2`: ESLint output must be consumed through JSON-formatted diagnostics when available.
- `TS-7.1`: TypeScript strict compiler mode is required; `strict: false` and disabled strict subflags are hard failures.
- `TS-7.2`: `noImplicitAny` must be enabled.
- `TS-7.3`: `strictNullChecks` must be enabled.
- `TS-7.4`: `noUncheckedIndexedAccess` must be enabled.
- `TS-7.5`: `exactOptionalPropertyTypes` must be enabled.
- `TS-7.6`: `noImplicitOverride` must be enabled.
- `TS-7.7`: `noPropertyAccessFromIndexSignature` must be enabled.
- `TS-7.8`: `useUnknownInCatchVariables` must be enabled.
- `TS-7.9`: `skipLibCheck` policy must be explicit.
- `TS-7.10`: A package-manager lockfile is required for TypeScript packages.
- `TS-7.11`: Loose npm versions (`^`, `~`, `*`, `latest`, git/file/link ranges) are forbidden.
- `TS-7.12`: CI and CI-like scripts must use `npm ci`, not `npm install`.
- `TS-7.13`: ESLint config must include no-floating-promises, no-explicit-any, and no-unsafe rules.
- `TS-7.14`: Zod dependencies are forbidden when the profile uses Effect Schema policy.
- `TS-7.15`: Duplicate package managers/lockfiles are forbidden by default.

## Enforcement

Prefer harnessed commands so Codex can query compact diagnostics instead of terminal dumps:

```bash
ocentra-enforcer run --root <repo> --tool tsc -- npx tsc --noEmit --pretty false
ocentra-enforcer run --root <repo> --tool eslint -- npx eslint . --format json
ocentra-enforcer runs diagnostics --root <repo> --limit 20
```

## Fails

- TypeScript compiler or ESLint output is ignored, parsed from a terminal wall, or weakened by suppressions.
- `tsconfig`/lint settings allow unchecked emit or bypass hard policy.

## Passes

- `tsc --noEmit` and ESLint JSON run through the harness and return compact diagnostics.
- Tool configs preserve strict type checking and no-bypass policy.

## Fix Recipe

1. Run compiler/linter commands through `ocentra-enforcer run`.
2. Fix typed diagnostics before source-shape or proof claims.
3. Keep tool configs strict and deterministic.

## Validator

- scanner: `typescript/toolchain`
- command: `ocentra-enforcer run --root <repo> --tool tsc -- npx tsc --noEmit --pretty false`
