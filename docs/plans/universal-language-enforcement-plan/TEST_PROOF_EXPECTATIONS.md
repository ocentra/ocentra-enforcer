# Test and Proof Expectations

| Workpack | Minimum inner gate | Required proof |
|---|---|---|
| UL00 | capability inventory test | generated counts and drift report from live registries |
| UL01 | config/domain resolver tests | same shape accepted/rejected under different profiles; malformed profile fails typed |
| UL02 | plan/decision validation | exact base SHA, freeze interval, prior/post owner, sequencing decision |
| UL03 | syntax + memory parity tests | all existing language fixtures unchanged; memory consumes shared syntax |
| UL04 | syntax fact contract tests | positive/negative/malformed/unsafe input plus parse-quality/provenance |
| UL05 | validator + scan tests | parse once; legacy raw validators unchanged; missing fact is visible |
| UL06 | registry/router/MCP schema tests | all registries derived; no identity silently falls to `Other` |
| UL07 | harness/domain adapter-contract tests | allowlisted bounded execution; missing/version/malformed/timeout are typed and required tools do not pass |
| UL08 | selected language crate + scan integration | same true positives, fewer false positives, honest unavailable analysis |
| UL09 | config + adapter fixture tests | recognized family separated from active-profile verdict and reuse decision recorded |
| UL10 | language crate + routed scan/MCP | one Dart, CFML, or Go route reaches validators/tool or an explicit unsupported/unavailable result |
| UL11 | language fact contract | one row per language with four fixture classes and no unsupported-as-clean |
| UL12 | rule registry + fixture parity | one capability-driven family across proved languages only |
| UL13 | graph/provider contract | typed bounded input/output and one exact predicate; no persistence coupling |
| UL14 | strict verify + CI + dogfood | `proof/universal-language/ul14/closure.json` and independent reproduction bind identical integration/tree/CI/artifact SHAs |

Every implementation packet also runs Enforcer route before edits, files/crate scan after edits, `git diff --check`, operation-aware guard, and the smallest compiler/linter gate for the changed surface.
