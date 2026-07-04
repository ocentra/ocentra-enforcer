# Parent-Repo Parity Map (historical record)

<!-- ai-dense -->
```yaml
status: HISTORICAL — records the legacy TS/.mjs-engine-era migration of a consumer repo's (internally called "Parent") local guard scripts/ESLint rules into the enforcer; the engine itself is now Rust (RUST_ARCHITECTURE.md), so the "eslint-rules/" mechanism described below no longer exists in the enforcer's own implementation
current_model: "the enforcer validates a consumer's TS/JS source via native Rust validators (enforcer-lang-ts, swc/tree-sitter-backed), not via a shipped ESLint plugin"
purpose_of_this_doc: "keep the migration decision log (what moved, what stayed, what the remaining gaps were) for historical parity auditing; do not treat any 'eslint-rules/' command below as a current enforcer surface"
consumer_repo_alias: "Parent" (a large all-Rust reference monorepo consuming the enforcer; see RUST_ARCHITECTURE.md 'Borrows from OcentraParent')
```
<!-- /ai-dense -->

This document is a **historical record** of what moved from a consumer
repository (internally referred to as "Parent") into the enforcer, back when
the enforcer's own engine was still TypeScript/Node. The engine itself has
since been rewritten as a pure Rust Cargo workspace (see
[RUST_ARCHITECTURE.md](plans/enforcer-selfhost-plan/RUST_ARCHITECTURE.md));
the `eslint-rules/` package and `.mjs` check-scripts named below no longer
exist as the enforcer's *own* implementation. The enforcer still **validates**
a consumer's TypeScript/JavaScript source — natively, in Rust, via the
`enforcer-lang-ts` crate — it simply no longer ships or requires an ESLint
plugin to do it.

Do not remove Parent guards until old-vs-new parity is proven in a read-only
comparison pass and the remaining existing debt is either fixed or explicitly
profiled.

## Current Answer

Yes: reusable checks from the consumer repo's local `eslint-rules/`,
`scripts/`, and generic coordination tooling belonged in the enforcer, and by
design were never copied blindly. The enforcer owns generic validation
engines, rule records, MCP tools, hooks, CI adapters, hub mail, exact-file
claims, lane/worktree coordination, peer sync, and architecture gates. The
consumer repo keeps product, portal, release, product proof semantics, and
thin consumer wrappers/config while migration parity is being proven.

Target repos should eventually keep only:

- `enforcer-config.json`
- optional thin wrappers while migration is in progress
- project-specific proof/product scripts that are not reusable yet
- no live hub, ledger, lane, mail, or worktree coordination implementation

## Migrated Or Covered (historical, TS-engine era)

| Consumer-repo source | Enforcer status at the time | Notes |
| --------------------- | ---------------------------- | ----- |
| `eslint-rules/no-app-string-literals.js` | copied as an ESLint rule package | Superseded by native Rust validators; the enforcer no longer ships an ESLint rule package. |
| `eslint-rules/no-naked-domain-string-types.js` | scanner-backed by `TS-1.3` | Now enforced by `enforcer-lang-ts`'s naked-domain-string validator. |
| `scripts/check-no-zod-source.mjs` | scanner-backed by `TS-1.2` | Now `enforcer check no-zod-source`, native Rust. |
| `scripts/check-no-naked-domain-strings.mjs` | scanner-backed by `RR-6.1`, `RR-6.5`, `RR-18.16`, `TS-1.3`, `PY-1.3` | Rust/TypeScript/Python findings point back to routed rule records. |
| `scripts/check-no-test-doubles.mjs` | scanner-backed by `TEST-1.1` | Generic common rule across TS/JS/Python/Rust source. |
| `scripts/check-no-weak-assertions.mjs` | `enforcer check weak-assertions`, `TEST-1.2` | Covers low-value JS matchers and Rust assertion shortcuts. |
| `scripts/check-no-skipped-focused-tests.mjs` | `enforcer check skipped-focused-tests`, `TS-3.1`, `PY-2.1`, `TEST-1.3` | Covers JS/Playwright skip/focus/todo, Python skip/xfail, and Rust `#[ignore]`. |
| `scripts/check-no-validation-bypass.mjs` | `enforcer check validation-bypass`, `RR-2.*`, `TS-2.1`, `PY-1.*` | Includes formatter-ignore, TS/lint suppressions, Python suppressions, Rust allow/expect, rustfmt skip, and Clippy suppressions. |
| `scripts/check-no-placeholder-implementation.mjs` | `enforcer check placeholder-implementation`, `SRC-1.2`, `RR-4.2`, `RR-4.3` | Covers source TODO/FIXME/TBD/placeholder comments and not-implemented/debug-print code paths. |
| `scripts/check-no-reexports.mjs`, `scripts/check-architecture-reexports.mjs` | `enforcer check reexports`, `TS-1.1`, `RR-7.2`, `RR-7.3` | One enforcer check now covers both TS/JS and Rust re-export bans. |
| `scripts/check-cross-platform-script-commands.mjs` | scanner-backed by `PORT-1.1` | Guards unguarded Windows-only script invocations. |
| `scripts/check-no-tracked-generated-artifacts.mjs` | `GEN-1.1`, `GEN-1.2`, `check generated-artifacts --tracked` | Marker, generated-output-path, and tracked-only generated artifact modes exist. |
| `scripts/security/scan-staged-secrets.mjs` | `SEC-1.1`, `SEC-1.2`, `check secrets --staged` | Inline secrets, sensitive paths, and staged-only mode exist. |
| Rust no-reexport architecture gate | `RR-7.2`, `RR-7.3` | Current consumer-repo debt failed at the time. Not hard-wired repo-wide until handled. |
| Rust runtime string boundary | `RR-18.16` with `ocentra-parent` profile globs | Kept comparing against the consumer script before it was deleted. |
| TS/JS re-export architecture gate | `TS-1.1` | The enforcer catches barrel/re-export forms. |
| suppression/bypass comments | `RR-2.*`, `TS-2.1`, `PY-1.*` | Project config can downgrade only by explicit policy. |
| skipped/focused/weak tests | `TS-3.1`, `PY-2.1`, `TEST-1.1`, `TEST-1.2`, `TEST-1.3` | Fixture tests exist for weak assertions and Rust ignored tests. |
| `scripts/check-source-shape.mjs` | `enforcer check source-shape`, `SRC-1.1` | Config-driven policies support TypeScript, Rust, and Python file/function/export/type limits. |
| `scripts/check-required-tests.mjs` | `enforcer check required-tests`, `TEST-2.1` | Packages/apps with `src/` and Rust crates require test scaffolds. |
| `scripts/check-single-source-contracts.mjs` | `enforcer check single-source-contracts`, `CONTRACT-1.1` | Accepts the migrated consumer-repo contract config shape via `--check-config`. |
| `scripts/security/check-dependency-policy.mjs` | `enforcer check dependency-policy`, `DEP-1.*` | Runs npm high audit, npm license policy, and cargo-audit when lockfiles exist. |
| `scripts/security/write-sbom.mjs` | `enforcer check sbom`, `SBOM-1.1` | Writes target-root artifacts under the requested output path; supports `--dry-run`. |
| `scripts/check-ai-rule-index.mjs` | `enforcer check ai-rule-index`, `AI-1.1` | Checks AGENTS-to-rule-index routing and oversized rule files. |

## Generic Gaps Before Consumer-Repo Deletion (historical)

| Consumer-repo source | Gap | Preferred enforcer shape |
| --------------------- | ---- | ------------------------- |
| local CodeQL runner script | workflow template only | Optional local CodeQL runner, or document that CodeQL is CI-only. |
| broader old-vs-new fixture comparison | remaining proof work | The enforcer modes exist, but consumer-repo deletion still needs old guard vs new guard fixture comparison before removing scripts. |
| project-specific import-boundary policy breadth | profile/config work | Configurable import-boundary schema exists; still needs a final project policy list before deleting local architecture orchestration. |
| local architecture-policy orchestration script | orchestration gap | The enforcer has equivalent named checks, but still needs a generated consumer-repo wrapper that runs the same set in the same order. |

## Keep In The Consumer Repo

These remain in the consumer repo only when they are product or
repo-specific. Generic harness coordination is not product code and should
move to the enforcer.

- Local dev-server/portal scripts.
- Temporary hub/ledger/lane wrappers until enforcer coordination parity is
  proven.
- Release/package/version scripts tied to consumer-repo artifacts.
- Product-specific pre-AI-proof and expectation/proof matrix checks.
- Product-specific UI-boundary/asset-import checks unless converted into a
  configurable, enforcer-owned profile.

## Current Consumer-Repo Profile Decisions

- `profile=ocentra-parent` keeps source-shape stricter than the old consumer
  script when it finds exported generated DTOs or oversized UI functions. Do
  not silently relax this in agent memory; if a generated subtree needs
  different limits, add explicit `sourceShapeOverrides` in the target repo
  config.
- `profile=ocentra-parent` ignores `vendor/` by default for generic
  enforcer source/bypass/test-double scans. Vendored code should not block
  generic reusable gates unless the target config opts a subtree back in as
  first-party.
- Required-test strict mode distinguishes placeholder-only trees from truly
  empty test/proof category trees in the diagnostic detail.

## Historical Smoke Result

The setup layer worked at the time of this record:

- `init --dry-run --root <consumer-repo-path> --profile ocentra-parent`
  produced a target-root plan for config, MCP, skill, hooks, and workflows.
- MCP smoke worked from the enforcer install path while targeting the
  consumer repo.

The focused scan intentionally failed the consumer repo's source at the
time: a Rust crate's `lib.rs` returned `RR-7.2`/`RR-7.3` public wildcard
re-export failures. This proved the enforcer detects real architecture debt
— but it also meant the consumer repo could not replace its guards with a
repo-wide hard gate until that debt was fixed or scoped by migration policy.

## Removal And Rewire Sequence (historical)

1. Keep the consumer repo read-only for comparison.
2. Finish the enforcer's generic gaps listed above.
3. Run the old consumer-repo guard and the new enforcer guard against the
   same fixtures/scopes.
4. Add `enforcer-config.json` to the consumer repo and wire adapters with
   `init --dry-run` first.
5. Add thin consumer-repo wrappers that call the enforcer, but leave old
   scripts present.
6. Run both old and new gates in CI/advisory mode until parity is green.
7. Convert wrappers to hard gates only after existing debt is resolved.
8. Delete duplicated consumer-repo scripts and ESLint rules in a separate
   cleanup change.

## God-File Risk (historical; resolved by the Rust rewrite)

The old TS-engine repo needed its own oversized-file risk noted here before
it could honestly dogfood a hard `source-shape` rule on itself. That concern
no longer applies to the current implementation: the Rust rewrite's own
d22 size/shape doctrine is applied to the enforcer's own crates from the
start, and z01 is the terminal self-enforcement gate that proves it.
