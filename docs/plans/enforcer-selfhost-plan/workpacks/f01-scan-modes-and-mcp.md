# f01 Scan Modes And MCP

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Scan Modes And MCP`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling completion, product status, PR readiness, or broad DONE.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `src/scan/modes.*`, `src/scan/modes-schema.*`, mcp `enforcer_scan` tool schema, cli scan-mode dispatch
- deps: `a01-ts-toolchain-and-build, d01-rule-mechanization-engine`
- tier: `P1/P3`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md).

## Where We Are
Scanning is all-or-nothing: the agent can only run a broad check with no named scope. There is no schema that lets an AI agent, while coding, pick "just this crate" or "just the diff." Whole-repo runs are the only path, which is slow and wrong as an inline default.

## Where We Want To Be
An `enforcer_scan` MCP tool plus `enforcer scan --mode <m>` CLI with named MODES the agent selects: `quick` (fast most-common T1 subset), `full` (everything the enforcer can do), `repo`/`workspace`, `scoped` (crate-or-folder), `diff` (changed files only), and `plan-scan` (validate a plan dir). Scope+depth are schema-driven; default is scoped-not-whole-repo. The tool is agent-callable so the AI decides what to run inline.

## Requirement Checklist
- [ ] A mode enum + JSON schema (scope path, depth, tier filter) validated at the MCP/CLI boundary.
- [ ] Each mode maps to a deterministic rule/scope selection over the d01 engine; `quick` = named T1 subset, `full` = all tiers.
- [ ] Default when no scope given is `scoped` (cwd crate/folder), never whole-repo.
- [ ] `diff` mode reads changed paths; `plan-scan` targets a plan dir.
- [ ] MCP tool name is `enforcer_scan` (mcp__enforcer__*); CLI is `enforcer scan --mode`.

## Acceptance And Proof
Tier P1/P3. Proof row `scan-modes-select` in TEST_PROOF_EXPECTATIONS.md:
- fail-fixture: a `full`-only violation seeded outside the scoped path -> asserts `scoped`/`quick` does NOT report it (scope honored).
- pass-fixture: same violation inside scope -> `scoped` reports it; `full` always reports it.
- detection test: invalid mode string is rejected at the schema boundary (non-zero/error), and each mode resolves to its expected rule/scope set.

## Parallel Ownership Notes
Owns `src/scan/modes.*` and the `enforcer_scan` schema only; consumes the d01 engine and a01 toolchain. Disjoint from f03 (project-config) and f04 (run-context mode), which f01 references but does not own.
