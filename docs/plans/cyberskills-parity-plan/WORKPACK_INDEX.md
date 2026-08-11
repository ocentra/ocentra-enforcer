# Workpack Index

Status is routing state only. Proof lives in `TEST_PROOF_EXPECTATIONS.md` and committed artifacts.

| Status | ID | Workpack | Owner class | Depends on | Batch limit | Primary owns |
|---|---|---|---|---|---:|---|
| DONE | CP00 | [Truth ledger](./workpacks/cp00-truth-ledger.md) | Luna audit + boss decision | none | one audit/schema report | `cyberskills-ledger-integrator` only: disposition ledger, gate, plan truth |
| DONE | CP01 | [Existing rule reconciliation](./workpacks/cp01-existing-rule-reconciliation.md) | Luna-safe | CP00 | at most 10 rules; aggregate closure packet required | mappings/evidence only; no parity promotion |
| DONE | CP02 | [Shared syntax consumer adoption](./workpacks/cp02-syntax-module-extraction.md) | Sol/architect | UL02, UL03 | one consumer contract | CyberSkills syntax demand/adoption only |
| DONE | CP03 | [CyberSkills fact demand contract](./workpacks/cp03-syntax-facts-contract.md) | Sol/architect | CP02, UL04 | one approved requirement slice | consumer requirements, never syntax facts |
| DONE | CP04 | [Existing rule syntax pilot](./workpacks/cp04-existing-rule-syntax-pilot.md) | Luna with boss review | CP01, CP03 | exactly 1 rule | one rule + fixtures |
| DONE | CP05 | [Native rule factory](./workpacks/cp05-native-rule-factory.md) | Sol then Luna consumer | CP00, CP03, CP04 | one scaffold | repeatable native packet gate |
| BLOCKED | CP06 | [Security engine consumer contract](./workpacks/cp06-external-engine-module.md) | Sol/consumer architect | CP00, UL07 | one security requirement slice | security demand + conformance fixtures; never generic runner/registry/schema |
| BLOCKED | CP07 | [External engine pilot](./workpacks/cp07-external-engine-pilot.md) | Luna with boss review | CP06 | exactly 1 engine | recorded + live adapter |
| DONE | CP08 | [Corpus decomposition waves](./workpacks/cp08-corpus-decomposition-waves.md) | Luna-safe | CP00, CP01 | 10 skills | ledger components only |
| BLOCKED | CP09 | [Native capability waves](./workpacks/cp09-native-capability-waves.md) | Luna-safe for simple predicates | CP05, CP08 | 1 capability, <=5 skills | rules + evidence |
| BLOCKED | CP10 | [External engine mapping waves](./workpacks/cp10-external-engine-mapping-waves.md) | Luna-safe after engine exists | CP07, CP08 | 1 engine, <=10 skills | mappings + fixtures |
| DONE | CP11 | [Advisory/manual retention](./workpacks/cp11-advisory-manual-retention.md) | Luna-safe | CP08 | 10 skills | retained references + reasons; no native/external promotion |
| BLOCKED | CP12 | [Cross-file graph rules](./workpacks/cp12-cross-file-graph-rules.md) | Sol/architect | CP03, CP08, UL13 | exactly 1 predicate | repository-scoped rule |
| BLOCKED | CP13 | [Closure and dogfood](./workpacks/cp13-closure-dogfood.md) | boss + independent gatekeeper | CP01-CP12 as applicable | terminal | derived closure + exact-SHA proof |

## Wave ordering

1. Truth before new coverage: CP00, then CP01 batches.
2. Adopt shared foundations under architect control: CP02/CP03 after UL02-UL04, then CP06/CP07 after UL07.
3. Decompose corpus in CP08 batches once the schema is stable.
4. Route each component to CP09, CP10, CP11, or CP12.
5. Recompute the frontier after every accepted batch.
6. CP13 runs only when no unexplained component remains.

## Intent packet expansion

`CYBERSKILLS_INTENT_MATRIX.json` is the canonical machine-readable expansion of the index. It covers 816 available IDs exactly once in 34 intent families and derives bounded packet IDs; it does not duplicate 816 Markdown files or copy CP08 component objects.

| Packet route | Owned component | Deterministic limit | Hard dependencies |
|---|---|---:|---|
| `WP/CP09/IF-<family>/B##` | `native-predicate` over typed/static/offline input | at most 5 skills | `WP/CP05`, `WP/CP08` |
| `WP/CP12/IF-<family>/B##` | `native-predicate` requiring repository facts | exactly 1 predicate | `WP/CP03`, `WP/CP08` |
| `WP/CP11/IF-<family>/B##` | `advisory` + `manual` retention | at most 10 skills | `WP/CP08` |

External-engine components remain explicitly blocked/reference-only under the Rust-native product decision. The graph validates family membership, packet cardinality, component ownership, source availability, and protected exclusion before exposing a packet through `graph next`.

Read-only audits with disjoint manifests may run in parallel. Ledger writers serialize through `cyberskills-ledger-integrator`; shared adapter writers serialize through `tool-adapter-integrator`; all other workers submit immutable packets. Only the boss changes this index. A worker reports evidence; it does not promote its own row.
