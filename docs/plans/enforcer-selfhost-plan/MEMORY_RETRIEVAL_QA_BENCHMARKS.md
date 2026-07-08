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

## 2.5 Rows QA-101..QA-250 — second tranche (150 rows)

Category-count arithmetic, grep-counted from this file (not estimated). QA-001..QA-100 carry no
category column, so the per-key classification used for the tranche-1 column is listed explicitly
below to make the arithmetic checkable. §2's eight named categories map onto grouping keys as:
Symbol traversal = {Symbol, CodeGraph}; Learning memory = {Lessons, Experience}; Retrieval quality
probes = {Retrieval, Reranking}; Continuous learning = {Learning}; the §2 rows-9-15 per-surface
minimums (10 each) apply per grouping key.

QA-001..QA-100 classification by grouping key (each ID counted exactly once):

- Symbol (12): QA-001, 002, 013, 015, 016, 020, 041, 042, 043, 055, 056, 059
- CodeGraph (9): QA-003, 005, 014, 026, 027, 028, 078, 079, 080
- Architecture (13): QA-006, 007, 033, 034, 039, 057, 058, 062, 090, 091, 092, 093, 096
- Repository (8): QA-004, 009, 010, 018, 019, 037, 060, 061
- GitHistory (4): QA-008, 046, 094, 095
- Lessons (5): QA-012, 021, 022, 023, 048
- Experience (7): QA-049, 050, 051, 070, 087, 088, 089
- Retrieval (17): QA-011, 025, 029, 032, 038, 040, 052, 053, 054, 064, 066, 071, 072, 073, 074, 086, 100
- Reranking (3): QA-031, 065, 067
- TokenReduction (3): QA-030, 069, 099
- Learning (3): QA-068, 097, 098
- Performance (8): QA-044, 045, 047, 075, 076, 077, 081, 082
- Federation (5): QA-024, 063, 083, 084, 085
- MCP (2): QA-017, 036
- CLI (1): QA-035

| Grouping key / §2 category | §2 min | QA-001..100 | QA-101..250 | Combined | Meets min |
|---|---|---|---|---|---|
| Symbol | (in Symbol traversal) | 12 | 12 | 24 | via Symbol traversal |
| CodeGraph | 10 | 9 | 6 | 15 | YES |
| Symbol traversal (Symbol+CodeGraph) | 30 | 21 | 18 | 39 | YES |
| Architecture | 30 | 13 | 17 | 30 | YES |
| Repository | 30 | 8 | 22 | 30 | YES |
| GitHistory | 20 | 4 | 16 | 20 | YES |
| Lessons | (in Learning memory) | 5 | 12 | 17 | via Learning memory |
| Experience | 10 | 7 | 6 | 13 | YES |
| Learning memory (Lessons+Experience) | 30 | 12 | 18 | 30 | YES |
| Retrieval | (in Retrieval quality) | 17 | 14 | 31 | via Retrieval quality |
| Reranking | 10 | 3 | 7 | 10 | YES |
| Retrieval quality (Retrieval+Reranking) | 30 | 20 | 21 | 41 | YES |
| TokenReduction | 10 | 3 | 7 | 10 | YES |
| Learning (continuous learning) | 10 | 3 | 7 | 10 | YES |
| Performance | 10 | 8 | 2 | 10 | YES |
| Federation | 10 | 5 | 5 | 10 | YES |
| MCP | 10 | 2 | 8 | 10 | YES |
| CLI | 10 | 1 | 9 | 10 | YES |
| **TOTAL** | — | **100** | **150** | **250** | — |

Federation rows QA-229..QA-233 anchor to the X06.8 federation subpack surfaces inside x06 scope
(`crates/enforcer-memory/src/federation.rs`, `src/share.rs`: signed personal/team bundles,
zero-trust import, imported-lesson inactive-until-x05-validation, community redaction golden) and
execute against x06's own fixtures. Rows in Retrieval / Reranking / TokenReduction / Learning state
their metric family; results record into `proof/memory/x06-rag-qa.json`.

| ID | Category | User-style query | Required retrieval behavior / proof expectation |
|---|---|---|---|
| QA-101 | Symbol | Find all callers of `enforcer_core::error::Result` type. | Return all consumer crates and modules that alias/use Result; correct usage count >= 20 |
| QA-102 | Symbol | Which functions return `DecodeError` from `enforcer-domain`? | Return constructors and error-mapping sites; link to serde boundaries |
| QA-103 | Symbol | Find trait implementations of `Validator` in the workspace. | Traverse `Validator` trait edges; return each implementing crate/module |
| QA-104 | Symbol | Which functions are called by `enforcer-scan` engine core? | Traversal from `crates/enforcer-scan/src/engine.rs` callee graph |
| QA-105 | Symbol | Find tests that directly instantiate the `RuleId` newtype. | Return test files and assertion paths; exclude doc-comment examples |
| QA-106 | Symbol | What modules export `pub` API in `enforcer-mcp`? | Return public symbols from `router.rs`, `registry.rs`, `tool_surface.rs`; exclude private items |
| QA-107 | Symbol | Which symbols in `enforcer-rules` have zero callers? | Return candidate dead code; rank by visibility (pub highest) |
| QA-108 | Symbol | Find all paths where `RepoRoot` is constructed from user input. | Return boundary parse sites in `enforcer-domain/src/paths.rs` consumers; include error handling |
| QA-109 | Symbol | What are all the generic instantiations of `Result<T>` in the lang crates? | Return per-language crate; group by error variant |
| QA-110 | Symbol | Which workpack first defined the `RuleId` type? | Ownership chain: `enforcer-domain/src/ids.rs` -> workpack anchor a03-branded-ruleid-and-registry.md |
| QA-111 | Symbol | Find every `pub use` statement in workspace crates. | Return barrel re-exports; report forbidden per workspace no-barrel doctrine |
| QA-112 | Symbol | Which type implements the `Sha256` branded newtype contract? | Return struct definition in `enforcer-domain/src/hashes.rs` + TryFrom/Display impls |
| QA-113 | CodeGraph | Find the dependency path from `enforcer-mcp` to `enforcer-core`. | Multi-hop transitive deps; return path length and intermediate crates |
| QA-114 | CodeGraph | Which crates have cyclic dependencies? | Return cycle path with nodes; expected zero cycles per workspace design |
| QA-115 | CodeGraph | Build a module dependency tree for `enforcer-scan`. | Return hierarchy: engine -> modes -> scope -> walk -> router |
| QA-116 | CodeGraph | What is the event flow from `enforcer-scan` to `enforcer-proof`? | Return event types + consumer crates through the arc-25 `enforcer-events` spine |
| QA-117 | CodeGraph | Which modules form the hot path for scan execution? | Traversal: `enforcer-cli/src/commands.rs` -> scan engine -> rule dispatch |
| QA-118 | CodeGraph | Find indirect dependencies on `tokio` across the workspace. | Return crates; rank by depth in dependency tree |
| QA-119 | Architecture | Find every module that violates the rule-owns-fixture invariant. | Scan `crates/enforcer-lang-*/src/rules/*.rs`; each rule must have `tests/fixtures/<rule>/**` |
| QA-120 | Architecture | Which rules lack a corresponding validator? | Traverse rule-id -> validator edge; return missing validators |
| QA-121 | Architecture | Where does `enforcer-coordination` cross into `enforcer-scan` scope? | Return call sites; check intended routing vs accidental coupling |
| QA-122 | Architecture | What is the contract between `enforcer-mcp` `tool_surface.rs` and `enforcer-core::context_budget`? | Return interface definition + test fixture pairs |
| QA-123 | Architecture | Which workpack is responsible for the detect-and-route router? | Link `crates/enforcer-scan/src/router/**` to workpack f05-detect-and-route.md |
| QA-124 | Architecture | Find code paths that should use the event spine but do not. | Search for direct cross-crate calls that bypass arc-25 `enforcer-events` |
| QA-125 | Architecture | Which crate owns the proof-artifact envelope schema? | Return `enforcer-proof/src/envelope.rs` definition + JSON fixture + version record |
| QA-126 | Architecture | What is the intended layering between Track A and Track D? | Return workpack dependency edges from WORKPACK_INDEX.md; arc crates before domain packs |
| QA-127 | Architecture | Find all instances of `enforcer-scan` calling back into `enforcer-cli`. | Return call sites; report as re-entrance anti-pattern (expected zero) |
| QA-128 | Architecture | Which Track C adapters share the same fixture shape? | Return adapter pairs under `crates/enforcer-install/src/adapters/**`; identify duplication |
| QA-129 | Architecture | Find the ownership chain from a rule violation back to a workpack. | Given `TS-1.1`, traverse: rule -> validator -> crate -> workpack (arc-07 / e-pack-frontend-react) |
| QA-130 | Architecture | What is the decision tree for choosing graph vs RAG retrieval? | Return query-routing logic from x06 docs; link to `enforcer-memory/src/retriever.rs` |
| QA-131 | Architecture | Which architecture decision governs the domain newtype pattern? | Return `enforcer-domain` charter (single-source schema, arc-02) + rationale |
| QA-132 | Architecture | Find all places where a Track D pack mechanizes a Track A runtime. | Return d01 engine + parity oracle call sites in `enforcer-mechanization` |
| QA-133 | Architecture | Which crate should own a new NDJSON stream reader? | Expected: `enforcer-core` (owns `ndjson_writer.rs`); return charter evidence from its lib.rs |
| QA-134 | Architecture | Which rule blocks barrel re-export files? | Return `TS-1.1` (TS) + `no_reexports.rs` (`enforcer-lang-rust`) with validators + fixtures |
| QA-135 | Architecture | What proof exists for the c05 Claude SessionStart hook? | Return `proof/install/c05-claude-hook-wiring.json` + c05 workpack proof row |
| QA-136 | Repository | Explain the charter of `enforcer-core` as a shared foundation. | Return crate docs + module list (error, telemetry, redaction, hash_chain) + dependency footprint |
| QA-137 | Repository | What is the public API surface of `enforcer-domain`? | Return exported types across ids/hashes/paths/records/run_record/severity/findings |
| QA-138 | Repository | Find all crates labeled P0/keystone vs P1+. | Return tier labels from WORKPACK_INDEX.md; group by status |
| QA-139 | Repository | Summarize the roles of Track A crates (arc-01..arc-25). | Return crate list + charter line per crate lib.rs |
| QA-140 | Repository | Which crates are marked skeleton-only? | Return crate names + the feature packs that own their feature files |
| QA-141 | Repository | Find all crates that vendor code from OcentraParent. | Return vendoring attribution comments + canonical source paths (enforcer-core lib.rs) |
| QA-142 | Repository | What is the test fixture directory convention? | Return `tests/fixtures/<feature>/**` hierarchy; fixtures per workpack |
| QA-143 | Repository | List the module roots of the single `enforcer-domain` crate. | Return exactly: findings, hashes, ids, paths, records, run_record, severity |
| QA-144 | Repository | Which modules in `enforcer-scan` are public vs private? | Return visibility matrix; engine/modes/scope/walk/router surface |
| QA-145 | Repository | Find all modules using `#[cfg(test)]` item gating. | Return test-only modules; verify fixtures live in separate files |
| QA-146 | Repository | What is the minimum Rust version required by the workspace? | Return `rust-version = "1.82"` from root Cargo.toml |
| QA-147 | Repository | Which crates re-export via `pub use` barrels? | Report as doctrine violations; expected zero |
| QA-148 | Repository | Summarize the purpose of each domain pack a02..a09. | Return branded-type ownership per pack anchor file |
| QA-149 | Repository | Which crates depend on network/async runtime libraries? | Return tokio consumers; identify network boundaries |
| QA-150 | Repository | Find all crates that parse JSON. | Return serde_json imports; check parse-at-boundary pattern (a07) |
| QA-151 | Repository | What schema fields do arc-25 `enforcer-events` records carry? | Return schemaVersion + eventType fields + JSON examples |
| QA-152 | Repository | Which crates forbid unsafe code? | Return workspace lint `unsafe_code = "forbid"`; expected all crates |
| QA-153 | Repository | Which crates own local error types vs delegate to `enforcer-core::error`? | Return per-crate error enum definitions |
| QA-154 | Repository | Give a module map of `crates/enforcer-memory`. | Return: graph, ingest, lesson, recall, record, retriever (+ X06.8 federation/share when landed) |
| QA-155 | Repository | Explain the coverage of `rules/typescript/source.md`. | Return rule list TS-1.1..TS-6.40 + examples + fix recipe sections |
| QA-156 | Repository | Which clippy lints are denied workspace-wide? | Return unwrap_used, expect_used, panic, todo, print_stdout etc from root Cargo.toml |
| QA-157 | Repository | What is the startup order of the `enforcer-cli` binary? | Return `main.rs` -> cli.rs parse -> commands.rs dispatch sequence |
| QA-158 | GitHistory | What is the git history of `enforcer-domain/src/ids.rs`? | Return commits touching that file; identify intent per change |
| QA-159 | GitHistory | Which commit introduced the first workpack anchor document? | Return commit hash + message + workpack id |
| QA-160 | GitHistory | Find the commit that last changed the Track A sequence in PLAN_EXECUTION_BLUEPRINT. | Return hash + message + diff vs prior blueprint |
| QA-161 | GitHistory | What changed in `enforcer-scan/src/engine.rs` over its last 50 commits? | Return intent summary (refactor vs feature vs fix) per commit |
| QA-162 | GitHistory | Which workpack/lane produced commit `e83fee6`? | Given the hash, return the lane/workpack that lists it (lessons ships-via audit) |
| QA-163 | GitHistory | Find the oldest file in the enforcer workspace. | Return file path + creation commit + initial intent |
| QA-164 | GitHistory | What lessons came from the PR that merged `arc-01`? | Return lesson records sharing the merge commit anchor |
| QA-165 | GitHistory | Find commits that touch both a rule file AND its fixtures. | Return parallel change patterns; identify test-driven commits |
| QA-166 | GitHistory | Which files have not changed since the last index baseline? | Return unchanged manifest rows; recommend skip from re-index queue |
| QA-167 | GitHistory | Trace the API evolution of the `RuleId` type across commits. | Return struct changes + trait impl additions per commit |
| QA-168 | GitHistory | What was the intent of the commit that introduced parse-at-boundary? | Return commit message + workpack reference (a07) |
| QA-169 | GitHistory | Find commits that modified a Track D workpack file without test changes. | Return commit hashes; identify risky landings |
| QA-170 | GitHistory | Which files were created in the most recent working session? | Return created-after timestamp; link to commit/lane |
| QA-171 | GitHistory | Find the commit that first defined the proof artifact schema. | Return commit + schema version + breaking changes since |
| QA-172 | GitHistory | What branch/workpack created `tests/fixtures/baseline_ratchet/**`? | Return workpack id (d02) from git blame + workpack file |
| QA-173 | GitHistory | Summarize the last 50 commits touching `crates/enforcer-install`. | Return commit summary tied to c03/c05/c07/c09/c10 workpacks |
| QA-174 | Lessons | Have we solved a domain-type issue before? | Search x05 lessons + incidents for branded-newtype / parse-at-boundary keywords; return active lessons |
| QA-175 | Lessons | What lesson prevented re-export anti-patterns? | Return lesson + evidence chain + outcome (recurrence avoided) |
| QA-176 | Lessons | Which lessons apply to rule-validator parity? | Return active lessons tagged doc-rule-parity; link d09 validator |
| QA-177 | Lessons | Find the strongest-evidence lesson about error handling. | Return lesson + t0 incidents + t1 landing commit + t2 clean-scan count |
| QA-178 | Lessons | Which lessons were superseded by the x06 memory system? | Return deprecated lessons + successor lesson id |
| QA-179 | Lessons | Find lessons that conflict with each other. | Return pairs with contradictory recommendations + resolution state |
| QA-180 | Lessons | Which lesson prevented the most recurrences? | Return lesson id + recurrence-prevention count + confidence |
| QA-181 | Lessons | Find lessons with zero measured effect. | Return lesson id + unchanged-recurrence evidence + review flag |
| QA-182 | Lessons | What are the active lessons about the newtype pattern? | Return lessons + implementation references in `enforcer-domain` + proof fixtures |
| QA-183 | Lessons | Find the lesson explaining why parse-at-boundary is required. | Return lesson + rationale + originating incident chain |
| QA-184 | Lessons | Find all lessons created in the past 30 days. | Return recent lesson records from `enforcer-memory/src/lesson.rs` store; identify emerging patterns |
| QA-185 | Lessons | Find lessons that contradict workspace lint policy. | Return lessons + policy text (root Cargo.toml lints) + resolution needed |
| QA-186 | Experience | What fix strategy worked for parse-at-boundary violations? | Return x05 incident + fix applied + verified outcome |
| QA-187 | Experience | Find previous instances of cyclic dependency issues. | Return incident records + resolution pattern + prevention strategy |
| QA-188 | Experience | Which error type change broke downstream code before? | Return incident + error variant change + dependent crates affected |
| QA-189 | Experience | What strategy worked for standing up a new language crate? | Return prior lang-crate implementation experience + gotchas |
| QA-190 | Experience | What failed strategy should be avoided for new validators? | Return failed attempt + reason + recommended pattern instead |
| QA-191 | Experience | What configuration pattern has worked for multi-harness installs? | Return proven pattern + Track C adapter examples + test fixtures |
| QA-192 | Retrieval | Find rule `TS-1.1` and its enforcement code. | Expected ids: rule doc anchor + validator + fixtures in top-5. Metrics: Recall@5, MRR@10 |
| QA-193 | Retrieval | Fuzzy query "TypeScript rules about exports". | Expected `enforcer-lang-ts` + TS-1.1/TS-6.13 anchors in top-5. Metrics: Recall@5, nDCG@10 |
| QA-194 | Retrieval | Search "how does bounded query context work". | Expected `context_budget.rs` + `tool_surface.rs` top-5. Metrics: Recall@5, MRR@10 |
| QA-195 | Retrieval | Retrieve all validator implementations for a given rule id. | Exact rule -> validator traversal; no semantic substitution. Metrics: exact-match rate, Recall@5 |
| QA-196 | Retrieval | Search "what prevents unwrap in Rust code". | Expected clippy deny config (root Cargo.toml) + d17 error_handling top-5. Metrics: Recall@5, nDCG@10 |
| QA-197 | Retrieval | Retrieve the error handling pattern used in `enforcer-coordination`. | Return error type + conversion sites. Metrics: Recall@5, MRR@10 |
| QA-198 | Retrieval | Search "state machines and transitions". | Expected d16 `fsm.rs` + fixtures top-5. Metrics: Recall@5, nDCG@10 |
| QA-199 | Retrieval | Find all code reading environment variables at startup. | Expected `enforcer-config` env boundary top-5. Metrics: Recall@5, precision@5 |
| QA-200 | Retrieval | Retrieve fixtures for rule `TS-6.1` (no `any`). | Exact fixture files + fail/pass examples. Metrics: exact-match rate, Recall@5 |
| QA-201 | Retrieval | Search "how redaction works". | Expected `enforcer-core/src/redaction.rs` + double-layer docs top-5. Metrics: Recall@5, MRR@10 |
| QA-202 | Retrieval | Retrieve the committed context-budget baseline for the MCP tool surface. | Exact artifact `crates/enforcer-mcp/context-budget-baseline.json`. Metrics: exact-match rate |
| QA-203 | Retrieval | Find code that validates workpack proofs. | Expected `enforcer-proof` harness/claim modules top-5. Metrics: Recall@5, nDCG@10 |
| QA-204 | Retrieval | Retrieve newtype examples from `enforcer-domain`. | Expected ids/hashes/paths modules + tests top-5. Metrics: Recall@5, MRR@10 |
| QA-205 | Retrieval | Retrieve tests exercising the fail-closed parity oracle. | Expected `enforcer-mechanization` parity tests top-5. Metrics: Recall@5, nDCG@10 |
| QA-206 | Reranking | Prove reranker improved ranking for "rule validator mapping". | Before/after nDCG@10 + MRR@10; reranker lift >= 0.05 |
| QA-207 | Reranking | Show reranker lift when a query mixes keywords + graph signals. | Candidate set size + reranked subset + lift@10 recorded |
| QA-208 | Reranking | Measure reranker behavior on exact rule-id lookup (`TS-1.1`). | Exact rule at rank 1; reranker must not displace exact matches; lift n/a recorded |
| QA-209 | Reranking | Find queries where reranking degraded ranking. | Return query + nDCG@10 drop + root cause classification |
| QA-210 | Reranking | Compare graph-only vs semantic-only vs hybrid routing. | nDCG@10 per route on the same query set; hybrid >= max(single) - epsilon |
| QA-211 | Reranking | Prove reranker filters false-positive candidates before context pack. | Candidate id + reranker score + filter reason + prevented wrong-source case |
| QA-212 | Reranking | Measure reranker latency on top-100 candidates. | p50/p95/p99 latency + throughput + lift-per-millisecond recorded |
| QA-213 | TokenReduction | Prove MCP retrieval beats agent-opens-42-files. | Baseline file-read tokens vs top-5 context tokens; ratio >= 10x median |
| QA-214 | TokenReduction | Measure token savings from the KG filter (top-100 -> top-25). | Pre/post filter token counts + files avoided. Metric: token ratio |
| QA-215 | TokenReduction | Measure token savings from reranking (top-25 -> top-5). | Candidate tokens vs final context tokens + reduction %. Metric: token ratio |
| QA-216 | TokenReduction | Find query classes with lowest token reduction (< 5x). | Return query class + cause + routing fix opportunity. Metric: token ratio distribution |
| QA-217 | TokenReduction | Report p95 token savings across the workpack query set. | Token-savings histogram + p95 ratio. Metric: token ratio curve |
| QA-218 | TokenReduction | Report cumulative token savings over 1,000 replayed queries. | Sum with-MCP vs without-MCP + trend. Metric: cumulative token ratio |
| QA-219 | TokenReduction | Measure file-open avoidance from context packing. | Agent-would-open vs MCP-avoided file counts + %. Metric: files-avoided ratio |
| QA-220 | Learning | Show retrieval improvement after 100 lessons on the fixed benchmark. | Recall@5/MRR@10/nDCG@10 before vs after; improvement curve recorded |
| QA-221 | Learning | Show false-positive reduction after lessons teach filter rules. | Hallucination/wrong-source rate before vs after. Metric: rate curve |
| QA-222 | Learning | Plot recall@5 vs lesson count (0/10/100/1,000/10,000). | 5-point learning curve on the deterministic synthetic corpus. Metric: recall curve |
| QA-223 | Learning | Measure how lessons reduce query-routing errors. | Mis-routed query rate before vs after. Metric: routing-accuracy lift |
| QA-224 | Learning | Show token-reduction improvement as lessons accumulate. | Median tokens/query before vs after lessons. Metric: token ratio curve |
| QA-225 | Learning | Measure reranker-lift improvement with lesson accumulation. | Lift@10 before vs after lessons. Metric: lift curve |
| QA-226 | Learning | Prove the learning curve does not regress (ratchet). | Recall@5 latest >= previous over 10 consecutive runs in `proof/memory/x06-rag-qa.json`. Metric: monotonic curve |
| QA-227 | Performance | Compare full index rebuild vs incremental update on the synthetic corpus. | Wall-time ratio recorded to `proof/memory/x06-longitudinal.json`; incremental must win on <5% change sets |
| QA-228 | Performance | Measure retrieval p50/p95 latency at the large synthetic graph tier vs baseline tier. | Latency per tier with monotonic run index; regression vs prior run fails |
| QA-229 | Federation | Import a signed personal bundle fixture. | `enforcer-memory/src/federation.rs` (X06.8) accepts bundle; manifest recorded; trust state = imported-untrusted |
| QA-230 | Federation | Import a bundle with a signature mismatch. | Rejected; rejection reason = signature-mismatch retrievable by bundle id; zero-trust proof |
| QA-231 | Federation | Query active lessons after importing an unvalidated bundle. | Imported lessons stay INACTIVE until x05 local validation; untrusted_active_count = 0 |
| QA-232 | Federation | Export a community share bundle. | `enforcer-memory/src/share.rs` (X06.8) output diffs clean against the redaction golden (no private paths/emails) |
| QA-233 | Federation | Import a checksum-tampered bundle. | Rejected; rejection reason = checksum retrievable by bundle id; no partial import side effects |
| QA-234 | MCP | Which handler serves `ocentra_enforcer_scan`? | Return `enforcer-mcp/src/router.rs` route + scan engine handler + DTO schema |
| QA-235 | MCP | Retrieve the tool schema for `ocentra_enforcer_check`. | Return registry entry (`registry.rs`) + CLI parity link |
| QA-236 | MCP | Ask `ocentra_enforcer_explain` about rule `TS-1.1`. | Returns rule text anchored to `rules/typescript/source.md` |
| QA-237 | MCP | Get proof rows for a workpack via `ocentra_enforcer_proof_status`. | Exact proof table rows for the given workpack id |
| QA-238 | MCP | Retrieve the most recent failing run via `ocentra_enforcer_last_failure`. | Exact run record, no similarity substitution |
| QA-239 | MCP | Request a RoutePlan via `ocentra_enforcer_route` on a mixed TS+Rust fixture. | f05 detect-and-route plan lists both language packs + native tools |
| QA-240 | MCP | Verify every MCP tool description fits the committed context budget. | Measured surface <= `crates/enforcer-mcp/context-budget-baseline.json` baseline (d05 ratchet) |
| QA-241 | MCP | Run `ocentra_enforcer_doctor` and retrieve harness wiring status. | Output matches install doctor fixtures; per-harness wiring rows returned |
| QA-242 | CLI | Run `ocentra-enforcer scan --root <fixture> --languages typescript,common`. | Findings include `TS-1.1` on the barrel-file fixture; exit code per `enforcer-core/src/exit_codes.rs` |
| QA-243 | CLI | Run `ocentra-enforcer run --root <fixture> --tool tsc`. | Harness captures compact tsc diagnostics; run record persisted |
| QA-244 | CLI | Run `ocentra-enforcer runs last-failure`. | Returns the exact last failing run record for the fixture repo |
| QA-245 | CLI | Map the `scan` subcommand to its handler and tests. | clap parser (`enforcer-cli/src/cli.rs`) -> `commands.rs` handler -> tests |
| QA-246 | CLI | Which lifecycle commands exist and where are they implemented? | Return `enforcer-cli/src/lifecycle.rs` + `lifecycle/` module + d06 workpack link |
| QA-247 | CLI | Which adapter does `enforcer install` select for Claude Code? | Return `enforcer-install/src/adapters/claude.rs` + detection evidence + fixtures |
| QA-248 | CLI | Prove CLI/MCP surface parity. | Every CLI subcommand maps to an MCP tool (aliases.rs/name.rs parity table); zero unmapped |
| QA-249 | CLI | Run `enforcer doctor` and compare against doctor fixtures. | Output matches `enforcer-install/src/doctor.rs` fixture expectations |
| QA-250 | CLI | Verify the legacy binary-name migration path. | `enforcer-cli/src/name.rs` + install `migrate_legacy_name.rs` retrieval; old name resolves with migration notice |

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
