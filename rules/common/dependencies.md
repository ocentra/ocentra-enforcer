# Dependency Policy Rules

## Covered Rules

- `DEP-1.1`: Dependency security audits must pass. Enforcer runs `npm audit --audit-level=high` when `package-lock.json` exists and `cargo audit --deny warnings` when `Cargo.lock` exists.
- `DEP-1.2`: External npm package licenses must match project policy.

## Enforcement

Run:

```bash
ocentra-enforcer check dependency-policy --root <repo>
```

Projects can tune `allowedExternalLicenses` in `ocentra-enforcer.config.json`.
