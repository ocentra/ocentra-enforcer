# x03 Rename Migration

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Rename Migration`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling completion, product status, PR readiness, or broad DONE.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `src/install/migrate-legacy-name.*`, `tests/migrate-legacy-name/**`
- deps: `x01`
- tier: `P1`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md), [ADBP_GAPS](../ADBP_GAPS.md).

## Where We Are
x01 renamed the shipped/config surfaces from `ocentra-enforcer` to `enforcer`. But existing installs still carry a legacy `ocentra-enforcer` MCP server registration in harness configs and legacy `rust_rules_*` / `ocentra_enforcer_*` tool-name usages. Nothing detects or rewrites those already-installed entries, so an upgraded machine would keep serving the old name. The doctrine forbids lingering `ocentra`: the fix is to UPGRADE existing installs, not to keep the old name alive.

## Where We Want To Be
A one-time transitional migration `src/install/migrate-legacy-name.*` (invoked by doctor/migrate) that: detects any existing `ocentra-enforcer` MCP registration in any harness config plus legacy `rust_rules_*` / `ocentra_enforcer_*` tool-name usages; rewrites the registration to `enforcer` (tools resolve as `mcp__enforcer__*`); drops the deprecated aliases with a single one-time migration notice; and reports exactly what changed. This is transitional migration, NOT a permanent alias — after migration, zero `ocentra-enforcer` entries remain.

## Requirement Checklist
- [ ] Detect existing `ocentra-enforcer` MCP registration across all supported harness config locations.
- [ ] Detect legacy `rust_rules_*` and `ocentra_enforcer_*` tool-name usages.
- [ ] Rewrite the registration to `enforcer` (`mcp__enforcer__*`) and drop deprecated aliases with one one-time notice.
- [ ] Report a diff of what changed; migration is idempotent (re-run is a no-op).
- [ ] No permanent alias retained — a post-migration re-scan finds zero `ocentra-enforcer` entries.

## Acceptance And Proof
Tier P1 (deterministic). Fail-fixture: `migrate-legacy-config-present` — a harness config containing the old `ocentra-enforcer` server entry left unmigrated (re-scan still finds the old entry = fail). Pass-fixture: `migrate-legacy-config-rewritten` — after migrate runs on that config, a re-scan finds zero `ocentra-enforcer` entries and the registration reads `enforcer`. Detection test: `rename-migration-contract` asserts detection of the legacy entry + legacy tool names, the rewrite to `enforcer`, the one-time notice, idempotent re-run, and the zero-lingering-`ocentra` post-scan. Rows in TEST_PROOF_EXPECTATIONS.md.

## Parallel Ownership Notes
Owns `src/install/migrate-legacy-name.*` and `tests/migrate-legacy-name/**` exclusively. Depends on x01 (the shipped rename) so the migration target name `enforcer` is stable. Does not edit x01's shipped/config surfaces or sibling install code; it only reads/rewrites already-installed harness configs. Transitional, not a permanent compatibility shim.
