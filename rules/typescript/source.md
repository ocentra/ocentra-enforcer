# TypeScript Source Rules

## Covered Rules

- `TS-1.1`: TypeScript and JavaScript re-exports are forbidden. Do not create barrel files with `export *`, `export { X } from`, `export type { X } from`, or default re-export shims.
- `TS-1.2`: Direct Zod usage is forbidden. Use Effect Schema through domain-owned schemas; do not import `zod`, expose `Zod*` types, use `zodResolver`, or keep stale `schema/zod` paths.
- `TS-1.3`: Naked domain string aliases and manual string brands are forbidden. Use Effect Schema brands and decode helpers instead of `type FooId = string` or `string & { readonly __brand: ... }`.
- `TS-2.1`: Suppression comments are forbidden. Do not use `eslint-disable`, `@ts-ignore`, `@ts-expect-error`, `@ts-nocheck`, `biome-ignore`, or equivalent bypass comments.
- `TS-4.1`: Configured import boundaries are hard gates. Do not cross package/layer/module boundaries unless the target repo config explicitly allows that import.
- `TS-6.1`: `any` is forbidden. Use precise types, `unknown` at external boundaries, and Effect Schema decoders before data enters domain code.
- `TS-6.2`: `unknown` cannot escape decoder/boundary code. Decode before returning or storing domain values.
- `TS-6.3`: Type assertions are forbidden. Replace `as Foo` and `as unknown as Foo` with real narrowing, constructors, or schema decoding.
- `TS-6.4`: Double assertions are forbidden. Replace `as unknown as Foo` with schema decoding or control-flow narrowing.
- `TS-6.5`: Non-null assertions are forbidden. Prove presence with control flow or return a modeled error.
- `TS-6.6`: Definite assignment assertions are forbidden. Initialize fields explicitly.
- `TS-6.7`: Raw string domain aliases are forbidden. Use Effect Schema brands instead of `type FooId = string`.
- `TS-6.8`: Raw number domain IDs/counts/durations are forbidden. Use branded numeric value objects.
- `TS-6.9`: Raw boolean domain parameters are forbidden. Use explicit state/value objects.
- `TS-6.10`: `Record<string, Domain>` APIs are forbidden. Use branded key types or domain-owned maps.
- `TS-6.11`: `Map<string, Domain>` APIs are forbidden. Use branded key maps.
- `TS-6.12`: `string[]` domain APIs are forbidden. Use typed collections or branded values.
- `TS-6.13`: Default exports are forbidden. Use named exports from owning modules.
- `TS-6.14`: Index barrels are forbidden. Do not use `index.ts` or `index.js` to re-export from other modules.
- `TS-6.15`: Namespace declarations are forbidden. Use modules and explicit imports.
- `TS-6.16`: Enums are forbidden by default. Use union literals or configured enum policy.
- `TS-6.17`: `declare global` is forbidden outside type-owner files.
- `TS-6.18`: `process.env` reads are forbidden outside config/environment boundary files.
- `TS-6.19`: `JSON.parse` is forbidden outside schema/decoder/boundary files.
- `TS-6.20`: Raw `Date` domain APIs are forbidden. Use branded time values.
- `TS-6.21`: `Promise<any>` and `Promise<unknown>` are forbidden. Return precise promise types.
- `TS-6.22`: Floating promises are forbidden. Await, return, or explicitly route async work through a tracked task boundary.
- `TS-6.23`: Empty `.catch(() => {})` handlers are forbidden. Handle failures explicitly.
- `TS-6.24`: `console.*` logging is forbidden in source. Use project logging domains or structured diagnostics.
- `TS-6.25`: Throwing string errors is forbidden. Throw typed `Error` objects or return modeled domain errors.
- `TS-6.26`: `return null` is forbidden in domain APIs. Use explicit option/result values.
- `TS-6.27`: `undefined` as domain state is forbidden. Model absence explicitly.
- `TS-6.28`: Optional domain fields are forbidden by default. Use explicit state unions.
- `TS-6.29`: `Partial<T>` is forbidden in domain logic. Use explicit patch/input types.
- `TS-6.30`: `Record<string, unknown>` command/event payloads are forbidden. Decode payloads first.
- `TS-6.31`: Real timer sleeps are forbidden by default. Use controlled clocks/events.
- `TS-6.32`: Dynamic imports are forbidden in domain code. Use static imports.
- `TS-6.33`: `child_process` is forbidden outside script boundaries.
- `TS-6.34`: Dynamic code execution via `eval`/`Function` is forbidden.
- `TS-6.35`: Spreading raw DTOs into domain objects is forbidden. Map fields explicitly.
- `TS-6.36`: Spreading `any` into domain objects is forbidden. Decode and construct explicitly.
- `TS-6.37`: Exported functions require explicit return types.
- `TS-6.38`: Exported object literals cannot be inferred public APIs.
- `TS-6.39`: Use `const` unless reassignment is required.
- `TS-6.40`: Mutating imported/shared objects is forbidden.

## Examples

Fails:

```ts
export default function parse(raw: string): any {
  const data = JSON.parse(raw) as User;
  console.log(process.env.API_URL);
  if (!data.id) throw "missing id";
  return data.id!;
}
```

Passes:

```ts
export function parseUser(raw: unknown): User {
  return UserSchema.pipe(Schema.decodeUnknownSync)(raw);
}
```

## Fix Recipe

1. Keep raw input at a boundary.
2. Decode with Effect Schema or a project-owned parser.
3. Return branded/domain types from the boundary.
4. Use named exports and structured logging.
5. Delete casts, non-null assertions, and string throws instead of suppressing them.

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

## Fails

- TypeScript source uses re-exports, `any`, unsafe casts, non-null assertions, raw env/JSON reads, console debugging, or naked string domain aliases.
- Generated artifacts are treated as handwritten source instead of routed generated outputs.

## Passes

- Boundary code decodes external data with Effect Schema and exports named domain-owned APIs.
- Native `tsc` and ESLint diagnostics are captured through the harness when configured.

## Fix Recipe

1. Replace raw boundary operations with schema decoders.
2. Remove casts, non-null assertions, default exports, and barrel exports.
3. Route generated files through generated-artifact policy.

## Validator

- scanner: `typescript/source`
- command: `ocentra-enforcer scan --root <repo> --languages typescript,common --files <changed-files>`
