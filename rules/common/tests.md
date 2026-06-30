# Common Test Rules

## Covered Rules

- `TEST-1.1`: Test doubles are forbidden by default. Avoid mocks, fakes, stubs, spies, and common test-double packages unless a project profile explicitly allows them.
- `TEST-1.2`: Weak assertions are forbidden. Avoid truthiness/existence-only checks and broad matcher placeholders.
- `TEST-1.3`: Hidden, focused, todo, or ignored tests are forbidden. Rust `#[ignore]` is a hard failure.
- `TEST-2.1`: Source workspaces must have test scaffolds. Packages/apps with `src/` need test files, and Rust crates need unit or integration tests.

## Enforcement

Run:

```bash
ocentra-enforcer scan --root <repo> --languages common --files <changed-files>
```

The default is intentionally strict: tests should exercise real domain contracts, parsers, fixtures, and local services rather than bypassing the behavior under test.
