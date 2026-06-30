# TypeScript Test Rules

## Covered Rules

- `TS-3.1`: Skipped, focused, or weak JavaScript/TypeScript tests are forbidden. This catches `.skip`, `.only`, and trivial assertions such as `expect(true).toBe(true)`.

## Enforcement

Run scoped source validation and then execute the test command through the harness:

```bash
ocentra-enforcer scan --root <repo> --languages typescript,common --files <test-files>
ocentra-enforcer run --root <repo> --tool vitest -- npm test -- --reporter=json
```

