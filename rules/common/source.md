# Common Source Rules

## Covered Rules

- `SRC-1.2`: Placeholder implementation markers are forbidden in production source. This includes TODO/FIXME/TBD comments, placeholder/stub/fake/temporary wording in comments, `throw new Error("not implemented")`, Python `NotImplementedError`, Rust `todo!()`/`unimplemented!()`, and checked-in debug print macros.

## Enforcement

Run:

```bash
ocentra-enforcer check placeholder-implementation --root <repo> --files <changed-files>
```

The rule is for source that claims to be production-ready. Test fixtures and explicit test files are excluded by path.

## Fails

- Production source contains placeholders, TODO-as-implementation, fake returns, or validation bypasses.
- Code claims completion while leaving behavior as a stub.

## Passes

- Source either implements behavior or explicitly lives in a test fixture/mock boundary.
- Placeholder markers are tracked as plan debt, not accepted production code.

## Fix Recipe

1. Replace placeholder behavior with real implementation.
2. Move intentional fixtures into test paths.
3. Re-run source and validation-bypass checks.

## Validator

- scanner: `common/source`
- command: `ocentra-enforcer check placeholder-implementation --root <repo> --files <changed-files>`
