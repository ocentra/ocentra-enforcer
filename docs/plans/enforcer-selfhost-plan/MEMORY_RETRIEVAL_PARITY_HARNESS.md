# MEMORY RETRIEVAL PARITY HARNESS — live KG baseline vs enforcer KG+RAG

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `MEMORY_RETRIEVAL_PARITY_HARNESS`
> Kind: hard proof harness for x06 live MCP/CLI parity and KG+RAG improvement.
> Read when: Implementing x06 live tests, comparing against codebase-memory-mcp, capturing proof logs, or closing x06.
> Stop rule: This doc defines the harness. Implementation remains inside x06 owns globs.
> Proves: nothing by itself. The generated proof artifacts prove the rows.
> Does not prove: x06 DONE unless all required artifacts are green and referenced by the feature-parity rollup.
<!-- /agent-capsule -->

Sources: [MEMORY_RETRIEVAL_KG_RAG_MASTER_PLAN](./MEMORY_RETRIEVAL_KG_RAG_MASTER_PLAN.md), [MEMORY_RETRIEVAL_QA_BENCHMARKS](./MEMORY_RETRIEVAL_QA_BENCHMARKS.md), [codebase-memory-mcp](https://github.com/DeusData/codebase-memory-mcp), [Rag-Guide](https://github.com/sujanmishra-simpro/Rag-Guide), [TabAgentServer MIA](https://github.com/ocentra/TabAgentServer).

---

## 0. Fake-green rule

x06 is fake green if it only proves compile/unit behavior. It must prove live behavior:

```text
same repo fixture
same git commit
same query/tool request
baseline: installed codebase-memory-mcp MCP/CLI
candidate: enforcer memory MCP/CLI from WIP Rust binary
machine-readable diff
proof artifact
```

The candidate must be **identical or better** for each required KG parity tool. Better is allowed only when the diff shows more expected ids, better source refs, stricter trust filtering, lower token cost, stronger rerank quality, or a valid extra KG+RAG/learning result.

---

## 1. Required harness command

x06 must implement a runner equivalent to:

```text
enforcer memory parity-harness \
  --repo <fixture-or-real-repo> \
  --baseline codebase-memory-mcp \
  --candidate enforcer-memory \
  --queryset docs/plans/enforcer-selfhost-plan/MEMORY_RETRIEVAL_QA_BENCHMARKS.md \
  --out proof/memory/x06-parity/
```

The runner must launch/call both systems through live MCP JSON-RPC and CLI mirrors. Raw MCP replay is the source of truth. Agent transcripts from Claude/Codex are additional proof, not a replacement for raw tool logs.

---

## 2. Required captured artifacts

```text
proof/memory/x06-parity/baseline-index.json
proof/memory/x06-parity/candidate-index.json
proof/memory/x06-parity/tool-results.ndjson
proof/memory/x06-parity/tool-diffs.ndjson
proof/memory/x06-parity/agent-transcripts.ndjson
proof/memory/x06-parity/quality-metrics.json
proof/memory/x06-parity/token-reduction.json
proof/memory/x06-parity/feature-parity-summary.json
```

Each tool result must include:

```text
toolName, queryId, repo, gitHead, request, normalizedResponse,
expectedIds, actualIds, missingIds, extraIds,
sourceRefs, scoreFamilies, rerankerScores,
latencyMs, tokenEstimate, capabilityState,
comparisonVerdict, betterBecause, worseBecause
```

---

## 3. Required tool parity list

| Tool class | Baseline expectation | Candidate expectation |
|---|---|---|
| project index | same repo/head indexed | same or richer manifest with memory tiers |
| list projects | same project visible | same project + x06 capability state |
| graph search | same required nodes/edges | same or superset with provenance |
| graph query | same structural answer | same or stricter safe query behavior |
| code search | same exact snippets | same or better path/symbol refs |
| semantic query | same expected semantic ids | same or better nDCG/MRR |
| snippet get | byte-exact path/range | byte-exact, hash-verified |
| trace path | same graph path where applicable | same or better explained path |
| detect changes | same changed files/symbols | same plus stale-index tasks |
| impact | same affected symbols | same plus rules/workpacks/proofs |
| architecture | same summary fields | same plus repo mind map and proof gaps |
| ADR memory | same add/search behavior | same plus graph links |
| watcher | same incremental update class | same plus hot/warm/cold queue state |
| diagnostics | same or better resource trace | no raw private text |

---

## 4. KG+RAG extra proof

Beyond parity, the candidate must prove features the KG baseline does not own:

- 100 NLP QA rows green;
- dense retrieval over code, lessons, artifacts, summaries;
- local reranker lift;
- repo mind map generation;
- git-history summary chunks linked to files;
- hot/warm/cold memory route selection;
- procedural memory from prior failures/successes;
- learning curve over recurrence and clean scans;
- bounded context packs with token-reduction proof;
- x05 lesson activation and federation trust gates.

---

## 5. Conscious harness behavior

The harness observes its own retrieval runs. Every run becomes procedural memory:

```text
query -> route -> baseline result -> candidate result -> diff -> verdict -> lesson/procedure candidate
```

If a row regresses, the harness must record:

- whether the failure was parsing, graph edge, full-text, embedding, reranker, filter, context packing, stale index, or model runtime;
- whether a new x05 lesson candidate should be proposed;
- whether a background enrichment task should be queued.

A later run must show the improvement curve. One green run is not enough if no curve exists.

---

## 6. Non-acceptance cases

The harness fails if:

- the baseline was not executed live;
- the candidate was not executed through MCP and CLI;
- a tool result is a stub or placeholder;
- semantic rows are green without dense retrieval and reranker evidence;
- exact rows return similar-but-not-exact artifacts;
- unauthorized or untrusted candidates enter rerank/context/logs;
- proof artifacts omit raw request/normalized response/diff;
- agent transcript exists but raw MCP replay is missing;
- `feature-parity-summary.json` does not account for every QA row and required tool class.
