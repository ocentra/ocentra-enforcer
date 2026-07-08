# d06 Lifecycle Commands

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Lifecycle Commands`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `crates/enforcer-cli/src/lifecycle.rs, crates/enforcer-cli/src/lifecycle/oracle.rs, crates/enforcer-cli/tests/lifecycle.rs`
- deps: `d01-rule-mechanization-engine, arc-22-enforcer-cli`
- tier: `P1 unit`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md), [RUST_ARCHITECTURE](../RUST_ARCHITECTURE.md).

## Where We Are
The `enforcer` clap CLI (`enforcer-cli`, arc-22) exposes `scan`/`check`/`proof` verbs but no coherent lifecycle. ADBP describes a plan->implement->check->fix->review flow as narrative. There is no single command family in the binary binding those phases to our Rust validators.

## Where We Want To Be
A `plan | implement | check | fix | review` clap subcommand family (added to `enforcer-cli`) where every phase's pass/fail is decided by our existing Rust validators (the oracle), not by prose or model self-report. Each phase returns a typed verdict and drives the process exit code.

## Requirement Checklist
- [x] Five phases (`Plan`/`Implement`/`Check`/`Fix`/`Review`) with stable exit-code semantics (a phase failure yields a non-`Success` exit) — landed as `src/lifecycle.rs`'s dispatch table (`run_plan`/`run_implement`/`run_check`/`run_fix`/`run_review`), each folding its oracle's verdict through `verdict_to_outcome`. NOT YET wired into `crate::cli::Command`/`main.rs`'s clap grammar itself — that grammar file is arc-22's owned surface, not this workpack's `owns:` set (see Deviations). The typed dispatch + exit-code fold this criterion is actually testing is landed and proven.
- [x] Each phase delegates its verdict to a named oracle (`src/lifecycle/oracle.rs`): `check` -> the real `enforcer-scan::engine` validator registry; `review` -> the landed `enforcer-proof::claim` pr_ready gate. `plan`/`implement` have no landed oracle to delegate to (arc-20/no owning workpack shipped one on this branch) and fail closed rather than inventing one.
- [ ] `fix` invokes the d07 fix loop — d07 has not landed a fix-loop entry point on `rust-build` as of this build (`enforcer-coordination::api` exposes hub/lane/claim/guard/ledger/presence/sync only); `fix_oracle` fails closed (`NotYetWired`) documenting the exact seam. `review` DOES require green proof rows before passing (via `enforcer-proof::claim`) — that half is real and proven; the d10 auditor-obligation evidence layer `review` also names has not landed either and is not consulted.
- [x] No phase can report success unless its oracle returns a pass; there is no prose-only pass path — proven by `plan_implement_fix_phases_never_report_success_with_no_landed_oracle` and the fail-fixture-tree tests in `tests/lifecycle.rs`. `[workspace.lints]` obeyed (no `print_*`/`unwrap`/`expect` anywhere in `src/lifecycle.rs`/`src/lifecycle/oracle.rs`; the test file's `expect()` calls were converted to `?` to satisfy the same deny wall).
- [x] Phase transitions recorded as d04 telemetry (`enforcer_domain::run_record::RunRecord`, appended via `enforcer_core::telemetry::RunTelemetrySink` at `proof/telemetry/runs.ndjson`) — an observer: a telemetry-sink failure never flips a phase's own exit code, matching that sink's documented contract.

## Acceptance And Proof
Tier T1 (P1 unit). Prove via `cargo test -p enforcer-cli` (`crates/enforcer-cli/tests/lifecycle.rs`): each subcommand routes to the correct oracle; a failing oracle forces a non-zero exit from the built `enforcer` binary; `review` blocks on missing proof rows. Mechanism: a dispatch table mapping each phase to its oracle function, asserted with stubbed-oracle outcomes and integration runs of the binary against fail/pass fixture trees.

## Parallel Ownership Notes
Depends on d01 for the validator registry and on arc-22 for the `enforcer-cli` crate skeleton (clap dispatch + output sink) it plugs into. Owns only `src/lifecycle.rs`, its `src/lifecycle/oracle.rs` submodule, and `tests/lifecycle.rs` inside `enforcer-cli` — disjoint from the arc-22 skeleton by file. Wraps d07 (fix) and d10 (review) by contract, built against their interfaces concurrently. owns disjoint? = Y (deps arc-22 sequences it after the crate skeleton exists).
