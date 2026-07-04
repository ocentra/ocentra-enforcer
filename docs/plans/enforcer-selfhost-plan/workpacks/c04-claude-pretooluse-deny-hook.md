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

- owns: `crates/enforcer-install/src/hooks/pretooluse.rs, crates/enforcer-install/tests/fixtures/pretooluse_hook/**`
- deps: `arc-23`, `c01-install-core-and-cli-contract`, `c03-claude-adapter`
- tier: `P5 install-proof`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md), [RUST_ARCHITECTURE](../RUST_ARCHITECTURE.md).

## Where We Are
The enforcer binary exposes MCP tools (`enforcer check`, `enforcer scan`, coordination guard) but nothing forces the agent to run them before writing. Guidance-only installs are prose, not proof: an agent can ignore a skill. There is no mechanical gate on file edits, and the arc-23 crate has no hook-emitter module.

## Where We Want To Be
A Rust EMITTER module `src/hooks/pretooluse.rs` in `enforcer-install` that generates and registers a Claude PreToolUse deny-hook config. The emitted hook, on `Edit|Write|MultiEdit`, shells out to the installed `enforcer` binary (`enforcer check`/`scan` + coordination guard) against the pending change and BLOCKS deterministic (T1) violations before the write lands. This is the T1 mechanical bridge that makes self-enforcement real, not advisory. The pack owns the RUST emitter (the module that writes the hook config + invocation), NOT a `.ts`/`.mjs` hook script.

## Requirement Checklist
- [ ] Implement the emitter as a Rust function/type in `src/hooks/pretooluse.rs` that produces the PreToolUse hook config (matcher `Edit|Write|MultiEdit` -> `enforcer` binary invocation) as a structured record the c03 Claude adapter registers.
- [ ] The emitted hook contract: read the PreToolUse payload (tool name + target path + proposed content) from stdin and invoke `enforcer check`/`scan` + coordination guard on the candidate content.
- [ ] T1 (hard `Validator` finding) -> exit deny with the `RuleId` and the `Fix:` hint string in the reason. T2 (scored literal-scan) -> allow with warning surfaced. T3 -> never blocks. (Tier vocabulary preserved.)
- [ ] Fail-closed on enforcer error/timeout for T1 scope; non-edit tools pass through untouched.
- [ ] The hook config is emitted/registered by the c03 Claude adapter (via the c01 apply path) and is idempotent to install. Obey `[workspace.lints]`; no `pub use` barrels.

## Acceptance And Proof
Tier P5 mechanical-bridge proof (`claude-deny-hook-blocks` in TEST_PROOF_EXPECTATIONS.md): `cargo test -p enforcer-install` exercises the emitter + the hook contract against fixtures under `tests/fixtures/pretooluse_hook/**` — a **seeded violating edit** payload asserts exit = deny AND the JSON/stderr reason contains the exact `RuleId` and its `Fix:` hint; a conforming edit asserts exit = allow; a T2-only finding asserts exit = allow-with-warning, never deny. Clean `cargo clippy` / `cargo fmt --check`.

## Parallel Ownership Notes
Owns only `crates/enforcer-install/src/hooks/pretooluse.rs` (+ its fixtures), disjoint from c05 which owns `src/hooks/sessionstart.rs`. Both are emitted/registered by the c03 adapter but live in separate files, so c04 and c05 run concurrently. Depends on arc-23 (skeleton) + c01 (report/apply + tier vocab) + c03 (adapter registration). owns disjoint? = Y.
