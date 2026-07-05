# MEMORY RETRIEVAL QA BENCHMARKS — 100 practical retrieval pass requirements

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `MEMORY_RETRIEVAL_QA_BENCHMARKS`
> Kind: QA benchmark set for x06 KG+RAG retrieval.
> Read when: Implementing or closing x06 retrieval quality, semantic search, graph traversal, source refs, or token-reduction proof.
> Stop rule: This doc defines benchmark rows only. Implementation remains inside the x06 workpack owns globs.
> Proves: nothing by itself. Rows prove only when implemented as tests and green in proof/memory/x06-rag-qa.json.
> Does not prove: feature completion unless all rows are represented in the x06 feature-parity rollup.
<!-- /agent-capsule -->

Sources: [MEMORY_RETRIEVAL_KG_RAG_MASTER_PLAN](./MEMORY_RETRIEVAL_KG_RAG_MASTER_PLAN.md), [MEMORY_RETRIEVAL_PARITY_HARNESS](./MEMORY_RETRIEVAL_PARITY_HARNESS.md), [Rag-Guide](https://github.com/sujanmishra-simpro/Rag-Guide), [codebase-memory-mcp](https://github.com/DeusData/codebase-memory-mcp), [TabAgentServer MIA](https://github.com/ocentra/TabAgentServer).

---

## 0. Global scoring gate

Every row below must map to a fixture query with expected ids. x06 fails if a row has no test.

Required aggregate thresholds:

```text
hit_at_1 >= 0.75
recall_at_5 >= 0.90
mrr_at_10 >= 0.80
ndcg_at_10 >= 0.85
reranker_lift_at_10 >= 0.05 on semantic rows
unauthorized_candidate_count = 0
untrusted_active_count = 0
exact_artifact_mismatch_count = 0
token_reduction_vs_file_reading_median >= 10x
```

Each row must record: query, route, expected ids, retrieved ids, score families, reranker scores, source refs, latency, token estimate, capability state, and failure explanation.

---

## 1. 100 practical Q&A rows

| ID | User-style query | Required retrieval behavior |
|---|---|---|
| QA-001 | Find all tests connected to function `validate_rule_record` | Return direct tests, fixture files, and graph path from function to tests. |
| QA-002 | Who calls this function in this file? | Return callers, caller files, line ranges, and call edges. |
| QA-003 | What upstream modules depend on this crate? | Traverse reverse dependency edges and rank by impact. |
| QA-004 | What crates are in this repository? | Return crate/package map with Cargo manifests and paths. |
| QA-005 | Show functions that call this endpoint handler | Follow route -> handler -> callers. |
| QA-006 | Find every rule that touches this validator | Return rule ids, validator symbols, fixtures, doc anchors. |
| QA-007 | Which workpack owns this file? | Link file path to plan workpack ownership row. |
| QA-008 | What changed around this function in git history? | Return commit summaries, changed lines, and affected graph nodes. |
| QA-009 | Give me a mind map of this crate | Return modules, public symbols, tests, dependencies, hotspots. |
| QA-010 | Summarize this repo for an agent entering cold | Return bounded repo summary tied to source graph nodes. |
| QA-011 | Find similar code to this function | Dense + near-clone retrieval with exact source refs. |
| QA-012 | Find semantically related lessons to this compiler error | Retrieve x05 lessons + incidents + proof refs. |
| QA-013 | Which files import this module? | Return import edges and exact import snippets. |
| QA-014 | What tests fail if this rule changes? | Impact analysis over rule -> validator -> tests. |
| QA-015 | Find dead code candidates in this crate | Return zero-inbound symbols excluding configured entry points. |
| QA-016 | Find all public API surfaces in this crate | Return public functions/types/routes/CLI/MCP tools. |
| QA-017 | Which MCP tool maps to this CLI command? | Return MCP/CLI parity link and DTO schema refs. |
| QA-018 | Find code that writes proof artifacts | Return symbols, paths, write calls, proof schema links. |
| QA-019 | Find code that reads `.enforce` memory | Return memory store accessors and callers. |
| QA-020 | Find all uses of this branded id type | Return definitions, constructors, parse boundaries, consumers. |
| QA-021 | Which lessons mention stale-base reset? | Hybrid retrieve active and superseded lessons with status. |
| QA-022 | What is the evidence chain for lesson L? | Return t0 incidents, t1 landing refs, t2 recurrence stats. |
| QA-023 | What changed after this lesson landed? | Return post-landing clean/recurrence observations. |
| QA-024 | Which imported lessons are inactive and why? | Return trust state, source bundle, missing validation reason. |
| QA-025 | Which artifact backs this lesson? | Exact artifact retrieval, not semantic substitute. |
| QA-026 | Find all code paths that emit this event | Follow event producer edges. |
| QA-027 | Find all listeners for this event | Follow event consumer edges. |
| QA-028 | Which route calls this internal function? | Cross route/function graph traversal. |
| QA-029 | What files should I read before editing this file? | Rank neighboring graph nodes and active lessons. |
| QA-030 | What is the smallest context pack for this task? | Return budgeted evidence pack with token estimate. |
| QA-031 | Explain why this candidate was selected | Return score family, graph path, reranker reason, filters. |
| QA-032 | Explain why this candidate was excluded | Return permission/trust/repo/time/filter reason. |
| QA-033 | Find code related to no-unwrap policy | Retrieve validators, clippy config, docs, fixtures. |
| QA-034 | Find all fixtures for this rule id | Exact rule -> fixture traversal. |
| QA-035 | Find implementation and tests for this CLI subcommand | CLI parser -> handler -> tests. |
| QA-036 | Find implementation and tests for this MCP tool | MCP schema -> handler -> tests. |
| QA-037 | Which modules perform redaction? | Return redaction code, call sites, tests, policy docs. |
| QA-038 | Find possible source of this diagnostic message | Search literals + graph callers. |
| QA-039 | Which crate owns branch protection verification? | Workpack/crate/symbol lookup. |
| QA-040 | Find all state machines in this repo | Semantic + structural search for FSM patterns. |
| QA-041 | Which functions mutate the coordination ledger? | Symbol/data-flow search with source refs. |
| QA-042 | Which code reads NDJSON logs? | Search parser/reader symbols and callers. |
| QA-043 | Which code appends NDJSON logs? | Search append APIs and graph callers. |
| QA-044 | Which source files have not changed since last index? | Return unchanged manifest rows and skip proof. |
| QA-045 | Which files changed and need reindex? | Return changed hashes, commits, index tasks. |
| QA-046 | What history should be kept for changed files? | Return commit count, last commit, prior chunk ids. |
| QA-047 | Find old summaries invalidated by this commit | Return stale summary chunks and rebuild tasks. |
| QA-048 | Find lessons that apply to this changed file | Retrieve active lessons by file/module/rule context. |
| QA-049 | What is the hot memory for current task? | Return current branch/workpack/recent observations. |
| QA-050 | What is warm memory for this repo? | Return active graph/lessons/recent incidents. |
| QA-051 | What is cold memory for this repo? | Return older history/imported/archive with trust state. |
| QA-052 | Find semantic matches for this vague bug report | Dense search + rerank over code, incidents, lessons. |
| QA-053 | Find exact matches for this rule id | Exact path, no semantic substitution. |
| QA-054 | Find all mentions of this path | Full-text + graph references. |
| QA-055 | Which tests cover this error variant? | Error enum -> matches -> tests. |
| QA-056 | Which modules import this error type? | Reverse import graph. |
| QA-057 | What is the architecture of Track A crates? | Crate graph summary with dependencies. |
| QA-058 | What is the architecture of memory crate? | Module map, storage/search/model/MCP layers. |
| QA-059 | Which functions create model manifests? | Search model artifact paths and constructors. |
| QA-060 | Which code loads local models? | Model runtime path + tests + backend info. |
| QA-061 | Which backend should run on Intel GPU/NPU? | Retrieve runtime capability docs and config. |
| QA-062 | Find all code that must not call remote models | Search model policy and tests. |
| QA-063 | Find all trust-filter code paths | Trust filters + callers + tests. |
| QA-064 | Find all permission-filter code paths | Permission filters before retrieval/rerank. |
| QA-065 | Which candidate reached reranker in this trace? | Trace query by id and list candidate ids. |
| QA-066 | Did unauthorized evidence enter context? | Return zero-count proof or failing trace. |
| QA-067 | Show reranker lift for this query class | Return before/after nDCG/MRR metrics. |
| QA-068 | Show retrieval improvement over last 10 runs | Return curve by date/run/index version. |
| QA-069 | Show token reduction from MCP vs file reading | Baseline estimate vs context pack estimate. |
| QA-070 | Show query routes that failed recently | Procedural memory of failures and causes. |
| QA-071 | What route should this query use? | Meta-memory route prediction with confidence. |
| QA-072 | Which queries should use graph only? | Return structured route decision examples. |
| QA-073 | Which queries should use RAG? | Return fuzzy/evidence route examples. |
| QA-074 | Which queries should use exact artifact retrieval? | Return exact lookup route examples. |
| QA-075 | Which queries need background enrichment? | Return queued summary/embedding/link tasks. |
| QA-076 | Which files are queued for embedding? | Return queue state and priorities. |
| QA-077 | Which summaries are queued? | Return summarizer backlog and source refs. |
| QA-078 | Which associative links were added recently? | Weaver-style enrichment audit. |
| QA-079 | Which entity links were added recently? | Entity linker audit with confidence. |
| QA-080 | Which graph edges were inferred vs parsed? | Edge provenance split. |
| QA-081 | Which code nodes are stale? | Manifest high-watermark stale check. |
| QA-082 | Which vector indexes are stale? | Model/chunker/log hash mismatch check. |
| QA-083 | Which bundles were imported? | Federation manifest/trust query. |
| QA-084 | Which imported bundle is rejected? | Signature/checksum reason. |
| QA-085 | Which community export fields were redacted? | Redaction report with golden check. |
| QA-086 | Find all paths where artifact get can fail | Exact retrieval error cases. |
| QA-087 | Find similar prior fixes for this error | Procedural memory + code/lesson retrieval. |
| QA-088 | What fix strategy worked last time? | Experience memory with evidence and outcome. |
| QA-089 | What strategy failed last time? | Failed action memory with reason. |
| QA-090 | Find all skipped tasks labeled deferred | Graph/table query over workpacks/proofs. |
| QA-091 | What proof is missing for this workpack? | Workpack -> proof rows -> status. |
| QA-092 | Which doc claim lacks a validator? | Doc-rule parity query. |
| QA-093 | Which code files exceed size caps? | Rule/scan results with paths. |
| QA-094 | Which code changed without tests? | Git diff -> tests mapping. |
| QA-095 | Which files are high-risk hotspots? | Change frequency + graph centrality + findings. |
| QA-096 | Which module has most violations? | Aggregate graph/finding query. |
| QA-097 | Which lesson reduced recurrence most? | Learning curve by lesson id. |
| QA-098 | Which lesson had no effect? | Lesson with unchanged recurrence. |
| QA-099 | What should Claude read before this workpack? | Bounded recall pack from KG+RAG. |
| QA-100 | Prove x06 is not fake green | Return all proof artifacts, matrix coverage, parity diff, QA score report. |
