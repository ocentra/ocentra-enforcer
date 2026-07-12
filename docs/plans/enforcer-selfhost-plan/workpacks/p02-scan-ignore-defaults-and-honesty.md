# p02 Scan Ignore Defaults And Ignored-Surface Honesty

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Scan Ignore Defaults And Ignored-Surface Honesty`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling completion, product status, PR readiness, or broad DONE.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `crates/enforcer-scan/src/ignore/**`, `crates/enforcer-scan/tests/ignore_defaults.rs`, `crates/enforcer-scan/tests/fixtures/ignore_defaults/**`, `crates/enforcer-ui/src/skipped/**`
- deps: `arc-15`, `a09`, `arc-03`, `g02`
- tier: `P4` (with `P1` unit for the ignore engine + `P3` for the UI surface as secondary rows)

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md), [RUST_ARCHITECTURE](../RUST_ARCHITECTURE.md).

## Where We Are
Lesson from 2026-07-12: a self-scan reported ~82.8k findings, of which roughly 55k came from `node_modules/`, `dist/`, vendored trees, and fixture dirs — because the ignore globs were leaky (a root-anchored `vendor/**`, no `node_modules` entry at all, no build-output defaults). The scan walk (arc-15 `src/walk.rs`) has no built-in default ignore set, so out-of-the-box it drowns real product findings in dependency and build noise. Worse, the noise was SILENT the other way too: nothing reported which trees were skipped or why, so a leaky-or-overzealous ignore was invisible. `a09` already models honest per-target outcomes (`Outcome::Ran | Outcome::Skipped { reason }`) at the validator-dispatch stage, but the WALK-stage ignore decisions (whole directories never descended into) are not accounted for anywhere.

## Where We Want To Be
A built-in DEFAULT IGNORE SET in the Rust scan engine (`crates/enforcer-scan/src/ignore/`), covering the universally-noise directories — `node_modules`, `vendor`, `target`, `build`, `dist`, `.git`, harness dot-dirs (`.enforce`, `.harness`, `.claude`, `.codex`), and declared fixture dirs — MERGED with per-project `ignoreFileGlobs` from `enforcer-config` (arc-03) and OVERRIDABLE (a project can un-ignore a default). Ignore globs are correctly anchored (a bare `vendor` matches at any depth, not just root). Alongside it, an IGNORED-SURFACE INVENTORY: the walk records, per ignore reason, how many files and directories it skipped, expressed via the `a09` `SkipReason`/`Outcome` vocabulary (reused, not re-invented) so the "what we did not scan and why" is a first-class, serialized part of the `enforcer-domain` `Report` — anti-silent-skip extended from the validator stage to the walk stage. Finally, a HUMAN UI surface (`crates/enforcer-ui/src/skipped/`, mounting into g01 beside the g02 report view) that shows the skip inventory — "we did not scan N files across these trees for these reasons" — with a TOGGLE to include an ignored tree in a re-scan. Out-of-the-box, scanning this repo yields PRODUCT-ONLY findings and a visible skip inventory, never a silent 55k-of-noise dump nor a silent over-skip.

## Requirement Checklist
- [ ] A `DefaultIgnoreSet` in `crates/enforcer-scan/src/ignore/` listing the built-in noise dirs (`node_modules`, `vendor`, `target`, `build`, `dist`, `.git`, harness dot-dirs, declared fixture dirs) with correct any-depth anchoring; the arc-15 walk consults it through the skeleton seam (arc-15 owns `walk.rs`; this pack owns the ignore provider it calls).
- [ ] Defaults MERGE with per-project `ignoreFileGlobs` (from `enforcer-config`, arc-03) and are OVERRIDABLE: a project can re-include a default-ignored path, and a project glob is additive to the defaults — neither silently wins.
- [ ] A per-reason SKIP INVENTORY records files/dirs skipped, keyed by the `a09` `SkipReason`/`Outcome::Skipped { reason }` vocabulary (reused, not duplicated), and is serialized into the `enforcer-domain` `Report` so a hollow or over-broad ignore is visible.
- [ ] `crates/enforcer-ui/src/skipped/` renders the skip inventory at the Rust boundary (mounts into g01 beside the g02 report seam), with a toggle to include an ignored tree in a re-scan; honors f04 silent mode (no UI during inline agent runs); disjoint by file from g02 `src/report/`.
- [ ] No ignore decision is silent in either direction: an ignored tree always appears in the inventory with a non-empty reason; re-including a tree is reflected in the ran-count.

## Acceptance And Proof
Tier P4 (self-enforce green), with a P1 unit row for the ignore engine and a P3 row for the UI. Proof row `scan-ignore-defaults-honesty` in TEST_PROOF_EXPECTATIONS.md:
- P1 fail/pass fixtures (`cargo test -p enforcer-scan --test ignore_defaults`): a fixture tree containing `node_modules/`, `dist/`, and a product `src/` -> the default set skips the noise dirs (each recorded in the inventory with its reason) and scans only `src/`; a per-project override that re-includes `dist/` makes those files RAN (ran-count rises); a bare `vendor` default matches a nested `a/b/vendor/**`, not only root `vendor/**` (anchoring regression fixture).
- P4 self-enforce: running the built enforcer's scan on THIS repo out-of-the-box yields product-only findings (no `node_modules`/`dist`/`vendor`/fixture noise) AND emits a skip inventory with nonzero skipped counts and reasons — a hollow scan (zero ran) still hard-fails per a09.
- P3 UI: `report-skipped-surface` asserts the `crates/enforcer-ui/src/skipped/` view renders the inventory rows (tree + reason + count) and suppresses all output under f04 silent mode; no external asset fetched.
Clean `cargo clippy` / `cargo fmt --check` (obey `[workspace.lints]`).

## Parallel Ownership Notes
Owns the new `crates/enforcer-scan/src/ignore/` sub-dir + its `tests/ignore_defaults.rs` + `tests/fixtures/ignore_defaults/**`, and the `crates/enforcer-ui/src/skipped/` view — all disjoint by file. Deps arc-15 (walk skeleton exposes the ignore-provider seam this pack fills, dep-sequenced), a09 (reuses its `Outcome`/`SkipReason` model rather than defining a parallel one — coordinate `mod`/`pub use` in `enforcer-scan/src/lib.rs`, additive), arc-03 (per-project `ignoreFileGlobs`), and g02/g01 (the report surface this view mounts beside). Disjoint from a09 (validator-stage coverage) which it EXTENDS to the walk stage, and from g02 `src/report/` (violation matrix) which shows what we DID scan while this shows what we did NOT. Orthogonal to `p01` (profiles) and `p03` (AST). `owns disjoint? = Y` (the `enforcer-scan/src/lib.rs` `mod` line is an additive coordination point with arc-15/a09, not a claimed file).
