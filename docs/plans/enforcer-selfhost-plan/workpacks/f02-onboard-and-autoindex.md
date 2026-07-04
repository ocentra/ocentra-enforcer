# f02 Onboard And Autoindex

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Onboard And Autoindex`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling completion, product status, PR readiness, or broad DONE.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `crates/enforcer-scan/src/onboard.rs`, `crates/enforcer-scan/tests/onboard.rs`, `crates/enforcer-scan/tests/fixtures/onboard/**`
- deps: `arc-15-enforcer-scan, arc-22-enforcer-cli, f03-project-tie-and-native-augment`
- tier: `P1/P5`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md), [RUST_ARCHITECTURE](../RUST_ARCHITECTURE.md).

## Where We Are
After harness install there is no first-run step that binds the enforcer to a repo. `.enforce/` does not exist until something writes it; there is no project profile, no baseline, and no registration. The enforcer has nothing to compare against on first scan. The arc-15 scan engine can run, but nothing scaffolds the per-repo state it needs.

## Where We Want To Be
An index-on-ask onboarding (codebase-memory style) in `crates/enforcer-scan/src/onboard.rs`: the agent or user triggers `enforcer onboard <repo>`, which creates `.enforce/`, resolves and writes the project profile (via the f03 `enforcer-config` `ProjectConfig` type — deserialized/serialized through serde, never hand-rolled), runs a baseline scan over the arc-15 engine, and registers the project (deterministic project id, an `enforcer-domain` newtype). Onboarding may be prompted right after the harness install but is always explicit and re-runnable (idempotent). The `enforcer onboard` CLI subcommand (clap, arc-22) and an MCP onboard tool both call this one Rust module.

## Requirement Checklist
- [ ] `enforcer onboard <repo>` (CLI subcommand in `enforcer-cli` + an MCP onboard tool) drives `onboard.rs`, which creates `.enforce/` with the resolved project profile written via the f03 `ProjectConfig` serde type.
- [ ] Runs a baseline scan through the arc-15 engine and persists the baseline artifact under `.enforce/` (typed serde record, not free-form JSON).
- [ ] Registers the project with a deterministic project id (`enforcer-domain` branded newtype) so later scans resolve it.
- [ ] Idempotent: re-running does not duplicate or corrupt `.enforce/`; existing waivers/config are preserved (byte-identical serialize of unchanged config).
- [ ] Onboarding is explicit (no silent auto-run); it may be surfaced as a post-install prompt only. Uses the `enforcer-core` `Result`/`Error`; no `unwrap`/`expect`/`print_*` per `[workspace.lints]`.

## Acceptance And Proof
Tier P1/P5. Proof row `onboard-scaffolds-enforce` in TEST_PROOF_EXPECTATIONS.md asserts `cargo test -p enforcer-scan --test onboard` exits 0:
- fail-fixture: run a scan on a repo with no `.enforce/` (`tests/fixtures/onboard/not-onboarded/`) -> asserts a "not onboarded" typed error (no baseline to compare).
- pass-fixture: `enforcer onboard` on a fresh repo fixture -> asserts `.enforce/` exists with profile + baseline + registration entry, each round-tripping through serde.
- detection test: onboard run twice -> second run is idempotent (byte-identical serialized config, preserved waivers), asserted by comparing `.enforce/` state.

## Parallel Ownership Notes
Owns ONLY `crates/enforcer-scan/src/onboard.rs` + its `tests/onboard.rs` and `tests/fixtures/onboard/**` — disjoint files inside the arc-15 crate (which owns the crate skeleton + fan-out engine). Consumes the f03 `ProjectConfig` type (dep) and does not define it; consumes the arc-22 CLI shell for the subcommand wiring. Disjoint by file from f01 (`modes.rs`) and f05 (`router/**`) though it invokes a baseline scan through the shared arc-15 engine. `owns disjoint? = Y`.
