# Branch Protection

<!-- ai-dense -->
```yaml
required_checks: "Enforcer / enforcer-ci (ubuntu-latest | windows-latest | macos-latest)"
ci_gate: "cargo build --workspace && cargo test --workspace, clippy -D warnings, fmt --check, self-scan, policy integrity, secret scan, dependency policy, SBOM, rule coverage"
rule: "check contexts resolve symbolically, never a hardcoded stale context name -- see x04 verifier"
```
<!-- /ai-dense -->

## Required Checks

Protected branches must require the enforcer CI workflow before merge:

- `Enforcer / enforcer-ci (ubuntu-latest)`
- `Enforcer / enforcer-ci (windows-latest)`
- `Enforcer / enforcer-ci (macos-latest)`

The workflow runs the CI-exact local/CI parity gate (`cargo build --workspace
&& cargo test --workspace`, clippy `-D warnings`, `cargo fmt --check`) plus
MCP smoke checks, self-scan, policy integrity, secret scan, dependency
policy, SBOM, and rule coverage.

## Branch Rules

- Require pull requests before merge.
- Require the enforcer checks above to pass.
- Require branches to be up to date before merge.
- Disallow admin bypass and force-push on the protected branch.
- Do not allow bypassing required checks for rule, schema, scanner, MCP, CI,
  or package changes.
