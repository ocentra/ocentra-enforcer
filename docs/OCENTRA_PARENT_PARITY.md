# Ocentra Parent Parity Map

This document records what should move from Ocentra Parent into Ocentra Enforcer
before Parent deletes local guard scripts or ESLint rules.

Do not remove Ocentra Parent guards until old-vs-new parity is proven in a
read-only comparison pass and the remaining existing debt is either fixed or
explicitly profiled.

## Current Answer

Yes: reusable checks from Ocentra Parent `eslint-rules/`, `scripts/`, and
generic coordination tooling belong in this repo. They should not be copied
blindly. Enforcer owns generic validation engines, rule docs, MCP tools, hooks,
CI adapters, hub mail, exact-file claims, lane/worktree coordination, peer sync,
and architecture gates. Ocentra Parent keeps product, portal, release, product
proof semantics, and thin consumer wrappers/config while migration parity is
being proven.

Target repos should eventually keep only:

- `ocentra-enforcer.config.json`
- optional ESLint config that imports `ocentra-enforcer/eslint-rules`
- optional thin npm wrappers while migration is in progress
- project-specific proof/product scripts that are not reusable yet
- no live hub, ledger, lane, mail, or worktree coordination implementation

## Migrated Or Covered Now

| Ocentra Parent source                                                           | Enforcer status                                                                                   | Notes                                                                                                                                    |
| ------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------- |
| `eslint-rules/no-app-string-literals.js`                                        | copied as `eslint-rules/no-app-string-literals.js`                                                | Exported via `ocentra-enforcer/eslint-rules`; project decides where to enable.                                                           |
| `eslint-rules/no-naked-domain-string-types.js`                                  | copied and scanner-backed by `TS-1.3`                                                             | Scanner catches naked domain aliases and manual brands without requiring ESLint.                                                         |
| `eslint-rules/no-runtime-string-types.js`                                       | copied as ESLint rule                                                                             | Not globally scanner-enforced yet because it needs profile/path targeting.                                                               |
| `scripts/check-no-zod-source.mjs`                                               | scanner-backed by `TS-1.2`                                                                        | Catches direct Zod source usage and Zod package dependencies.                                                                            |
| `scripts/check-no-naked-domain-strings.mjs`                                     | scanner-backed by `RR-6.1`, `RR-6.5`, `RR-18.16`, `TS-1.3`, and `PY-1.3`                          | Rust/TypeScript/Python findings now point back to routed rule docs.                                                                      |
| `scripts/check-no-test-doubles.mjs`                                             | scanner-backed by `TEST-1.1`                                                                      | Generic common rule across TS/JS/Python/Rust source.                                                                                     |
| `scripts/check-no-weak-assertions.mjs`                                          | covered by `ocentra-enforcer check weak-assertions` and `TEST-1.2`                                | Covers low-value JS matchers and Rust assertion shortcuts.                                                                               |
| `scripts/check-no-skipped-focused-tests.mjs`                                    | covered by `ocentra-enforcer check skipped-focused-tests`, `TS-3.1`, `PY-2.1`, and `TEST-1.3`     | Covers JS/Playwright skip/focus/todo, Python skip/xfail, and Rust `#[ignore]`.                                                           |
| `scripts/check-no-validation-bypass.mjs`                                        | covered by `ocentra-enforcer check validation-bypass`, `RR-2.*`, `TS-2.1`, and `PY-1.*`           | Includes prettier-ignore, TS/ESLint suppressions, Python suppressions, Rust allow/expect, rustfmt skip, and Clippy suppressions.         |
| `scripts/check-no-placeholder-implementation.mjs`                               | covered by `ocentra-enforcer check placeholder-implementation`, `SRC-1.2`, `RR-4.2`, and `RR-4.3` | Covers source TODO/FIXME/TBD/placeholder comments and not-implemented/debug-print code paths.                                            |
| `scripts/check-no-reexports.mjs` and `scripts/check-architecture-reexports.mjs` | covered by `ocentra-enforcer check reexports`, `TS-1.1`, `RR-7.2`, and `RR-7.3`                   | Parent wrappers can later call one Enforcer check for both TS/JS and Rust re-export bans.                                                |
| `scripts/check-cross-platform-script-commands.mjs`                              | scanner-backed by `PORT-1.1`                                                                      | Guards unguarded Windows-only npm invocations in scripts.                                                                                |
| `scripts/check-no-tracked-generated-artifacts.mjs`                              | covered by `GEN-1.1`, `GEN-1.2`, and `check generated-artifacts --tracked`                        | Marker, generated-output-path, and tracked-only generated artifact modes exist.                                                          |
| `scripts/security/scan-staged-secrets.mjs`                                      | covered by `SEC-1.1`, `SEC-1.2`, and `check secrets --staged`                                     | Inline secrets, sensitive paths, and staged-only mode exist.                                                                             |
| Rust no-reexport architecture gate                                              | covered by `RR-7.2` and `RR-7.3`                                                                  | Current Parent debt still fails. Do not hard-wire repo-wide until handled.                                                               |
| Rust runtime string boundary                                                    | covered by `RR-18.16` with `ocentra-parent` profile globs                                         | Keep comparing against Parent's script before deleting it.                                                                               |
| TS/JS re-export architecture gate                                               | covered by `TS-1.1`                                                                               | Enforcer catches barrel/re-export forms.                                                                                                 |
| suppression/bypass comments                                                     | covered by `RR-2.*`, `TS-2.1`, `PY-1.*`                                                           | Project config can downgrade only by explicit policy.                                                                                    |
| skipped/focused/weak tests                                                      | covered by `TS-3.1`, `PY-2.1`, `TEST-1.1`, `TEST-1.2`, and `TEST-1.3`                             | Fixture tests exist for weak assertions and Rust ignored tests.                                                                          |
| `scripts/check-source-shape.mjs`                                                | covered by `ocentra-enforcer check source-shape` and `SRC-1.1`                                    | Config-driven policies support TypeScript, Rust, and Python file/function/export/type limits.                                            |
| `scripts/check-required-tests.mjs`                                              | covered by `ocentra-enforcer check required-tests` and `TEST-2.1`                                 | Packages/apps with `src/` and Rust crates require test scaffolds.                                                                        |
| `scripts/check-single-source-contracts.mjs`                                     | covered by `ocentra-enforcer check single-source-contracts` and `CONTRACT-1.1`                    | Accepts the migrated Ocentra Parent contract config shape via `--check-config`.                                                          |
| `scripts/security/check-dependency-policy.mjs`                                  | covered by `ocentra-enforcer check dependency-policy` and `DEP-1.*`                               | Runs npm high audit, npm license policy, and cargo-audit when lockfiles exist. Cargo-deny remains covered by Rust cargo gates/workflows. |
| `scripts/security/write-sbom.mjs`                                               | covered by `ocentra-enforcer check sbom` and `SBOM-1.1`                                           | Writes target-root artifacts under the requested output path; supports `--dry-run`.                                                      |
| `scripts/check-ai-rule-index.mjs`                                               | covered by `ocentra-enforcer check ai-rule-index` and `AI-1.1`                                    | Checks AGENTS-to-rule-index routing and oversized rule files.                                                                            |

## Generic Gaps Before Parent Deletion

| Ocentra Parent source                           | Gap                    | Preferred Enforcer shape                                                                                                                     |
| ----------------------------------------------- | ---------------------- | -------------------------------------------------------------------------------------------------------------------------------------------- |
| `scripts/ci/run-local-codeql.mjs`               | workflow template only | Add optional local CodeQL runner or document that CodeQL is CI-only.                                                                         |
| broader old-vs-new fixture comparison           | remaining proof work   | The Enforcer modes exist, but Parent deletion still needs old guard vs new guard fixture comparison before removing scripts.                 |
| project-specific import-boundary policy breadth | profile/config work    | Configurable import-boundary schema exists; Parent still needs a final project policy list before deleting local architecture orchestration. |
| `scripts/check-architecture-policy.mjs`         | orchestration gap      | Enforcer has equivalent named checks, but still needs a generated Parent wrapper that runs the same set in the same order.                   |

## Keep In Ocentra Parent

These remain in Parent only when they are product or repo-specific. Generic
harness coordination is not product code and should move to Enforcer.

- `scripts/dev/*` and local portal/dev server scripts.
- temporary hub/ledger/lane wrappers until Enforcer coordination parity is proven.
- release/package/version scripts tied to Ocentra Parent artifacts.
- `scripts/check-pre-ai-proof.mjs` and expectation/proof matrix checks.
- `scripts/check-portal-route-panel-contracts.mjs`.
- `scripts/check-portal-ui-boundaries.mjs` unless converted into a configurable UI-boundary profile.
- `scripts/check-vendor-portal-asset-imports.mjs` unless converted into a configurable asset-import rule.

## Current Parent Profile Decisions

- `profile=ocentra-parent` keeps source-shape stricter than the old Parent script when it finds exported generated DTOs or oversized portal UI functions. Do not silently relax this in agent memory; if a generated subtree needs different limits, add explicit `sourceShapeOverrides` in the target repo config.
- `profile=ocentra-parent` ignores `vendor/` by default for generic Enforcer source/bypass/test-double scans. Vendored code should not block generic reusable gates unless the target config opts a subtree back in as first-party.
- Required-test strict mode distinguishes `.gitkeep` placeholder-only trees from truly empty test/proof category trees in the diagnostic detail.

## Current Parent Smoke Result

The setup layer works:

- `init --dry-run --root E:\OcentraParent --profile ocentra-parent` produces a target-root plan for config, MCP, Codex skill, hooks, and workflows.
- MCP smoke works from the Enforcer install path while targeting `E:\OcentraParent`.

The focused scan intentionally fails current Parent source:

- `crates/agent-protocol/src/lib.rs` returns `RR-7.2` and `RR-7.3` public wildcard re-export failures.
- This proves Enforcer detects the architecture debt, but it also means Parent cannot replace its guards with a repo-wide Enforcer hard gate until that debt is fixed or scoped by migration policy.

## Removal And Rewire Sequence

1. Keep Ocentra Parent read-only for comparison.
2. Finish Enforcer generic gaps listed above.
3. Run old Parent guard and new Enforcer guard against the same fixtures/scopes.
4. Add `ocentra-enforcer.config.json` to Parent and wire adapters with `init --dry-run` first.
5. Add thin Parent wrappers that call Enforcer, but leave old scripts present.
6. Run both old and new gates in CI/advisory mode until parity is green.
7. Convert wrappers to hard gates only after existing debt is resolved.
8. Delete duplicated Parent scripts and ESLint rules in a separate cleanup PR.

## God-File Risk

`source-shape` exists for target projects, but before making it a hard self-gate
for Enforcer itself, split Enforcer's own large files:

- `scripts/rust-rules.mjs` should become a thin CLI entrypoint.
- Rust scanner/config/Cargo gates should move under `src/rust/`.
- Harness storage/parsers/queries/retention should move under `src/harness/`.
- MCP tool dispatch should move under `src/mcp/`.

This must happen before Enforcer can honestly dogfood a hard source-shape rule.
