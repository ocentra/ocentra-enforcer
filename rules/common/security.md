# Common Security Rules

## Covered Rules

- `SEC-1.1`: Inline secret-like assignments are forbidden across Rust, TypeScript, JavaScript, Python, and policy files.
- `SEC-1.2`: Sensitive file paths are forbidden in source scope. This catches `.env*` files except examples/templates, private key bundles, mobile service secrets, and similar credential files.
- `SEC-2.1`: GitHub tokens are forbidden.
- `SEC-2.2`: AWS access keys are forbidden.
- `SEC-2.3`: Google service-account JSON markers are forbidden.
- `SEC-2.4`: Azure credential assignments are forbidden.
- `SEC-2.5`: Slack and Discord tokens are forbidden.
- `SEC-2.6`: JWT-looking bearer tokens are forbidden.
- `SEC-2.7`: Private key blocks are forbidden.
- `SEC-2.8`: npm, PyPI, and Cargo registry tokens are forbidden.
- `SEC-2.9`: Stripe keys are forbidden.
- `SEC-2.10`: High-entropy secret assignments are forbidden.
- `SEC-2.11`: `.env.example` may contain placeholders only.
- `SEC-2.12`: `.env.template` may contain placeholders only.
- `SEC-2.13`: Secrets are forbidden in snapshots and test artifacts.
- `SEC-2.14`: Fixture credentials require explicit fake/example markers.
- `SEC-2.15`: Secret diagnostics must redact matched values.
- `SEC-2.16`: Secret scanner commands must emit SARIF or structured output.
- `SEC-2.17`: Gitleaks output must be normalized to `SEC-*` findings.
- `SEC-2.18`: TruffleHog output must be normalized to `SEC-*` findings.
- `SEC-2.19`: Committed SSH key files are forbidden.
- `SEC-2.20`: Mobile secret config files are forbidden.

## Enforcement

Run:

```bash
ocentra-enforcer scan --root <repo> --languages common --files <changed-files>
```

CI adapters may also call GitHub secret scanning or a dedicated scanner such as Gitleaks. Enforcer keeps the local hard gate small and deterministic.

## Fails

- Source or config contains obvious secrets, insecure random usage, or shell execution bypasses.
- Secret scanning is skipped for staged or changed files.

## Passes

- Secrets are stored outside source and local/CI scanners inspect changed files.
- Security findings are mapped to rule IDs and compact diagnostics.

## Fix Recipe

1. Remove the secret or insecure call from source.
2. Rotate exposed credentials when needed.
3. Re-run staged and repository security checks.

## Validator

- scanner: `common/security`
- command: `ocentra-enforcer scan --root <repo> --languages common --files <changed-files>`
