# Source Shape Rules

## Covered Rules

- `SRC-1.1`: Source files must stay within configured shape limits: file length, function length, export count, class count, Rust function count, and Rust type count.

## Enforcement

Run:

```bash
ocentra-enforcer check source-shape --root <repo>
```

Projects can tune `sourceShapePolicies` in `ocentra-enforcer.config.json`.
