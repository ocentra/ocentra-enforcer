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

- owns: `src/install/core.*, src/install/index.*, src/install/cli-contract.*, src/install/report-types.*`
- deps: `none`
- tier: `P1 unit`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md).

## Where We Are
`src/codex-install.mjs` (417 lines) hardcodes the Codex adapter: report/apply pairs, managed-block upsert, timestamped backups, and doctor checks all live in one file. There is no harness-neutral core, and the CLI wires Codex directly in `src/cli-command-dispatch.mjs` (`codex-install`, `codex-doctor`).

## Where We Want To Be
A harness-neutral `src/install/core` exposing `install / uninstall / update / doctor` over a pluggable adapter interface, plus a stable `granby` CLI contract with `--scope`, `--dry-run`, non-TTY JSON output.

## Requirement Checklist
- [ ] Lift managed-block, backup, and report/apply helpers out of `codex-install.mjs` into shared core (adapter-agnostic).
- [ ] Define the adapter interface: `plan(ctx) -> report`, `apply(report) -> result`, `verify(ctx) -> checks`.
- [ ] `install/uninstall/update/doctor` orchestrators iterate adapters and aggregate reports.
- [ ] `granby` CLI contract: `--scope user|project`, `--dry-run` produces no writes, non-TTY emits machine-readable JSON.
- [ ] `--dry-run` report is byte-identical in shape to the applied report minus `applied:true`.

## Acceptance And Proof
T1: unit tests in TEST_PROOF_EXPECTATIONS.md (`install-core-contract`) assert the adapter interface shape, that `--dry-run` writes zero files (temp-dir fixture, filesystem diff empty), and that non-TTY output parses as JSON with a stable `command`/`checks` schema. Fail-closed: unknown adapter id must error, not skip silently.

## Parallel Ownership Notes
Blocks c02-c08 (they consume the adapter interface and core orchestrators). Its `owns:` set is the core/CLI only; adapters live under `src/install/adapters/**` owned by siblings, so all Track C adapter work runs concurrently once this lands.
