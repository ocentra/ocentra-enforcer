# Dependency Policy Rules

## Covered Rules

- `DEP-1.1`: Dependency security audits must pass. Enforcer runs `npm audit --audit-level=high` when `package-lock.json` exists and `cargo audit --deny warnings` when `Cargo.lock` exists.
- `DEP-1.2`: External npm package licenses must match project policy.
- `NPM-1.1`: JavaScript projects must commit `package-lock.json` so CI can use `npm ci`.
- `NPM-1.2`: CI and local parity gates must use `npm ci`, not `npm install`.
- `NPM-1.3`: Package dependency versions must be exact. `^`, `~`, `*`, `latest`, `git:`, `file:`, and path-like dependency ranges fail deterministic package policy.
- `NPM-1.4`: `packageManager` must pin npm with an exact version such as `npm@11.7.0`.
- `NPM-1.5`: `engines.node` must be bounded, for example `>=20 <23`, not open-ended `>=20`.
- `NPM-1.6`: Dependency install scripts require reviewed approval.
- `NPM-1.7`: Git dependencies are forbidden.
- `NPM-1.8`: File and path dependencies are forbidden unless an explicit workspace policy allows them.
- `NPM-1.9`: `npm audit --audit-level=high` findings fail dependency policy.
- `NPM-1.10`: Dependency licenses must match allowed license policy.
- `NPM-1.11`: Suspicious dependency names fail package policy.

## Enforcement

Run:

```bash
ocentra-enforcer check dependency-policy --root <repo>
ocentra-enforcer check package-determinism --root <repo>
```

Projects can tune `allowedExternalLicenses` in `ocentra-enforcer.config.json`.

## Fails

- Missing package lockfiles, open dependency ranges, or unbounded runtime engines.
- Audit or license findings that violate configured policy.

## Passes

- Lockfiles are committed, package metadata is pinned, and dependency/security checks pass.
- Project config explicitly declares allowed licenses and audit behavior.

## Fix Recipe

1. Commit the lockfile produced by the package manager.
2. Use `npm ci` in local CI and remote CI.
3. Pin direct dependencies to exact versions.
4. Add `packageManager` with an exact npm version.
5. Bound the Node engine to supported majors.

## Validator

- scanner: `common/dependency-policy` and `common/package-determinism`
- command: `ocentra-enforcer check package-determinism --root <repo>`
