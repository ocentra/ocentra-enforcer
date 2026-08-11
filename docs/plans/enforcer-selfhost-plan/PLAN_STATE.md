# PLAN_STATE — `enforcer-selfhost-plan`

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `PLAN_STATE`
> Kind: index / orientation. The single source of "where the plan is".
> Read when: First thing after README/AGENTS, every time you resume.
> Stop rule: Orientation only. Do not execute from here; go to your one assigned workpack.
> Proves: nothing. It reports status; it does not confer it.
> Does not prove: any workpack DONE. Only proof rows in TEST_PROOF_EXPECTATIONS.md do.
> Proof rule: A row here flips to DONE only after the backing workpack's proof is green.
<!-- /agent-capsule -->

## Scope

Make the enforcer self-hosting and self-enforcing, and package it to enforce anywhere. **The enforcer is a RUST Cargo-workspace engine** (governing doc: [RUST_ARCHITECTURE.md](./RUST_ARCHITECTURE.md)) — one binary is both the MCP stdio server and the CLI; rules are structured data; dogfood is native Rust; TS is used only for the Tauri UI; Node/`.mjs` is dropped. **This supersedes the earlier `.mjs` -> TypeScript decision.** Concretely:

1. **Stand up the Cargo workspace (dogfood).** Build the enforcer as a workspace of Rust crates (`arc-01`..`arc-25`: core, domain, config, events, rules, validator, per-language validators, literal-scan, mechanization, scan, coordination, proof, harness, security, plan, mcp, cli, install, ui) — dependency-ordered CRATE-BUILD packs, not a file-1:1 `.mjs` -> TS conversion. The crate map is **29 crates total** = 25 arc crates + 3 lang crates built by Track E (`enforcer-lang-dart`, `enforcer-lang-cfml`, and the OPT-IN `enforcer-lang-crypto`, OFF by default) + 1 crate built by x06 (`enforcer-memory`, the harness-memory graph/recall crate). `arc-25` (`enforcer-events`, the lean in-process typed event spine consumed only by scan/coordination/proof) is VENDORED as-is from OcentraParent's `ocentra-eventing` and renamed. Cargo + clippy/rustfmt/deny/audit + `rust-toolchain.toml` is the toolchain contract. Execution is bootstrap-safe (a separate git worktree + branch; the live `.mjs` MCP stays intact until the Rust engine is proven green, then the MCP is swapped) — see [EXECUTION_MODEL.md](./EXECUTION_MODEL.md).
2. **Brand its own domains.** RuleId, RepoRoot/RelPath, Sha256/fingerprint ids, hub/lane coordination ids as `enforcer-domain` branded newtypes with serde, validated (parse-at-boundary) at every boundary.
3. **Real self-enforcement.** `enforcer:self` runs the enforcer's own Rust rules (plus `cargo clippy`/`fmt`/`deny`/`audit`) over its own crates and hard-fails on any real finding; gates wired into local CI and a GitHub workflow.
4. **Install + enforce across any harness.** Harness-neutral install core, adapters (Claude, Codex, generic, stubs), and a PreToolUse deny-hook that mechanically blocks T1 violations.
5. **Borrow ADBP ideas, mechanized.** Every borrow dragged up the T1/T2/T3 ladder (ratchet, deferred-work gate, telemetry, context brake, fix loop, doc-rule parity, drift, layered/frontend rules), never copied as prose.
6. **Ship a planning skill.** `ocentra plan new` scaffolder + `PLAN-*` structure validator + `/plan` skill that self-validates against this plan.

Out of scope: reusing the legacy `.mjs`/Node runtime (dropped); business logic in TypeScript (TS is only the Tauri UI frontend); changing the enforcer's external MCP/CLI contract except where a workpack explicitly owns it. Consumer contract: BOTH the MCP stdio surface and the CLI are FIRST-CLASS (neither is "secondary" — one binary excellent at both). All tracks are now re-framed to Rust crates: Track B is the `enforcer-plan` crate, Track C `enforcer-install`, Track D `enforcer-mechanization` + validator crates, Track E `enforcer-lang-*`, Track F `enforcer-scan`, Track G the Tauri `enforcer-ui` (Rust backend + TS frontend), Track H `enforcer-security` — no `src/**`/`.ts` engine surface remains.

## Resume route

`README.md` -> `AGENTS.md` -> `RUST_ARCHITECTURE.md` (the Rust engine doctrine + crate map) -> **this file** -> `NEXT_ACTIONS.md` -> `WORKPACK_INDEX.md` -> your one workpack + its rows in `TEST_PROOF_EXPECTATIONS.md`. For sequencing/parallelism, `PLAN_EXECUTION_BLUEPRINT.md`.

## What is present

- **Workpacks authored: 118.** Track A = 35; Track C = 11; Track D = 25; Track E = 6; Track B = 6; Track F = 5; Track G = 9; Track H = 10; cross-cutting = 8 (`x01`–`x06`, `x08`, `z01`); Track P = 3 (`p01`–`p03`). The totals are reconciled to `WORKPACK_INDEX.md` (118 rows).
- Each workpack carries an agent-capsule, `owns:`/`deps:`/`tier:` frontmatter, Where-We-Are / Where-We-Want-To-Be, a Requirement Checklist, an Acceptance And Proof block, and Parallel Ownership Notes.
- The index/contract set (this file, README, AGENTS, NEXT_ACTIONS, WORKPACK_INDEX, PLAN_EXECUTION_BLUEPRINT, TEST_PROOF_EXPECTATIONS, PLAN_HEALTH) exists.

## Current open gaps (live snapshot)

This orientation page is not the completion ledger. The graph commit and the
`WORKPACK_INDEX.md`/`TEST_PROOF_EXPECTATIONS.md` rows are the live status
surfaces; proof artifacts and exact candidate branches decide whether a row can
move. At the graph snapshot dated 2026-08-11, this plan contains 118 rows: 8
are `DONE` and 110 remain `TODO`.

- `d15` is `DONE` with its documentation-only cross-link proof.
- `a01` remains `TODO`/proof `PENDING`: the Rust workspace/toolchain gates are
  present, but the package runtime cutover is explicitly deferred behind the
  later arc dependencies.
- `g01` remains `TODO`/proof `PENDING`: the exact-current retained proof records
  browser click-through as unavailable and desktop/Tauri smoke as not run. The
  older graph artifact is not accepted as `GREEN`.
- The Cargo workspace, toolchain, install crates, and native UI skeleton are
  present in this checkout; their presence is not a claim that the remaining
  workpacks or terminal dogfood gate are complete.
- Remaining product, plan, install, UI, language, and security gaps are
  represented by the `TODO` rows and dependency edges in the index/graph. The
  Rust `.mjs` authority remains live until the terminal proof gate is green.

## Workpack summary (by track)

| Track | Packs | Root / keystone | Shape |
|---|---|---|---|
| **A00 — Rust toolchain** | `a01` | `a01` (deps none) | Blocks every Track A pack; owns the workspace root; Cargo + clippy/rustfmt/deny/audit + `rust-toolchain.toml` contract. |
| **A — crate-build swarm (Rust)** | `arc-01`…`arc-25` | roots **a01 -> arc-01 (`enforcer-core`, first member crate) -> arc-02 (`enforcer-domain`, schema keystone)** | Dependency-ordered Cargo crates (core/domain/config/events/rules/validator/per-lang/literal-scan/mechanization/scan/coordination/proof/harness/security/plan/mcp/cli/install/ui); disjoint `crates/<name>/**` globs. Includes the new `arc-25` (`enforcer-events`, VENDORED from OcentraParent's `ocentra-eventing`, consumed by arc-15/16/17). Replaces the old 50-pack `.mjs`->TS conversion swarm. Crate map = **29 crates** (25 arc + 3 Track-E lang crates + 1 x06-built `enforcer-memory`). |
| **A — domain hardening (Rust)** | `a02`…`a10` | mostly deps `a01` | `enforcer-domain` branded newtypes, serde parse-at-boundary, waiver honesty, anti-silent-skip; `a10` is the CI self-enforce capstone that runs the enforcer's own Rust rules on its own crates (deps `a01`,`a09`). |
| **B — planning skill (Rust `enforcer-plan`)** | `b01`…`b05` | `b01`,`b02`,`b03` (feature files inside the `arc-20` `enforcer-plan` skeleton) | Rust `enforcer-plan` crate: scaffolder + PLAN-* validator + templates parallel; `b04` deps `b02`,`arc-16`; `b05` capstone (`/plan` skill + emitter in `enforcer-install`) deps `b01`,`b02`,`b03`,`arc-23`. |
| **C — install/enforce** | `c01`…`c11` | `c01` (deps none) | Core blocks adapters; `c03` Claude adapter, `c04`/`c05` hooks, `c06` Codex parity, `c07` generic, `c08` stubs, `c09` remaining six adapters (Antigravity/Windsurf/OpenCode/Aider/KiloCode/Kiro; deps `c01`,`c02`) — with c03/c06/c07/c08 all 11 harnesses are covered; `c10` CI integration + `c11` onboarding skill. |
| **D — ADBP borrows + mechanized families** | `d01`…`d15`, `d16`–`d18`, `d21`–`d23`, `d25`–`d28` | `d01` (deps none) | Rule-mechanization engine keystone; `d02`–`d13` mostly deps `d01`; `d14`,`d15` deps none. New: FSM validity (`d16`), Rust error-handling (`d17`), security STOP watchlist (`d18`), change discipline (`d21`), size/shape caps (`d22`, +`d02`), test companion/quality (`d23`, +`d16`), orchestrator verify gates (`d25`), dispatch prompt assembly (`d26`), loop resilience/telemetry (`d27`, +`d04`), target-repo CI parity (`d28`). |
| **E — new languages + universal scanning** | `e01`, `e-pack-dart`, `e-pack-cfml`, `e-pack-frontend-react`, `e-pack-python`, `e-pack-crypto-blockchain` (OPTIONAL) | `e01` + `d16`,`d22` | Always-on universal literal-scan T2 floor (`e01`, non-blocking, ~65 languages); four first-class language packs — Dart, CFML via CFLint shell-out, React/Next Effect-only, and Python FastAPI layered/clean-arch (`e-pack-python`, layering/DI + Python security) — all consuming `d16`/`d22`; plus the OPTIONAL, opt-in `e-pack-crypto-blockchain` (Solana/Anchor on-chain as the example, OFF by default, deps `d01`/`d17`/`d18`/`h01`) that mechanizes the §2.5 on-chain abuse surface only when the project opts in, consuming h06 signing + the h07 localnet adapter read-only. |
| **F — scan surface, onboarding & agent-shaping** | `f01`–`f05` | `f01`,`f03` (early leaves, deps `a01`/`d01`) | Agent-selectable scan MODES (`f01`, `enforcer_scan`, scoped-not-whole-repo default); index-on-ask onboarding scaffolding `.enforce/` (`f02`, after `f03`); per-project native-tie `.enforce/config` schema (`f03`, contract `f01`/`f02`/`c04` consume); formal AGENT-INLINE (silent) vs HUMAN-REVIEW run context (`f04`, after `c04`,`f01`) — the gate Track G's UI honors; and the foundational detect-and-route router (`f05`, deps `a01`/`d01`/`f03`) that mechanically detects languages/structure/scope, emits a serializable ROUTE PLAN, and routes each language to its rule packs AND native tools — consolidating the per-tool surface so `f01`/`f03`/`c04` and the check/scan/run tools CONSUME it. |
| **G — UI layer (vendored hub dashboard/server)** | `g01`–`g09` | `g01` (serve surface, deps `arc-24`; lands first) | Human-invoked local UI on the vendored server/dashboard surface (read-only reuse), mounted over the `arc-24` `enforcer-ui` skeleton. `g01` promotes it to `enforcer serve`/`ui` with a view-mount registry; `g02`–`g06` provide report/actions/dispatch/settings/dashboard views; `g07` supplies UI security; `g08` explores rules and skills; `g09` is the read-only memory/KG/RAG explorer. All G packs mount into `g01` and honor `f04` silent mode. |
| **H — money-critical & security-testing mandate (generic)** | `h01`–`h08`, `h11`, `h12` | `h01` (classifier keystone, deps `d01`) | Generic mechanization of the ingested [refs/security-testing-source.md](./refs/security-testing-source.md) spec into T1/T2 rules — GENERIC across ANY value system behind untrusted infra, never crypto/game-specific. `h01` money-critical classifier (keystone; emits the manifest `h02`/`h03`/`h05`/`h06` consume read-only), `h02` required-test-categories gate (+`d23`), `h03` threat↔invariant↔test mapping ("unmapped logic is forbidden logic"), `h04` security-test-quality banned patterns (+`d23`, composes `d03`), `h05` economic-invariant property suite, `h06` money-critical mechanics (signing/time/boundary/kill-switch + economic/rollback), `h07` security-tooling CI + observability (deps `d01`,`a10`,`c01`; exposes the opt-in crypto-localnet adapter seam), and `h08` testing-mandate SKILL + profile + policy-ingestion (deps `d01`,`b01`). **`h08` ships the previously-missing pieces: the generic security-testing SKILL, the NEUTRAL loadable profile `profiles/money-critical-security.json` (no product/company/game branding — the FIRST ingested policy profile), and POLICY-SPEC-INGESTION mapping any project's spec doc to a mechanized profile (backed rules enable; un-backed asserted rules flagged for mechanization, fed to `d01`/`d08`).** **`h11`/`h12` mechanize the vendored `anthropic-cybersecurity-skills` corpus: `h11` (deps `d01`,`h03`,`f05`) reimplements the fundamental-logic (a)/(b) cyberskills (IaC/cloud/manifest/header predicates + a scored WAF-SQLi matcher) as native Rust rules with no subprocess, seeds `h03`'s threat vocab from the corpus MITRE/NIST frontmatter, registers a `security-audit` scope into the `f05` router, and fail-closes the dogfood via a `vendor/**` ignore-glob; `h12` (deps `d01`,`f05`,`h11`) is the OPTIONAL, out-of-dogfood complement of python/CLI run-adapters wrapping the (d) irreplaceable engines (symbolic-exec/fuzzers/scanners/forensics/SDK-fetchers) that graceful-skip honestly and feed a thin severity gate.** |
| **Cross-cutting** | `x01`, `x02`, `x03`, `x04`, `x05`, `x06`, `x08`, `z01` | `x01` early / `z01` terminal | `x01` neutral rename; `x02` docs refresh; `x03` rename migration; `x04` main-branch-protection CI; `x05` lesson capture/self-heal; `x06` harness memory graph; `x08` cross-harness worklog; `z01` terminal dogfood-proof gate. |
| **P — policy/honesty/accuracy gap-fillers** | `p01`–`p03` | late frontier after their declared crate/data dependencies | Choosable doctrine profiles, scan-ignore defaults with skip honesty, and optional AST-accurate matching; all remain dependency-gated. |

Full per-pack owns/deps/tier: **`WORKPACK_INDEX.md`**. Sequencing and parallel model: **`PLAN_EXECUTION_BLUEPRINT.md`**.
