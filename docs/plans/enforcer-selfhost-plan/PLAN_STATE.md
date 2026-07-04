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

Make the enforcer self-hosting and self-enforcing, and package it to enforce anywhere. Concretely:

1. **Dogfood the migration.** Big-bang convert the `.mjs` codebase (`src/`, `mcp/`, `tests/`) to strict TypeScript, with a compiler contract, branded domain types, parse-at-boundary, waiver honesty, anti-silent-skip, and real self-enforcement in CI.
2. **Brand its own domains.** RuleId, RepoRoot/RelPath, Sha256/fingerprint ids, hub/lane coordination ids as branded Effect schema types validated at the boundary.
3. **Real self-enforcement.** `enforcer:self` runs the full TS lane and hard-fails on any real finding; gates wired into local CI and a GitHub workflow.
4. **Install + enforce across any harness.** Harness-neutral install core, adapters (Claude, Codex, generic, stubs), and a PreToolUse deny-hook that mechanically blocks T1 violations.
5. **Borrow ADBP ideas, mechanized.** Every borrow dragged up the T1/T2/T3 ladder (ratchet, deferred-work gate, telemetry, context brake, fix loop, doc-rule parity, drift, layered/frontend rules), never copied as prose.
6. **Ship a planning skill.** `ocentra plan new` scaffolder + `PLAN-*` structure validator + `/plan` skill that self-validates against this plan.

Out of scope: rewriting the Rust `rust-rules` crate internals; new language scanners beyond what already exists; changing the enforcer's external CLI contract except where a workpack explicitly owns it.

## Resume route

`README.md` -> `AGENTS.md` -> **this file** -> `NEXT_ACTIONS.md` -> `WORKPACK_INDEX.md` -> your one workpack + its rows in `TEST_PROOF_EXPECTATIONS.md`. For sequencing/parallelism, `PLAN_EXECUTION_BLUEPRINT.md`.

## What is present

- **Workpacks authored: 129.** Track A dogfood (`a01`–`a10`, 10) + conversion swarm (`a-conv-01`–`a-conv-50`, 50) = 60; Track C install/enforce (`c01`–`c09`, 9); Track D ADBP borrows + mechanized families (`d01`–`d15` plus the ten new `d16`–`d18`, `d21`–`d23`, `d25`–`d28`, 25); Track E new languages + universal scanning (`e01`, `e-pack-dart`, `e-pack-cfml`, `e-pack-frontend-react`, `e-pack-python`, plus the OPTIONAL opt-in `e-pack-crypto-blockchain`, 6); Track B planning skill (`b01`–`b05`, 5); Track F scan-surface/onboarding/agent-shaping (`f01`–`f05`, 5); Track G UI layer on the vendored hub dashboard/server (`g01`–`g07`, 7); Track H money-critical & security-testing mandate (`h01`–`h08`, 8) — the generic mechanization of the ingested [refs/security-testing-source.md](./refs/security-testing-source.md) spec; cross-cutting (`x01` neutral rename, `x02` docs refresh, `x03` rename migration, `z01` dogfood-proof-gate, 4).
- Each workpack carries an agent-capsule, `owns:`/`deps:`/`tier:` frontmatter, Where-We-Are / Where-We-Want-To-Be, a Requirement Checklist, an Acceptance And Proof block, and Parallel Ownership Notes.
- The index/contract set (this file, README, AGENTS, NEXT_ACTIONS, WORKPACK_INDEX, PLAN_EXECUTION_BLUEPRINT, TEST_PROOF_EXPECTATIONS, PLAN_HEALTH) exists.

## What is NOT present yet (open gaps)

- **No workpack is DONE.** All product-code, tests, and validators described are specifications, not yet implemented. Status across the board is `NOT STARTED`.
- The repo is still 100% `.mjs`: no `tsconfig.json`, no `build`/`typecheck` script, `enforcer:self` runs only `check source-shape` and does not hard-fail on real findings.
- No `ocentra plan new`, no `PLAN-*` validator, no `/plan` skill.
- No harness-neutral install core; `src/codex-install.mjs` still hardcodes the Codex adapter and there is no PreToolUse deny-hook.
- No mechanized ADBP borrows (no ratchet, deferred-work gate, telemetry schema, context brake, fix loop, doc-rule parity oracle, rule-new scaffolder).
- **Root keystones not yet built:** `a01` (TS toolchain) blocks all of Track A; `a-conv-01` (leaf surface) roots the conversion swarm; `d01` (rule mechanization engine) is the keystone for most of Track D; `c01` (install core) blocks Track C adapters. Nothing downstream is claimable until its root lands.

## Workpack summary (by track)

| Track | Packs | Root / keystone | Shape |
|---|---|---|---|
| **A00 — toolchain** | `a01` | `a01` (deps none) | Blocks every Track A pack; strict `tsconfig` + `typecheck`/`build`. |
| **A-conv — conversion swarm** | `a-conv-01`…`a-conv-50` | `a-conv-01` (deps `a01`) | Leaf surface first, then clusters, then rollups, then tests; disjoint `src/`/`mcp/`/`tests/` globs. |
| **A — domain hardening** | `a02`…`a10` | mostly deps `a01` | Branded ids, parse-at-boundary, waiver honesty, anti-silent-skip; `a10` is the CI self-enforce capstone (deps `a01`,`a09`). |
| **B — planning skill** | `b01`…`b05` | `b01`,`b02`,`b03` (deps none) | Scaffolder + validator + templates parallel; `b04` deps `b02`; `b05` capstone deps `b01`,`b02`,`b03`. |
| **C — install/enforce** | `c01`…`c09` | `c01` (deps none) | Core blocks adapters; `c03` Claude adapter, `c04`/`c05` hooks, `c06` Codex parity, `c07` generic, `c08` stubs, `c09` remaining six adapters (Antigravity/Windsurf/OpenCode/Aider/KiloCode/Kiro; deps `c01`,`c02`) — with c03/c06/c07/c08 all 11 harnesses are covered. |
| **D — ADBP borrows + mechanized families** | `d01`…`d15`, `d16`–`d18`, `d21`–`d23`, `d25`–`d28` | `d01` (deps none) | Rule-mechanization engine keystone; `d02`–`d13` mostly deps `d01`; `d14`,`d15` deps none. New: FSM validity (`d16`), Rust error-handling (`d17`), security STOP watchlist (`d18`), change discipline (`d21`), size/shape caps (`d22`, +`d02`), test companion/quality (`d23`, +`d16`), orchestrator verify gates (`d25`), dispatch prompt assembly (`d26`), loop resilience/telemetry (`d27`, +`d04`), target-repo CI parity (`d28`). |
| **E — new languages + universal scanning** | `e01`, `e-pack-dart`, `e-pack-cfml`, `e-pack-frontend-react`, `e-pack-python`, `e-pack-crypto-blockchain` (OPTIONAL) | `e01` + `d16`,`d22` | Always-on universal literal-scan T2 floor (`e01`, non-blocking, ~65 languages); four first-class language packs — Dart, CFML via CFLint shell-out, React/Next Effect-only, and Python FastAPI layered/clean-arch (`e-pack-python`, layering/DI + Python security) — all consuming `d16`/`d22`; plus the OPTIONAL, opt-in `e-pack-crypto-blockchain` (Solana/Anchor on-chain as the example, OFF by default, deps `d01`/`d17`/`d18`/`h01`) that mechanizes the §2.5 on-chain abuse surface only when the project opts in, consuming h06 signing + the h07 localnet adapter read-only. |
| **F — scan surface, onboarding & agent-shaping** | `f01`–`f05` | `f01`,`f03` (early leaves, deps `a01`/`d01`) | Agent-selectable scan MODES (`f01`, `enforcer_scan`, scoped-not-whole-repo default); index-on-ask onboarding scaffolding `.enforce/` (`f02`, after `f03`); per-project native-tie `.enforce/config` schema (`f03`, contract `f01`/`f02`/`c04` consume); formal AGENT-INLINE (silent) vs HUMAN-REVIEW run context (`f04`, after `c04`,`f01`) — the gate Track G's UI honors; and the foundational detect-and-route router (`f05`, deps `a01`/`d01`/`f03`) that mechanically detects languages/structure/scope, emits a serializable ROUTE PLAN, and routes each language to its rule packs AND native tools — consolidating the per-tool surface so `f01`/`f03`/`c04` and the check/scan/run tools CONSUME it. |
| **G — UI layer (vendored hub dashboard/server)** | `g01`–`g07` | `g01` (serve surface, deps `a01`; lands first) | Human-invoked local UI on the vendored `src/coordination/vendor/{server,dashboard}.js` (read-only reuse). `g01` promotes it to `enforcer serve`/`ui` with a view-mount registry; `g02` scan report (after `f01`), `g03` violation actions writing a08-shaped waivers, `g04` Run-dispatch into the coordination ledger (a-conv-23/24), `g05` settings via c-track adapters (`c01`), `g06` hub dashboard (a-conv-20/23) all MOUNT into g01 and honor `f04` silent mode; `g07` UI-security (deps `g01`,`g04`) is the dedicated `src/ui/security/*` layer — loopback-bind assertion + same-origin/CSRF on the g03/g05 mutation endpoints + intent-token/sandbox on the g04 dispatch — reused by every g0x endpoint (guards HUMAN surface only). |
| **H — money-critical & security-testing mandate (generic)** | `h01`–`h08` | `h01` (classifier keystone, deps `d01`) | Generic mechanization of the ingested [refs/security-testing-source.md](./refs/security-testing-source.md) spec into T1/T2 rules — GENERIC across ANY value system behind untrusted infra, never crypto/game-specific. `h01` money-critical classifier (keystone; emits the manifest `h02`/`h03`/`h05`/`h06` consume read-only), `h02` required-test-categories gate (+`d23`), `h03` threat↔invariant↔test mapping ("unmapped logic is forbidden logic"), `h04` security-test-quality banned patterns (+`d23`, composes `d03`), `h05` economic-invariant property suite, `h06` money-critical mechanics (signing/time/boundary/kill-switch + economic/rollback), `h07` security-tooling CI + observability (deps `d01`,`a10`,`c01`; exposes the opt-in crypto-localnet adapter seam), and `h08` testing-mandate SKILL + profile + policy-ingestion (deps `d01`,`b01`). **`h08` ships the previously-missing pieces: the generic security-testing SKILL, the NEUTRAL loadable profile `profiles/money-critical-security.json` (no product/company/game branding — the FIRST ingested policy profile), and POLICY-SPEC-INGESTION mapping any project's spec doc to a mechanized profile (backed rules enable; un-backed asserted rules flagged for mechanization, fed to `d01`/`d08`).** |
| **Cross-cutting** | `x01`, `x02`, `x03`, `z01` | `x01` early / `z01` terminal | `x01` neutral rename (product becomes `enforcer`, deps none, run early); `x02` docs refresh (after `x01`) reads product docs to `enforcer` everywhere and adds a doc section per new capability (router f05, scan modes, UI, multi-harness, onboarding, silent-vs-human, dart/cfml/frontend/python); `x03` rename migration (after `x01`) is a transitional one-time migrate that rewrites already-installed `ocentra-enforcer` MCP registrations + legacy tool names to `enforcer` (no permanent alias); `z01` dogfood-proof-gate depends on ALL tracks and is the terminal plan-DONE gate that runs the enforcer against its own multi-language self (zero self-violations, fail-closed). |

Full per-pack owns/deps/tier: **`WORKPACK_INDEX.md`**. Sequencing and parallel model: **`PLAN_EXECUTION_BLUEPRINT.md`**.
