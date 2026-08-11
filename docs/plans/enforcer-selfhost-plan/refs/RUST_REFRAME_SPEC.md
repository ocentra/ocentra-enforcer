# RUST_REFRAME_SPEC — the authoritative TS→Rust transformation contract for finishing the plan

This spec governs the re-frame pass that converts every non-Track-A workpack (Tracks C/D/E/F/G/H +
cross-cutting) from its old TypeScript/`.mjs` framing to the pure-Rust Cargo-workspace framing defined in
[RUST_ARCHITECTURE.md](../RUST_ARCHITECTURE.md). Every re-frame agent MUST read RUST_ARCHITECTURE.md first,
then apply this spec to ONLY its assigned pack files.

## GOLDEN RULES (non-negotiable)
1. **Edit ONLY your assigned pack files** (`workpacks/<id>-*.md`). Do NOT touch any shared index/state/proof
   file (WORKPACK_INDEX, TEST_PROOF_EXPECTATIONS, PLAN_STATE, PLAN_EXECUTION_BLUEPRINT, DOC_INDEX,
   NEXT_ACTIONS, ROUTE_INDEX, PLAN_HEALTH, PROOF_INDEX, CHECKLIST_INDEX, ARCHIVE_INDEX, totals). A single
   later reconciliation pass owns those — concurrent edits there corrupt the file.
2. **Preserve the workpack anatomy:** keep the `<!-- agent-capsule -->` block verbatim; keep the section
   headings (`## Where We Are`, `## Where We Want To Be`, `## Requirement Checklist`, `## Acceptance And Proof`,
   `## Parallel Ownership Notes`); keep the `Sources:` line; keep the `- owns:` / `- deps:` / `- tier:` machine
   fields. Only the CONTENT changes from TS to Rust.
3. **No TS-engine residue.** The enforcer engine is pure Rust. Remove all mentions of the engine being TS:
   `.ts`/`.mjs` source, `tsconfig`, `eslint` (as our linter), `Effect-Schema`, jest/`node:test`, npm scripts.
   ALLOWED to remain: the `enforcer-ui` Tauri **frontend** (TS/web, presentation only), and any reference to
   VALIDATING a user's TS/JS/Python/Dart/etc. code (that is the enforcer's job, not its implementation).
4. **Keep the tier vocabulary** (P0-P5 proof tiers; T1/T2/T3 rule tiers) and the rule/doctrine INTENT — only
   the implementation language changes.

## UNIVERSAL TS→Rust TRANSFORMATION RULES
- **Source paths:** `src/**/*.ts` / `*.mjs` → `crates/<crate>/src/**/*.rs` in the crate assigned below.
  `owns:` globs move from `src/...`/`eslint-rules/...` to `crates/<crate>/src/...` (see disjoint-owns model).
- **Validators:** a TS validator function → a Rust type implementing the `Validator` trait (from
  `enforcer-validator`, arc-05), returning structured `Finding`s (from `enforcer-domain`, arc-02). Registered
  in its crate's rule set.
- **AST linters:** an ESLint/TS-AST rule → a Rust AST Validator: use `syn` for Rust targets, `tree-sitter` (or
  `swc`) for TS/JS/Dart/CFML targets. Never a standalone println/exit binary — always a `Validator` emitting
  `Finding`s + a fix hint.
- **Schemas:** `Effect-Schema` / Zod → `serde` structs + **branded newtypes** in `enforcer-domain` with
  parse-at-boundary (`TryFrom<String>`/`deserialize_with` validators + a `thiserror` typed error). Never a
  bare `String` for an id.
- **Rules-as-data:** every rule doc (`rules/**/*.md`) → a typed rule RECORD in `enforcer-rules` (arc-04)
  carrying `ruleId ↔ validator ↔ {fail+pass fixtures} ↔ doc-anchor ↔ tier`. `.md` may stay as optional
  human-canonical text; the engine consumes the structured record.
- **Tests:** jest/`node:test`/`*.test.mjs` → `cargo test` (`#[test]` / `#[cfg(test)]`) with fail-fixture +
  pass-fixture under `crates/<crate>/tests/fixtures/<rule>/{bad,good}/`. 5-way parity is Rust-native.
- **Deps:** re-point `deps:` to the arc crate pack(s) the feature builds on — a validator pack deps its lang
  crate (arc-06..13) + `arc-05` (validator trait) + `arc-04` (rules) and/or `arc-14` (mechanization); a
  config pack deps `arc-03`; a scan pack deps `arc-15`; a UI pack deps `arc-24`; an install pack deps `arc-23`.
  Keep existing intra-track deps (e.g. `d01` keystone). Drop deps on deleted packs.
- **Native tools:** shell-outs to `tsc`/`eslint`/`ruff`/`cargo`/`dart`/`CFLint` run through `enforcer-harness`
  (arc-18) run-adapters, not ad-hoc.

## DISJOINT-OWNS MODEL (avoid re-creating overlaps)
Many feature packs (D/E/F/G/H rules) live INSIDE an arc crate that an arc-* pack builds. To stay disjoint:
- The **arc-* pack owns the crate SKELETON**: `crates/<crate>/Cargo.toml`, `src/lib.rs`, the crate's
  `Validator`-registration/module-root, and any BASELINE items. (Reconciliation may narrow an arc `owns:` to
  exclude `src/rules/**` where feature packs live.)
- A **feature pack owns SPECIFIC files** under that crate: `crates/<crate>/src/rules/<name>.rs` (+ a
  `src/rules/<name>/` module dir if needed) and `crates/<crate>/tests/fixtures/<name>/**`. It does NOT own the
  whole crate. It `deps:` the owning arc pack (sequenced after the skeleton exists).
- Result: feature packs are disjoint from each other and from the arc skeleton BY FILE, and sequenced by
  `deps:` — mark `owns disjoint? = Y` (or `Y*` only if two feature packs genuinely share one file, with the
  sequencing note). Never let two no-dep-edge packs own overlapping globs.

## PER-TRACK PACK → CRATE MAPPING (authoritative)

### Track C — install → `enforcer-install` (arc-23)
All C packs own `crates/enforcer-install/src/<module>.rs` + tests; `deps:` include `arc-23` (+ `arc-03` config
for policy-reading packs). Harness-side artifacts (Claude PreToolUse/SessionStart hooks, MCP JSON, cargo-alias,
pre-commit) are EMITTED by Rust installer modules — the pack owns the Rust emitter, not a `.ts` hook.
- c01 → `src/core.rs` + `src/cli_contract.rs` + `src/report.rs`; c02 → `src/detect.rs`;
  c03 → `src/adapters/claude.rs`; c04 → `src/hooks/pretooluse.rs` (emitter); c05 → `src/hooks/sessionstart.rs`;
  c06 → `src/adapters/codex.rs`; c07 → `src/adapters/generic.rs` + `src/doctor.rs`;
  c08 → `src/adapters/{gemini,cursor,zed}.rs`; c09 → `src/adapters/{antigravity,windsurf,opencode,aider,kilocode,kiro}.rs`.

### Track D — ADBP borrows → distributed by concern
- d01 rule-mechanization-engine → **`enforcer-mechanization` (arc-14)** `src/{scaffold,parity}.rs` (KEYSTONE).
- d02 baseline-ratchet → `enforcer-scan` (arc-15) `src/rules/baseline_ratchet.rs`.
- d03 deferred-work-gate → `enforcer-lang-common` (arc-09) `src/rules/deferred_work.rs`.
- d04 run-telemetry → `enforcer-core` (arc-01) telemetry records + NDJSON sink (fold with logging borrow).
- d05 context-budget → `enforcer-mcp` (arc-21) tool-surface measure + `enforcer-core` ratchet.
- d06 lifecycle-commands → `enforcer-cli` (arc-22) `src/lifecycle.rs`.
- d07 self-correct-fix-loop → `enforcer-coordination` (arc-16) `src/fix_loop.rs`.
- d08 harness-feedback → `enforcer-mechanization` (arc-14) `src/feedback.rs`.
- d09 perstack-agents + doc-rule-parity → `docs/agents/**` (prose, T3) + `enforcer-validator` (arc-05)
  `src/doc_rule_parity.rs` (T1 citation check).
- d10 resilience-auditor → `enforcer-lang-common` (arc-09) `src/rules/resilience.rs`.
- d11 ci-parity → `enforcer-harness` (arc-18) `src/ci_parity.rs`.
- d12 layered-frontend-ruleids → `enforcer-lang-ts` (arc-07) `src/rules/layered_frontend.rs` (tree-sitter/swc).
- d13 rule-version-drift → `enforcer-rules` (arc-04) `src/version_drift.rs`.
- d14 ideation-skills → `skills/ideation/**` (prose, T3-labeled) + a T1 labeling check in `enforcer-validator`.
- d15 readme-research-grounding → `docs/` + `README.md` (doc-only).
- d16 fsm → `enforcer-lang-common` (arc-09) `src/rules/fsm.rs`.
- d17 rust-error-handling → `enforcer-lang-rust` (arc-06) `src/rules/error_handling.rs` (syn).
- d18 security-stop → `enforcer-lang-security` (arc-10) `src/rules/security_stop.rs` + `enforcer-security` (arc-19).
- d21 change-discipline → `enforcer-lang-common` (arc-09) `src/rules/change_discipline.rs`.
- d22 size-shape → `enforcer-lang-common` (arc-09) `src/rules/size_shape.rs`.
- d23 test-companion → `enforcer-lang-common` (arc-09) `src/rules/test_quality.rs`.
- d25 orchestrator-verify-gates → `enforcer-plan` (arc-20) `src/verify_gates.rs`.
- d26 dispatch-prompt-assembly → `enforcer-coordination` (arc-16) `src/dispatch/quality_blocks.rs`.
- d27 loop-resilience → `enforcer-coordination` (arc-16) `src/loop_resilience.rs` + `enforcer-core` meter.
- d28 target-ci-parity → `enforcer-harness` (arc-18) `src/target_ci_parity.rs`.

### Track E — languages → lang crates (ADDS three crates)
- e01 literal-scan-universal → `enforcer-literal-scan` (arc-13) bridge module.
- e-pack-dart → **NEW crate `enforcer-lang-dart`** (`crates/enforcer-lang-dart/**`; tree-sitter-dart; deps
  arc-05/04). e-pack-cfml → **NEW crate `enforcer-lang-cfml`** (CFLint shell-out via arc-18 + structural).
  e-pack-crypto-blockchain → **NEW crate `enforcer-lang-crypto`** (OPTIONAL/opt-in, OFF by default).
- e-pack-frontend-react → `enforcer-lang-ts` (arc-07) `src/rules/frontend_react.rs` (module, not a new crate).
- e-pack-python → `enforcer-lang-py` (arc-08) `src/rules/fastapi_layered.rs` (module, not a new crate).
> Crate-map delta from Track E: **+3 lang crates** (`enforcer-lang-dart`, `enforcer-lang-cfml`,
> `enforcer-lang-crypto` [opt-in]). These are BUILT BY their e-pack (no separate arc-* pack). Note in the pack
> that it stands up the crate skeleton itself (Cargo.toml + lib + register) since no arc pack pre-builds it.

### Track F — scan/config → `enforcer-scan` (arc-15) + `enforcer-config` (arc-03) + `enforcer-core` (arc-01)
- f01 scan-modes → `enforcer-scan` `src/modes.rs`; f02 onboard → `enforcer-scan` `src/onboard.rs` +
  `enforcer-cli`; f03 project-tie → `enforcer-config` `src/project_tie.rs` (+ the declarative policy
  externalization borrow: owner/exempt globs, allow-regex, cfg(test) skipping); f04 silent-vs-human →
  `enforcer-core` `src/run_context.rs`; f05 detect-and-route → `enforcer-scan` `src/router/**`.

### Track G — UI → `enforcer-ui` (arc-24), all modules
- g01 → `src/serve.rs` (Tauri shell + served HTML fallback); g02 → `src/report/`; g03 → `src/actions/`;
  g04 → `src/run_dispatch/` (deps `arc-16`); g05 → `src/settings/` (config control-plane); g06 → `src/hub/`
  (live lane/claim/lease/mail panel); g07 → `src/security/`; g08 → `src/explorer/` (rules-&-skills explorer);
  g09 → `src/memory_explorer/` (read-only memory/KG/RAG explorer over x06).
  Frontend is TS/web under `crates/enforcer-ui/frontend/`; types DERIVED from `enforcer-domain` via `ts_rs`
  (the g-track ui pack that owns type-gen adds the export bin + fail-closed drift test).

### Track H — security → `enforcer-security` (arc-19) + rules-as-data + `enforcer-lang-security` (arc-10)
- h01-h08 → `enforcer-security` `src/rules/<name>.rs` + rule records in `enforcer-rules`; h08 skill/profile/
  policy-ingest → `enforcer-security` `src/policy_ingest.rs` + `profiles/money-critical-security.json` +
  `skills/security-testing/`. h11 cyberskills-corpus → `enforcer-lang-security` (arc-10) + `enforcer-security`
  Rust validators + `enforcer-scan` security-audit scope. h12 cyberskills-python-adapters → OPTIONAL
  out-of-dogfood `enforcer-harness` run-adapters (`crates/enforcer-harness/adapters/cyberskills/**`, graceful-skip).

### Cross-cutting
- x01 neutral-rename → workspace Cargo package names + `enforcer` binary/MCP-server name (owns `Cargo.toml`
  package fields + `crates/enforcer-cli`/`enforcer-mcp` server-name consts). x02 docs-refresh → `README.md` +
  `docs/**` (doc-only). x03 rename-migration → `enforcer-install` (arc-23) `src/migrate_legacy_name.rs`.
  z01 dogfood-proof-gate → `xtask/src/dogfood.rs` + `crates/enforcer-cli/tests/dogfood_gate.rs` (terminal gate).

## BORROW-FOLDS to apply where relevant (see RUST_ARCHITECTURE "Borrows" section)
- Any pack writing structured records/telemetry → versioned serde records in `enforcer-domain` + two-layer
  redaction (via `enforcer-core`); append-only NDJSON via the core sink; tamper-evident chains via the core
  hash-chain util (proof journal in `enforcer-proof`).
- Any pack under `enforcer-config`/policy → declarative committed policy (owner/exempt globs, allow-regex,
  cfg(test) skipping), never inline-disable.
- Any Rust-validator pack → obey `[workspace.lints]` (no `unwrap/expect/panic/print_*`); no `pub use` barrels.
- UI/type packs → `ts_rs` derive + fail-closed drift test; camelCase wire casing.
- Event-observable packs (scan lifecycle, coordination, proof) → emit typed events via `enforcer-events`
  (arc-25); pure-compute packs use plain calls.

## WHAT THE RECONCILIATION PASS (separate, later) WILL DO — do NOT do it yourself
Update WORKPACK_INDEX rows (owns/deps/tier/parallel-safe) to match your re-framed packs; add the `arc-25`
(enforcer-events) row + the three new `enforcer-lang-{dart,cfml,crypto}` crate notes; refresh
TEST_PROOF_EXPECTATIONS proof rows to `cargo test -p <crate>`; refresh PLAN_STATE/BLUEPRINT/DOC_INDEX/
NEXT_ACTIONS/ROUTE_INDEX/PLAN_HEALTH/PROOF_INDEX/CHECKLIST_INDEX/totals; re-verify DAG + disjoint-owns.
