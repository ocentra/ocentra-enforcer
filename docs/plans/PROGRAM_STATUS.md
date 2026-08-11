# Program Status Dashboard

This is the boss-owned projection for the four-program Enforcer system. Managers report evidence; only the boss changes this dashboard after independently checking the named artifact or run. A green label here never substitutes for the owning workpack's proof.

## Current program state

| Program | Visible manager | Current legal bundle | State | Terminal dependency |
|---|---|---|---|---|
| CyberSkills parity | `019fc636-24d5-7200-9827-c6af2c5c4bf3` | Cyber-owned graph audit; then approved CP08 decomposition and native dogfood | ACTIVE; CP08 is graph-ready and remains manager-owned | every catalog identity accounted for; every available component has honest disposition and proof |
| Universal language enforcement | `019fc4c6-b2fc-7201-ab87-8a47e5c0a188` | UL00 capability truth audit before any build or cleanup | ACTIVE; manager hold at UL00 | required profile capability rows proved; UL14 exact-SHA closure |
| Rust/MJS parity and retirement | `019fc4c6-b2fb-78b3-985d-d5c235130a6e` | RM02-RM07 bounded oracles; RM08 remains blocked | ACTIVE; RM01 accepted complete-unproved at 837 unique rows, with no parity promotion | native local/MCP/CI/install cutover observed; RM14 delete-not-merge retirement |
| Enforcer self-hosting and native Rust delivery | primary boss / graph control plane | graph frontier a01 and g01; d15 retained as DONE | ACTIVE; a01 and g01 are ready but proof remains pending | z01 terminal exact-SHA dogfood gate and Rust runtime cutover |

## Graph reconciliation snapshot

The validated graph at `807bad778` is the dependency/readiness control plane for this dashboard. It currently imports 118 self-host workpacks and reports 4 ready, 140 blocked, 15 active, 39 done, and 1 planned node. The ready frontier is exactly `CP08`, `UL00`, `a01`, and `g01`; manager-owned Cyber/Universal rows are not acted on by the boss lane.

## Cross-program dependency board

| Foundation | Owner | Consumers | Acceptance condition |
|---|---|---|---|
| Executable graph control plane | primary boss | all programs | `program-graph.mjs validate` passes; next/blocked edges and Markdown/proof surfaces remain reconciled |
| Product doctrine | primary boss | all programs | committed north star and mechanical plan tests |
| Grammar ownership and shared syntax | Universal UL02/UL03 integrators | Universal, CyberSkills, graph/memory | accepted ownership record plus one parity-preserving migrated slice |
| Facts and parse honesty | Universal UL04/UL05 integrators | native rules, CyberSkills, graph | unsupported/malformed/unavailable cannot become clean; validators consume typed analysis |
| Reuse-first tool adapter | Universal UL07 tool-adapter-integrator | language tooling and CyberSkills engines | bounded allowlisted execution, typed availability, normalized diagnostics, same local/CI path |
| Graph provider interface | Universal UL13 integrator | CyberSkills cross-file predicates | bounded provider contract and one proved predicate without persistence coupling |
| Native runtime authority | Rust/MJS RM00/RM08 boss decisions | final cutover | frozen authority manifest and machine-readable equal-or-stricter capability matrix |

## Heartbeat record

For every heartbeat, retain a compact boss record outside this hand-edited table or in the coordination ledger:

```text
time=<utc> integrationSha=<sha>
cyber=<cursor/state> universal=<cursor/state> parity=<cursor/state>
mail=<new/none> conflicts=<count> accepted=<packet/artifact-or-none>
nudge=<task/reason-or-none> next=<one dependency-legal action>
```

## Promotion rules

- `HOLD -> READY`: planning commit is available and every dependency artifact is accepted.
- `READY -> ACTIVE`: manager acknowledges exact bundle/base/owns/gates.
- `ACTIVE -> BLOCKED`: one concrete decision or unavailable prerequisite is recorded with evidence.
- `ACTIVE -> ACCEPTED`: manager submits `DONE` and boss reproduces the decisive gate on the exact head SHA.
- `ACCEPTED -> INTEGRATED`: packet is applied to current `rust-build` and impacted gates pass there.
- `INTEGRATED -> CLOSED`: terminal exact-SHA proof and independent reproduction pass; branch/runtime retirement rules are satisfied.

Managers and workers never self-promote a program or write terminal closure state.
