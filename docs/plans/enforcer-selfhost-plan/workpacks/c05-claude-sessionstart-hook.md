# c05 Claude SessionStart Hook

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Claude SessionStart Hook`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `src/install/hooks/sessionstart-*`
- deps: `c01-install-core-and-cli-contract, c03-claude-adapter`
- tier: `P5 install-proof`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md).

## Where We Are
The Codex install writes a global `AGENTS.md` block (`globalAgentsInstructionBlock`) so the agent sees enforcer guidance, but this is static text loaded only if the harness reads that file. Claude Code has no equivalent injected-at-session-start reminder wired by the installer, so the enforcer-first doctrine is not reliably present each session.

## Where We Want To Be
A Claude SessionStart hook, installed by the c03 adapter, that injects an enforcer-first reminder plus the mechanical-enforcement doctrine (T1/T2/T3, prefer scan/check before edits) into every new session's context.

## Requirement Checklist
- [ ] SessionStart hook emits an additionalContext payload with the enforcer-first reminder and doctrine summary.
- [ ] Reminder names the concrete tools (`ocentra_enforcer_scan`/`check`, coordination guard) and the PreToolUse deny gate from c04.
- [ ] Doctrine text is generated from a single source-of-truth constant (no drift vs the deny-hook reason strings).
- [ ] Hook install is idempotent and registered via the c03 adapter; uninstall removes it.
- [ ] Reminder is emitted deterministically (same input -> byte-identical output) for testability.

## Acceptance And Proof
P5 (`claude-sessionstart-injects` in TEST_PROOF_EXPECTATIONS.md): invoking the hook produces output containing the enforcer-first marker string and the T1/T2/T3 doctrine tokens; a snapshot test pins the exact reminder body so drift fails the build.

## Parallel Ownership Notes
Owns only `src/install/hooks/sessionstart-*`, disjoint from c04's `pretooluse-*`/`guard-*`. Both are registered by c03 but never touch the same file, so they run concurrently. Depends on c01/c03.
