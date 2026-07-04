# c02 Harness Autodetect

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Harness Autodetect`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `src/install/detect.*, src/install/detect-probes.*`
- deps: `c01-install-core-and-cli-contract`
- tier: `P1 unit`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md).

## Where We Are
`codex-install.mjs` only knows Codex; it derives `CODEX_HOME` from env and never probes for other harnesses. Installing across "any harness" requires knowing which harnesses are present. Today there is no detection layer.

## Where We Want To Be
A deterministic autodetect module that probes for installed harnesses (`~/.claude`, `~/.codex`, `~/.gemini`, plus cursor/zed markers) and returns a normalized list of detected adapters with their home paths.

## Requirement Checklist
- [ ] Probe candidate home dirs from env overrides then defaults (`USERPROFILE`/`HOME`), honoring `CODEX_HOME`, `CLAUDE_HOME`, etc.
- [ ] Emit a normalized record per harness: `{ id, present, homePath, evidence }`.
- [ ] Detection is pure over an injected `fs`/`env` (no ambient globals) so it is unit-testable with fixtures.
- [ ] `granby doctor` and `install` consume detection to pick adapters when `--scope`/adapter list is not pinned.
- [ ] Unknown/ambiguous state is reported as `present:false` with `evidence`, never guessed.

## Acceptance And Proof
T1: unit tests (`harness-autodetect` in TEST_PROOF_EXPECTATIONS.md) run against a temp home fixture with seeded `.claude`/`.codex` dirs and assert exact detected-adapter sets, including the empty-home case (no false positives) and env-override precedence.

## Parallel Ownership Notes
Depends on c01 for the adapter id vocabulary. Owns only the detect module; it does not write adapter behavior, so it runs concurrently with c03-c08 once c01 lands.
