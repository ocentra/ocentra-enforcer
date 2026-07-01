# Portability Rules

## Covered Rules

- `PORT-1.1`: Platform-specific script commands must be guarded. Windows-only invocations such as `cmd /c npm` must sit behind an explicit `process.platform` check or be replaced by a cross-platform command helper.

## Enforcement

Run:

```bash
ocentra-enforcer scan --root <repo> --languages common --files scripts
```

This keeps reusable enforcement scripts portable across Windows, macOS, Linux, local hooks, CI, and agent harnesses.

## Fails

- Scripts depend on one shell, one path separator, or one OS-specific command shape.
- Hooks or CI call commands that cannot run on Windows, macOS, and Linux.

## Passes

- Paths are normalized and commands work through Node or explicit cross-platform adapters.
- CI validates the same command family on all supported operating systems.

## Fix Recipe

1. Replace shell-specific glue with Node APIs or platform adapters.
2. Normalize paths at API boundaries.
3. Add or run cross-platform smoke coverage for the changed command.

## Validator

- scanner: `common/portability`
- command: `ocentra-enforcer scan --root <repo> --languages common --files <changed-scripts>`
