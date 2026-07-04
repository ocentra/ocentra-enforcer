# enforcer-selfhost-plan — README (route)

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `README (route)`
> Kind: index / route. Read first to find where to go; do not treat as work.
> Read when: You just arrived at this plan and need the entry route.
> Stop rule: Do NOT read every workpack. Follow the Default agent path below and stop.
> Proves: nothing. This file gates no status and is not proof of anything.
> Does not prove: workpack completion, product status, or DONE. Only proof rows do that.
> Proof rule: Product status changes only via a workpack's named tests in TEST_PROOF_EXPECTATIONS.md.
<!-- /agent-capsule -->

This plan makes the `enforcer` **eat its own dog food**. It is a **pure-Rust, 28-crate Cargo-workspace engine** built across **111 workpacks** in tracks A/B/C/D/E/F/G/H plus a cross-cutting (X) track. Governing docs: [`RUST_ARCHITECTURE.md`](./RUST_ARCHITECTURE.md) (WHAT the engine is) and [`EXECUTION_MODEL.md`](./EXECUTION_MODEL.md) (HOW it is built — bootstrap-safe worktree + orchestrated worker swarm).

- **A — Self-host (dogfood), 35 packs:** stand up the enforcer as a **RUST Cargo workspace** — `arc-01`..`arc-25` (25 crate-build packs, dependency-ordered) plus `a01`..`a10` (10 Rust hardening packs: Cargo/clippy/rustfmt/deny/audit toolchain, `enforcer-domain` branded newtypes with parse-at-boundary, waiver honesty, anti-silent-skip, and real self-enforcement in CI — the enforcer's own Rust rules over its own crates). See [`RUST_ARCHITECTURE.md`](./RUST_ARCHITECTURE.md).
- **B — Planning skill, 6 packs:** ship the OcentraParent plan methodology as a mechanical `enforcer plan new` scaffolder + `PLAN-*` structure validator + `/plan` skill that self-validates against this very plan (lands in `enforcer-plan`), plus `b06` the AGENTS.md decision-forest.
- **C — Install + enforce anywhere, 9 packs:** harness-neutral install core and adapters (Claude, Codex, generic, stubs, + the remaining 11-harness fleet) with a **PreToolUse deny-hook** that mechanically blocks T1 violations before a write lands (lands in `enforcer-install`).
- **D — ADBP borrows, mechanized, 25 packs:** every idea borrowed from ADBP is dragged UP the enforcement ladder (grandfather ratchet, deferred-work gate, telemetry, context brake, fix loop, doc-rule parity, plus FSM validity, Rust error handling, security STOP watchlist, change discipline, size/shape caps, test companion/quality, orchestrator verify gates, dispatch prompt assembly, loop resilience, target-repo CI parity), never copied as prose.
- **E — New languages + universal scanning, 6 packs:** an always-on universal literal-scan T2 floor plus first-class Dart, CFML/ColdFusion, React/Next (Effect-only), and Python/FastAPI language packs, and an OPTIONAL opt-in crypto/blockchain pack (OFF by default). Three E packs BUILD their own new lang crates (`enforcer-lang-dart`, `enforcer-lang-cfml`, `enforcer-lang-crypto`).
- **F — Scan surface / onboarding / agent-shaping, 5 packs:** named scan modes, index-on-ask onboarding, per-project native-tie config, detect-and-route router, and the silent (agent-inline) vs human-review split.
- **G — UI layer, 8 packs:** the OPTIONAL Tauri control-plane cockpit (`enforcer-ui`) — serve surface, scan report, violation actions, run-dispatch, settings, hub dashboard, UI-security, and the rules-&-skills explorer (`g08`).
- **H — Money-critical & security testing, 10 packs:** the `enforcer-security` validators.
- **Cross-cutting (X), 5 packs:** early rename to `enforcer`; the `x04` main-branch-protection CI; the terminal `z01` dogfood-proof-gate that runs the finished `enforcer` against its own multi-language self (bootstrap swap point) and gates plan-DONE on zero self-violations.

## Locked decisions (scope)

The enforcer is a **pure-Rust, 28-crate Cargo-workspace engine** (governing WHAT-doc: [`RUST_ARCHITECTURE.md`](./RUST_ARCHITECTURE.md); governing HOW-doc: [`EXECUTION_MODEL.md`](./EXECUTION_MODEL.md)). This **supersedes the earlier `.mjs` -> TypeScript decision** — the tracks and doctrine are unchanged; only the implementation language is Rust.

- **One binary IS the engine.** A per-platform Rust binary is both the MCP stdio server AND the CLI (`enforcer scan|check|install|serve|plan|...`). **Node / `.mjs` is DROPPED entirely** — no shims, no runtime toolchain required by consumers.
- **BOTH MCP and CLI are FIRST-CLASS.** Neither is secondary: MCP is the harness-native, install-once, zero-per-repo-config agent UX; CLI is equally first-class for direct/CI/precommit/cargo-alias use (tri-modal scope `<paths...> | --base/--head | --all`, exit-code-driven, Windows-first, NO override flag). One binary, excellent at both; `enforcer-config` is the single declarative control-plane both surfaces + the UI read.
- **29 crates.** 25 `arc` crates (`enforcer-core`/`domain`/`config`/`rules`/`validator`/`lang-{rust,ts,py,common,security,iac,k8s}`/`literal-scan`/`mechanization`/`scan`/`coordination`/`proof`/`harness`/`security`/`plan`/`mcp`/`cli`/`install`/`ui`/`events`) + 3 lang crates built by Track E (`enforcer-lang-dart`, `enforcer-lang-cfml`, `enforcer-lang-crypto` [OPT-IN, off by default]) + 1 crate built by x06 (`enforcer-memory` — the harness-memory graph/recall over the x05 lesson corpus). See the crate map in [`RUST_ARCHITECTURE.md`](./RUST_ARCHITECTURE.md).
- **Rules are structured data, not prose.** Typed rule records (`enforcer-domain` / `rules.json` / RON) carry `ruleId <-> validator <-> {fail+pass fixtures} <-> doc-anchor <-> tier`. `.md` is optional human-canonical reading only; the AI consumes the structured rule.
- **5-way parity is Rust-native:** a `Validator` impl + fail/pass fixtures + a `cargo test` detection test. Proofs are `cargo test -p <crate>` + fail/pass fixtures + `clippy`/`fmt`/`deny`/`audit` — NOT `tsc`/`jest`/typecheck.
- **Native dogfood:** the enforcer's own Rust rules (plus `cargo clippy`/`fmt`/`deny`/`audit`) validate its own crates — no TS detour.
- **TS only for the `enforcer-ui` Tauri frontend.** The desktop UI is Tauri (Rust backend + TS/web frontend); served self-contained HTML for headless. No business logic in TS; UI types are DERIVED from `enforcer-domain` via `ts_rs`, not hand-written.
- **OcentraParent borrows adopted:** `[workspace.lints]` deny-wall (`unsafe_code=forbid` + clippy denies) in `a01`; `no-reexports` as an `enforcer-lang-rust` Validator; a SHA-256 hash-chained proof journal in `enforcer-proof`; Rust->TS via `ts_rs` derive + a fail-closed drift test; two-layer redaction in `enforcer-core`. `arc-25` `enforcer-events` is **VENDORED as-is** from OcentraParent's `ocentra-eventing` (renamed), consumed by `arc-15`/`16`/`17`. **Logging is FOLDED** into `enforcer-core`/`domain`/`proof` (vendored `logging-core` primitives) — there is NO `enforcer-log` crate.
- **Bootstrap-safe execution** (per [`EXECUTION_MODEL.md`](./EXECUTION_MODEL.md)): build in a separate git worktree + branch; keep the live `.mjs` MCP registered until the Rust engine is proven GREEN, then swap. An orchestrator (Fable 5) spawns Sonnet/Haiku/Opus workers per disjoint workpack via the coordination hub.
- **Track A is `arc-01`..`arc-25`** (25-crate build swarm) **+ `a01`..`a10`** (10 Rust hardening packs). The old 50-pack `.mjs -> TS` conversion swarm is removed; Tracks B–H are already re-framed to Rust crates in [`WORKPACK_INDEX.md`](./WORKPACK_INDEX.md). There is NO residual `.mjs`/`.ts`-engine/Effect-Schema/eslint-as-our-linter surface.

## DOCTRINE (governs every workpack)

Rules are conditions. **Enforcement MUST be mechanical.** Prose without a backing check is hope, not proof. Three tiers:

- **T1 — Hard / deterministic validator.** ruleId <-> validator <-> doc <-> fixtures parity, **fail-closed**. Blocks. This is the bar for anything that gates.
- **T2 — Scored / advisory but still mechanical.** regex / AST / heuristic emitting `score` + `confidence`, non-blocking (the Rust literal-scan model). Mechanized, just not fatal.
- **T3 — Justified prose.** Only when mechanization is genuinely impossible, and it MUST be labeled `advisory, no mechanization possible + <reason>`. The *label* is enforced at T1 even when the content is judgment.

Every ADBP borrow is dragged UP this ladder, never left as prose to trust.

## Default agent path

1. Read [`AGENTS.md`](./AGENTS.md) — the operating contract (what you may and may not do).
2. Read [`PLAN_STATE.md`](./PLAN_STATE.md) — scope, resume route, what's present, open gaps.
3. Read [`NEXT_ACTIONS.md`](./NEXT_ACTIONS.md) — the ordered ready-now frontier.
4. Read [`WORKPACK_INDEX.md`](./WORKPACK_INDEX.md) — pick / confirm your one assigned workpack.
5. Read **only that one** workpack under [`workpacks/`](./workpacks/), plus [`TEST_PROOF_EXPECTATIONS.md`](./TEST_PROOF_EXPECTATIONS.md) for its proof rows.

Then do the work, produce the named proof, update that workpack's row. Stop.

## Do not default-read

- Any workpack other than the one assigned to you (there are 111; reading siblings wastes context and risks cross-scope edits).
- [`README_FULL_ORIGINAL.md`](./README_FULL_ORIGINAL.md) — long-form narrative; open only for background, never as a task list.
- [`PLAN_HEALTH.md`](./PLAN_HEALTH.md) — for the hub / auditor, not for a workpack executor.
- [`PLAN_EXECUTION_BLUEPRINT.md`](./PLAN_EXECUTION_BLUEPRINT.md) — for whoever is sequencing/orchestrating, not for a single-pack executor (your capsule already tells you your deps).

## Map of index files

| File | For whom | Purpose |
|---|---|---|
| [`AGENTS.md`](./AGENTS.md) | every agent | operating contract; read order; failure conditions |
| [`RUST_ARCHITECTURE.md`](./RUST_ARCHITECTURE.md) | every agent | governing WHAT-doc: the enforcer is a pure-Rust 28-crate Cargo workspace (crate map, OcentraParent borrows, distribution, track re-cast) — supersedes the `.mjs`->TS decision |
| [`EXECUTION_MODEL.md`](./EXECUTION_MODEL.md) | every agent / orchestrator | governing HOW-doc: bootstrap-safe build (separate worktree+branch, keep `.mjs` MCP live until Rust green then swap), vendoring, and the orchestrator + worker-swarm model |
| [`PLAN_STATE.md`](./PLAN_STATE.md) | every agent | scope, resume route, present/open gaps, workpack summary |
| [`NEXT_ACTIONS.md`](./NEXT_ACTIONS.md) | executor / hub | the ordered ready-now frontier |
| [`WORKPACK_INDEX.md`](./WORKPACK_INDEX.md) | executor / hub | status table over all workpacks |
| [`PLAN_EXECUTION_BLUEPRINT.md`](./PLAN_EXECUTION_BLUEPRINT.md) | orchestrator | tracks, sequence, parallel model |
| [`TEST_PROOF_EXPECTATIONS.md`](./TEST_PROOF_EXPECTATIONS.md) | every agent | proof tiers P0–P5 + decision tree |
| [`PLAN_HEALTH.md`](./PLAN_HEALTH.md) | hub / auditor | invariants and health checks |
| [`README_FULL_ORIGINAL.md`](./README_FULL_ORIGINAL.md) | background | long-form narrative |
