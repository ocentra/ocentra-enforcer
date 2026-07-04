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

- owns: `crates/enforcer-install/src/detect.rs, crates/enforcer-install/tests/fixtures/detect/**`
- deps: `arc-23`, `c01-install-core-and-cli-contract`
- tier: `P1 unit`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md), [RUST_ARCHITECTURE](../RUST_ARCHITECTURE.md).

## Where We Are
The legacy `.mjs` install scripts only know Codex; they derive `CODEX_HOME` from env and never probe for other harnesses. Installing across "any harness" (the global-install thesis) requires knowing which harnesses are present. The arc-23 crate skeleton exists, but there is no Rust detection layer.

## Where We Want To Be
A deterministic autodetect module `src/detect.rs` in `enforcer-install` that probes for installed harnesses (`~/.claude`, `~/.codex`, `~/.gemini`, plus antigravity/cursor/windsurf/zed/opencode/aider/kilocode/kiro markers) and returns a normalized `Vec` of detected-adapter records with their home paths, for the c01 orchestrators to drive.

## Requirement Checklist
- [ ] Probe candidate home dirs from env overrides then defaults (`USERPROFILE`/`HOME`), honoring `CODEX_HOME`, `CLAUDE_HOME`, etc.
- [ ] Emit a normalized `serde` record per harness: `DetectedHarness { id: HarnessId, present: bool, home_path: RelPath|AbsPath, evidence: Vec<Evidence> }` — `HarnessId` a branded `enforcer-domain` newtype, camelCase on the wire; no bare `String` ids.
- [ ] Detection is pure over injected filesystem + env accessors (no ambient globals / no direct `std::env` reads in the probe core) so it is unit-testable with temp fixtures.
- [ ] `enforcer install`/`doctor` (c01 core) consume detection to pick adapters when `--scope`/adapter list is not pinned.
- [ ] Unknown/ambiguous state is reported as `present:false` with `evidence`, never guessed (fail-closed). Obey `[workspace.lints]`; no `pub use` barrels.

## Acceptance And Proof
Tier P1 (`harness-autodetect` in TEST_PROOF_EXPECTATIONS.md): `cargo test -p enforcer-install` runs the detect module against a temp home fixture (`tests/fixtures/detect/**`) with seeded `.claude`/`.codex` dirs and asserts the exact detected-adapter set, the empty-home case (no false positives), and env-override precedence. Clean `cargo clippy` / `cargo fmt --check`.

## Parallel Ownership Notes
Depends on arc-23 (crate skeleton) + c01 (adapter id vocabulary / orchestrator interface). Owns only `crates/enforcer-install/src/detect.rs` (+ its fixtures); it does not write adapter behavior, so it runs concurrently with c03-c09 once arc-23 and c01 land. owns disjoint? = Y.
