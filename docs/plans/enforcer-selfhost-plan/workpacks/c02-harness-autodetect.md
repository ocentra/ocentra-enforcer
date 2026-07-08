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

Beyond detecting WHICH harness is present, c02 also DETECTS/DECLARES each present harness's AGENTIC CAPABILITIES + LIMITS as a per-harness **capability manifest**, produced at install/doctor time and attached to each `DetectedHarness` record. The manifest is the machine-readable answer to "what agentic primitives does this harness actually have?" — consumed by the orchestrator (EXECUTION_MODEL §3, arc-16) so it can ADAPT / gracefully degrade to fit the harness's real primitive set instead of assuming the enforcer's full target model exists everywhere (see AUDIT_FINDINGS WAVE 5 — HARNESS CAPABILITY-DETECTION + ADAPTIVE DEGRADATION). The enforcer never assumes a primitive exists; where one is missing the orchestrator degrades honestly and LABELS the degradation, it does not silently pretend.

## Requirement Checklist
- [ ] Probe candidate home dirs from env overrides then defaults (`USERPROFILE`/`HOME`), honoring `CODEX_HOME`, `CLAUDE_HOME`, etc.
- [ ] Emit a normalized `serde` record per harness: `DetectedHarness { id: HarnessId, present: bool, home_path: RelPath|AbsPath, evidence: Vec<Evidence> }` — `HarnessId` a branded `enforcer-domain` newtype, camelCase on the wire; no bare `String` ids.
- [ ] Detection is pure over injected filesystem + env accessors (no ambient globals / no direct `std::env` reads in the probe core) so it is unit-testable with temp fixtures.
- [ ] `enforcer install`/`doctor` (c01 core) consume detection to pick adapters when `--scope`/adapter list is not pinned.
- [ ] Unknown/ambiguous state is reported as `present:false` with `evidence`, never guessed (fail-closed). Obey `[workspace.lints]`; no `pub use` barrels.
- [ ] Emit a per-harness **capability manifest** on each detected record: `DetectedHarness` carries `capabilities: HarnessCapabilities` (a `serde` record, camelCase on the wire) declaring the harness's agentic primitives + limits:
  - `maxConcurrentAgents: Cap` — concurrency cap (`Cap::Bounded(u32)` | `Cap::Unbounded` | `Cap::Unknown`); orchestrator throttles to it.
  - `subAgentNestingDepth: Cap` — max nesting depth (e.g. 3-tier vs flat-only); orchestrator flattens when nesting is unavailable.
  - `backgroundTasks: Support` — background-task support (`Support::Yes` | `No` | `Unknown`).
  - `scheduledTasks: Support` — scheduled-task / cron / automation support; when absent the orchestrator POLLS for mail instead of relying on a scheduled mail-check.
  - `crossSessionMessaging: Support` — cross-session / direct messaging (Codex strong; others weaker/none); when absent the orchestrator falls back to manual / human-relayed handoff.
  - `implicitInvocation: Support` — implicit-invocation support (e.g. Codex `allow_implicit_invocation`).
  - each field pairs a value with `Evidence` (how it was determined) and defaults to `Unknown`/`Support::Unknown` fail-closed — an undetectable primitive is NEVER declared present.
- [ ] Capability values are branded via `enforcer-domain` newtypes/enums (`Cap`, `Support`), no bare `bool`/`String`/`u32` on the wire; pure over the injected fs/env accessors like the rest of the probe (unit-testable with fixtures).
- [ ] The manifest is produced at install AND doctor time and is CONSUMED (not enforced) here — c02 only DETECTS/DECLARES; the ADAPT/degrade logic lives in the orchestrator (EXECUTION_MODEL §3.b) which reads this manifest. Cross-ref: c03/c06/c08/c09 adapters may refine their own harness's declared matrix; c02 provides the detection-time baseline.

## Acceptance And Proof
Tier P1 (`harness-autodetect` in TEST_PROOF_EXPECTATIONS.md): `cargo test -p enforcer-install` runs the detect module against a temp home fixture (`tests/fixtures/detect/**`) with seeded `.claude`/`.codex` dirs and asserts the exact detected-adapter set, the empty-home case (no false positives), and env-override precedence. It ALSO asserts the emitted **capability manifest**: a Codex-marker fixture (with `agents/openai.yaml` carrying `allow_implicit_invocation`) declares `implicitInvocation: Yes` + strong `crossSessionMessaging`; a bare-`.claude` fixture declares the fields it cannot prove as `Unknown`/`Support::Unknown` (fail-closed, never guessed `Yes`); and an empty-home fixture emits no manifests. Fixtures: `tests/fixtures/detect/caps-codex/**`, `caps-claude-bare/**`. Clean `cargo clippy` / `cargo fmt --check`.

## Parallel Ownership Notes
Depends on arc-23 (crate skeleton) + c01 (adapter id vocabulary / orchestrator interface). Owns only `crates/enforcer-install/src/detect.rs` (+ its fixtures); it does not write adapter behavior, so it runs concurrently with c03-c09 once arc-23 and c01 land. owns disjoint? = Y.
