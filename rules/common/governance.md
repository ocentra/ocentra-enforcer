# Governance Rules

## Covered Rules

- `CI-1.1`: CI must use `npm ci`, not `npm install`, for deterministic installs.
- `CI-1.11`: Hard gates must not use `continue-on-error: true`.
- `CI-1.12`: Hard gates must not hide failures with `|| true`.
- `CI-1.13`: Workflow actions must be pinned by full commit SHA; mutable refs such as tags, `main`, `master`, `latest`, or other unpinned refs fail.
- `CI-1.14`: Workflows must declare least-privilege `permissions`.
- `CI-1.15`: Enforcer CI must run on pull requests and pushes to main.
- `CI-1.16`: Cross-platform projects must run Linux, Windows, and macOS CI legs.
- `CI-1.17`: Workflows must run the Enforcer local parity gate, usually `npm run ci:local`.
- `CI-1.18`: Workflows must not call legacy weaker commands as canonical CI gates.
- `CI-1.19`: Branch protection policy documentation is required.
- `CI-1.20`: Required checks documentation must include Enforcer.
- `CI-1.21`: Tests and harness code that parse child-process stdout/stderr as JSON must use file-backed capture or an explicit large `maxBuffer`.
- `REPO-1.1`: CODEOWNERS is required.
- `REPO-1.2`: CODEOWNERS must protect `rules/**`.
- `REPO-1.3`: CODEOWNERS must protect `scripts/**`, `src/**`, and `mcp/**`.
- `REPO-1.4`: CODEOWNERS must protect `schemas/**`, `profiles/**`, and `adapters/**`.
- `REPO-1.5`: CODEOWNERS must protect `.github/workflows/**`.
- `REPO-1.6`: Package lockfiles are required for package-managed projects.
- `REPO-1.7`: `packageManager` is required for npm projects.
- `REPO-1.8`: Node engine policy must be bounded.
- `REPO-1.9`: npm dependency versions must be deterministic.
- `REPO-1.15`: Generated schema artifacts must not drift after schema tests.
- `ENF-2.1`: Policy-critical mutations require stronger proof before acceptance.

## Enforcement

Run:

```bash
ocentra-enforcer check ci-integrity --root <repo>
ocentra-enforcer check repo-governance --root <repo>
ocentra-enforcer check package-determinism --root <repo>
ocentra-enforcer check mutation-risk --root <repo>
ocentra-enforcer verify --root <repo>
```

## Fails

- CODEOWNERS, workflows, local hooks, or package metadata let policy-critical changes land unreviewed.
- CI uses weak install commands, bypass constructs, or non-deterministic dependency metadata.
- Tests or harnesses parse large child-process JSON through default pipe buffers, causing platform-specific truncation in CI.

## Passes

- Policy-critical paths have explicit owners and CI runs the same hard gates as local validation.
- Mutation-risk changes trigger the extra proof set before acceptance.

## Fix Recipe

1. Add CODEOWNERS entries for every policy-critical directory.
2. Ensure workflows run on PR and main.
3. Use `npm ci`, not `npm install`.
4. Remove `continue-on-error` and `|| true` from hard gates.
5. Pin package manager, lockfiles, Node engine, and dependencies.
6. Route large subprocess stdout/stderr into artifact files or use an explicit large `maxBuffer` before `JSON.parse`.
7. When policy-critical files change, run the mutation-risk proof set before accepting the change.

## Validator

- scanner: `common/ci-integrity`, `common/repo-governance`, and `common/mutation-risk`
- command: `ocentra-enforcer check repo-governance --root <repo>`
