# c01 Install Core And CLI Contract

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Install Core And CLI Contract`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `crates/enforcer-install/src/core.rs, crates/enforcer-install/src/cli_contract.rs, crates/enforcer-install/src/report.rs, crates/enforcer-install/tests/fixtures/install_core/**`
- deps: `arc-23`, `arc-03`
- tier: `P1 unit`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md), [RUST_ARCHITECTURE](../RUST_ARCHITECTURE.md).

## Where We Are
The legacy `.mjs` installer (`scripts/rust-rules-output-codex-*.mjs`, `validate-codex-assets.mjs`) hardcodes the Codex adapter: report/apply pairs, managed-block upsert, timestamped backups, and doctor checks all live in ad-hoc scripts. The arc-23 `enforcer-install` crate skeleton (`Cargo.toml`, `src/lib.rs`, adapter module-root) now exists, but the harness-neutral install CORE and the CLI contract it exposes are not yet built as Rust modules.

## Where We Want To Be
A harness-neutral install core in `enforcer-install` (`src/core.rs`) exposing `install / uninstall / update / doctor` over a pluggable `HarnessAdapter` trait, plus a stable `enforcer install` CLI contract (`src/cli_contract.rs`) with tri-modal scope (`--scope user|project`), `--dry-run`, and non-TTY JSON output. Report/result types (`src/report.rs`) are `serde` records built on `enforcer-domain` branded newtypes, camelCase on the wire, so the CLI and the Tauri UI read the same shapes.

## Requirement Checklist
- [ ] Lift managed-block, backup, and report/apply helpers out of the legacy `.mjs` scripts into `src/core.rs` (adapter-agnostic), returning structured `Report`/`ApplyResult`/`Checks` records defined in `src/report.rs`.
- [ ] Define the `HarnessAdapter` trait (in `src/core.rs`): `fn plan(&self, ctx: &InstallCtx) -> Result<Report>`, `fn apply(&self, report: &Report) -> Result<ApplyResult>`, `fn verify(&self, ctx: &InstallCtx) -> Result<Checks>`. Errors are `thiserror` typed (`enforcer-core` Result/Error).
- [ ] `install/uninstall/update/doctor` orchestrators in `src/core.rs` iterate registered adapters and aggregate their reports into one typed result.
- [ ] `src/cli_contract.rs` defines the clap-facing `enforcer install` contract: `--scope user|project`, `--dry-run` produces zero writes, non-TTY emits machine-readable JSON (camelCase serde) with a stable `command`/`checks` schema; NO override flag.
- [ ] `--dry-run` report is byte-identical in shape to the applied report minus the `applied` flag (serde round-trip asserted).
- [ ] Report/result records are `serde` newtypes over `enforcer-domain` (e.g. `RepoRoot`, `RelPath`) with parse-at-boundary; no bare `String` for paths/ids. Obey `[workspace.lints]` (no `unwrap/expect/panic/print_*`); no `pub use` barrels.
- [ ] Fold the skill-asset VALIDATOR from `scripts/validate-codex-assets.mjs` into the install crate as a `doctor`/`verify` check (a `HarnessAdapter::verify` contribution, or a shared core check the doctor orchestrator runs): assert each skill's declared `SKILL.md` assets exist on disk AND that the `.codex-plugin/plugin.json` `plugin.skills === "./skills/"` publish contract holds. Emit typed `Checks` records (`src/report.rs`), fail-closed. This replaces the ad-hoc `.mjs` script; the legacy Codex-specific canonical/legacy skill paths become adapter-provided asset manifests, not hardcoded here. (WAVE 6: `validate-codex-assets.mjs` re-home → c01.)
- [ ] The shared install payload every adapter (c03/c06/c08/c09) ships to its harness includes a **parallel-execution doctrine snippet**: for multi-agent parallel work via this coordination hub, spawn one worktree per lane using `enforcer coordination lane new` (arc-16) — never share `Cargo.lock`/`target/`/`node_modules`/build cache across lanes; parking or deleting-and-rebuilding a lane's worktree is normal, not a failure. This is harness-neutral text `src/core.rs` hands adapters to embed (each adapter places it wherever its harness reads doctrine — e.g. a CLAUDE.md/AGENTS.md block, a Cursor rule, a skill file), not a Claude-specific file.

## Acceptance And Proof
Tier P1 (`install-core-contract` in TEST_PROOF_EXPECTATIONS.md): `cargo test -p enforcer-install` covers the `HarnessAdapter` trait shape via a fake in-crate adapter, asserts `--dry-run` writes zero files (temp-dir fixture under `tests/fixtures/install_core/**`, filesystem diff empty), and that non-TTY output deserializes as JSON with a stable `command`/`checks` schema. Fail-closed: an unknown adapter id must return a typed error, not skip silently. The skill-asset doctor check has a fail fixture (a missing/renamed skill `SKILL.md` asset, or a `.codex-plugin/plugin.json` with `plugin.skills` not equal to `"./skills/"`, must make the check FAIL) and a pass fixture (intact assets + correct `plugin.skills` path resolves clean), both under `tests/fixtures/install_core/**`. Clean `cargo clippy` / `cargo fmt --check`.

## Parallel Ownership Notes
Blocks c02-c09 (they consume the `HarnessAdapter` trait and core orchestrators). Owns only the core/CLI-contract/report modules of the arc-23 crate; the arc-23 pack owns the crate skeleton (`Cargo.toml`, `src/lib.rs`, adapter module-root) and adapters live under `crates/enforcer-install/src/adapters/**` + hooks under `src/hooks/**` owned by siblings, so all Track C adapter/hook work runs concurrently once this and arc-23 land. owns disjoint? = Y.
