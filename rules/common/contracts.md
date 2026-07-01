# Contract Rules

## Covered Rules

- `CONTRACT-1.1`: Single-source contract values must not be copied across source. Import or derive from the configured owner contract.

## Enforcement

Run:

```bash
ocentra-enforcer check single-source-contracts --root <repo> --check-config scripts/check-single-source-contracts.json
```

The Enforcer command accepts legacy contract config shapes so projects can migrate without changing contract data first.

## Fails

- Contract constants are copied into multiple source files without a declared owner.
- Generated outputs drift from their source contract or schema.

## Passes

- One source owns the contract and every generated output can be traced back to it.
- Contract drift tests or generated-artifact checks prove output freshness.

## Fix Recipe

1. Pick the owning contract source.
2. Replace copied values with generated or imported values from that source.
3. Regenerate outputs and run the single-source contract validator.

## Validator

- scanner: `common/single-source-contracts`
- command: `ocentra-enforcer check single-source-contracts --root <repo> --check-config <config>`
