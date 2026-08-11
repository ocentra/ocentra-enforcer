# NEXT_ACTIONS — `enforcer-selfhost-plan`

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `NEXT_ACTIONS`
> Kind: index / frontier. The ordered list of what is claimable right now.
> Read when: After PLAN_STATE, to pick your one workpack.
> Stop rule: Claim ONE ready-now pack, then go read only that workpack. Do not batch.
> Proves: nothing. It reports readiness, not completion.
> Does not prove: any workpack DONE. Proof rows do.
> Proof rule: A pack leaves this list only when its proof is green and its row is DONE.
<!-- /agent-capsule -->

"Ready now" = every dep is DONE **and** the pack's `owns:` set is disjoint from all currently-claimed lanes. The graph imports **118 workpacks**, with **8 DONE** and **110 TODO**. Its self-host ready frontier is `a01` and `g01`; `d15` is already DONE. Manager-owned `CP08` and `UL00` are ready in the cross-program graph but are not claimable from this plan. Track totals are A=35, C=11, D=25, E=6, B=6, F=5, G=9, H=10, cross-cutting/X=8, and P=3; `z01` remains terminal.

## Current graph frontier — claim only what the graph marks ready

Under the current graph, `a01` is ready with no dependencies and `g01` is ready because `arc-24` is DONE. `x01` is blocked by `arc-21` and `arc-22`; `d15` is already DONE and must not be reclaimed. Land only rows whose graph state is `ready`; the table below is the recommended dependency sequence, not permission to bypass the graph.

| Priority | Workpack | Track | deps | Why it's first |
|---|---|---|---|---|
| P0-a | [`a01`](./workpacks/a01-cargo-workspace-and-toolchain.md) | A00 | none | Rust toolchain contract (Cargo + clippy/rustfmt/deny/audit + `rust-toolchain.toml`) + workspace root; blocks **all** of Track A (35 packs). |
| P0-a2 | [`arc-01`](./workpacks/arc-01-enforcer-core.md) | A | a01 | `arc-01` (`enforcer-core`, the first member crate; a01 owns the workspace root); the foundation every crate depends on. Then `arc-02` (`enforcer-domain`) — the schema keystone the whole engine is typed against. |
| P0-b | [`d01`](./workpacks/d01-rule-mechanization-engine.md) | D | arc-14 | Rule scaffolder + fail-closed parity oracle; keystone for the rest of D (and Tracks E/F/H fan out from it). |
| P0-c | [`c01`](./workpacks/c01-install-core-and-cli-contract.md) | C | arc-23, arc-03 | Harness-neutral install core + first-class CLI contract; blocks `c02`–`c09`. |
| P0-d | [`b01`](./workpacks/b01-plan-scaffolder.md) | B | arc-20 | Deterministic plan emitter (feature of `enforcer-plan`); feeds `b04`/`b05`. |

## Dependency roadmap — not the current claimable frontier

These rows describe the planned dependency sequence. They are not claimable merely because they appear here; re-check `node scripts/program-graph.mjs status` and claim exactly one row only when it reports `ready`.

| Workpack | Track | deps | owns (disjoint) |
|---|---|---|---|
| [`x01`](./workpacks/x01-neutral-rename.md) | X | a01, arc-21, arc-22 | blocked until the workspace and MCP/CLI name surfaces are complete; do not claim from this roadmap alone |
| [`d15`](./workpacks/d15-readme-research-grounding.md) | D | none | DONE with retained documentation-only proof; do not reclaim |
| [`d14`](./workpacks/d14-ideation-skills-t3.md) | D | arc-05 | `skills/ideation/**` (T3 content, T1-labeled) + `crates/enforcer-validator/src/rules/ideation_labeling.rs` |
| [`a08`](./workpacks/a08-waiver-honesty-overrides-to-waivers.md) | A | a01, a03 | `crates/enforcer-rules/src/waiver.rs`, `crates/enforcer-rules/waivers.ron` |
| [`b03`](./workpacks/b03-capsule-index-templates.md) | B | arc-20 | `crates/enforcer-plan/templates/*`, `crates/enforcer-plan/src/templates.rs` |
| [`b02`](./workpacks/b02-plan-structure-validator.md) | B | arc-20, arc-05, arc-04 | `crates/enforcer-plan/src/validator.rs`, `crates/enforcer-rules/src/rules/plan.rs` |

## Frontier 1 — unlocks the moment its root is DONE

Do **not** claim these until the named dep row is DONE.

- After **`a01`**: `arc-01` (`enforcer-core`, the first member crate; a01 owns the workspace root), then `a02`–`a07`, `a09`. Early Track F/G leaves ride their host crate rather than `a01` directly: `f03` (project-tie config) once `arc-03` (`enforcer-config`) lands, and `g01` (UI serve surface) once `arc-24` (`enforcer-ui`) lands.
- After **`arc-01`**: `arc-02` (`enforcer-domain`), then the crate fan-out — `arc-03` (config), `arc-04`/`arc-05` (rules/validator), `arc-06`..`arc-13` (per-language + literal-scan, all parallel), `arc-14` (mechanization), `arc-16` (coordination), `arc-18` (harness), and `arc-25` (`enforcer-events`, the VENDORED lean typed event spine) which need only `arc-01`+`arc-02`; converging on the engine crates (`arc-15` scan, `arc-17` proof, `arc-19` security, `arc-20` plan — `arc-15`/`arc-17` also consume the `arc-25` event spine) and the surfaces (`arc-21` mcp, `arc-22` cli, `arc-23` install, `arc-24` ui), each respecting its own deps chain.
- After **`a09`** (and `a01`): `a10` — the CI self-enforce capstone (enforcer's own Rust rules on its own crates).
- After **`d01`**: `d02`,`d03`,`d04`,`d06`,`d07`,`d08`,`d09`,`d11`,`d12`,`d13` (parallel), then `d05` and `d10` (also need `d04`). New D families: `d16`,`d17`,`d18`,`d21`,`d25`,`d26`,`d28` (parallel after `d01`); `d22` (needs `d01`,`d02`); `d23` (needs `d01`,`d16`); `d27` (needs `d01`,`d04`).
- After **`d01`**: `e01` (universal literal-scan floor). After **`d01`+`d16`+`d22`**: `e-pack-dart`,`e-pack-cfml`,`e-pack-frontend-react`,`e-pack-python` (parallel; the dart/cfml/e01 trio append-only-coordinate on the `Tools/ocentra-literal-scan` registry; `e-pack-python` adds the FastAPI layered/clean-arch + Python-security family, consuming d16 enum + d22 size/shape). **OPTIONAL** `e-pack-crypto-blockchain` (opt-in, OFF by default) after **`d01`+`d17`+`d18`+`h01`** — never on the default frontier; enable only when a project opts into crypto; consumes h06 signing + the h07 localnet adapter read-only.
- After **`d01`**: `h01` (money-critical classifier — Track H keystone). After **`h01`** (and `d01`): `h03`,`h05`,`h06` (parallel; each consumes the h01 money-critical manifest read-only); `h02` also needs `d23`. After **`d01`+`d23`**: `h04` (security-test-quality; does not need h01). After **`d01`+`a10`+`c01`**: `h07` (security-tooling CI/observability; rides a10 self-CI + c01 install contract). After **`d01`+`b01`**: `h08` (testing-mandate SKILL + neutral profile + policy-ingestion — ships the missing SKILL/profile/ingest, references h01-h07 rule IDs by string only). After **`d01`+`h03`+`f05`**: `h11` (cyberskills-corpus-to-rust-rules — reimplements the fundamental-logic cyberskills as native Rust rules, seeds the h03 threat vocab, registers a security-audit scope into the f05 router, adds the `vendor/**` dogfood exclusion). After **`d01`+`f05`+`h11`**: `h12` (cyberskills-python-adapters — the OPTIONAL out-of-dogfood python/CLI adapter complement to h11; graceful-skips honestly, feeds a thin severity gate).
- After **`arc-15`+`d01`**: `f01` (scan modes) — early Track F leaf. After **`arc-03`**: `f03` (project-tie config). After **`f03`** (with `arc-15`,`arc-22`): `f02` (onboard, consumes the `.enforce/config` schema); `f05` (detect-and-route router — needs `arc-15`,`arc-13`,`d01`,`f03`; reads the f03 tie config + literal-scan ext registry, emits the ROUTE PLAN `f01`/`c04`/check/scan/run consume). After **`c04`+`f01`**: `f04` (silent-vs-human run context — the gate Track G's UI honors).
- After **`g01`** (the serve surface, dep `arc-24`; must land before the rest of G mounts): `g05`/`g06`/`g08`/`g09` mount their views as their own dependencies land; `g02` follows `f01`; `g03`/`g04` follow their crate and row-surface dependencies; `g07` supplies UI security. All `g02`–`g09` honor `f04` silent mode and remain blocked until the graph marks their declared dependencies DONE.
- After **`c01`**: `c02`,`c06`,`c08` (parallel); then `c03` (needs `c01`,`c02`), `c07` (needs `c01`,`c02`), `c09` (remaining six adapters — needs `c01`,`c02`); then `c04`,`c05` (need `c01`,`c03`).
- After **`x01`**: `x02` (docs refresh — reads product docs to `enforcer` + adds a section per new capability) and `x03` (rename migration — rewrites already-installed `ocentra-enforcer` regs/tool-names to `enforcer`, transitional, no permanent alias); disjoint owns, both parallel-safe once `x01` lands.
- After **`b02`**: `b04`; after **`b01`+`b02`+`b03`**: `b05` (capstone).
- After **ALL tracks (A, C, D, E, B, F, G, H) are DONE**: `z01` (dogfood-proof-gate) — the terminal gate; it RUNS the enforcer against its own multi-language self and its zero-self-violation green authorizes plan-DONE. (The OPTIONAL opt-in `e-pack-crypto-blockchain`, being OFF by default, does not gate z01 unless the plan explicitly opts into crypto.)

## Frontier P — policy / honesty / accuracy gap-fillers (late frontier, fully parallel)

The new **Track P** (`p01`/`p02`/`p03`) addresses three owner-flagged gaps and is claimable only late (its deps land well into the build); all three are disjoint from each other and from the rest of their host tracks. **`p01` choosable-doctrine-profiles** (deps `arc-03`,`arc-04`,`g05`) makes the boundary doctrine — parse-at-boundary, schemas-required, no-raw-boundary-strings, brand-domain-values — LIBRARY-agnostic: it models which library family (Effect/zod/valibot, pydantic/attrs, serde-newtypes) satisfies each universal requirement as a per-project profile, exposes a resolver the library-family rules (`e-pack-frontend-react` `FE-EFFECT-1.1`, `e-pack-python`, `d17`) consult instead of hard-coding Effect, and ships the owner's Effect-only stance as the DEFAULT profile — scanning one codebase under effect-profile vs zod-profile flips findings, and toggles round-trip through config. **`p02` scan-ignore-defaults-and-honesty** (deps `arc-15`,`a09`,`arc-03`,`g02`) gives the Rust scan engine a built-in default ignore set (`node_modules`/`vendor`/`target`/`build`/`dist`/`.git`/harness dot-dirs/fixtures, merged with per-project `ignoreFileGlobs`, overridable) that fixes the 2026-07-12 leaky-ignore lesson (82.8k findings, ~55k noise), plus an anti-silent-skip inventory (reusing a09's `Outcome`/`SkipReason`) of what was NOT scanned and why, surfaced beside the g02 report — so out-of-the-box this repo yields product-only findings AND a visible skip inventory. **`p03` ast-accurate-rule-matching** (deps `arc-07`,`arc-05`,`x06`,`d01`) adds an OPTIONAL, feature-gated (`ast`, OFF by default) AST provider seam backed by `enforcer-memory` (x06) — the first consumer of that crate outside the UI — and migrates the highest-false-positive TS family (`TS-6.3/6.4/6.5` assertions & non-null) off regex onto tree-sitter queries, with fixture parity proving fewer false positives while the regex path stays as the feature-off fallback.

## Recommended global sequence (see PLAN_EXECUTION_BLUEPRINT for the model)

`a01` (Rust toolchain + workspace root) -> arc crate swarm (rooted at `arc-01` enforcer-core, then `arc-02` enforcer-domain the schema keystone, then the fan-out incl. the vendored `arc-25` enforcer-events) -> `x01` (neutral rename, as soon as `arc-21`/`arc-22` lay the mcp/cli name surfaces) + Track A domain packs (`a02`–`a09`, Rust newtypes/boundaries) -> `d01` -> rest of D + all of C + all of E + all of F + all of G + Track H in parallel (within F: `f05` router folds the per-tool check/scan/run surface into one routed call that `f01`/`c04` consume; within G: `g01` serve surface first, then `g02`–`g06`/`g08`/`g09` mount into it, `g07` guards the mutation/dispatch endpoints; within H: `h01` classifier first, then `h02`–`h06` in parallel — `h07` rides `a10`+`c01`, `h08` rides `b01`, the `h11`/`h12` cyber-skills pair rides `f05`) + `x02`/`x03` after `x01` -> Track B -> `a10` (self-enforce capstone) -> `z01` (dogfood-proof-gate) LAST. Both **MCP and CLI are first-class** surfaces of the one binary (neither secondary). The OPTIONAL `e-pack-crypto-blockchain` is opt-in only (after `h01`), never in the default spine.

Rationale: stand up the Rust Cargo workspace (A) so everything else builds on the crates (`a01` root, then `enforcer-core`/`enforcer-domain` first); rename the product to `enforcer` (`x01`) as soon as `arc-21`/`arc-22` lay the mcp/cli name surfaces so later packs cite the new name; land `d01` early because Tracks D, E, and F fan out from it (Track E's language packs also need `d16`+`d22`; Track F's scan modes need `d01`); C, the rest of D, all of E, all of F, and all of G are independent and run in parallel — within G, `g01` (the UI serve surface built on the vendored hub server) must land before `g02`–`g09` mount their views into it, and every G pack honors `f04`'s silent-vs-human gate; B is a self-contained tool track; the CI hard-fail capstone (`a10`) precedes the terminal `z01` dogfood gate, which RUNS the finished enforcer against its own multi-language self and gates plan-DONE on zero self-violations. The self-validation code is authored across packs; the dogfood RUN is the last thing that happens.

## Claiming discipline

Pick exactly one row. Confirm its dep rows are DONE in `WORKPACK_INDEX.md`. Confirm its `owns:` set does not intersect any lane already claimed (`PLAN_EXECUTION_BLUEPRINT.md` -> parallel model). Claim the lane, guard the scope, read only that workpack, produce proof, **commit+push a checkpoint to the lane branch** (EXECUTION_MODEL §2e — no local undo; a step isn't done until its bytes are on the remote), close out. Then return here for the next frontier.

**Frontier X addendum (2026-07-12):** `x08` cross-harness-worklog — owner feature request sparked by a
real practitioner post ("I am running four AI tools in parallel… The tooling multiplied. The tracking
did not."). The enforcer is the one chokepoint every installed harness passes through, so the ledger
(lanes/claims/mail/presence) + d04 telemetry + proof records already contain the unified work trail;
x08 adds the read-model + `enforcer worklog` CLI/MCP/UI surfaces that answer "what did I work on,
where, with which tool" — no new write paths.
