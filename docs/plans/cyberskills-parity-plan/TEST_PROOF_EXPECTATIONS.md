# Test and Proof Expectations

`PENDING` means no completion claim. Replace it only with exact command, run ID, artifact, exit code, commit SHA, and date.

| Workpack | Required proof | Required gates | State |
|---|---|---|---|
| CP00 | 817 identities reconcile as 816 readable + one `sourceUnavailable` tracked blob `df48fa4149dd25956e730443d3582693a3f825a8`; unavailable cannot increase coverage; schema rejects missing/duplicate/malformed components | disposition focused test; plan test; enforcer-rules test; diff checks | PENDING |
| CP01 | every accepted existing mapping has fingerprint, anchors, precise predicate, fail/pass evidence, and `notProved` | rule registry + relevant rule tests; disposition gate | PENDING |
| CP02 | accepted UL02 ownership and UL03 extraction evidence; CyberSkills consumes the shared interface with no parser/grammar ownership or duplication | consumer adoption test; dependency-policy; clippy/fmt | PENDING |
| CP03 | accepted UL04 fact capability evidence; required facts and unsupported/invalid/resource-limit outcomes are visible to CyberSkills consumers | consumer contract tests; parser parity; mutation-risk | PENDING |
| CP04 | one existing text rule has differential old/new corpus with fewer false positives and no lost true positives | focused rule test; lang-security crate; syntax tests | PENDING |
| CP05 | generated native packet fails when any mapping/doc/fixture/validator link is absent | factory contract test; rule parity; plan validation | PENDING |
| CP06 | CyberSkills security requirements and conformance fixtures consume the accepted Universal UL07 `enforcer-harness` contract; no second generic runner/registry/schema exists | consumer conformance tests; UL07 acceptance artifact; plan test | PENDING |
| CP07 | one real engine and recorded adapter normalize identical fixture output; unavailable is not pass | focused live/recorded tests; harness crate; optional CI proof | PENDING |
| CP08 | exactly 10 source fingerprints retained and each proposed component schema-valid; no enforcement inflation | disposition gate; derived count check | PENDING |
| CP09 | one capability with <=5 skills; each native predicate has positive/negative/malformed/boundary fixtures | focused rules; changed crates; disposition gate | PENDING |
| CP10 | one engine with <=10 skill mappings; per-component coverage and recorded evidence | adapter tests; disposition gate | PENDING |
| CP11 | 10 skills retain advisory/manual content and mechanization reason; no item disappears | retention test; disposition gate | PENDING |
| CP12 | one repository predicate proves file-local insufficiency, bounded traversal, cycles, ambiguity, and limits | memory graph + repository validator tests; mutation-risk | PENDING |
| CP13 | `proof/cyberskills/cp13/closure.json` binds integration/tree/base SHA, source identities including `sourceUnavailable`, policy/tool digests, run IDs, artifact hashes, same-SHA substantive CI, built CLI/MCP evidence, and independent clean-worktree reproduction | full strict verify, mutation-risk, workspace tests, clippy/fmt/deny/audit, MCP/CLI smoke | PENDING |

No row may cite a docs-only CI job as source proof.
