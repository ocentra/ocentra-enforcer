# Release Policy

Releases must be cut from a green `main` branch after the local CI parity gate
and GitHub Actions pass on Linux, Windows, and macOS.

Release requirements:

- Tag releases with the package version.
- Publish only the explicit `package.json` files allowlist.
- Run tests, self-scan, policy integrity, rule coverage, MCP smoke, secret scan,
  dependency policy, and SBOM checks.
- Review generated schema artifacts before publishing.
- Prefer signed tags when the publishing environment supports signing.

No release may depend on local uncommitted proof output or ledger state.
