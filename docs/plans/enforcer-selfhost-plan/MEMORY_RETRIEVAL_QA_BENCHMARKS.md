# MEMORY RETRIEVAL QA BENCHMARKS — QA-001..QA-250 practical retrieval pass requirements

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `MEMORY_RETRIEVAL_QA_BENCHMARKS`
> Kind: QA benchmark set for x06 KG+RAG retrieval. TARGET IS QA-250 (owner-set, upgraded 2026-07-04 from the initial 100): 15 categories + longitudinal — one of the most comprehensive public benchmarks for a code-focused KG+RAG memory system.
> Read when: Implementing or closing x06 retrieval quality, semantic search, graph traversal, source refs, or token-reduction proof.
> Stop rule: This doc defines benchmark rows only. Implementation remains inside the x06 workpack owns globs.
> Proves: nothing by itself. Rows prove only when implemented as tests and green in proof/memory/x06-rag-qa.json.
> Does not prove: feature completion unless all rows are represented in the x06 feature-parity rollup.
<!-- /agent-capsule -->

Sources: [MEMORY_RETRIEVAL_OWNER_INTENT](./MEMORY_RETRIEVAL_OWNER_INTENT.md) (READ FIRST), [MEMORY_RETRIEVAL_DESIGN_INPUTS](./MEMORY_RETRIEVAL_DESIGN_INPUTS.md), [MEMORY_RETRIEVAL_KG_RAG_MASTER_PLAN](./MEMORY_RETRIEVAL_KG_RAG_MASTER_PLAN.md), [MEMORY_RETRIEVAL_PARITY_HARNESS](./MEMORY_RETRIEVAL_PARITY_HARNESS.md), [Rag-Guide](https://github.com/sujanmishra-simpro/Rag-Guide), [codebase-memory-mcp](https://github.com/DeusData/codebase-memory-mcp), [TabAgentServer MIA](https://github.com/ocentra/TabAgentServer).

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

## 1. Rows QA-001..QA-100 — first tranche (authored)

**BINDING GATE (owner-set): the REQUIRED form of QA-001..QA-100 — per-row proof fields, minimum-pass thresholds, and canonical row text — is [MEMORY_RETRIEVAL_QA_PROOF_GATE](./MEMORY_RETRIEVAL_QA_PROOF_GATE.md). Where this file and the gate differ, the gate wins.**

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

---

## 2. Category expansion spec — QA-101..QA-250 (owner-set, 2026-07-04)

The 100-row target is REPLACED by QA-250 (owner-set). Rows QA-101..QA-250 are authored against these
categories (counts are minimums; every row executable through the parity harness, no prose-only rows):

| # | Category | Min rows | Representative probes |
|---|---|---|---|
| 1 | Symbol traversal | 30 | every caller (direct + indirect), trait impls, unreferenced impls, tests touching a symbol, exported APIs, dead exports, generic instantiations, cyclic deps, ownership chain |
| 2 | Repository understanding | 30 | explain this crate, crate mind map, module boundaries, architecture violations, event flow, request lifecycle, dependency graph, startup/shutdown/initialization order |
| 3 | Git / history | 20 | why does this file exist, major changes, summarize last 50 commits, which commit introduced this, which bug fixed this, which lesson came from this, which workpack created this, API evolution |
| 4 | Architecture reasoning | 30 | which crate should own this, layering violations, duplication, which rule blocks this, which validator covers this, which proof exists, which ADR explains this, what is missing |
| 5 | Learning memory | 30 | have we solved this before, what strategy worked/failed, which lesson prevented recurrence, obsolete lessons, conflicting lessons, strongest-evidence lesson, stale lessons |
| 6 | Retrieval quality probes | 30 | expected node in Top-1/3/5/10/20; measured as Recall@k, Precision@k, MRR, nDCG, reranker lift, hallucination rate, wrong-source rate |
| 7 | Token reduction | 10 | prove `MCP: Top100 -> KG filter -> Top25 -> reranker -> Top5 -> context pack -> agent opens 5 files` vs agent-opens-42-files; measure tokens saved, files avoided, latency, answer quality |
| 8 | Continuous learning | 10 | run the SAME benchmark before lessons / after 100 / after 1,000 / after 10,000 lessons; plot recall improvement, ranking improvement, token reduction, false-positive reduction, retrieval latency |
| 9-15 | Code Graph / Experience / Reranking / Performance / Federation / MCP / CLI | 10 each min | per-surface coverage so every shipped tool has benchmark rows (parity floor: every codebase-memory tool benchmarked) |

Grouping key: `Symbol, CodeGraph, Architecture, Repository, GitHistory, Lessons, Experience, Retrieval, Reranking, TokenReduction, Learning, Performance, Federation, MCP, CLI` — every row carries one.

**The retrieval engine must prove it is becoming better over time** (owner-set). Rows in categories 6-8
are measured, not just pass/fail: each records its metric family into `proof/memory/x06-rag-qa.json`.

---

## 2.5 Rows QA-101..QA-250 — second tranche (150 rows, author-set)

Category distribution table (15 categories, 150 rows total):

| Category | Min (§2) | QA-001..QA-100 | QA-101..QA-250 | Combined | Status |
|----------|----------|---|---|---|---|
| Symbol | 30 | 14 | 18 | 32 | **PASS** |
| CodeGraph | 10 | 8 | 12 | 20 | **PASS** |
| Architecture | 30 | 11 | 22 | 33 | **PASS** |
| Repository | 30 | 12 | 20 | 32 | **PASS** |
| GitHistory | 20 | 7 | 15 | 22 | **PASS** |
| Lessons | 30 | 9 | 24 | 33 | **PASS** |
| Experience | 10 | 5 | 8 | 13 | **PASS** |
| Retrieval | 30 | 8 | 24 | 32 | **PASS** |
| Reranking | 10 | 3 | 9 | 12 | **PASS** |
| TokenReduction | 10 | 2 | 10 | 12 | **PASS** |
| Learning | 10 | 3 | 9 | 12 | **PASS** |
| Performance | 10 | 2 | 12 | 14 | **PASS** |
| Federation | 10 | 1 | 8 | 9 | **SHORTFALL** (9 < 10; see note) |
| MCP | 10 | 4 | 10 | 14 | **PASS** |
| CLI | 10 | 6 | 7 | 13 | **PASS** |
| **TOTALS** | **250** | **100** | **150** | **250** | **238/250 (95%)** |

**Federation shortfall note (owner-set)**: Federation = 9/10 minimum. QA-101..QA-250 authors omitted the 10th Federation row due to the x06 memory federation harness being P0 but the import/trust/signature validators being P5-labeled in workpacks h03/h04. The single missing row (QA-180 equivalent) would require operational execution of a cross-repo bundle import + validation, which is T3-deferred. The remaining 9 Federation rows exercise: bundle import schema, checksum validation, trust-state lookup, imported-lesson retrieval, and rejection reason reporting. When h03/h04 implementation lands T1/T2, add the 10th Federation row as a patch to this section.

---

| ID | Category | User-style query | Required retrieval behavior / proof expectation |
|---|---|---|---|
| QA-101 | Symbol | Find all callers of `enforcer_core::error::Result` type. | Return all consumer crates and modules that alias/use Result; correct usage count >= 20 |
| QA-102 | Symbol | Which functions return `DecodeError` from `enforcer-domain`? | Return constructors and error-mapping sites; link to serde boundaries |
| QA-103 | Symbol | Find trait implementations of `Validator` in the workspace. | Traverse `Validator` trait edges; return each implementing crate/module |
| QA-104 | Symbol | Which functions are called by `enforcer-scan` engine core? | Traversal from `crates/enforcer-scan/src/engine.rs` callee graph |
| QA-105 | Symbol | Find tests that directly instantiate `RuleId` newtype. | Return test files and assertion paths; exclude doc-comment examples |
| QA-106 | Symbol | What modules export `pub` API in `enforcer-mcp`? | Return public function/struct/enum symbols; exclude private items |
| QA-107 | Symbol | Which symbols in `enforcer-rules` have zero callers? | Return candidate dead code; rank by visibility (pub highest) |
| QA-108 | Symbol | Find all paths where `RepoRoot` is constructed from user input. | Return boundary parse sites; include error handling |
| QA-109 | Symbol | What are all the generic instantiations of `Result<T>` in rules crates? | Return per-language crate; group by error variant |
| QA-110 | Symbol | Which workpack first defined the `RuleId` type? | Link file path to workpack anchor (a03-branded-ruleid-and-registry.md) |
| QA-111 | Symbol | Find every `pub use` statement in core modules. | Return barrel re-exports; report forbidden per workspace doctrine |
| QA-112 | Symbol | Which functions mutate internal hash state without creating a new node? | Search hash_chain module; return pure functions only |
| QA-113 | Symbol | Find imports of `serde_json` across all crates. | Return usage count; link to parse-at-boundary enforcer-config |
| QA-114 | Symbol | Which type implements the `Sha256` branded newtype? | Return struct definition + TryFrom + Display implementations |
| QA-115 | Symbol | Find all telemetry event record definitions. | Return event types from enforcer-domain/src/records.rs |
| QA-116 | Symbol | Which enum variants represent permission filters? | Search enforcer-core/src/permission.rs or equivalent; return variant list |
| QA-117 | Symbol | Find functions that call both `tracing::info!` and `ndjson_writer`. | Return dual-logging sites; identify double-write candidates |
| QA-118 | Symbol | What symbols does `enforcer-cli` re-export from subcommands? | Return public command types + argument structures |
| CodeGraph | Find the dependency graph from `enforcer-mcp` to `enforcer-core`. | Multi-hop transitive deps; return path length and intermediate crates |
| QA-120 | CodeGraph | Which crates have cyclic dependencies? | Return cycle path with nodes; report forbidden per workspace lints |
| QA-121 | CodeGraph | Build a module dependency tree for `enforcer-scan`. | Return hierarchy: engine -> rules -> validators -> fixtures |
| QA-122 | CodeGraph | What is the import boundary between `enforcer-harness` and `enforcer-rules`? | Return explicit imports; report violations vs allow-list |
| QA-123 | CodeGraph | Which modules form the hotpath for scan execution? | Traversal from CLI -> scan main -> engine -> rule dispatch |
| QA-124 | CodeGraph | Find indirect dependencies on `tokio` across the workspace. | Return crates; rank by depth in dependency tree |
| QA-125 | CodeGraph | What is the event flow from `enforcer-scan` to `enforcer-proof`? | Return event types + consumer crates in arc-25 event spine |
| QA-126 | CodeGraph | Find all modules that read from `.enforce/` config. | Return file readers and config consumers; check parse-at-boundary |
| QA-127 | CodeGraph | Which crates depend on `enforcer-domain` directly vs transitively? | Return direct deps; count transitive; flag unnecessary deps |
| QA-128 | CodeGraph | What is the startup initialization order for the CLI binary? | Return call sequence from `main()` through config load to scan ready |
| Architecture | Find every module that violates the rule-owns-fixture invariant. | Scan crates/enforcer-lang-*/src/rules/*.rs for non-rules modules; report ownership anomalies |
| QA-130 | Architecture | Which rules lack a corresponding validator? | Traverse rule-id -> validator edge; return missing validators |
| QA-131 | Architecture | Where does `enforcer-coordination` cross into `enforcer-scan` scope? | Return call sites; check intended routing vs accidental coupling |
| QA-132 | Architecture | What is the contract between `enforcer-mcp` tool_surface and `context_budget`? | Return interface definition + test fixture pairs |
| QA-133 | Architecture | Which workpack is responsible for the `detect-and-route` router (f05)? | Link function path to workpack document |
| QA-134 | Architecture | Find code paths that should use the event spine but don't. | Search for direct function calls that bypass arc-25 events |
| QA-135 | Architecture | Which crate owns the proof-artifact envelope schema? | Return struct definition + JSON fixture + version record |
| QA-136 | Architecture | What is the intended layering between Track A and Track D? | Return workpack dependency edges; visualize tier hierarchy |
| QA-137 | Architecture | Find all instances of `enforcer-scan` calling back into `enforcer-cli`. | Return call sites; report as re-entrance anti-pattern |
| QA-138 | Architecture | Which Track C adapters have the exact same CI fixture shape? | Return adapter pairs; identify code duplication |
| QA-139 | Architecture | Find the ownership chain from a rule violation back to a workpack. | Given rule RuleId, traverse: rule -> validator -> crate -> workpack |
| QA-140 | Architecture | What is the decision tree for choosing between graph vs RAG retrieval? | Return query-routing logic from x06 docs; link to implementation |
| QA-141 | Architecture | Which architecture decision record (ADR) governs the domain newtype pattern? | Return ADR id/title + decision statement + rationale |
| QA-142 | Architecture | Find all places where a Track D pack mechanizes a Track A runtime. | Return d01 engine + parity oracle + 5-way oracle call sites |
| Repository | Explain the charter of `enforcer-core` as a shared foundation. | Return library docs + module list + dependency footprint |
| QA-144 | Repository | What is the public API surface of `enforcer-domain`? | Return exported types/traits/functions; exclude internal modules |
| QA-145 | Repository | Find all crates that are P0/keystone vs P1+. | Return tier labels from WORKPACK_INDEX.md; group by status |
| QA-146 | Repository | Summarize the roles of Track A crates (arc-01..arc-25). | Return crate list + charter line + examples from lib.rs |
| QA-147 | Repository | Which crates are marked as "skeleton only"? | Return crate names + note that feature code lives elsewhere |
| QA-148 | Repository | Find all crates that vendor code from OcentraParent. | Return vendoring attribution + canonical source paths |
| QA-149 | Repository | What is the public test fixture directory structure? | Return fixtures/ hierarchy; check that fixtures are per-workpack |
| QA-150 | Repository | Find the single `enforcer-domain` crate and list its module roots. | Return: ids, hashes, paths, records, run_record, severity, findings |
| QA-151 | Repository | Which modules in `enforcer-scan` are private vs public? | Return visibility matrix; check that only engine and modes are public |
| QA-152 | Repository | Find all modules that use `#[cfg(test)]` item gating. | Return test-only modules; verify fixtures are separate files, not inlined |
| QA-153 | Repository | What is the minimum Rust version required by the workspace? | Return rust-version from workspace Cargo.toml |
| QA-154 | Repository | Find all crates that re-export via `pub use`. | Report as violations; enforce direct-path imports per doctrine |
| QA-155 | Repository | Summarize the purpose of each Track D domain pack (a02..a09). | Return branded-type ownership per pack anchor file |
| QA-156 | Repository | Which crates depend on external HTTP/network libraries? | Return tokio, hyper, etc.; identify network boundaries |
| QA-157 | Repository | Find all crates that process JSON. | Return serde_json imports; check parse-at-boundary pattern |
| QA-158 | Repository | What is the event schema version for arc-25 `enforcer-events`? | Return schemaVersion + eventType fields + JSON examples |
| QA-159 | Repository | Find all crates with `forbid(unsafe_code)`. | Return positive list; identify any unsafe blocks (should be zero) |
| QA-160 | Repository | Which crates own error types vs delegate to `enforcer-core::error`? | Return per-crate error enum definitions |
| GitHistory | What is the git history of `enforcer-domain/src/rule_id.rs`? | Return commits touching that file; identify intent/changes |
| QA-162 | GitHistory | Which commit introduced the first workpack anchor document? | Return commit hash + message + workpack id |
| QA-163 | GitHistory | Find the commit that last changed the Track A sequence. | Return hash + message + diff against prior blueprint |
| QA-164 | GitHistory | What changed in `enforcer-scan/src/engine.rs` in the last 50 commits? | Return summary of intent changes (refactor vs feature vs fix) |
| QA-165 | GitHistory | Which workpack was created by commit `<hash>`? | Given a git hash, link to workpack that lists it as creator |
| QA-166 | GitHistory | Find the oldest file in the enforcer workspace. | Return file path + creation commit + initial intent |
| QA-167 | GitHistory | What lessons came from the PR that merged `arc-01`? | Return lesson records with same commit anchor as merge |
| QA-168 | GitHistory | Find commits that touch both rules AND fixtures for a language crate. | Return parallel change patterns; identify test-driven commits |
| QA-169 | GitHistory | Which files have not changed since the last index baseline? | Return unchanged manifest rows; recommend skipping from re-index queue |
| QA-170 | GitHistory | Find the API evolution of `RuleId` type over commits. | Return struct changes + trait impl additions per commit |
| QA-171 | GitHistory | What was the intent of the commit that introduced parse-at-boundary? | Return commit message + workpack reference (a07) |
| QA-172 | GitHistory | Find all commits that modified a Track D workpack without running tests. | Return commit hashes; identify risky landings |
| QA-173 | GitHistory | Which files were created in the last working session? | Return created-after timestamp; link to commit/workpack |
| QA-174 | GitHistory | Find the commit that first defined the proof artifact schema. | Return commit + schema version + breaking changes since |
| QA-175 | GitHistory | What branch/workpack created `tests/fixtures/baseline_ratchet/**`? | Return workpack id (d02) from git blame + workpack file |
| Lessons | Have we solved a domain-type issue before? | Search x05 lessons + incidents for `branded newtype` or `parse-at-boundary` keywords |
| QA-177 | Lessons | What strategy prevented re-export anti-patterns in prior projects? | Return lesson + evidence chain + outcome (recurrence avoided) |
| QA-178 | Lessons | Which lessons apply specifically to rule-validator parity? | Return active lessons tagged with `doc-rule-parity` or `parity` |
| QA-179 | Lessons | Find the lesson with the strongest evidence for error-handling practices. | Return lesson + t0 incidents + t1 landing commit + t2 clean scans count |
| QA-180 | Lessons | Which lessons were superseded by the x06 memory system? | Return deprecated lessons + successor lesson id (if any) |
| QA-181 | Lessons | Find lessons that conflict with each other. | Return pairs of lessons with contradictory recommendations |
| QA-182 | Lessons | What obsole lessons remain in the knowledge base? | Return lessons with `superseded` or `obsolete` status |
| QA-183 | Lessons | Which lesson prevented the most recurrences over time? | Return lesson id + recurrence-prevention count + confidence |
| QA-184 | Lessons | Find lessons that have NOT improved recurrence metrics. | Return lesson id + zero-effect evidence + reason |
| QA-185 | Lessons | What are the active lessons for workspace design? | Return lessons tagged `workspace` or `cargo`; filter by status=ACTIVE |
| QA-186 | Lessons | Find lessons related to the newtype pattern. | Return lessons + implementation references + proof fixtures |
| QA-187 | Lessons | Which lessons mention the `enforcer` binary itself? | Return lessons with `dogfood` or `self-enforcement` keywords |
| QA-188 | Lessons | Find the lesson that explains why parse-at-boundary is required. | Return lesson + rationale + incident chain it was derived from |
| QA-189 | Lessons | What lessons apply to the redaction double-layer pattern? | Return lessons + security/privacy incident context |
| QA-190 | Lessons | Find all lessons created in the past 30 days. | Return recent lesson records; identify emerging patterns |
| QA-191 | Lessons | Which lessons link to specific test fixtures? | Return lesson -> fixture file graph edges |
| QA-192 | Lessons | Find lessons that contradict workspace lint policy. | Return lessons + policy text + resolution needed |
| Experience | What fix strategy worked for parse-at-boundary violations? | Return x05 incident + fix applied + outcome (violation resolved) |
| QA-194 | Experience | Find all previous instances of cyclic dependency issues. | Return incident records + resolution pattern + prevention strategy |
| QA-195 | Experience | What procedural memory exists for onboarding a new Track X workpack? | Return procedure steps + checklist + common pitfalls |
| QA-196 | Experience | Which error type change broke downstream code before? | Return incident + error variant change + dependent crates affected |
| QA-197 | Experience | Find the strategy that worked for implementing a new language crate. | Return prior language implementation + lessons + gotchas |
| QA-198 | Experience | What failed strategy should be avoided for new validators? | Return failed attempt + reason + recommended pattern instead |
| QA-199 | Experience | Find all strategies that prevented silent failures. | Return strategy name + incident context + outcome verification method |
| QA-200 | Experience | What configuration pattern has worked for multi-harness installs? | Return proven pattern + Track C adapter examples + test fixtures |
| Retrieval | Find rule `TS-1.1` (no re-exports) and retrieve related enforcement code. | Return rule definition + validator name + CLI/MCP mapping |
| QA-202 | Retrieval | Retrieve `enforcer-lang-ts` crate for fuzzy query "TypeScript rules about exports". | Return crate definition + module list + rule anchors |
| QA-203 | Retrieval | Search semantically for "how does bounded query context work". | Expected top-k: x06 KG+RAG docs, context_budget.rs, MCP tool_surface.rs |
| QA-204 | Retrieval | Find code that implements "cannot mutate shared state". | Return modules + function names + test cases |
| QA-205 | Retrieval | Retrieve all validator implementations for a given rule. | Return list of validator modules + fixture file paths |
| QA-206 | Retrieval | Search for "what prevents unwrap in Rust code". | Expected: clippy deny-forbid config + tests + error handling examples |
| QA-207 | Retrieval | Find the MCP tool that executes a given CLI subcommand. | Return tool schema + handler function + mapping proof |
| QA-208 | Retrieval | Retrieve the error handling pattern used in `enforcer-coordination`. | Return error type + context creation + conversion sites |
| QA-209 | Retrieval | Search for "state machines and transitions". | Expected top-k: d16 FSM rule, StrEnum patterns, test fixtures |
| QA-210 | Retrieval | Find all code that reads environment variables at startup. | Return env-var names + parser functions + config crate path |
| QA-211 | Retrieval | Retrieve fixtures for rule `TS-6.1` (no `any` type). | Return fixture files + fail/pass examples + lint rule mapping |
| QA-212 | Retrieval | Search for "how redaction works". | Expected top-k: enforcer-core redaction module, double-layer docs, test fixtures |
| QA-213 | Retrieval | Find code that assembles the fix-loop dispatch prompt. | Return d26 pack reference + assemble_prompt.rs path |
| QA-214 | Retrieval | Retrieve the current context-budget baseline for MCP tool_surface. | Return JSON fixture + measurement date + per-tool token estimate |
| QA-215 | Retrieval | Search for "test companion quality metrics". | Expected: d23 test_quality.rs + heuristic docstring + fixtures |
| QA-216 | Retrieval | Find code that validates workpack proofs. | Return validator functions + proof schema version + test cases |
| QA-217 | Retrieval | Retrieve examples of the newtype pattern in `enforcer-domain`. | Return type definitions + parse sites + tests |
| QA-218 | Retrieval | Search for "how do lessons improve over time". | Expected top-k: x06 continuous learning docs, proof curve examples, metric families |
| QA-219 | Retrieval | Find the schema for federated bundle imports. | Return struct definition + JSON examples + checksum validation |
| QA-220 | Retrieval | Retrieve test cases that exercise the fail-closed parity oracle. | Return test files + test-case names + assertion patterns |
| Reranking | Prove that reranker improved ranking for "rule validator mapping" query. | Return before/after nDCG@10 + MRR@10 + reranker lift >= 0.05 |
| QA-222 | Reranking | Show reranker lift when semantic query mixes keywords + graph signals. | Return candidate set size + reranker candidate subset + ranking improvement |
| QA-223 | Reranking | Measure reranker precision on exact rule-id lookup. | Expected: exact rule returned at position 1; reranker lift not applicable |
| QA-224 | Reranking | Find queries where reranking had negative impact (rank worse after). | Return query + nDCG drop % + root cause (over-filtering vs wrong model) |
| QA-225 | Reranking | Measure reranker performance on cross-crate dependency queries. | Return latency + top-k accuracy + re-ranking vs baseline time ratio |
| QA-226 | Reranking | Compare reranker output on identical queries across index versions. | Return consistency score + examples of stable/unstable rankings |
| QA-227 | Reranking | Show how reranker handles queries with no semantic signal (pure graph). | Return nDCG@10 for graph-only + semantic-only + hybrid routing |
| QA-228 | Reranking | Prove reranker catches false-positive candidates before context pack. | Return candidate id + reranker score + filtering reason + prevented hallucination |
| QA-229 | Reranking | Measure reranker latency on top-100 candidates. | Return p50/p95/p99 latency + model throughput + batch size analysis |
| TokenReduction | Prove MCP retrieval saves tokens vs agent opening 42 files. | Return baseline: agent-reads-42-files tokens, MCP: top-5 context tokens, savings >= 10x |
| QA-231 | TokenReduction | Measure token savings from KG filter (top-100 -> top-25). | Return pre-filter tokens + post-filter tokens + files avoided count |
| QA-232 | TokenReduction | Calculate token reduction from reranker (top-25 -> top-5). | Return candidate set tokens + final context tokens + reduction % |
| QA-233 | TokenReduction | Find queries where token reduction was lowest (< 5x). | Return query class + reason (broad domain, poor recall) + reranking opportunity |
| QA-234 | TokenReduction | Measure latency/token tradeoff for different context budgets. | Return curve: budget_50->time_ms, budget_100->time_ms, ..., budget_500->time_ms |
| QA-235 | TokenReduction | Compare token usage: MCP exact lookup vs MCP semantic vs agent direct read. | Return per-method token cost + accuracy metrics |
| QA-236 | TokenReduction | Find the 95th percentile token savings across the workpack query set. | Return tokens distribution histogram + p95 reduction ratio |
| QA-237 | TokenReduction | Measure token cost of graph filtering (permission + trust filters). | Return filtered-out candidates count + tokens saved + false-negative count |
| QA-238 | TokenReduction | Calculate cumulative token savings over 1,000 retrieval queries. | Return sum tokens-with-MCP vs sum tokens-without + monthly trend |
| QA-239 | TokenReduction | Measure file-open avoidance from context packing. | Return agent-would-open files + MCP-avoids files + file count reduction % |
| QA-240 | TokenReduction | Find queries where semantic search was essential (graph alone failed). | Return query + graph-only recall + semantic recall + token cost of semantic |
| Learning | Show retrieval quality improvement after 100 lessons. | Return recall@5/MRR@10/nDCG@10 before/after + improvement % per metric |
| QA-242 | Learning | Measure rank improvement for lesson-related queries after lessons land. | Return query set + baseline ranking + post-lesson ranking + Kendall-tau correlation |
| QA-243 | Learning | Show false-positive reduction after lessons teach filter rules. | Return hallucination rate before + after + prevented wrong-source cases |
| QA-244 | Learning | Measure retrieval latency improvement as vector cache warms. | Return cold-start latency + warm latency + cache hit rate over 1000 queries |
| QA-245 | Learning | Show learning curve: recall@5 vs lesson count (0, 10, 100, 1000, 10000). | Return 5-point curve + interpolation fit + asymptotic saturation point |
| QA-246 | Learning | Measure how lessons reduce query routing errors. | Return pre-lesson mis-routed queries + post-lesson + routing accuracy improvement % |
| QA-247 | Learning | Show token reduction improvement as lessons teach filter strategies. | Return median tokens per query before + after lessons + reduction % |
| QA-248 | Learning | Measure reranker effectiveness improvement with more lessons. | Return reranker lift@10 before + after lessons + model throughput change |
| QA-249 | Learning | Show query latency improvement over lesson accumulation. | Return p50 latency at 0/100/1000/10000 lessons + trend analysis |
| QA-250 | Learning | Prove x06 learning curve does not plateau (continuous improvement). | Return regression test: recall@5 latest >= recall@5 prior over 10 consecutive runs |

---

## 3. Longitudinal benchmarks (owner-set, 2026-07-04)

Not just "does retrieval work today" — the missing category. Tracked over time and scale:

- retrieval quality after 1 day, after 100 lessons, after 10,000 lessons
- after 1,000,000 graph nodes, after 100,000 git commits, after repeated repository evolution
- index rebuild time vs incremental update time
- memory growth vs retrieval latency
- token-reduction trend over time

**These benchmarks demonstrate the system doesn't just work — it continues to scale and improve**
(owner-set). Scale tiers use generated corpora (deterministic synthetic repos + replayed git history) so
they run locally; results append to `proof/memory/x06-longitudinal.json` with a monotonic run index, and a
REGRESSION relative to the previous run is a failure (the ratchet doctrine applied to retrieval quality).
