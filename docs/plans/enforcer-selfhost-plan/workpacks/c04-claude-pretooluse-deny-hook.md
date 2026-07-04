# c04 Claude PreToolUse Deny Hook

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Claude PreToolUse Deny Hook`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `src/install/hooks/pretooluse-*, src/install/hooks/guard-*`
- deps: `c01-install-core-and-cli-contract, c03-claude-adapter`
- tier: `P5 install-proof`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md).

## Where We Are
Installed enforcer tools (`ocentra_enforcer_scan`, `ocentra_enforcer_check`) exist but nothing forces the agent to run them before writing. Guidance-only installs are prose, not proof: an agent can ignore a skill. There is no mechanical gate on file edits.

## Where We Want To Be
A Claude PreToolUse deny-hook that, on `Edit|Write|MultiEdit`, runs the enforcer scan/check plus coordination guard against the pending change and BLOCKS deterministic (T1) violations before the write lands. This is the T1 mechanical bridge that makes self-enforcement real, not advisory.

## Requirement Checklist
- [ ] Hook reads the PreToolUse payload (tool name + target path + proposed content) from stdin.
- [ ] On `Edit|Write|MultiEdit` it invokes scan/check + coordination guard on the candidate content.
- [ ] T1 (hard validator) violation -> exit deny with `ruleId` and the `fix` string in the reason. T2 (scored) -> allow with warning surfaced. T3 -> never blocks.
- [ ] Fail-closed on enforcer error/timeout for T1 scope; other tools pass through untouched.
- [ ] Hook is emitted/registered by the Claude adapter (c03) and is idempotent to install.

## Acceptance And Proof
P5 mechanical-bridge proof (`claude-deny-hook-blocks` in TEST_PROOF_EXPECTATIONS.md): feed the hook a PreToolUse payload for a **seeded violating edit** and assert exit code = deny AND stderr/JSON reason contains the exact `ruleId` and its `fix`. A conforming edit must exit allow. A T2-only finding must exit allow-with-warning, never deny.

## Parallel Ownership Notes
Owns only `src/install/hooks/pretooluse-*` and `guard-*`, disjoint from c05 which owns `src/install/hooks/sessionstart-*`. Both are registered by the c03 adapter but live in separate files, so c04 and c05 run concurrently. Depends on c01/c03 for interface and registration.
