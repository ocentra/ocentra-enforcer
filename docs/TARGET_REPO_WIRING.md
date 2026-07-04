# Target Repo Wiring

<!-- ai-dense -->
```yaml
model: target repos call the INSTALLED enforcer binary with an explicit --root; they never copy enforcer source
rules_lookup: "enforcer explain <ruleId> -- rules are compiled into the binary (rules-as-data, arc-04), there is no external INDEX.md/rules file to locate at any filesystem path, local or otherwise"
scopes: "file (--files) | crate/package (--crate) | diff (--base/--head) | full (--workspace)"
consumer_ci: "zero-Rust-toolchain: install.sh/install.ps1 script + .github/actions/enforcer-scan composite action + optional npm wrapper (mechanism = c10; this doc's prose only)"
historical_bug_fixed_2026_07_04: "this doc used to instruct reading a literal machine-local absolute path (a drive-letter rules/INDEX.md path) that could not resolve on any other machine or CI runner; replaced by `enforcer explain <ruleId>`, which needs no filesystem lookup at all"
```
<!-- /ai-dense -->

Target repos should not copy enforcer source. They should call the
installed enforcer with an explicit `root`.

## Dry-Run First

From anywhere, using the installed binary:

```bash
enforcer init --root <target-repo> --profile strict --adapters codex,mcp,precommit,github-actions --dry-run
```

Review the file plan before writing anything.

## What Init Is Supposed To Write

The default adapter set plans:

- `enforcer-config.json`, tiny target repo config.
- `.mcp.json`, project MCP config pointing to the installed enforcer binary
  by absolute path (project scope is an explicit opt-in; user/global scope
  is the default install target — see [INSTALL.md](../INSTALL.md)).
- `.git/hooks/pre-commit`, plain Git pre-commit hook.
- `.github/workflows/enforcer.yml`.
- `.github/workflows/codeql.yml`.
- `.github/workflows/dependency-policy.yml`.
- `.github/workflows/secret-scan.yml`.
- `.github/workflows/sbom.yml`.

Husky is not default. Add `husky` only when requested or when the target
repo already uses Husky.

## Minimal Manual Wiring

If you do not want generated files yet, create only a target repo script or
doc that calls:

```bash
enforcer scan --root . --profile strict --files Cargo.toml
enforcer scan --root . --profile strict --languages typescript,python,common --files src tests
enforcer check no-zod-source --root . --profile strict --files src/index.ts
enforcer check validation-bypass --root . --profile strict --files src/index.ts
enforcer check weak-assertions --root . --profile strict --files tests/example.test.ts
enforcer check placeholder-implementation --root . --profile strict --files src/index.ts
enforcer check source-shape --root . --profile strict --workspace
enforcer check required-tests --root . --profile strict --workspace
enforcer run --root . --tool tsc -- npx tsc --noEmit --pretty false
enforcer doctor --root . --profile strict --workspace
```

## Config vs Profile

Use `profile` when policy is owned by the enforcer pack:

```text
profile = strict
profile = ocentra-parent
```

Use `configPath` when the target repo owns policy:

```text
configPath = <target-repo>/enforcer-config.json
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

`violations` fail hooks/CI/MCP. `warnings` are reported but do not fail
unless `failOn` includes `warning`.

## Scopes

Use the smallest honest scope:

- File: `scan --files <file-or-dir>...`
- Crate/package: `scan --crate <cargo-package-name>` or
  `cargo --crate <cargo-package-name>`
- Diff: `scan --base origin/main --head HEAD`
- Full repo: `scan --workspace` or `cargo --workspace`

`cargo` mode adds cargo gates when the selected scope allows them. `scan`
mode is faster and deterministic for source/config policy.

## Runtime Flow (Any Harness)

When an AI harness is working in a target repo:

1. Read target repo instructions first.
2. Run `enforcer explain <ruleId>` for any rule that needs explaining. Rules
   are compiled into the binary (rules-as-data); there is no external
   `INDEX.md`/rules file to locate at a filesystem path, local or otherwise.
   (This replaces an earlier version of this doc that pointed at a
   machine-local absolute path — that path could never resolve on any
   machine but the original author's, let alone a CI runner; `explain`
   needs no filesystem lookup at all.)
3. Call MCP `mcp__enforcer__route` with target `root`, profile/config, and
   exact touched files.
4. Open only docs/rules returned by the route result.
5. Run `mcp__enforcer__scan` for broad source/config policy, or
   `mcp__enforcer__check` for named guards such as `source-shape`,
   `required-tests`, `single-source-contracts`, `dependency-policy`, `sbom`.
6. Run native tool checks through `mcp__enforcer__run`.
7. Query `mcp__enforcer__last_failure` or `mcp__enforcer__diagnostics`
   before opening raw terminal artifacts.
8. Treat `violations` as hard failures. Report `warnings`, but do not block
   completion unless the profile `failOn` includes `warning`.

## Zero-Toolchain Consumer CI

A target repo's CI never needs a Rust toolchain to run the enforcer:

- A curl/iwr-installable `install.sh`/`install.ps1` downloads the matching
  release binary (checksum-verified) and works identically on GitHub
  Actions, GitLab CI, CircleCI, Bitbucket, Jenkins, or bare shell.
  `cargo install` from source is a documented fallback only.
- A reusable composite GitHub Action (`.github/actions/enforcer-scan`) wraps
  the installer and caches the downloaded binary, keyed by version and
  platform.
- An optional npm wrapper (thin JS shim + per-platform optional
  dependencies) lets consumers already wired the Node-centric way
  (`npm install` + `npx enforcer ...`) keep working unchanged even though
  the package ships a compiled binary.
- CI always regenerates proof fresh — it never trusts a pre-computed or
  uploaded artifact as a substitute for running the binary. CI may
  separately upload its own freshly generated report as a build artifact
  for human PR review afterward; that is a distinct, legitimate act, not a
  trust shortcut.

The mechanism (installer script, composite action, npm wrapper) is owned by
the release/CI-integration track; this document owns only the prose
describing how a target repo consumes it.

## Consumer Migration Sequence

1. Keep the target repo's existing guards until parity is proven.
2. Wire the target repo to the installed enforcer.
3. Prove file-scope, crate/package-scope, diff-scope, and workspace
   behavior.
4. Replace generic guard logic with thin wrappers.
5. Point wrappers at `enforcer check <name>`, `scan`, or `run` as
   appropriate.
6. Remove duplicated repo-local generic guard scripts only after parity.

Do not keep generic ledger, hub, lane, mail, exact-file-claim, or
architecture tooling in a consumer repo long term. Those are enforcer
coordination concerns. Consumer repos should keep only product-specific dev
server logic, release packaging, proof semantics, and thin wrappers/config
while parity is being proven.
