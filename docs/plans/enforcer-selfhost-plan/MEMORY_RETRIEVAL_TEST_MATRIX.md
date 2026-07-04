# MEMORY RETRIEVAL TEST MATRIX

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `MEMORY_RETRIEVAL_TEST_MATRIX`
> Kind: x06 proof matrix.
> Read when: Closing x06.
> Stop rule: Matrix only.
> Proves: nothing by itself.
> Does not prove: product DONE.
<!-- /agent-capsule -->

Sources: [MEMORY_RETRIEVAL_KG_RAG_MASTER_PLAN](./MEMORY_RETRIEVAL_KG_RAG_MASTER_PLAN.md), [MEMORY_RETRIEVAL_IMPLEMENTATION_PACKS](./MEMORY_RETRIEVAL_IMPLEMENTATION_PACKS.md), [MEMORY_RETRIEVAL_PARITY_HARNESS](./MEMORY_RETRIEVAL_PARITY_HARNESS.md), [MEMORY_RETRIEVAL_QA_BENCHMARKS](./MEMORY_RETRIEVAL_QA_BENCHMARKS.md).

## Required rollup

`proof/memory/x06-feature-parity.json` must list every prefix below with status, test name, artifact path, and failure reason when not green.

| Prefix | Area | Artifact |
|---|---|---|
| STO | store, logs, manifests | `proof/memory/x06-store.json` |
| IDX | incremental indexing and git history | `proof/memory/x06-indexing.json` |
| COD | code graph nodes and edges | `proof/memory/x06-code-graph.json` |
| GPH | graph traversal and architecture | `proof/memory/x06-kg.json` |
| TXT | full-text and exact search | `proof/memory/x06-fulltext.json` |
| VEC | dense vectors and HNSW | `proof/memory/x06-vector.json` |
| RRK | reranking and rank fusion | `proof/memory/x06-reranker.json` |
| SUM | summaries and mind maps | `proof/memory/x06-summaries.json` |
| WVR | background enrichment workers | `proof/memory/x06-weaver.json` |
| MCP | live MCP tools | `proof/memory/x06-mcp.json` |
| CLI | CLI mirror tools | `proof/memory/x06-cli.json` |
| PAR | baseline parity comparison | `proof/memory/x06-kg-parity.json` |
| QA | 100 query benchmark rows | `proof/memory/x06-rag-qa.json` |
| LRN | learning and recurrence curves | `proof/memory/x06-learning-curve.json` |
| FED | sharing and import/export | `proof/memory/x06-federation.json` |
| DIA | diagnostics and resource traces | `proof/memory/x06-diagnostics.json` |
| SEC | policy filters | `proof/memory/x06-policy.json` |
| TOK | token-reduction proof | `proof/memory/x06-token-reduction.json` |
| MOD | local model runtime proof | `proof/memory/x06-models.json` |
| DOG | dogfood self-index proof | `proof/memory/x06-dogfood.json` |

## Required final fields

`allMatrixPrefixesGreen`, `qaRowsTotal`, `qaRowsGreen`, `kgParityComparedAgainstBaseline`, `mcpCliParity`, `localDenseRetrievalPresent`, `localRerankerPresent`, `retrievalImprovementCurvePresent`, `tokenReductionMedianAtLeast10x`, `exactArtifactMismatchCount`, and `externalModelProviderUsed` are mandatory fields.
