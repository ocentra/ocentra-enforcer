# SBOM Rules

## Covered Rules

- `SBOM-1.1`: SBOM and dependency metadata generation must complete when requested.

## Enforcement

Run:

```bash
ocentra-enforcer check sbom --root <repo> --output target/security
```

Use `--dry-run` to validate the command path without writing generated artifacts.
