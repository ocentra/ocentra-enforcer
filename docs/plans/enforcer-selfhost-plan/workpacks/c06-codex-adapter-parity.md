# c06 Codex Adapter Parity

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Codex Adapter Parity`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `src/install/adapters/codex.*`
- deps: `c01-install-core-and-cli-contract`
- tier: `P5 install-proof`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md).

## Where We Are
`src/codex-install.mjs` is the working, shipped Codex integration (TOML `mcp_servers` upsert, skill copy, `AGENTS.md` managed block, doctor with 15+ checks, backups). Once c01 lifts the shared helpers, Codex must be re-expressed as an adapter behind the c01 interface without regressing any existing behavior.

## Where We Want To Be
A `src/install/adapters/codex.*` adapter that produces exactly the same on-disk result as today's `codex-install.mjs`, now driven through the c01 `plan/apply/verify` interface.

## Requirement Checklist
- [ ] Re-express TOML `mcp_servers.<name>` upsert, ledger env, and skill copy as adapter `plan/apply`.
- [ ] Preserve `AGENTS.md` managed-block start/end markers and content byte-for-byte.
- [ ] Preserve timestamped backup filenames and the doctor check set (including warning severities).
- [ ] `verify` mirrors the existing doctor checks (node, effect dep, server file, TOML section, cwd, enabled).
- [ ] Existing CLI entrypoints (`codex-install`, `codex-doctor`) keep working via the new adapter.

## Acceptance And Proof
P5 parity (`codex-adapter-parity` in TEST_PROOF_EXPECTATIONS.md): a golden-file test asserts the adapter's generated TOML block and `AGENTS.md` block equal the current `codexMcpTomlBlock`/`globalAgentsInstructionBlock` output; doctor check names/severities match a pinned snapshot. Any diff fails the build.

## Parallel Ownership Notes
Owns only `src/install/adapters/codex.*`. Disjoint from claude (c03), generic (c07), and stub (c08) adapters, so it runs concurrently. Depends only on c01.
