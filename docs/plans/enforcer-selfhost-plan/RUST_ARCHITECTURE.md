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
- **The real user of this engine is an AI agent, not a human** (owner directive, 2026-07-04). A human CAN
  trigger the CLI manually from an IDE/shell — that is incidental, not the design target. Consequences:
  install is not "one script and done" — it is install (MCP registration) -> inspect the target's real build
  system -> configure (author a fitting `enforcer-config`) -> wire CI for that specific project -> VERIFY the
  wiring actually works, and steps 2-5 require judgment an agent (AI or human) must make; this is NOT fully
  mechanically automatable and is not supposed to be — the product is a SKILL (`c11`) that makes that judgment
  reliable and repeatable, not a script that eliminates it. Every install/onboard/CI surface (c01-c11) is
  designed agent-first; a human using it directly is a fully-supported but secondary path.
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
- `enforcer-config` — typed config load, parse-at-boundary, **3-layer resolution** (embedded global profiles
  `strict`/`ocentra-enforcer`/`ocentra-parent`/`default` -> per-project `ocentra-enforcer.config.json` override
  -> one resolved `EffectiveConfig`; zero-config projects get the `default` profile alone). `.enforce/` per-run
  OUTPUT is a separate concern owned by arc-17/arc-18, which read `EffectiveConfig` but never live here.
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
  Ships a **lane-worktree spawn primitive** (`enforcer coordination lane new/park/rm`, also an MCP tool) so
  ANY harness (Claude Code, Cursor, Codex, Windsurf, ...) gets "one isolated worktree per parallel worker,
  zero shared build state" for free — see EXECUTION_MODEL.md §2b. The ledger itself is the opposite of
  isolated: presence/heartbeat is multi-machine + multi-project aware (`byPc`/`byProject`/`byWorktree`), and
  named peers sync over HTTP or local FS (`pull`/`push`/`both`, append-only, conflict-detected) — a genuinely
  distributed swarm across machines and projects, not just worktrees on one box. See EXECUTION_MODEL.md §2c.
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
Net crate-map delta: **+2 crates (`enforcer-events`, `enforcer-memory`)** — `enforcer-memory` (x06) is the harness-memory graph/vector-recall crate over the x05 lesson corpus (local-first, codebase-memory model, zero-trust federation); logging folds into core/domain/proof/harness (no `enforcer-log`). Crate map = 25 arc + 3 E-built + 1 x06-built = **29 crates**.

## Distribution (codebase-memory model)
`cargo build --release` per target -> GitHub Actions matrix (win/mac/linux, incl. musl + apple-silicon) ->
released binaries. `enforcer install` (or npm/winget/curl one-liner) downloads the platform binary and
registers it as each harness's MCP server (the binary itself speaks MCP on stdio). No runtime toolchain
required by consumers. Optional graceful-skip adapters (Python/CLI tools) only where an engine is
irreplaceable (symbolic-exec/fuzz/network-scan/binary-forensics).

### Global-install scope contract (canonical, harness-neutral)
The canonical/default install is **USER/GLOBAL scope** — install-once, zero-per-repo-config, so any repo the
agent opens already has the enforcer. Each adapter writes the enforcer's MCP registration into its harness's
**user-level** config, NEVER a per-repo project file, and points `command` at the **absolute** path of the
installed `enforcer` binary (a relative path cannot resolve from an arbitrary repo cwd). Concretely, mirroring
how `codebase-memory-mcp` is registered machine-wide:
- **Claude Code** -> the top-level `mcpServers` map in **`~/.claude.json`** — the SAME file/registry
  `codebase-memory-mcp` lives in. NOT a per-repo `.mcp.json`, and NOT `~/.claude/.mcp.json`. User-scope
  entries here need no per-project `enabledMcpjsonServers` approval, unlike checked-in project `.mcp.json`
  servers (which is why project scope is not the release default).
- **Codex** -> the user `~/.codex/config.toml` `[mcp_servers.<name>]` table.
- **Every other harness** -> that harness's equivalent user/global registry (c02 autodetect resolves the home;
  c07 generic writer handles plain-`.mcp.json` harnesses at their **user** home, not the target repo).

`enforcer install` with no scope resolves to `user`; `--scope project` is an explicit, NON-default opt-in (useful
only for developing the enforcer itself — that is the sole role of this repo's own root `.mcp.json`, which the
installer does NOT emit into consumers). The install CORE and this default path are **harness-neutral**: Codex is
one adapter among ~11, never the canonical/reference/default target, and no shipped name or default surface is
named after any single harness (product/binary/MCP-server name is `enforcer`; see x01). Every adapter registers
under the x01-owned server-name const — `mcpServers["enforcer"]`, tools surface as `mcp__enforcer__*` — never a
hardcoded legacy literal.

### Update UX (binary swap, not a repo pull)
Once installed, staying current is a **binary swap**: `enforcer update` (or a harness-exposed pub/sub trigger — e.g.
the user says "update enforcer") checks the release channel and, if a newer signed build exists, removes the old
binary and downloads the new one — no source checkout, no toolchain, nothing else. Because the MCP registration
points at the **binary path** (not a repo/folder/branch), the harness keeps firing the identical command; only the
bytes behind it change. Freshness is the a02 fingerprint over the running artifact (`mcp_status` reports `stale`
when a newer build is available). This is what makes the enforcer harness-, repo-, and folder-agnostic in steady
state: whether the host is Claude or anything else, an MCP fires a versioned binary, not a checkout.

### The learning thesis — experience over pretraining, PROVEN by the proof system (owner-set, 2026-07-04)
The enforcer's end state is a **human-like learning system**: it gets better BY BEING USED, not by waiting for a
better pretrained model. Lessons are DUAL-DOMAIN — `harness` (orchestration/protocol/coordination, e.g. the
mail-lifecycle and stale-base lessons) AND `code` (fault patterns the gates catch, fix patterns that survive
review, tooling drift like a deny-schema migration). Both flow through the same x05 capture → x06 memory →
retrieval-at-action loop: a coding lesson lands as a rule-candidate WITH fail/pass fixtures, ships as an enforced
gate, and is recalled at session start — so the next agent cannot repeat the fault even if its base model would.
The intelligence accumulates in the SYSTEM (rules + fixtures + graph), not the weights — which is why a
capsule-disciplined small model performs like a frontier model here (measured live: L8).
**And the claim is FALSIFIABLE, not aspirational — the proof system is the instrument:** a learning event is the
triple (t0: violation/incident observed, recorded with `observedIn` provenance) → (t1: artifact landed — rule +
fixtures green, doctrine block, forest node — x05 fail-closed doctor) → (t2+: recurrence query over the x06 graph
+ the tamper-evident `enforcer-proof` hash-chained journal shows the fault class caught-at-gate or extinct).
"The harness learned X" is thus a query with evidence, never a vibe; the aggregate curve (violation/recurrence
rates declining over usage) is the benchmark that proves experience-driven learning works — bad code prevented
IS the movement toward good code, made cumulative and measurable. z01 (dogfood gate) is the terminal instance:
the system proving, against itself, that everything it learned holds.

### Dev-time transition wiring (until the Rust binary exists) — anti-recursion
While the enforcer itself is being converted to Rust, the live MCP still runs the legacy `.mjs` from a repo
checkout. Running the harness's live tool out of the SAME code under active edit is a self-editing recursion
(the tool validating a change IS the change). Until `enforcer install` can wire the compiled binary, the live
global MCP is therefore pinned to a **separate, frozen worktree** from the one being edited:
- live global MCP -> a frozen worktree's entrypoint by ABSOLUTE path (e.g.
  `C:/Projects/ocentra-enforcer/mcp/ocentra-enforcer-mcp.mjs`), registered in `~/.claude.json` (user scope);
- active edits happen in a DIFFERENT worktree (e.g. `C:/Projects/enforcer-rust`) — the frozen tool enforces the
  work without being the work;
- refresh on a checkpoint, never mid-edit: commit+push in the edit worktree, then `git -C <frozen-worktree> pull`
  (or checkout the good commit) to advance the live MCP deliberately.
This is transitional scaffolding, torn out the moment the binary path (above) is wired.

## CI integration for CONSUMER projects (c10 — a different surface from AI-harness install)
Researched 2026-07-04: `cargo-dist` (axodotdev) is the standard tool for exactly this — it generates its own
GitHub release pipeline (plan/build/host/publish across the target matrix above) plus portable `install.sh`/
`install.ps1` scripts and can produce an npm wrapper package. Adopted shape (see [c10](./workpacks/c10-ci-integration-and-binary-bootstrap.md)):
- **Zero Rust toolchain required in consumer CI.** A curl/iwr-installable `install.sh`/`install.ps1` downloads
  the matching release binary (checksum-verified) — works identically on GitHub Actions, GitLab CI, CircleCI,
  Bitbucket, Jenkins, or bare shell. `cargo install` from source is a documented FALLBACK only.
- **A reusable composite GitHub Action** (`.github/actions/enforcer-scan`) wraps the installer + caches the
  downloaded binary (`actions/cache`, keyed by version+platform) for GitHub users specifically.
  Wired by the existing `github-actions` install adapter.
- **Optional npm wrapper** (thin JS shim + per-platform `optionalDependencies`, the biome/esbuild/swc pattern,
  glibc/musl runtime-detected on Linux) so consumers ALREADY wired the old Node-centric way (`npm install` +
  `npx enforcer ...`) keep working unchanged even though the package now ships a compiled binary.
- **CI always regenerates proof fresh** — never trusts a pre-computed/uploaded artifact as a substitute for
  running the binary (extends the swarm's own zero-trust/no-upload doctrine, EXECUTION_MODEL.md §2b/§2c, to the
  consumer-CI boundary). CI MAY separately upload its OWN freshly-generated report as a build artifact for
  human PR review afterward — a distinct, legitimate act, not a trust shortcut; do not conflate the two.
- **Fixes a real independent bug found during this research:** `docs/TARGET_REPO_WIRING.md` currently hardcodes
  a local absolute path (`E:/ocentra-enforcer/rules/INDEX.md`) that cannot resolve on any machine but the
  original author's, let alone a cloud CI runner. Rules are compiled into the binary (rules-as-data, arc-04);
  `enforcer explain <ruleId>` replaces "go read a file at a hardcoded path." No doc/adapter/generated CI config
  may ever reference a literal local absolute path.
- **This repo's OWN CI builds+publishes the release; every consumer project's CI only ever POINTS AT it.**
  There is exactly one producer (this repo's release pipeline) and many consumers (any project's CI downloads
  the published binary). Consumer CI is FULLY MECHANICAL — no Codex/Claude/agent judgment is present there, so
  the binary's exit code is the entire verdict, and a broken release binary is categorically worse than a
  broken rule (it can silently fail closed/open for EVERY consumer at once). Hence: a pre-publish cross-platform
  smoke gate blocks any release where the binary itself is broken on ANY target platform (see c10); the
  exit-code taxonomy (arc-22) hard-separates "target project failed a check" from "the enforcer crashed"; and
  version-pinning is the DEFAULT for consumers (an explicit "floating latest" channel is opt-in, not automatic)
  so an enforcer-side rule change cannot silently break many unrelated projects' CI with zero warning.
- **`full` vs `lite` binary variants (Cargo feature split, arc-22).** CI never needs the coordination hub
  (arc-16) or the UI (arc-24) — a headless mechanical CI run has no lanes, no mail, no Tauri surface to serve.
  `enforcer-cli` ships a `lite` feature profile excluding both from the compiled binary (smaller, faster,
  smaller attack surface) from the SAME source tree as `full` (DRY — no forked CI-only crate). CI tooling
  (installer/GH Action/npm wrapper) defaults to `lite`; `full` remains an explicit opt-in.
- **CI-runner platform coverage is independent of the target project's own build platform.** The win/mac/
  linux(+musl+apple-silicon) matrix covers every real CI-runner family; a project targeting mobile/embedded/
  etc. still runs its CI on an ordinary x86/arm mac/linux/windows runner. We do NOT cross-compile the enforcer
  itself to run "on a phone" — that would be solving a problem that does not exist.
- **`PORT-1.1` (platform-specific script commands must be guarded) gains a declared-scope relaxation it lacks
  today** (`enforcer-config`'s `supportedPlatforms` field, arc-03/arc-09): a project that legitimately only
  targets one platform's CI is not hard-failed for platform-specific code within that declared scope; absence
  of the field defaults to the current strict all-three-platforms behavior — no silent relaxation by omission.

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
