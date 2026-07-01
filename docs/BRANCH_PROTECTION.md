# Branch Protection

## Required Checks

Protected branches must require the Ocentra Enforcer workflow before merge:

- `Ocentra Enforcer / ocentra-enforcer (ubuntu-latest)`
- `Ocentra Enforcer / ocentra-enforcer (windows-latest)`
- `Ocentra Enforcer / ocentra-enforcer (macos-latest)`

The workflow runs `npm run ci:local`, which is the local/CI parity gate for tests,
MCP smoke checks, self-scan, policy integrity, secret scan, dependency policy,
SBOM, and rule coverage.

## Branch Rules

- Require pull requests before merge.
- Require the Enforcer checks above to pass.
- Require branches to be up to date before merge.
- Do not allow bypassing required checks for rule, schema, scanner, MCP, CI, or package changes.
