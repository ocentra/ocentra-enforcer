# c07 Generic Writer And Doctor

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Generic Writer And Doctor`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `crates/enforcer-install/src/adapters/generic.rs, crates/enforcer-install/src/doctor.rs, crates/enforcer-install/src/emitters/consumer_ci.rs, crates/enforcer-install/src/emitters/git_hooks.rs, crates/enforcer-install/tests/fixtures/generic/**, crates/enforcer-install/tests/fixtures/doctor/**, crates/enforcer-install/tests/fixtures/consumer-ci/**, crates/enforcer-install/tests/fixtures/git-hooks/**`
- deps: `c01-install-core-and-cli-contract, c02-harness-autodetect, arc-23`
- tier: `P1 unit`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md), [RUST_ARCHITECTURE](../RUST_ARCHITECTURE.md).

## Where We Are
The reference doctor logic is Codex-specific and tangled into one install path. Harnesses that only need a plain `.mcp.json` server entry have no adapter today, and there is no shared, mechanical doctor in `enforcer-install` that verifies an install regardless of adapter. Once c01/arc-23 land the `Adapter` trait and typed `Check`/`Report` records, both gaps get first-class Rust modules.

## Where We Want To Be
A generic adapter (`crates/enforcer-install/src/adapters/generic.rs`) that upserts a standard `.mcp.json` server entry for harnesses with no bespoke needs, plus a shared doctor module (`crates/enforcer-install/src/doctor.rs`) that mechanically re-reads disk and aggregates per-adapter `verify` checks into one typed `Report` across all adapters. c07 additionally owns the two harness-neutral **install-emitters** that `init`/`buildInitWrites` writes today but no c-track pack claims (WAVE 6 "Install-EMITTER gaps"):
1. **Consumer CI-workflow emitters** (`emitters/consumer_ci.rs`) — the per-consumer `.github/workflows/` set generated from `adapters/github-actions/{ocentra-enforcer,codeql,dependency-policy,secret-scan,sbom}.yml` (the reference `GITHUB_ACTIONS_ADAPTERS` list + `workflowMap`). Distinct from c10, which owns the ENFORCER'S OWN release pipeline + the `enforcer-scan` action — c10 explicitly does NOT generate the per-consumer `codeql`/`sbom`/`secret-scan`/`dependency-policy` workflows.
2. **Pre-commit hook emitters** (`emitters/git_hooks.rs`) — the consumer git-hook (`adapters/git-hooks/pre-commit.sh` → `.git/hooks/pre-commit`), husky (`adapters/husky/pre-commit` → `.husky/pre-commit`), and lefthook (`adapters/lefthook/lefthook.yml` → `lefthook.yml`) emitters, selected per-adapter. Distinct from c04/c05, which emit Claude PreToolUse/SessionStart hooks (a different mechanism). Both emitter modules honor `--dry-run` (returns the planned write set, touches zero files) and select strictly by requested adapter (choosing one hook flavor never writes the others).

## Requirement Checklist
- [ ] Generic adapter upserts `mcpServers[<x01 server-name const -> "enforcer">]` into the harness's USER/GLOBAL `.mcp.json` at the resolved home path from arc-23 (`serde_json` value edit, preserving unrelated keys) — never a per-repo project file, and never the legacy `ocentra-enforcer` literal; command points at the ABSOLUTE `enforcer` binary path.
- [ ] Shared doctor aggregates each registered adapter's `verify` checks into one `Report` with typed `Severity` (from `enforcer-domain`, arc-02).
- [ ] Doctor is mechanical: every check re-reads the actual file and resolves the server binary path from disk, never trusts the plan.
- [ ] Doctor result is fail-closed — any `Severity::Error` check drives a non-zero CLI exit (arc-22); `Severity::Warning` checks do not fail.
- [ ] Generic adapter and doctor are pure over an injected filesystem abstraction (or a temp-dir root) for fixture testing; obey `[workspace.lints]` (no `unwrap`/`expect`/`panic`/`print_*`, no `pub use` barrels).
- [ ] Consumer CI-workflow emitter (`emitters/consumer_ci.rs`) writes the 5 workflow files into `<root>/.github/workflows/` from the bundled `adapters/github-actions/*.yml` templates, one file per requested adapter, matching the reference `GITHUB_ACTIONS_ADAPTERS` set + `workflowMap` target paths.
- [ ] Pre-commit hook emitter (`emitters/git_hooks.rs`) writes the plain git-hook (`.git/hooks/pre-commit`), husky (`.husky/pre-commit`), and lefthook (`lefthook.yml`) files, each only when its adapter is requested; selecting one flavor never writes the others.
- [ ] Both emitters honor `--dry-run` (return the planned write set, zero files touched) and respect `force`/skip-existing semantics consistent with the reference `initWrite`.
- [ ] Both emitters are pure over the same injected filesystem abstraction / temp-dir root as the generic adapter, and their outputs feed the shared doctor's `verify` aggregation; obey `[workspace.lints]`.

## Acceptance And Proof
T1 (`generic-writer` and `install-doctor` in TEST_PROOF_EXPECTATIONS.md), proved by `cargo test -p enforcer-install`: a `#[test]` asserts the generic adapter's `.mcp.json` output against a golden file under `tests/fixtures/generic/`, and doctor returns all-green on a good fixture and red (naming the failing check) on a `tests/fixtures/doctor/` fixture with a missing/renamed server binary.

Additional T1 rows (same `cargo test -p enforcer-install` gate) proving the two homed emitters:

| Row | Capability | Fail (red) fixture | Pass (green) fixture |
| --- | --- | --- | --- |
| `consumer-ci-emitter` | Consumer CI-workflow emitters | `enforcer init` with the 5 github-actions adapters that emits into a temp repo but MISSES any of `.github/workflows/{ocentra-enforcer,codeql,dependency-policy,secret-scan,sbom}.yml`, OR whose bytes drift from the bundled `adapters/github-actions/*.yml` templates → fails naming the missing/drifted file; a `--dry-run` run that touches ≥1 file on disk → fails | `enforcer init` in a temp repo writes exactly the expected 5-workflow set (golden bytes under `tests/fixtures/consumer-ci/`); the same `--dry-run` run returns the 5-entry planned write set with ZERO files created on disk |
| `git-hooks-emitter` | Pre-commit hook emitters (git-hook / husky / lefthook) | selecting `precommit` alone that also writes `.husky/pre-commit` or `lefthook.yml` (cross-flavor bleed), OR any selected flavor whose bytes drift from its `adapters/{git-hooks/pre-commit.sh,husky/pre-commit,lefthook/lefthook.yml}` template → fails naming the offending file | each flavor selected alone writes ONLY its own file (`.git/hooks/pre-commit` / `.husky/pre-commit` / `lefthook.yml`) with golden bytes under `tests/fixtures/git-hooks/`, and `--dry-run` writes zero |

Both new rows share the `proof/install/c07-generic.json` artifact and are gated by the same `cargo test -p enforcer-install` run as the generic-writer/doctor rows.

## Parallel Ownership Notes
Owns `crates/enforcer-install/src/adapters/generic.rs`, `crates/enforcer-install/src/doctor.rs`, and the two install-emitters `crates/enforcer-install/src/emitters/{consumer_ci.rs,git_hooks.rs}` (+ their `tests/fixtures/`) only — the crate skeleton, `Adapter` trait, `emitters` module barrel, and registry belong to arc-23. Disjoint by file from codex (c06), claude (c03), and stub (c08) adapters. The consumer-CI emitter is disjoint from c10 (which owns the enforcer's OWN release pipeline + `enforcer-scan` action, not the per-consumer workflow generation); the git-hook emitters are disjoint from c04/c05 (Claude PreToolUse/SessionStart hooks — a different mechanism). Depends on c01/c02 and arc-23. Runs concurrently with all other adapter workpacks. owns disjoint? = Y
