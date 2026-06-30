# Contract Rules

## Covered Rules

- `CONTRACT-1.1`: Single-source contract values must not be copied across source. Import or derive from the configured owner contract.

## Enforcement

Run:

```bash
ocentra-enforcer check single-source-contracts --root <repo> --check-config scripts/check-single-source-contracts.json
```

The Enforcer command accepts the existing Ocentra Parent contract config shape so projects can migrate without changing contract data first.
