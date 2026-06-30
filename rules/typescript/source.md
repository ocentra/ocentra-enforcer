# TypeScript Source Rules

## Covered Rules

- `TS-1.1`: TypeScript and JavaScript re-exports are forbidden. Do not create barrel files with `export *`, `export { X } from`, `export type { X } from`, or default re-export shims.
- `TS-1.2`: Direct Zod usage is forbidden. Use Effect Schema through domain-owned schemas; do not import `zod`, expose `Zod*` types, use `zodResolver`, or keep stale `schema/zod` paths.
- `TS-1.3`: Naked domain string aliases and manual string brands are forbidden. Use Effect Schema brands and decode helpers instead of `type FooId = string` or `string & { readonly __brand: ... }`.
- `TS-2.1`: Suppression comments are forbidden. Do not use `eslint-disable`, `@ts-ignore`, `@ts-expect-error`, `@ts-nocheck`, `biome-ignore`, or equivalent bypass comments.

## ESLint Adapter Rules

Enforcer also exports `ocentra-enforcer/eslint-rules` for projects that want the same checks inside ESLint:

- `no-app-string-literals`
- `no-naked-domain-string-types`
- `no-runtime-string-types`

## Enforcement

Run:

```bash
ocentra-enforcer scan --root <repo> --languages typescript,common --files <changed-files>
```

Use `tsc --noEmit` and ESLint JSON through the harness for compiler/lint output:

```bash
ocentra-enforcer run --root <repo> --tool tsc -- npx tsc --noEmit --pretty false
ocentra-enforcer runs last-failure --root <repo>
```
