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

"Ready now" = every dep is DONE **and** the pack's `owns:` set is disjoint from all currently-claimed lanes. Because no workpack is DONE yet, the frontier is exactly the four `deps: none` roots plus the other `deps: none` leaves. With the plan now at **129 workpacks** (Track E new-language packs incl. `e-pack-python` + the OPTIONAL opt-in `e-pack-crypto-blockchain` [6] + the ten new Track D families + Track F scan-surface/onboarding/router [5] + Track G UI layer incl. `g07` UI-security [7] + Track C incl. `c09` remaining-adapters [9] + Track H money-critical & security-testing mandate [8] + the `x01`/`x02`/`x03`/`z01` cross-cutting quartet [4]), `x01` (neutral rename) is a `deps: none` leaf best done EARLY, and `z01` (dogfood-proof-gate) is the terminal gate that only becomes claimable once every other pack is DONE. Track H rides `d01`: `h01` (money-critical classifier) is its keystone — it emits the money-critical manifest `h02`/`h03`/`h05`/`h06` consume — and `h08` ships the previously-missing testing-mandate SKILL + the neutral loadable profile `profiles/money-critical-security.json` + policy-spec ingestion (mechanizing [refs/security-testing-source.md](./refs/security-testing-source.md)). The crypto pack is OPTIONAL/opt-in (OFF by default), never on the default frontier. Tracks F and G ride the foundation: `g01` (UI serve surface, dep only `a01`) and `f01`/`f03` (scan modes / project-tie, deps only `a01`/`d01`) are early frontier leaves; within G, `g01` must land before `g02`–`g07` mount into/guard it. `f05` (detect-and-route) is the **foundational router**: given nothing it detects languages/structure/scope and emits a serializable ROUTE PLAN routing each language to its rule packs AND native tools — `f01`, `f03`, the c04 deny-hook, and the check/scan/run surface all CONSUME `f05` rather than hardcoding a per-tool command.

## Frontier 0 — root keystones (start here; unblock the tracks)

These have `deps: none` and gate the largest downstream fan-out. Land them first.

| Priority | Workpack | Track | Why it's first |
|---|---|---|---|
| P0-a | [`a01`](./workpacks/a01-ts-toolchain-and-build.md) | A00 | Compiler contract; blocks **all** of Track A (60+ packs). |
| P0-b | [`d01`](./workpacks/d01-rule-mechanization-engine.md) | D | Rule scaffolder + parity oracle; keystone for `d02`–`d13`. |
| P0-c | [`c01`](./workpacks/c01-install-core-and-cli-contract.md) | C | Harness-neutral install core; blocks `c02`–`c08`. |
| P0-d | [`b01`](./workpacks/b01-plan-scaffolder.md) | B | Deterministic plan emitter; feeds `b02`/`b05`. |

## Frontier 0b — other `deps: none` leaves (fully parallel, no root needed)

Claimable immediately alongside Frontier 0; they touch disjoint scope.

| Workpack | Track | owns (disjoint) |
|---|---|---|
| [`x01`](./workpacks/x01-neutral-rename.md) | X | rename to `enforcer`: `package.json` (name/bin), `scripts/enforcer.mjs`, `mcp/enforcer-mcp.mjs`, `enforcer.config.json`, MCP fingerprint path list — **do early** so later packs cite the new name |
| [`b02`](./workpacks/b02-plan-structure-validator.md) | B | `src/plan/validator/**`, `src/rules/plan/**` |
| [`b03`](./workpacks/b03-capsule-index-templates.md) | B | `src/plan/templates/*` |
| [`a08`](./workpacks/a08-waiver-honesty-overrides-to-waivers.md) | A | `ocentra-enforcer.config.json` |
| [`d14`](./workpacks/d14-ideation-skills-t3.md) | D | `skills/ideation/**` (T3 content, T1-labeled) |
| [`d15`](./workpacks/d15-readme-research-grounding.md) | D | `docs/research-grounding.md`, `README.md#research-grounding` |

## Frontier 1 — unlocks the moment its root is DONE

Do **not** claim these until the named dep row is DONE.

- After **`a01`**: `a-conv-01` (leaf surface, roots the conversion swarm), then `a02`–`a07`, `a09`, `a-conv-47`, `a-conv-48`; also `f03` (project-tie config) and `g01` (UI serve surface) — early frontier leaves whose only root dep is `a01`.
- After **`a-conv-01`**: the `a-conv-*` cluster fan-out (`a-conv-02` onward), respecting each cluster's own deps chain up to the rollups (`a-conv-10/12/26/40/46`) and the test packs (`a-conv-49/50`).
- After **`a09`** (and `a01`): `a10` — the CI self-enforce capstone.
- After **`d01`**: `d02`,`d03`,`d04`,`d06`,`d07`,`d08`,`d09`,`d11`,`d12`,`d13` (parallel), then `d05` and `d10` (also need `d04`). New D families: `d16`,`d17`,`d18`,`d21`,`d25`,`d26`,`d28` (parallel after `d01`); `d22` (needs `d01`,`d02`); `d23` (needs `d01`,`d16`); `d27` (needs `d01`,`d04`).
- After **`d01`**: `e01` (universal literal-scan floor). After **`d01`+`d16`+`d22`**: `e-pack-dart`,`e-pack-cfml`,`e-pack-frontend-react`,`e-pack-python` (parallel; the dart/cfml/e01 trio append-only-coordinate on the `Tools/ocentra-literal-scan` registry; `e-pack-python` adds the FastAPI layered/clean-arch + Python-security family, consuming d16 enum + d22 size/shape). **OPTIONAL** `e-pack-crypto-blockchain` (opt-in, OFF by default) after **`d01`+`d17`+`d18`+`h01`** — never on the default frontier; enable only when a project opts into crypto; consumes h06 signing + the h07 localnet adapter read-only.
- After **`d01`**: `h01` (money-critical classifier — Track H keystone). After **`h01`** (and `d01`): `h03`,`h05`,`h06` (parallel; each consumes the h01 money-critical manifest read-only); `h02` also needs `d23`. After **`d01`+`d23`**: `h04` (security-test-quality; does not need h01). After **`d01`+`a10`+`c01`**: `h07` (security-tooling CI/observability; rides a10 self-CI + c01 install contract). After **`d01`+`b01`**: `h08` (testing-mandate SKILL + neutral profile + policy-ingestion — ships the missing SKILL/profile/ingest, references h01-h07 rule IDs by string only).
- After **`a01`+`d01`**: `f01` (scan modes) — early Track F leaf. After **`f03`**: `f02` (onboard, consumes the `.enforce/config` schema); `f05` (detect-and-route router — needs `a01`,`d01`,`f03`; reads the f03 tie config + literal-scan ext registry, emits the ROUTE PLAN `f01`/`c04`/check/scan/run consume). After **`c04`+`f01`**: `f04` (silent-vs-human run context — the gate Track G's UI honors).
- After **`g01`** (the serve surface, dep `a01`; must land before the rest of G mounts): `g05` (needs `c01`), `g06` (needs `a-conv-20`,`a-conv-23`), and `g02` (needs `f01`); then onto `g02`'s row surface: `g03` (needs `a08`), `g04` (needs `a-conv-23`,`a-conv-24`); then `g07` UI-security (needs `g01`,`g04`) — guards the g03/g05 mutation endpoints + g04 dispatch. g02/g03/g04 share `.enforce/` scan+waiver/ledger state but own disjoint `src/ui/*` sub-dirs; all g02–g07 honor `f04` silent mode.
- After **`c01`**: `c02`,`c06`,`c08` (parallel); then `c03` (needs `c01`,`c02`), `c07` (needs `c01`,`c02`), `c09` (remaining six adapters — needs `c01`,`c02`); then `c04`,`c05` (need `c01`,`c03`).
- After **`x01`**: `x02` (docs refresh — reads product docs to `enforcer` + adds a section per new capability) and `x03` (rename migration — rewrites already-installed `ocentra-enforcer` regs/tool-names to `enforcer`, transitional, no permanent alias); disjoint owns, both parallel-safe once `x01` lands.
- After **`b02`**: `b04`; after **`b01`+`b02`+`b03`**: `b05` (capstone).
- After **ALL tracks (A, C, D, E, B, F, G, H) are DONE**: `z01` (dogfood-proof-gate) — the terminal gate; it RUNS the enforcer against its own multi-language self and its zero-self-violation green authorizes plan-DONE. (The OPTIONAL opt-in `e-pack-crypto-blockchain`, being OFF by default, does not gate z01 unless the plan explicitly opts into crypto.)

## Recommended global sequence (see PLAN_EXECUTION_BLUEPRINT for the model)

`x01` (neutral rename, early) -> `a01` -> conversion swarm (rooted at `a-conv-01`) -> Track A domain packs (`a02`–`a09`) -> `d01` -> rest of D + all of C + all of E + all of F + all of G + Track H in parallel (within F: `f05` router folds the per-tool check/scan/run surface into one routed call that `f01`/`c04` consume; within G: `g01` serve surface first, then `g02`–`g06` mount into it, `g07` guards the mutation/dispatch endpoints; within H: `h01` classifier first, then `h02`–`h06` in parallel — `h07` rides `a10`+`c01`, `h08` rides `b01`) + `x02`/`x03` after `x01` -> Track B -> `a10` (self-enforce capstone) -> `z01` (dogfood-proof-gate) LAST. The OPTIONAL `e-pack-crypto-blockchain` is opt-in only (after `h01`), never in the default spine.

Rationale: rename the product to `enforcer` first (`x01`, deps none) so later packs cite the new name; get the codebase typed (A) so everything else builds on TS; land `d01` early because Tracks D, E, and F fan out from it (Track E's language packs also need `d16`+`d22`; Track F's scan modes need `d01`); C, the rest of D, all of E, all of F, and all of G are independent and run in parallel — within G, `g01` (the UI serve surface built on the vendored hub server) must land before `g02`–`g06` mount their views into it, and every G pack honors `f04`'s silent-vs-human gate; B is a self-contained tool track; the CI hard-fail capstone (`a10`) precedes the terminal `z01` dogfood gate, which RUNS the finished enforcer against its own multi-language self and gates plan-DONE on zero self-violations. The self-validation code is authored across packs; the dogfood RUN is the last thing that happens.

## Claiming discipline

Pick exactly one row. Confirm its dep rows are DONE in `WORKPACK_INDEX.md`. Confirm its `owns:` set does not intersect any lane already claimed (`PLAN_EXECUTION_BLUEPRINT.md` -> parallel model). Claim the lane, guard the scope, read only that workpack, produce proof, close out. Then return here for the next frontier.
