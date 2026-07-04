# RUST ARCHITECTURE — the enforcer is a pure Rust engine (governing doc)

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `RUST_ARCHITECTURE`
> Kind: governing architecture doctrine. Defines WHAT the enforcer is built as (a Rust Cargo workspace) and the crate map every Track A workpack targets. Supersedes the earlier `.mjs` -> TypeScript decision.
> Read when: Orienting on the engine's language/architecture, or building/claiming any Track A (`arc-*` / `a0*`) workpack.
> Stop rule: This sets architecture doctrine; it does not authorize touching any scope or move status. Proof still gates DONE.
> Proves: nothing on its own — it is doctrine + a crate map. Proof lives in TEST_PROOF_EXPECTATIONS.md.
> Does not prove: any workpack DONE or product status.
<!-- /agent-capsule -->

PIVOT: the enforcer is rebuilt as a **Cargo workspace of Rust crates**, not a TS migration. This supersedes
the earlier `.mjs -> TypeScript` decision. Same ideas/tracks; Rust implementation. Rationale: type-safety
without dynamic bullshit, fast + parallel, a single self-contained binary, native dogfood, and the
codebase-memory distribution model (binary + MCP).

## Doctrine (Rust-native)
- **One binary IS the engine.** A per-platform Rust binary is the MCP stdio server AND the CLI
  (`enforcer scan|check|install|serve|plan|...`). Node/`.mjs` is DROPPED entirely — no shims.
- **Rules are STRUCTURED DATA, not prose.** Typed rule records (in `enforcer-domain` / `rules.json` / RON),
  each carrying `ruleId <-> validator <-> {fail+pass fixtures} <-> doc-anchor <-> tier`. `.md` is kept ONLY
  as optional human-canonical reading, or dropped — the AI consumes the structured rule, never prose.
- **5-way parity is Rust-native:** a `Validator` impl + fail/pass fixtures + a `cargo test` detection test.
- **Dogfood is native:** the enforcer's own Rust rules validate its own crates (`cargo clippy`/`fmt`/`deny`/
  `audit` + the enforcer's Rust validators) — no TS detour. z01 runs the enforcer on its own workspace.
- **Presentation only is TS:** Tauri (Rust backend + TS/web frontend) for the desktop UI; served
  self-contained HTML for headless. No business logic in TS.
- **Parallelism:** rayon for CPU-bound scan fan-out; the coordination hub (Rust) for multi-agent editing.

## Cargo workspace crate map (`crates/`) — centralized + reusable
FOUNDATION (reused everywhere):
- `enforcer-core` — Result/Error, tracing, shared primitives.
- `enforcer-domain` — the SINGLE-SOURCE schema: branded newtypes + serde — RuleId, RepoRoot, RelPath,
  Sha256, HubName, LaneId, Severity, Tier, Finding, Violation, Report, ScanScope, ThreatId (MITRE/OWASP).
- `enforcer-config` — typed config load, parse-at-boundary.
- `enforcer-events` — LEAN in-process typed event spine (serde/sha2; SYNC-first): `DomainEvent` +
  `EventEnvelope<E>` (stored-decode re-verifies the contract), correlation/causation IDs, panic-isolated
  Sequential/Concurrent dispatch. Consumed ONLY by scan/coordination/proof. Deliberately EXCLUDES
  contract-registry RPC, aggregate-ordering gates, TTL queue, request/response, external transport.
RULES & VALIDATION:
- `enforcer-rules` — the structured rule registry (rules-as-data, typed load + parity metadata).
- `enforcer-validator` — the `Validator` trait + the fixture/parity harness (reusable base).
- `enforcer-lang-{rust,ts,py,common,security,iac,k8s}` — per-family validator crates (impl `Validator`).
- `enforcer-literal-scan` — the existing Rust scored (T2) scanner, folded in.
- `enforcer-mechanization` — d01: rule scaffolder + fail-closed parity oracle.
ENGINE:
- `enforcer-scan` — parallel scan engine (rayon) + detect-and-route router (f05) + scan modes (f01).
- `enforcer-coordination` — hub/lane/claim/guard/ledger/presence/sync (port `src/coordination/vendor/*.js` -> Rust).
- `enforcer-proof` — proof harness (routed proofs, artifacts, freshness, PR-ready claims).
- `enforcer-harness` — native-tool run-adapters (cargo/tsc/ruff/dart/CFLint...) + compact diagnostics.
- `enforcer-security` — Track H money-critical & security-testing validators.
- `enforcer-plan` — Track B plan scaffolder + PLAN-* structure validator.
SURFACES:
- `enforcer-mcp` — the stdio MCP server (tool surface; consolidated via the router).
- `enforcer-cli` — clap CLI; compiles to the `enforcer` binary (also serves MCP on stdio).
- `enforcer-install` — multi-harness installer (adapters: claude/codex/gemini/antigravity/cursor/windsurf/
  zed/opencode/aider/kilocode/kiro) + per-platform binary distribution.
- `enforcer-ui` — UI server / Tauri backend; frontend = TS/web (Tauri app + served HTML fallback).

## UI — Tauri control-plane app (Track G)
The enforcer is a CONTROL PLANE any harness reads via MCP; the Tauri desktop app (Rust `enforcer-ui`
backend + TS/web frontend) is the OPTIONAL human cockpit — not required (CLI/any-harness works without it).
- **Config / controls (fully customizable):** auto-discover projects (f02); enable-for-this-project or not;
  apply-by-language; toggle any rule (on/off, severity, waiver); global-vs-project scope. Stored in
  `enforcer-config`; the MCP READS this and acts accordingly (f03). This is "set up your controls, choose
  what applies."
- **Rules & skills explorer (g08, new):** every rule/skill rendered as UI — meaning, behavior, why-it-matters,
  fail/pass examples, tier, framework mapping. THIS is where `.md` lives (humans browse via UI); the AI reads
  the STRUCTURED rule, never the prose.
- **Live lane/hub activity (g06):** real-time coordination-ledger view — lanes, claims, locks, leases,
  releases, mail/intercommunication — streaming live while Codex/Claude work in parallel.
- **Scan/security report + actions:** g02 report, g03 per-violation fix/ignore/later/comment (-> honest
  waivers), g04 Run->dispatch.
- **Harness-agnostic:** identical whether driven from Claude Code / Claude Desktop / Codex / Cursor / CLI —
  the harness just consumes the enforcer as an MCP layer + reads the config; no violation of any harness; a
  helper for all. Track G packs: g01 Tauri shell, g02 report, g03 actions, g04 run-dispatch, g05 config/
  customization, g06 live lane-hub panel, g07 ui-security, g08 rules-&-skills explorer.

## Borrows from OcentraParent (all-Rust reference — adopt the discipline, not the runtime)
Studied OcentraParent (a ~31-crate all-Rust monorepo that consumes the enforcer). Verdict: borrow the
mechanical discipline; trim the service-runtime machinery (the enforcer is a batch CLI/MCP engine, not a
multi-service product).
- **Decoupling + workspace lints (verbatim, owned by a01):** workspace-root `Cargo.toml` owns
  `[workspace.lints.rust] unsafe_code=forbid` + a clippy DENY wall (`unwrap_used, expect_used, panic, todo,
  unimplemented, dbg_macro, print_stdout, print_stderr, await_holding_lock, future_not_send, clone_on_ref_ptr,
  redundant_clone, needless_pass_by_value, map_err_ignore, large_enum_variant, ...`); every crate opts in via
  `[lints] workspace=true`. `print_stdout/print_stderr` allowed ONLY in ONE output-sink module of
  `enforcer-cli`/`enforcer-mcp`. Also shipped as a T1 rule in `enforcer-rules` so we govern consumer repos too.
- **No re-export barrels (adapt):** ban `pub use`/`pub(crate) use` barrels; import concrete module paths.
  Reimplement OcentraParent's `no-reexports` syn-AST check as an `enforcer-lang-rust` Validator (structured
  Findings, not a println binary). Reject the `const _ = size_of` keep-alive idiom.
- **Eventing (lean, bounded):** new `enforcer-events` leaf crate (see crate map). Wired ONLY between
  long-lived/observable subsystems (scan lifecycle, coordination lane/claim/lease, proof); pure compute
  (validators/domain/config/lang-*) uses plain calls. REJECTED as over-engineering: contract-registry RPC
  catalog, aggregate-ordering gates, TTL/overflow queue, request/response brokering, external transport.
- **Logging = structured data (NO new crate):** telemetry/audit records are versioned serde structs
  (`schema_version` + `eventType`) in `enforcer-domain` reusing branded newtypes; TWO-layer redaction
  (key-based field names + value-pattern secret regexes, both always run) in `enforcer-core`; a generic
  append-only `NdjsonWriter<T>` + a pure hash-chain primitive in `enforcer-core`; tracing structured fields
  keyed by `correlation_id`. No `enforcer-log` crate (would duplicate `enforcer-proof`).
- **Audit/proof tamper-evidence:** `enforcer-proof` gets an append-only SHA-256 hash-chained NDJSON journal
  (verify-on-open + on-replay), keeping its existing rich envelope (git-state/in-toto/retention).
- **Schema + Rust->TS (adopt, upgraded):** `enforcer-domain` is the single serde-only dependency-light leaf
  owning DTO shape; every id a validated branded newtype (parse-at-boundary). TS types for the UI are DERIVED
  via `#[derive(ts_rs::TS)]` -> export bin/xtask -> committed generated `.ts`, guarded by a fail-closed
  `cargo test` drift test (byte-compare committed vs freshly-emitted). ONE `enforcer-domain` crate with
  modules — NOT per-feature `*-domain` crates.
- **Consumer contract (design for best-possible usage, NOT any WIP snapshot):** OcentraParent is a
  DIRECTION-SETTER, not a spec — it is actively-built WIP (Codex mid-build, only wip-push commits), so do NOT
  reverse-engineer its exact current wiring. Design for how we WANT parent AND any project to use the enforcer
  in the best possible way: **BOTH surfaces first-class.** (1) **MCP** = the harness-native, install-once,
  ZERO-per-repo-config agent UX (the global-install thesis: any repo just uses it — the primary agent
  experience). (2) **CLI** = equally first-class for direct/CI/precommit/cargo-alias use — tri-modal scope
  `<paths...> | --base <sha> --head <sha> | --all`, exit-code-driven, Windows-first (argv-quoting + backslash
  normalization), terse `Fix:` hints, NO override flag. Neither is "secondary" — the engine is one binary that
  is excellent at both. `enforcer-config` is the single DECLARATIVE control-plane (owner/exempt globs,
  allow-regex, per-rule toggles, cfg(test) skipping) that BOTH surfaces + the UI read; never an inline-disable;
  a no-bypass meta-check (`enforcer-security`) bans inline suppressions. The installer emits whatever a target
  wants — harness MCP auto-register, a cargo-alias, a pre-commit hook, and/or a tool-neutral doctrine ref.

**Decisions locked (flip on request):** event dispatch = **SYNC-first** (rayon/CPU-bound; tokio only if a
`serve` daemon appears); TS codegen = **`ts_rs`** derive; JSON/wire casing = **camelCase** (MCP/UI surface).
Net crate-map delta: **+1 crate (`enforcer-events`)**; logging folds into core/domain/proof/harness (no `enforcer-log`).

## Distribution (codebase-memory model)
`cargo build --release` per target -> GitHub Actions matrix (win/mac/linux, incl. musl + apple-silicon) ->
released binaries. `enforcer install` (or npm/winget/curl one-liner) downloads the platform binary and
registers it as each harness's MCP server (the binary itself speaks MCP on stdio). No runtime toolchain
required by consumers. Optional graceful-skip adapters (Python/CLI tools) only where an engine is
irreplaceable (symbolic-exec/fuzz/network-scan/binary-forensics).

## Track re-cast (same tracks, Rust)
- **A (dogfood):** `.mjs -> Cargo workspace`. The 50-file conversion swarm becomes cohesive CRATE-BUILD
  workpacks (one per crate/subsystem, dependency-ordered), not file-1:1. Branded domains -> `enforcer-domain`
  newtypes. Toolchain (tsconfig/eslint) -> Cargo + clippy/rustfmt/deny/audit + `rust-toolchain.toml`.
  Fingerprint -> hash the built artifacts. Self-enforcement -> enforcer's Rust rules on its own crates.
- **B:** `enforcer-plan` crate. **C:** `enforcer-install` crate + binary release/download.
- **D:** `enforcer-mechanization` + validator crates. **E:** `enforcer-lang-*` validator crates (validate
  target-language code from Rust). **F:** `enforcer-scan`. **G:** Tauri (`enforcer-ui` + TS frontend).
  **H:** `enforcer-security`. **I (queued):** Rust validators. **cyber-skills (h11):** Rust validators (already planned Rust).

## What stays / drops
- DROP: the TS toolchain packs, `.mjs` entrypoints/shims, Effect-Schema (-> Rust serde/newtypes + `ts_rs`
  codegen), node engine concerns, AND the full ocentra-eventing feature set (contract-registry brokering,
  aggregate-ordering, TTL queue, request/response, external transport).
- KEEP as-is conceptually: every rule/track/doctrine — only the implementation language changes to Rust.
- KEEP TS: only the Tauri UI frontend (types DERIVED from `enforcer-domain`, not hand-written).
- ADD (OcentraParent borrows): `enforcer-events` (lean), the `[workspace.lints]` deny wall, two-layer
  redaction, the hash-chained proof journal, and the Rust->TS drift-test pipeline.
