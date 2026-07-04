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
- [ ] Five subcommands registered in the clap dispatch of `enforcer-cli` with stable exit-code semantics (a phase failure yields a non-zero `enforcer` exit).
- [ ] Each phase delegates its verdict to a named oracle: a Rust type invoking the relevant `Validator`s / `Report` (e.g. `check` -> the `enforcer-scan` registry validators; `review` -> the d10 auditor obligation rows).
- [ ] `fix` invokes the d07 `enforcer-coordination` fix loop; `review` requires green proof rows (from `enforcer-proof`) before it can pass.
- [ ] No phase can report success unless its oracle returns a pass `Finding` set; there is no prose-only pass path (obey `[workspace.lints]` — no `print_*`/`unwrap`, verdicts flow through the CLI output sink module).
- [ ] Phase transitions recorded as d04 telemetry records (versioned serde struct in `enforcer-domain`, appended via the `enforcer-core` NDJSON sink).

## Acceptance And Proof
Tier T1 (P1 unit). Prove via `cargo test -p enforcer-cli` (`crates/enforcer-cli/tests/lifecycle.rs`): each subcommand routes to the correct oracle; a failing oracle forces a non-zero exit from the built `enforcer` binary; `review` blocks on missing proof rows. Mechanism: a dispatch table mapping each phase to its oracle function, asserted with stubbed-oracle outcomes and integration runs of the binary against fail/pass fixture trees.

## Parallel Ownership Notes
Depends on d01 for the validator registry and on arc-22 for the `enforcer-cli` crate skeleton (clap dispatch + output sink) it plugs into. Owns only `src/lifecycle.rs`, its `src/lifecycle/oracle.rs` submodule, and `tests/lifecycle.rs` inside `enforcer-cli` — disjoint from the arc-22 skeleton by file. Wraps d07 (fix) and d10 (review) by contract, built against their interfaces concurrently. owns disjoint? = Y (deps arc-22 sequences it after the crate skeleton exists).
