# Common Test Rules

## Covered Rules

- `TEST-1.1`: Test doubles are forbidden by default. Avoid mocks, fakes, stubs, spies, and common test-double packages unless a project profile explicitly allows them.
- `TEST-1.2`: Weak assertions are forbidden. Avoid truthiness/existence-only checks and broad matcher placeholders.
- `TEST-1.3`: Hidden, focused, todo, or ignored tests are forbidden. Rust `#[ignore]` is a hard failure.
- `TEST-2.1`: Source workspaces must have test scaffolds. Packages/apps with `src/` need test files, and Rust crates need organized tests under `tests/`.
- `TEST-2.2`: Tests must live in organized test roots. Do not hide unit tests inside production `src/` files, including Rust `#[cfg(test)] mod tests`, TypeScript/JavaScript `describe`/`it`/`test` blocks, or Python `def test_*` functions.

## Enforcement

Run:

```bash
ocentra-enforcer scan --root <repo> --languages common --files <changed-files>
```

The default is intentionally strict: tests must exercise real domain contracts, parsers, fixtures, and local services rather than bypassing the behavior under test.

Unit tests are allowed, but they must still live in an organized test tree.
Inline test modules make source files larger, hide coverage ownership, and let
agents treat implementation and validation as one blob. Put unit, integration,
contract, and proof tests under explicit test roots so routing, ownership, and
CI gates can reason about them.

## Fails

- Tests are skipped, focused, empty, inline in production source, or built on weak assertions.
- Test doubles replace the behavior that must be validated.

## Passes

- Tests live in organized test roots and assert exact outcomes or exact errors.
- Required test directories contain real tests, not only placeholders.

## Fix Recipe

1. Move inline tests into the nearest organized test tree.
2. Replace weak assertions with exact values, errors, or contract checks.
3. Remove skipped/focused tests or mark unavailable proof explicitly.

## Validator

- scanner: `common/tests`
- command: `ocentra-enforcer check required-tests --root <repo> --files <changed-files>`
