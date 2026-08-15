# Workpack Index

| Status | ID | Workpack | Owner class | Depends on | Batch limit | Singleton surface |
|---|---|---|---|---|---:|---|
| READY-AUDIT | UL00 | [Capability truth inventory](./workpacks/ul00-capability-truth-inventory.md) | small-model audit + boss | none | one report/schema | capability schema |
| DESIGN-READY | UL01 | [Shape-driven doctrine](./workpacks/ul01-shape-driven-doctrine.md) | Sol/architect | UL00 | one resolver contract | doctrine profile |
| DECISION-READY | UL02 | [Grammar ownership transfer](./workpacks/ul02-grammar-ownership-transfer.md) | boss + current owner | UL00 | one decision record | grammar/parser ownership |
| BLOCKED | UL03 | [Shared syntax extraction](./workpacks/ul03-shared-syntax-extraction.md) | Sol/architect | UL02 | one crate move | workspace/parser manifests |
| BLOCKED | UL04 | [Facts and parse honesty](./workpacks/ul04-facts-and-parse-honesty.md) | Sol/architect | UL03 | one fact slice | fact domain types |
| BLOCKED | UL05 | [Validator analysis bridge](./workpacks/ul05-validator-analysis-bridge.md) | Sol/architect | UL04 | one compatibility seam | validator/scan dispatch |
| BLOCKED | UL06 | [Canonical language routing](./workpacks/ul06-canonical-language-routing.md) | Sol/architect + integrator | UL03, UL05 | one registry migration | language registry/matrix |
| BLOCKED | UL07 | [Reuse-first tool adapter](./workpacks/ul07-reuse-first-tool-adapter.md) | Sol/architect + tool-adapter integrator | UL00 | one contract + one real-tool pilot | `enforcer-harness` execution/diagnostic contract |
| BLOCKED | UL08 | [Fact-backed rule pilot](./workpacks/ul08-fact-backed-rule-pilot.md) | small-model safe with review | UL05, UL06 | exactly 1 rule | pilot registry row |
| BLOCKED | UL09 | [Schema-framework adapters](./workpacks/ul09-schema-framework-adapters.md) | small-model packets + integrator | UL01, UL04, UL08 | one language + one requirement | framework registry |
| BLOCKED | UL10 | [Existing Dart/CFML/Go routing](./workpacks/ul10-existing-language-routing.md) | small-model packets + integrator | UL06, UL08, UL07 as needed | one language | scan/router registry |
| BLOCKED | UL11 | [Language capability waves](./workpacks/ul11-language-capability-waves.md) | manager, <=3 children | UL06, UL08, UL07 as needed | <=3 disjoint languages | matrix integrator only |
| BLOCKED | UL12 | [Generic fact-rule families](./workpacks/ul12-generic-fact-rule-families.md) | small-model after design | UL08, UL11 evidence | one rule family | rule registry |
| BLOCKED | UL13 | [Graph and semantic providers](./workpacks/ul13-graph-and-semantic-providers.md) | Sol/architect | UL04, UL05, UL06, UL07, UL08 | exactly 1 predicate/provider | graph/semantic provider seam |
| BLOCKED | UL14 | [Closure and exact-SHA dogfood](./workpacks/ul14-closure-and-dogfood.md) | boss + independent gatekeeper | profile-derived UL01-UL13; UL07 always | terminal | closure artifact |

Only the boss changes status. A manager recommends; a worker never promotes its own row.
