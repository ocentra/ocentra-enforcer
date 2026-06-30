# Common Security Rules

## Covered Rules

- `SEC-1.1`: Inline secret-like assignments are forbidden across Rust, TypeScript, JavaScript, Python, and policy files.
- `SEC-1.2`: Sensitive file paths are forbidden in source scope. This catches `.env*` files except examples/templates, private key bundles, mobile service secrets, and similar credential files.

## Enforcement

Run:

```bash
ocentra-enforcer scan --root <repo> --languages common --files <changed-files>
```

CI adapters may also call GitHub secret scanning or a dedicated scanner such as Gitleaks. Enforcer keeps the local hard gate small and deterministic.
