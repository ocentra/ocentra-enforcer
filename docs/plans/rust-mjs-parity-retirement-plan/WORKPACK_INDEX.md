# Workpack Index

| ID | Workpack | Owner | Depends on | Batch | Status |
|---|---|---|---|---|---|
| RM00 | [Authority freeze](workpacks/rm00-authority-freeze.md) | boss/Sol | none | one manifest | ACCEPTED |
| RM01 | [Capability inventory](workpacks/rm01-capability-inventory.md) | boss + Luna audits | RM00 | <=40 rows | READY-AUDIT |
| RM02 | [CLI and scanner oracle](workpacks/rm02-cli-scanner-oracle.md) | Luna read-only | RM01 | <=15 commands | BLOCKED |
| RM03 | [MCP oracle](workpacks/rm03-mcp-oracle.md) | Luna read-only | RM01 | <=10 tools | BLOCKED |
| RM04 | [Config, rule, route, proof oracle](workpacks/rm04-config-rule-proof-oracle.md) | Luna read-only | RM01 | <=25 rules | BLOCKED |
| RM05 | [Coordination oracle](workpacks/rm05-coordination-oracle.md) | Luna read-only | RM01 | one operation | BLOCKED |
| RM06 | [Install and harness oracle](workpacks/rm06-install-harness-oracle.md) | Luna read-only | RM01 | one adapter/action | BLOCKED |
| RM07 | [CI, hook, dogfood oracle](workpacks/rm07-ci-hook-dogfood-oracle.md) | Luna read-only | RM01 | one job path | BLOCKED |
| RM08 | [Gap adjudication](workpacks/rm08-gap-adjudication.md) | boss/Sol | RM02-RM07 | serial | BLOCKED |
| RM09 | [Native core repairs](workpacks/rm09-native-core-repairs.md) | Luna by assignment | RM08 | one family | BLOCKED |
| RM10 | [Native edge repairs](workpacks/rm10-native-edge-repairs.md) | Luna by assignment | RM08 | one adapter/op | BLOCKED |
| RM11 | [Exact-SHA aggregate](workpacks/rm11-exact-sha-aggregate.md) | boss + independent reproducer | RM09-RM10 | one SHA | BLOCKED |
| RM12 | [Cutover rehearsal](workpacks/rm12-cutover-rehearsal.md) | boss/Sol | RM11 | one candidate | BLOCKED |
| RM13 | [Production cutover](workpacks/rm13-production-cutover.md) | boss/Sol | RM12 | one window | BLOCKED |
| RM14 | [Delete-not-merge retirement](workpacks/rm14-delete-not-merge-retirement.md) | boss/Sol | RM13 | one deletion series | BLOCKED |

Only the boss changes status. Luna recommends; workers do not self-promote.
