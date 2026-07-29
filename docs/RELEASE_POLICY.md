# Release Policy

<!-- ai-dense -->
```yaml
gate: "green main after cargo build --workspace && cargo test --workspace on Linux/Windows/macOS + clippy -D warnings + fmt --check"
publish_matrix: "win/mac/linux incl. musl + apple-silicon, via GitHub Actions -> per-platform enforcer binary"
required_checks: "workspace tests, self-scan (dogfood), policy integrity, rule coverage, MCP smoke, secret scan, dependency policy, SBOM"
no_dependency_on: "local uncommitted proof output or ledger state"
```
<!-- /ai-dense -->

Releases must be cut from a green `main` branch after the CI-exact gate
(`cargo build --workspace && cargo test --workspace`, clippy `-D warnings`,
`cargo fmt --check`) passes on Linux, Windows, and macOS.

Release requirements:

- Tag releases with the crate/binary version.
- Publish the per-platform `enforcer` binary matrix (win/mac/linux incl.
  musl + apple-silicon) built by `cargo build --release`.
- Run the full workspace test suite, self-scan (native dogfood), policy
  integrity, rule coverage, MCP smoke, secret scan, dependency policy, and
  SBOM checks.
- Review generated artifacts (TS bindings, installer scripts) before
  publishing — including their pinned EOL, since a generated file's
  source-of-truth is its renderer, not file-type convention.
- Prefer signed tags when the publishing environment supports signing.

No release may depend on local uncommitted proof output or ledger state.
