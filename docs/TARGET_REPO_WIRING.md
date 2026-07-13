# Target Repository Wiring

<!-- ai-dense -->
```yaml
integration_order: "path scope -> diff scope -> workspace scope -> CI/MCP adoption"
scoped_commands: "check/scan/verify accept paths OR --base/--head OR --all"
verify_command: "verify combines a named mode with the same explicit scope grammar"
mcp: "call route first, then run the smallest returned validation scope"
rules: "use the released command help and rule documentation; do not depend on a checkout-local rules path"
release_boundary: "consumer automation must use a released binary and its tested command contract"
```
<!-- /ai-dense -->

This guide describes the current, supportable integration boundary. It is not
a release-installation promise: verify command and MCP-tool help from the
actual Enforcer build before wiring a repository or CI job.

## Current CLI contract

During local development, invoke the CLI through Cargo:

```powershell
cargo run -p enforcer-cli -- check path/to/file
cargo run -p enforcer-cli -- scan --base origin/main --head HEAD
cargo run -p enforcer-cli -- verify --mode local --all
```

`check`, `scan`, and `verify` accept exactly one scope:

- one or more paths;
- a `--base <ref>` and `--head <ref>` diff pair; or
- `--all` for the workspace.

`verify` also takes a named verification mode. The CLI does not treat a
configuration file, a profile, or a target root as a substitute for an
explicit scope. Consult `enforcer --help` and command help for the exact build
you install.

## MCP use

For an MCP-enabled coding assistant, call the installed Enforcer MCP route
tool first with the target root and exact files under review. Then run the
smallest returned validation scope, inspect structured diagnostics, and repair
reported conditions before widening the gate.

MCP coordination is optional. When enabled, its ledger and exact-file claims
belong to the Enforcer installation, not to the target repository.

When a rule needs an explanation, use the command help and rule documentation
shipped with the release under test. Do not make CI or an assistant depend on a
hardcoded checkout-local rule path.

## CI and release integration

Automated installation, package distribution, and CI adapters must use a
released binary and a documented command contract. Until that release surface
is available, build from this workspace and keep CI wiring explicit rather
than copying provisional commands into consumer repositories.

CI should run fresh validation and retain the report it produced. A stored
artifact may support review, but it never replaces the validation run itself.

## Integration sequence

1. Keep the repository's existing guards in place.
2. Prove Enforcer on a path scope, then a diff scope, then `--all`.
3. Add an MCP or CI integration only after those commands work in that
   environment.
4. Retire duplicated wrappers only after the replacement has equivalent,
   validated coverage.

Use the smallest honest scope while editing and a workspace gate before a
release or merge decision.
