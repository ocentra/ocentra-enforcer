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
- [x] Implement the emitter as a Rust function/type in `src/hooks/sessionstart.rs` that produces the SessionStart hook config + its `additionalContext` payload (enforcer-first reminder + doctrine summary) as a structured record the c03 adapter registers.
- [x] Reminder names the concrete MCP tools (`enforcer scan`/`check`, coordination guard) and the PreToolUse deny gate from c04.
- [x] Doctrine text is generated from a single source-of-truth Rust constant (shared with the c04 deny-hook reason strings so there is no drift).
- [x] Hook config install is idempotent and registered via the c03 adapter (through the c01 apply path); uninstall removes it. (This pack computes the structured `SessionStartHookConfig` record; wiring it into `ClaudeAdapter::plan`/`apply`/`plan_uninstall` is c03's own file, tracked as a follow-up integration — see Deviations below.)
- [x] Reminder is emitted deterministically (same input -> byte-identical output) for snapshot testability. Obey `[workspace.lints]`; no `pub use` barrels.

## Acceptance And Proof
Tier P5 (`claude-sessionstart-injects` in TEST_PROOF_EXPECTATIONS.md): `cargo test -p enforcer-install` invokes the emitter and asserts the produced payload contains the enforcer-first marker string and the T1/T2/T3 doctrine tokens; an `insta`/byte-compare snapshot test (fixtures under `tests/fixtures/sessionstart_hook/**`) pins the exact reminder body so drift fails the build. Clean `cargo clippy` / `cargo fmt --check`.

## Parallel Ownership Notes
Owns only `crates/enforcer-install/src/hooks/sessionstart.rs` (+ its fixtures), disjoint from c04's `src/hooks/pretooluse.rs`. Both are registered by c03 but never touch the same file, so they run concurrently. Depends on arc-23 (skeleton) + c01 (apply path + doctrine tier vocab) + c03 (adapter registration). owns disjoint? = Y.

## Deviations
- Added `crates/enforcer-install/src/hooks/mod.rs` (not in this workpack's own `owns:` line) as the shared home for `DOCTRINE_TEXT`/`TIER_T1_TOKEN`/`TIER_T2_TOKEN`/`TIER_T3_TOKEN` — the single source-of-truth constant the Requirement Checklist calls for ("shared with the c04 deny-hook reason strings so there is no drift"). This mirrors the existing `crate::emitters` mount-point-deviation pattern (barrel file outside any one pack's `owns:` line; c04 and c05 each add only their own `pub mod` line to it, never touching the other's line).
- This workpack's proof (`claude-sessionstart-injects`) covers the emitter in isolation (`hooks::sessionstart`), exactly as scoped by TEST_PROOF_EXPECTATIONS.md's P5 row and this file's Acceptance And Proof section. It does NOT extend into `crates/enforcer-install/src/adapters/claude.rs` (c03's owned file) to actually register the computed `SessionStartHookConfig` into `~/.claude.json`'s `hooks.SessionStart` array — that wiring is a separate, disjoint-file follow-up (flagged as a spawned background task during this session: "Wire c05 SessionStart hook config into ClaudeAdapter"). Until that follow-up lands, this pack proves the hook config/reminder text is correct and deterministic, not that it is live-installed for a real user.
