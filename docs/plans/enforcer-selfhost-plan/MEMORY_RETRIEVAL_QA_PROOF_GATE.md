# x06 Retrieval QA Proof Gate — 100 Required Queries

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `MEMORY_RETRIEVAL_QA_PROOF_GATE`
> Kind: OWNER-DICTATED REQUIRED GATE for x06 retrieval. These 100 rows are the BINDING pass bar — x06 cannot claim DONE while any row lacks a green, machine-recorded result.
> Read when: Implementing X06.9 (parity/benchmark harness), closing any x06 subpack that claims retrieval quality, or auditing fake-green.
> Stop rule: Rows are executable probes, not prose. A row without a fixture query + recorded result is a FAILING row, not a pending one.
> Proves: retrieval quality, when every row's result is recorded in `proof/memory/x06-rag-qa.json` and thresholds hold.
> Does not prove: the QA-101..QA-250 expansion set (MEMORY_RETRIEVAL_QA_BENCHMARKS §2) or longitudinal tiers (§3) — those extend this gate, never replace it.
> Protection: `(owner-set)` lines are OWNERSET-1.1-protected invariants.
<!-- /agent-capsule -->

_Author: owner (sujan.mishra), 2026-07-05, dictated verbatim as "the required gate"._

## Per-row proof requirements (owner-set)

Each row must produce:

- expected node/artifact ids
- actual top-k ids
- Recall@5
- MRR@10
- nDCG@10
- reranker lift
- token reduction estimate
- source refs
- pass/fail verdict

## Minimum pass (owner-set)

```text
Recall@5 >= 0.90
MRR@10  >= 0.80
nDCG@10 >= 0.85
no unauthorized candidate
no untrusted active lesson
no exact-artifact mismatch
```

## 100 required practical QA probes

|     ID | Query / task                                                 | Proof expectation                             |
| -----: | ------------------------------------------------------------ | --------------------------------------------- |
| QA-001 | Find all tests directly connected to this function.          | Returns exact test nodes and file ranges.     |
| QA-002 | Find all indirect tests affected by this function.           | Traverses callers/modules to test graph.      |
| QA-003 | Find every caller of this function.                          | Direct `Calls` edges correct.                 |
| QA-004 | Find every upstream caller of this function.                 | Multi-hop caller graph correct.               |
| QA-005 | Find every downstream function this function calls.          | Callee traversal correct.                     |
| QA-006 | Find every trait/interface implementation for this type.     | `Implements` edges correct.                   |
| QA-007 | Find every type implementing this trait/interface.           | Reverse implementation lookup works.          |
| QA-008 | Find all crates in this repo.                                | Crate/package graph correct.                  |
| QA-009 | Find all modules inside this crate.                          | Module tree correct.                          |
| QA-010 | Find the public API surface of this crate.                   | Exported symbols only.                        |
| QA-011 | Find dead public exports.                                    | Zero-inbound + export status.                 |
| QA-012 | Find unused private functions.                               | No false positives on entrypoints/tests.      |
| QA-013 | Find cyclic dependencies between modules.                    | Cycle path shown.                             |
| QA-014 | Find all imports of this module.                             | Import graph correct.                         |
| QA-015 | Find all files importing this symbol.                        | File + line refs returned.                    |
| QA-016 | Find all routes handled by this module.                      | Route/API graph correct.                      |
| QA-017 | Find the request lifecycle for this route.                   | Route → handler → service → store path.       |
| QA-018 | Find all event producers for this event.                     | `Emits` edges correct.                        |
| QA-019 | Find all event consumers for this event.                     | `ListensOn` edges correct.                    |
| QA-020 | Find the full event flow from producer to consumer.          | Path trace correct.                           |
| QA-021 | Find all config files used by this crate.                    | Config/resource nodes correct.                |
| QA-022 | Find all environment variables read by this code.            | Symbol/text hybrid retrieval.                 |
| QA-023 | Find all database tables touched by this function.           | Data/resource path correct.                   |
| QA-024 | Find all filesystem paths touched by this module.            | Literal + graph retrieval.                    |
| QA-025 | Find all network calls made by this crate.                   | HTTP/API call nodes correct.                  |
| QA-026 | Explain what this crate does.                                | Summary tied to source nodes.                 |
| QA-027 | Generate a repo mind map.                                    | Crates/modules/flows grouped.                 |
| QA-028 | Generate a module mind map.                                  | Key files, symbols, edges.                    |
| QA-029 | Explain how startup works.                                   | Entrypoint path correct.                      |
| QA-030 | Explain how shutdown works.                                  | Shutdown path correct.                        |
| QA-031 | Find initialization order.                                   | Ordered graph path.                           |
| QA-032 | Find where this error type is created.                       | Constructor sites correct.                    |
| QA-033 | Find where this error is handled.                            | Handler/catch/match sites correct.            |
| QA-034 | Find all logs emitted by this module.                        | Log call sites correct.                       |
| QA-035 | Find all telemetry emitted by this feature.                  | Event/metric nodes correct.                   |
| QA-036 | Find all TODO/FIXME/deferred markers in this area.           | Text + path filter correct.                   |
| QA-037 | Find all security-sensitive code paths.                      | Rule/security tags returned.                  |
| QA-038 | Find all auth/permission checks for this route.              | Guard path correct.                           |
| QA-039 | Find routes missing auth checks.                             | Negative graph proof.                         |
| QA-040 | Find code paths touching secrets.                            | Secret/resource refs correct.                 |
| QA-041 | Find all validators for this rule id.                        | Rule → validator edge correct.                |
| QA-042 | Find all fixtures for this validator.                        | Fail/pass fixtures returned.                  |
| QA-043 | Find all docs linked to this rule.                           | Doc anchor refs correct.                      |
| QA-044 | Find all rules affecting this file.                          | File → rule graph correct.                    |
| QA-045 | Find all rules affecting this crate.                         | Crate → rule aggregation.                     |
| QA-046 | Find missing validator for doc claim.                        | Doc-rule parity query.                        |
| QA-047 | Find missing fail fixture for validator.                     | Proof gap detected.                           |
| QA-048 | Find missing pass fixture for validator.                     | Proof gap detected.                           |
| QA-049 | Find all workpacks touching this file.                       | Workpack ownership graph correct.             |
| QA-050 | Find workpack that created this file.                        | Git/workpack/proof linkage.                   |
| QA-051 | Find proof required for this workpack.                       | Workpack → proof row.                         |
| QA-052 | Find missing proof for this workpack.                        | Missing artifact detected.                    |
| QA-053 | Find all DONE claims without proof.                          | Fake-green detection.                         |
| QA-054 | Find all PENDING proof rows.                                 | Proof table query.                            |
| QA-055 | Find all files changed without tests.                        | Git diff → test graph.                        |
| QA-056 | Find tests affected by this git diff.                        | Changed nodes → tests.                        |
| QA-057 | Find rules affected by this git diff.                        | Changed nodes → rules.                        |
| QA-058 | Find workpacks affected by this git diff.                    | Changed nodes → workpacks.                    |
| QA-059 | Find architecture impact of this diff.                       | Impact summary correct.                       |
| QA-060 | Find what changed in this file over time.                    | Git history summary.                          |
| QA-061 | Find commit that introduced this function.                   | Git blame/history correct.                    |
| QA-062 | Find commit that last changed this behavior.                 | Semantic/history lookup.                      |
| QA-063 | Summarize last 20 commits for this module.                   | Git summary tied to files.                    |
| QA-064 | Find files with highest churn.                               | Git metric query.                             |
| QA-065 | Find high-risk hotspots.                                     | Churn + centrality + findings.                |
| QA-066 | Find duplicated logic similar to this function.              | Semantic + clone result.                      |
| QA-067 | Find previous bug similar to this error.                     | Lesson/incident retrieval.                    |
| QA-068 | Find previous fix similar to this change.                    | Procedural memory retrieval.                  |
| QA-069 | Find what fix strategy worked last time.                     | Experience memory with outcome.               |
| QA-070 | Find what fix strategy failed last time.                     | Failed action memory.                         |
| QA-071 | Find lessons related to this workpack.                       | Active x05 lessons returned.                  |
| QA-072 | Find lessons related to this rule.                           | Rule → lesson graph.                          |
| QA-073 | Find lessons related to this file.                           | File → lesson graph.                          |
| QA-074 | Find lessons related to this error.                          | Semantic + incident recall.                   |
| QA-075 | Find stale lessons.                                          | Superseded/low-evidence detected.             |
| QA-076 | Find conflicting lessons.                                    | Contradictory lessons detected.               |
| QA-077 | Find lesson with strongest evidence.                         | Evidence curve ranking.                       |
| QA-078 | Find lesson that reduced recurrence most.                    | Learning curve proof.                         |
| QA-079 | Find lesson that had no effect.                              | No recurrence improvement.                    |
| QA-080 | Find recurring issue after lesson landing.                   | t0/t1/t2 evidence chain.                      |
| QA-081 | Find clean scans after lesson landing.                       | Negative evidence counted.                    |
| QA-082 | Find all observations for this workpack.                     | Observation log query.                        |
| QA-083 | Find all failures for this rule.                             | Observation + rule graph.                     |
| QA-084 | Find all successful fixes for this rule.                     | Procedural memory.                            |
| QA-085 | Find all rejected imported lessons.                          | Federation trust state.                       |
| QA-086 | Find imported lessons not locally validated.                 | Inactive trust filter.                        |
| QA-087 | Find all exact artifacts for this proof.                     | Artifact ids exact.                           |
| QA-088 | Retrieve exact file snippet for this symbol.                 | Path/range/hash exact.                        |
| QA-089 | Retrieve exact proof artifact by id.                         | No similarity substitution.                   |
| QA-090 | Retrieve exact lesson artifact by id.                        | No false artifact match.                      |
| QA-091 | Search semantically for "where retry logic is handled."      | Expected retry nodes top-k.                   |
| QA-092 | Search semantically for "where we prevent silent skip."      | Expected rule/validator top-k.                |
| QA-093 | Search semantically for "how branch protection is enforced." | Expected x04/rule/proof top-k.                |
| QA-094 | Search semantically for "where local models are loaded."     | Expected model runtime nodes.                 |
| QA-095 | Search semantically for "where memory recall is injected."   | Expected c05/x06 seam.                        |
| QA-096 | Return top100 candidates, rerank top50, emit top5.           | Candidate/rerank trace complete.              |
| QA-097 | Prove reranker improved ranking.                             | Positive reranker lift.                       |
| QA-098 | Prove token reduction versus reading files.                  | >=10x median reduction.                       |
| QA-099 | Prove retrieval improved after lessons.                      | Improvement curve present.                    |
| QA-100 | Prove x06 is not fake green.                                 | Parity diff + QA report + proof rollup green. |
