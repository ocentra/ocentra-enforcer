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

- owns: `crates/enforcer-install/src/hooks/sessionstart.rs, crates/enforcer-install/tests/fixtures/sessionstart_hook/**`
- deps: `arc-23`, `c01-install-core-and-cli-contract`, `c03-claude-adapter`
- tier: `P5 install-proof`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md), [RUST_ARCHITECTURE](../RUST_ARCHITECTURE.md).

## Where We Are
The Codex adapter writes a global `AGENTS.md` doctrine block so that harness sees enforcer guidance, but this is static text loaded only if the harness reads that file (and Claude does not read `AGENTS.md`). Claude Code has no equivalent injected-at-session-start reminder wired by the installer, so the enforcer-first doctrine is not reliably present each session, and the arc-23 crate has no SessionStart emitter.

## Where We Want To Be
A Rust EMITTER module `src/hooks/sessionstart.rs` in `enforcer-install` that generates and registers a Claude SessionStart hook config (installed by the c03 adapter). The emitted hook injects an enforcer-first reminder plus the mechanical-enforcement doctrine (T1/T2/T3, prefer `enforcer check`/`scan` before edits) into every new session's context. The pack owns the RUST emitter, NOT a `.ts`/`.mjs` hook script.

## Requirement Checklist
- [ ] Implement the emitter as a Rust function/type in `src/hooks/sessionstart.rs` that produces the SessionStart hook config + its `additionalContext` payload (enforcer-first reminder + doctrine summary) as a structured record the c03 adapter registers.
- [ ] Reminder names the concrete MCP tools (`enforcer scan`/`check`, coordination guard) and the PreToolUse deny gate from c04.
- [ ] Doctrine text is generated from a single source-of-truth Rust constant (shared with the c04 deny-hook reason strings so there is no drift).
- [ ] Hook config install is idempotent and registered via the c03 adapter (through the c01 apply path); uninstall removes it.
- [ ] Reminder is emitted deterministically (same input -> byte-identical output) for snapshot testability. Obey `[workspace.lints]`; no `pub use` barrels.

## Acceptance And Proof
Tier P5 (`claude-sessionstart-injects` in TEST_PROOF_EXPECTATIONS.md): `cargo test -p enforcer-install` invokes the emitter and asserts the produced payload contains the enforcer-first marker string and the T1/T2/T3 doctrine tokens; an `insta`/byte-compare snapshot test (fixtures under `tests/fixtures/sessionstart_hook/**`) pins the exact reminder body so drift fails the build. Clean `cargo clippy` / `cargo fmt --check`.

## Parallel Ownership Notes
Owns only `crates/enforcer-install/src/hooks/sessionstart.rs` (+ its fixtures), disjoint from c04's `src/hooks/pretooluse.rs`. Both are registered by c03 but never touch the same file, so they run concurrently. Depends on arc-23 (skeleton) + c01 (apply path + doctrine tier vocab) + c03 (adapter registration). owns disjoint? = Y.
