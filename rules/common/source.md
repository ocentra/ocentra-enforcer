# Common Source Rules

## Covered Rules

- `SRC-1.2`: Placeholder implementation markers are forbidden in production source. This includes TODO/FIXME/TBD comments, placeholder/stub/fake/temporary wording in comments, `throw new Error("not implemented")`, Python `NotImplementedError`, Rust `todo!()`/`unimplemented!()`, and checked-in debug print macros.

## Enforcement

Run:

```bash
ocentra-enforcer check placeholder-implementation --root <repo> --files <changed-files>
```

The rule is for source that claims to be production-ready. Test fixtures and explicit test files are excluded by path.
