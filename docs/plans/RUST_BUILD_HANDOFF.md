# Rust-build integration handoff

## Objective

Deliver a genuinely Rust-only Ocentra Enforcer.  The proof is behavioral, not
cosmetic: every safety-relevant frozen MJS capability must have a native Rust
executor with a failing fixture, a passing fixture, a reachable CLI/MCP route,
and CI/dogfood evidence.  A retained vendor skill is not removable until the
same standard is met by a Rust rule or executable adapter.

`main` is eligible only after the complete evidence set is green.  Retirement
of `safety-main` is a later cutover action, never a prerequisite shortcut.

## Non-negotiable boundaries

- `E:\ocentra-enforcer` is the frozen MJS safety authority.  Do not modify,
  rewire, merge, or use it as the Rust integration checkout.
- `rust-build` is the sole durable integration branch.  Every accepted packet
  must be committed and pushed to `origin/rust-build`.
- Do not weaken rules, add waivers/bypass comments, or fake a clean result.
- Never stage, restore, or commit this protected deletion without an explicit
  user decision and complete mechanical parity evidence:

  ```text
  vendor/anthropic-cybersecurity-skills/skills/detecting-fileless-malware-techniques/SKILL.md
  ```

- Do not merge `rust-build` into `main` until final frozen scans, full
  validation, CI, vendor disposition, and product-documentation checks pass.

## Live repository state — 2026-08-02

| Item | Verified state |
|---|---|
| Integration checkout | `E:\ocentra-enforcer-rust-build` on `rust-build` |
| Pushed tip | `565095fc2 feat(policy): add native config and waiver checks` |
| Remote alignment | `HEAD == origin/rust-build` |
| Branch delta | `rust-build` is 96 commits ahead of `origin/main`; `main` is 0 ahead |
| Remote branches | `main`, `safety-main`, `rust-build`, `codex/private-rust-test-allowlist` |
| Frozen checkout | `E:\ocentra-enforcer`, local branch `codex/private-rust-test-allowlist`; do not touch |
| Additional local worktree | `E:\ocentra-enforcer-rust-parity-core` on local-only `rust-parity-core`; nonintegration, audit separately before cleanup |

### Local-only dirty state

These are deliberately **not** on `rust-build`:

| Path | Meaning | Required treatment |
|---|---|---|
| `proof/dogfood-journal.ndjson` | A July 30 failed dogfood run | Do not commit as successful evidence; replace only with a new accepted clean run if appropriate. |
| `proof/dogfood-manifest.json` | The matching failed manifest: 8 new violations | Same: do not commit this failing snapshot. |
| protected vendor `SKILL.md` deletion | One of 817 vendor skills is locally absent | Do not restore, stage, or commit until the user chooses after parity evidence. |

## Accepted Rust work already pushed

The recent `rust-build` series contains real native implementations and/or
contract repairs for:

- architecture-policy and seven independently callable architecture families;
- generated-artifact, source-shape, required-test, single-source-contract,
  import-boundary, AI rule-index, secrets, dependency-policy, SBOM, and
  literal-risk paths;
- mutation-risk and documentation-completeness checks;
- config-lockdown and waiver-policy checks, implemented with JSON parsing only
  in the config serde boundary;
- MCP named-check routing, language-selection propagation, and native scan
  contracts;
- parser hostile-input isolation/guards and deterministic memory-graph path
  ordering;
- CI/dogfood and generated-artifact traversal hardening.

Do not treat named-check registration as parity by itself.  A route is only
accepted when it executes native semantics and has pass/fail proof.

## Verified gates and current failure

Latest pushed run: `30668956726` for `565095fc2`.

| Gate | Status |
|---|---|
| Frozen + native dogfood: Windows | Passed |
| Frozen + native dogfood: macOS | Passed |
| Frozen + native dogfood: Ubuntu | Passed |
| Security, dependencies, SBOM | Passed |
| Graph-impacted crate gate | Failed |
| Workspace matrix / exact local parity | Skipped because graph gate failed |
| Required aggregate | Failed because graph gate failed |

The graph-gate root cause is a stale test expectation, not an unavailable
engine:

```text
crates/enforcer-mcp/tests/stdio_smoke.rs
stdio_smoke_check_no_zod_source_returns_the_typed_unavailable_refusal
```

It expects `native_engine_not_implemented`, but `no-zod-source` now routes to
the native architecture executor.  Update that test to assert the real native
result, run the exact stdio test and the CI-selected `enforcer-mcp` test batch,
then commit/push the scoped fix to `rust-build`.

## Parity status

### Completed evidence

- The frozen workspace scanner has passed in the latest CI dogfood jobs on all
  three operating systems.
- The recent config/waiver packet passed its focused frozen scan (eight files),
  config tests, scanner tests, MCP tests, formatting, and diff check before
  commit `565095fc2`.
- Memory-graph evidence at `proof/memory/x06-kg-parity.json` records 23
  compared tools: 15 equal, 8 better, 0 worse, 0 unrunnable.  Treat this as
  current implementation evidence, not a general performance promise.

### Still required

There is no canonical, complete frozen-MJS-to-Rust capability/proof matrix yet.
Build it before claiming a percentage or retiring MJS.  Each row must contain:

```text
frozen capability -> Rust executor/adapter -> rule IDs -> fail fixture ->
pass fixture -> CLI/MCP proof -> CI/dogfood proof -> disposition
```

Classify every frozen capability as one of:

1. native Rust parity proven;
2. executable adapter parity proven;
3. retained vendor/advisory content, explicitly not removable; or
4. unimplemented gap.

### CyberSkills vendor disposition

Current audit counts are:

| Corpus result | Count |
|---|---:|
| Tracked vendor `SKILL.md` files | 817 |
| Native mapped per-skill evidence | 6 |
| Adapter-deferred | 398 |
| Advisory-only prose | 135 |
| Unported | 278 |
| Without complete executable parity | 811 |

Therefore no broad vendor removal is authorized.  For any eventual removal,
require an inventory mapping, hashes/anchors, unique native/adaptor ownership,
pass/fail fixtures, native CLI/MCP execution, dogfood/full CI, and explicit
user approval.

## Required execution order

1. Fix the stale `no-zod-source` stdio smoke test.  Use the smallest scope,
   run the focused test plus the CI-selected MCP batch, frozen focused scan,
   diff check, detached-parent introduced-findings audit, then commit and push.
2. Wait for and inspect the exact-SHA CI result.  Do not claim green from a
   previous SHA.
3. Build the canonical frozen MJS -> Rust capability/proof matrix.  Route work
   by complete capability families, not by small RR comment packets.
4. Close each unimplemented capability with native code or an executable
   adapter.  Validate each packet locally before pushing:

   ```powershell
   # first route the exact files/crate with Enforcer MCP
   node E:\ocentra-enforcer\scripts\rust-rules.mjs scan --root . --languages rust --files <changed-files>
   cargo test -p <affected-crate>
   git diff --check
   ```

   Then use exact-path coordination claims/guards, commit, and push only the
   accepted source packet.
5. On the final candidate SHA, run the authoritative frozen scanner twice:

   ```powershell
   node E:\ocentra-enforcer\scripts\rust-rules.mjs scan --root . --languages rust --workspace
   ```

6. Run full local validation and obtain a complete green CI run, including
   graph gate, workspace matrix, exact-local parity, and required aggregate.
7. Update README/product documentation from verified current behavior only;
   keep migration/research history in `docs/plans/`.
8. Confirm retained vendor corpus is unchanged or obtain the explicit user
   vendor disposition.  Only then prepare a normal `rust-build` -> `main`
   merge.  Retire `safety-main` only after the merged Rust cutover is proven.

## Reporting discipline

- Report live Git/CI facts, never historical counts as current truth.
- State separately: source present, focused-test proven, native route proven,
  CI proven, and unverified.
- Preserve unrelated dirty work; do not use `reset --hard`, broad restore, or
  deletion to make a status look clean.
