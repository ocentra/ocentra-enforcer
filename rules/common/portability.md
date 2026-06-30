# Portability Rules

## Covered Rules

- `PORT-1.1`: Platform-specific script commands must be guarded. Windows-only invocations such as `cmd /c npm` must sit behind an explicit `process.platform` check or be replaced by a cross-platform command helper.

## Enforcement

Run:

```bash
ocentra-enforcer scan --root <repo> --languages common --files scripts
```

This keeps reusable enforcement scripts portable across Windows, macOS, Linux, local hooks, CI, and agent harnesses.
