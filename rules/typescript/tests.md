# TypeScript Test Rules

## Covered Rules

- `TS-3.1`: Skipped, focused, or weak JavaScript/TypeScript tests are forbidden. This catches `.skip`, `.only`, and trivial assertions such as `expect(true).toBe(true)`.
- `TS-8.1`: `test.skip`, `describe.skip`, `it.only`, `test.only`, and `.todo` are forbidden.
- `TS-8.2`: `expect.anything()` and `expect.any(String|Number)` are forbidden.
- `TS-8.3`: `toBeTruthy`, `toBeDefined`, and `not.toThrow` are forbidden as primary assertions.
- `TS-8.4`: Empty TypeScript tests are forbidden.
- `TS-8.5`: TypeScript tests without assertions are forbidden.
- `TS-8.6`: Network calls are forbidden in unit tests.
- `TS-8.7`: Real timers are forbidden in deterministic tests.
- `TS-8.8`: Mocks, fakes, stubs, and spies are forbidden by default.
- `TS-8.9`: Snapshots cannot contain timestamps, UUIDs, or random IDs.
- `TS-8.10`: Decoder, codec, and schema tests must include invalid-input negative cases.

## Enforcement

Run scoped source validation and then execute the test command through the harness:

```bash
ocentra-enforcer scan --root <repo> --languages typescript,common --files <test-files>
ocentra-enforcer run --root <repo> --tool vitest -- npm test -- --reporter=json
```

## Fails

- Tests are skipped, focused, inline in production source, empty, or use weak assertions.
- Vitest/Jest output reports zero tests for a claimed proof.

## Passes

- Tests live in organized test roots and assert exact behavior or exact errors.
- Structured test output is stored as a proof artifact or harness diagnostic.

## Fix Recipe

1. Move inline test code into the configured test tree.
2. Replace weak assertions with exact checks.
3. Run the test command through Enforcer and inspect compact diagnostics.

## Validator

- scanner: `typescript/tests`
- command: `ocentra-enforcer scan --root <repo> --languages typescript,common --files <test-files>`
