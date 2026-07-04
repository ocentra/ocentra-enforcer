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

- owns: `src/onboard/*`, `.enforce/` scaffolding writer, cli `enforcer onboard`, mcp onboard tool
- deps: `a01-ts-toolchain-and-build, f03-project-tie-and-native-augment`
- tier: `P1/P5`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md).

## Where We Are
After harness install there is no first-run step that binds the enforcer to a repo. `.enforce/` does not exist until something writes it; there is no project profile, no baseline, and no registration. The enforcer has nothing to compare against on first scan.

## Where We Want To Be
An index-on-ask onboarding (codebase-memory style): the agent or user triggers `enforcer onboard <repo>`, which creates `.enforce/`, resolves and writes the project profile (via the f03 config schema), runs a baseline scan, and registers the project. Onboarding may be prompted right after the harness install but is always explicit and re-runnable (idempotent).

## Requirement Checklist
- [ ] `enforcer onboard <repo>` (CLI + MCP onboard tool) creates `.enforce/` with the resolved project profile written via the f03 schema.
- [ ] Runs a baseline scan and persists the baseline artifact under `.enforce/`.
- [ ] Registers the project (deterministic project id) so later scans resolve it.
- [ ] Idempotent: re-running does not duplicate or corrupt `.enforce/`; existing waivers/config are preserved.
- [ ] Onboarding is explicit (no silent auto-run); may be surfaced as a post-install prompt only.

## Acceptance And Proof
Tier P1/P5. Proof row `onboard-scaffolds-enforce` in TEST_PROOF_EXPECTATIONS.md:
- fail-fixture: run a scan on a repo with no `.enforce/` -> asserts "not onboarded" error (no baseline to compare).
- pass-fixture: `enforcer onboard` on a fresh repo -> asserts `.enforce/` exists with profile + baseline + registration entry.
- detection test: onboard run twice -> second run is idempotent (byte-identical config, preserved waivers), asserted by comparing `.enforce/` state.

## Parallel Ownership Notes
Owns `src/onboard/*` and the `.enforce/` scaffolding writer only. Consumes the f03 config schema (dep) and does not define it. Disjoint from f01 (scan modes) though it invokes a baseline scan through the shared engine.
