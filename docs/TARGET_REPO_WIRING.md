# Target Repo Wiring

Target repos should not copy Ocentra Enforcer source. They should call the
installed enforcer with an explicit `root`.

## Dry-Run First

From anywhere:

```powershell
ocentra-enforcer init --root C:/path/to/target-repo --profile strict --adapters codex,mcp,precommit,github-actions --dry-run
```

Review the file plan before writing anything.

## What Init Is Supposed To Write

The default adapter set plans:

- `ocentra-enforcer.config.json`, tiny target repo config.
- `.mcp.json`, project MCP config pointing to the external enforcer path.
- `.git/hooks/pre-commit`, plain Git pre-commit hook.
- `.github/workflows/ocentra-enforcer.yml`.
- `.github/workflows/codeql.yml`.
- `.github/workflows/dependency-policy.yml`.
- `.github/workflows/secret-scan.yml`.
- `.github/workflows/sbom.yml`.

Husky is not default. Add `husky` only when requested or when the target repo
already uses Husky.

## Minimal Manual Wiring

If you do not want generated files yet, create only a target repo script or doc
that calls:

```powershell
ocentra-enforcer scan --root . --profile strict --files Cargo.toml
ocentra-enforcer scan --root . --profile strict --languages typescript,python,common --files src tests
ocentra-enforcer check no-zod-source --root . --profile strict --files src/index.ts
ocentra-enforcer check validation-bypass --root . --profile strict --files src/index.ts
ocentra-enforcer check weak-assertions --root . --profile strict --files tests/example.test.ts
ocentra-enforcer check placeholder-implementation --root . --profile strict --files src/index.ts
ocentra-enforcer check source-shape --root . --profile strict --workspace
ocentra-enforcer check required-tests --root . --profile strict --workspace
ocentra-enforcer run --root . --tool tsc -- npx tsc --noEmit --pretty false
ocentra-enforcer doctor --root . --profile strict --workspace
```

## Config vs Profile

Use `profile` when policy is owned by the enforcer pack:

```text
profile = strict
profile = ocentra-parent
```

Use `configPath` when the target repo owns policy:

```text
configPath = C:/path/to/target-repo/ocentra-enforcer.config.json
```

Do not pass both unless you intentionally want `configPath` to win.

Minimal target repo policy:

```json
{
  "schemaVersion": 2,
  "profileName": "my-project",
  "languages": ["rust", "typescript", "python", "common"],
  "failOn": ["error"],
  "rules": {
    "DOC-1.1": { "enabled": true, "severity": "warning" }
  },
  "tools": {
    "cargoDoc": { "enabled": false, "severity": "warning" },
    "cargoDeny": { "enabled": true, "severity": "error" }
  }
}
```

`violations` fail hooks/CI/MCP. `warnings` are reported but do not fail unless
`failOn` includes `warning`.

## Scopes

Use the smallest honest scope:

- File: `scan --files <file-or-dir>...`
- Crate/package: `scan --crate <cargo-package-name>` or `cargo --crate <cargo-package-name>`
- Diff: `scan --base origin/main --head HEAD`
- Full repo: `scan --workspace` or `cargo --workspace`

`cargo` mode adds cargo gates when the selected scope allows them. `scan` mode
is faster and deterministic for source/config policy.

## Codex Runtime Flow

When Codex is working in a target repo:

1. Read target repo instructions first.
2. Read `E:/ocentra-enforcer/rules/INDEX.md`.
3. Call MCP `ocentra_enforcer_route` with target `root`, profile/config, and exact touched files.
4. Open only docs returned by the route result.
5. Run `ocentra_enforcer_scan` for broad source/config policy, or `ocentra_enforcer_check` for migrated named guards such as `source-shape`, `required-tests`, `single-source-contracts`, `dependency-policy`, `sbom`, and scanner-backed Parent checks.
6. Run native tool checks through `ocentra_enforcer_run`.
7. Query `ocentra_enforcer_last_failure` or `ocentra_enforcer_diagnostics` before opening raw terminal artifacts.
8. Treat `violations` as hard failures. Report `warnings`, but do not block completion unless the profile `failOn` includes `warning`.

## Consumer Migration Sequence

1. Keep the target repo's existing guards until parity is proven.
2. Wire the target repo to the external Enforcer install.
3. Prove file-scope, crate/package-scope, diff-scope, and workspace behavior.
4. Replace generic guard logic with thin wrappers.
5. Point wrappers at `ocentra-enforcer check <name>`, `scan`, or `run` as appropriate.
6. Remove duplicated repo-local generic guard scripts only after parity.

Do not keep generic ledger, hub, lane, mail, exact-file-claim, or architecture
tooling in a consumer repo long term. Those are Enforcer coordination concerns.
Consumer repos should keep only product-specific dev server logic, release
packaging, proof semantics, and thin wrappers/config while parity is being
proven.
