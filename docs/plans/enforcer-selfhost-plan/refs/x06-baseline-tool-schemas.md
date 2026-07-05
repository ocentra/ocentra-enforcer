# x06 Baseline Tool Schemas — exact wire contracts extracted from codebase-memory-mcp (C source)

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `x06-baseline-tool-schemas`
> Kind: baseline-schema extraction (sonnet-tier, 2026-07-05). Ground-truth wire contracts for the 14 MCP
> tools codebase-memory-mcp exposes, extracted directly from its pure-C source (shallow clone of
> `DeusData/codebase-memory-mcp`, tag/HEAD `0.8.1` per `server.json`), so enforcer-memory implementers stop
> guessing param names, defaults, response shapes, and algorithmic constants.
> Read when: implementing any X06.2-X06.9 MCP tool handler, building the parity harness in
> `MEMORY_RETRIEVAL_PARITY_HARNESS.md`, or writing a Rust struct/serializer that must byte-match a
> codebase-memory-mcp response shape.
> Stop rule: this doc describes the BASELINE (C) system only. It does not prescribe what enforcer-memory
> must do — see `MEMORY_RETRIEVAL_OWNER_INTENT.md` / `MEMORY_RETRIEVAL_DECISIONS.md` for binding choices,
> and `x06-source-scout-digests.md` §1 for the high-level parity floor. Where this doc's fine-grained
> findings correct or refine that digest (e.g. semantic search DOES use a bundled neural embedding, not
> pure-lexical; BM25's true default limit is 100 not 200), the correction is called out inline.
> Proves: nothing by itself — this is a reference extraction, not a proof artifact. Verify at point of
> copy; every claim below is cited to a `file:line` in the cloned C source. Anything not independently
> re-derivable from source in the time available is marked **UNVERIFIED**.
<!-- /agent-capsule -->

## 0. Source and method

- Source: shallow clone of `https://github.com/DeusData/codebase-memory-mcp` (pure C, ~67.5K LOC per prior
  digest; version `0.8.1` per `server.json`), cloned read-only to a scratch directory outside this repo.
- All `file:line` citations below refer to paths inside that clone, e.g. `src/mcp/mcp.c:339`.
- Method: direct reading of the tool-registration/schema-literal sites, the handler functions, and the
  JSON-response-building code in `src/mcp/mcp.c` (6226 lines — every one of the 14 tools is registered and
  dispatched here), cross-referenced against the actual algorithm implementations in `src/store/store.c`,
  `src/cypher/cypher.c` (4523 lines), `src/pipeline/*.c`, `src/cli/cli.c` (4732 lines), and
  `src/foundation/log.c`. No field name, default value, or numeric constant below was guessed; where the
  extraction could not fully verify a claim, it is marked **UNVERIFIED** with a note on what was checked.
- The clone's `docs/llms.txt` and `docs/CONFIGURATION.md` were also read directly (outside the delegated
  research scope) for env-var and top-level product-fact confirmation; see §12 and §13.

**Correction to the x06-source-scout-digests.md §1 characterization**: that digest states `"Semantic"
search has NO neural model — 11 in-process lexical/structural signals`. The clone's own `docs/llms.txt`
states semantic search uses **bundled nomic-embed-code embeddings (768-dim, compiled into the binary)**
combined with "11-signal combined scoring" — i.e. the neural embedding is itself one of the signals feeding
a combined score, not "no neural model at all." The `search_graph` `semantic_query` mode specifically (the
one wire-exposed as an MCP tool param, traced in full in §2) turns out to implement a **token-vector cosine
scheme with a random-projection fallback for out-of-vocabulary tokens** — see §2's semantic_query
subsection for the exact algorithm as found in `store.c`. Whether that token-vector table (`token_vectors`)
is itself populated by the bundled nomic-embed-code model at index time, or by a separate lexical process,
was **not traced** in this pass (`token_vectors` population happens in an indexing pass file not covered by
the delegated file lists) — flagged **UNVERIFIED**, worth a follow-up read of the semantic/embedding pass
before final enforcer-memory design lock.

---

## 1. MCP JSON-RPC envelope, `tools/list`, and stdio framing

- **Transport**: stdio only, JSON-RPC 2.0 framing. Confirmed by `server.json`: `"transport": {"type":
  "stdio"}` for both npm and PyPI package entries; `docs/llms.txt` and `main.c` confirm `codebase-memory-mcp`
  with no args runs the MCP server on stdio (`main.c:494`: `"codebase-memory-mcp Run MCP server on
  stdio"`). Exact byte-framing (newline-delimited JSON vs `Content-Length:` headers) was **UNVERIFIED** —
  not traced to the low-level stdio read/write loop in this pass; the delegated agents focused on tool
  schemas/handlers, not the transport loop itself. Given the MCP spec's stdio transport is
  newline-delimited JSON (no `Content-Length` framing, unlike LSP), and nothing in the traced code
  referenced a `Content-Length` header, newline-delimited is the working assumption but is marked
  **UNVERIFIED** pending a direct read of the stdio read loop (likely in `src/mcp/mcp.c` outside the
  traced line ranges, or in `main.c`).
- **Tool result envelope** (confirmed for every one of the 14 tools via `cbm_mcp_text_result`,
  `src/mcp/mcp.c:248-276`):
  ```json
  {
    "content": [{ "type": "text", "text": "<the tool's JSON response, serialized as a string>" }],
    "structuredContent": { "...the same JSON, as a real object — only added when isError:false AND the text parses as a JSON object..." },
    "isError": false
  }
  ```
  On error, `structuredContent` is omitted entirely — only `content[0].text` (plain string or a small JSON
  error object, tool-dependent) and `"isError": true` are present.
- **`tools/list` response shape**: **UNVERIFIED** in fine detail (the delegated agents extracted individual
  tool schema *literals* at their registration sites in `mcp.c`, e.g. lines 339-374 for `search_graph`,
  376-397 for `query_graph`, 399-418 for `trace_path`, 420-427 for `get_code_snippet`, 429-432 for
  `get_graph_schema`, 434-449 for `get_architecture`, 451-473 for `search_code` — these are the literal
  `inputSchema` JSON Schema objects returned when the tool table is serialized for `tools/list`), but the
  exact wrapping (whether `tools/list` returns `{"tools":[{name,description,inputSchema},...]}` per the
  standard MCP spec, and which JSON Schema draft version is declared, e.g. `"$schema":
  "http://json-schema.org/draft-07/schema#"` or none at all) was not independently confirmed by reading the
  `tools/list` handler itself. Standard MCP servers return `{"tools":[...]}` with no `$schema` field on each
  tool's `inputSchema` (bare JSON Schema object, draft-07-compatible subset); this is the working assumption
  but is marked **UNVERIFIED** for this specific server.
- **`initialize` handshake / server info**: **UNVERIFIED** — not traced. `server.json` gives the
  registry-level identity (`"name": "io.github.DeusData/codebase-memory-mcp"`, `"version": "0.8.1"`) which
  is very likely what the `initialize` response's `serverInfo.name`/`serverInfo.version` echoes, but the
  actual `initialize` handler was not read in this pass.
- **stderr logging contract** (directly verified, `src/foundation/log.c` + `log.h`, outside delegated
  agents' scope):
  - stdout is reserved for JSON-RPC; all logs go to stderr via `emit_line` → `fprintf(stderr, "%s\n",
    line)` (`log.c:198-206`).
  - Two formats, selected by `CBM_LOG_FORMAT` env var (`text` default, `json` alternative):
    - **text**: `level=<lvl> msg=<event> key1=val1 key2=val2 ...` (`log.c:236-251`)
    - **json**: `{"level":"<lvl>","event":"<event>","key1":"val1",...}` (`log.c:218-234`)
  - Levels (`log.h:21-25`): `CBM_LOG_DEBUG=0`, `CBM_LOG_INFO=1`, `CBM_LOG_WARN=2`, `CBM_LOG_ERROR=3`,
    `CBM_LOG_NONE=4` (disables all logging).
  - `CBM_LOG_LEVEL` env var (`docs/CONFIGURATION.md:116`): default `info`; accepts `debug|info|warn|error|none`
    **or** the numeric forms `0`-`4`.
  - A log call below the configured level is a no-op (`log.c:209-211`: `if (level < g_log_level) return;`).
  - This matches the digest's "never silent skip, never fail the run" framing: log lines are advisory only,
    never gate tool success/failure.

---

## 2. `search_graph`

Registration/schema literal: `src/mcp/mcp.c:339-374`. Three modes selected by which of `query` /
`name_pattern` (+ filters) / `semantic_query` is populated — **not fully independent/combinable**; see the
mode-interaction note below (a real discrepancy vs. the tool's own docstring, flagged by the extraction
agent).

### 2.1 Params (merged across all 3 modes)

| Param | Type | Required | Mode(s) | Default | Notes |
|---|---|---|---|---|---|
| `project` | string | **yes** | all | — | only required field (`"required":["project"]`, mcp.c:374) |
| `query` | string | no | BM25 | — | "Tokens split on whitespace; camelCase indexed as individual words." When set (and BM25 succeeds), `name_pattern` is **ignored** (mcp.c:353-358) |
| `name_pattern` | string | no | regex | — | regex on `n.name` (store.c:2518-2523) |
| `qn_pattern` | string | no | regex | — | regex on `n.qualified_name` (store.c:2524-2529) |
| `label` | string | no | regex mode only | — | has **zero effect** on BM25 path (fixed noise-exclude list instead) and on `semantic_query` path (hardcoded to Function/Method/Class) |
| `file_pattern` | string | no | BM25 (as SQL LIKE) + regex | — | glob→LIKE; bare literal (no `*`/`?`) becomes `%literal%` |
| `relationship` | string | no | regex mode | `"CALLS"` (for `include_connected`) | must match `^[A-Z_]+$`, else 400 error |
| `min_degree` / `max_degree` | integer | no | regex mode | `-1` sentinel = no filter | filters `(in_deg+out_deg)` |
| `exclude_entry_points` | boolean | no | regex mode | `false` | excludes nodes with 0 inbound + ≥1 outbound CALLS |
| `include_connected` | boolean | no | regex mode | `false` | adds `connected_names` via 1-hop BFS using `relationship` |
| `semantic_query` | array of strings | no | semantic | — | **must be a JSON array**; a bare string is a type error |
| `limit` | integer | no | all | **100 for BM25** (`BM25_DEFAULT_LIMIT`, mcp.c:1570,1928), **200 for regex mode** (`CBM_DEFAULT_SEARCH_LIMIT`, mcp.c:1950), **16 for semantic internal fallback** (`CBM_SZ_16`) if `limit<=0` reaches that layer | see discrepancy note below |
| `offset` | integer | no | BM25 + regex only (not semantic) | `0` | |

**Discrepancy flagged by extraction**: the tool's own registered docstring says default limit is 200 for
all modes (mcp.c:347, 369), but the BM25 code path's actual default is 100 (`BM25_DEFAULT_LIMIT`, used at
mcp.c:1928 when `limit` arg is absent). Implementers should port the *code* default (100 for BM25), not the
docstring's claim.

**Mode-interaction discrepancy**: the tool docstring claims "the three modes are independent and can be
combined in a single call" (mcp.c:346), but tracing the handler shows: if `query` is set and BM25 succeeds,
the function **returns immediately** with only the BM25 shape — `name_pattern`/label/degree filters and
`semantic_query` are silently ignored in that branch. If `query` is empty/absent (or BM25 finds 0 usable
tokens), execution falls through to the regex/label/degree path, and `semantic_query` (if present) is then
evaluated as an *additional* field on top of that path's response. **Net effect: BM25 and `semantic_query`
are mutually exclusive in one call; regex-mode filters and `semantic_query` DO combine.**

### 2.2 BM25 specifics

- **k1/b constants**: SQLite's compiled-in `bm25()` defaults (**k1=1.2, b=0.75**) — no custom weight
  override found anywhere (`bm25(nodes_fts) AS base_rank`, mcp.c:1702, no extra args passed).
- **Label-boost table** (exact, mcp.c:1694-1700; `rank = fts.base_rank - boost`, more-negative wins since
  FTS5 `bm25()` returns negative-is-better scores, `ORDER BY rank ASC`):

  | Label(s) | Boost |
  |---|---|
  | `Function`, `Method` | **+10.0** |
  | `Route` | **+8.0** |
  | `Class`, `Interface`, `Type`, `Enum` | **+5.0** |
  | anything else | +0.0 |

  Correction to the digest's summary ("+10 Function/+8 Route/+5 Class"): the +5.0 tier actually covers
  **four** labels (`Class`, `Interface`, `Type`, `Enum`), not just `Class`.
- **Fixed noise-label exclusion** (BM25 path only, independent of the `label` param):
  `n.label NOT IN ('File','Folder','Module','Section','Variable','Project')` (mcp.c:1708).
- **camelCase splitting** — `cbm_camel_split()`, a registered SQLite scalar function (`store.c:452-492`,
  registration `store.c:645-646,803-804`), applied **only to the `name` FTS column at index time**
  (`pipeline_incremental.c:660-665`; `qualified_name`/`label`/`file_path` columns get the raw value).
  Split rule (`camel_should_split`, store.c:454-468): insert a space before `input[i]` when
  (a) `input[i]` is uppercase and `input[i-1]` is lowercase (`updateCloud` → boundary before `C`), or
  (b) `input[i]` is uppercase, `input[i-1]` is uppercase, and `input[i+1]` is lowercase (`XMLParser` →
  boundary before the `P` in `Parser`, not between every letter of `XML`). No digit-boundary rule exists.
  The indexed FTS text is the **original identifier concatenated with the split version**, e.g.
  `"updateCloudClient"` indexes as `"updateCloudClient update Cloud Client"` — both the whole-token and the
  split sub-words are searchable.
  **snake_case** is not handled by this function at all; it relies on FTS5's own `unicode61` tokenizer
  treating `_` as a separator (no `tokenchars='_'` override found, store.c:307-311), so
  `update_cloud_client` naturally tokenizes into `update`/`cloud`/`client`.
  **Query-side** tokenization (`bm25_build_match`, mcp.c:1605-1642) is separate and simpler: split the raw
  `query` string on any run of non-`[A-Za-z0-9_]` characters, join surviving tokens with literal `" OR "`
  (e.g. `"update settings"` → `update OR settings`) — the query string itself is NOT camelCase-split, only
  indexed `name` values were pre-split at index time.
  FTS5 table definition: `CREATE VIRTUAL TABLE nodes_fts USING fts5(name, qualified_name, label, file_path,
  content='', tokenize='unicode61 remove_diacritics 2')` (store.c:306-311) — contentless (index only, no
  stored source text).
- **Inner candidate cap**: `BM25_INNER_LIMIT = 2000` (mcp.c:1592) — FTS5 subquery capped to top 2000
  `bm25()`-ranked rows *before* label-exclude/file-pattern filters and boosting are applied. `total` is
  computed only over this same 2000-row window, so true-match counts silently under-report past 2000 raw
  FTS5 hits (documented perf tradeoff in source comment, mcp.c:1586-1592, not a bug).

### 2.3 `semantic_query` cosine specifics

- Fixed vector dimension **768**, int8-quantized (`VS_VEC_DIM = 768`, store.c:6166). Max 32 keywords
  accepted (`VS_MAX_KW`/`MAX_KW_SEARCH = 32`).
- Per-keyword vector (`vs_build_keyword_vectors`, store.c:6245-6262):
  1. Look up a pre-computed vector from `token_vectors` table (`SELECT vector, idf FROM token_vectors WHERE
     project=? AND token=?`); if found, de-quantize int8→float (`/127.0`). **Where `token_vectors` itself
     is populated (embedding model vs. lexical process) was not traced — UNVERIFIED, see the correction
     note in §0.**
  2. If no vector exists for that token (**out-of-vocabulary fallback**): deterministic sparse
     random-projection — seed = `XXH3_64bits(token)`, for `VS_SPARSE_NNZE = 8` iterations hash
     `(i, seed + VS_RI_SEED)` (`VS_RI_SEED = 0x52494E44`) to pick a dimension `pos = h % 768` and a sign,
     accumulate `out[pos] += sign`. Not a zero vector — always produces a usable pseudo-embedding.
  3. Normalize to unit L2 norm, re-quantize to int8 (store.c:6226-6240); if magnitude `< 1e-10` the
     keyword is skipped entirely (contributes nothing, reduces effective keyword count).
- **Cosine formula**: `cos = dot(a,b) / (sqrt(Σa²) · sqrt(Σb²))` over int8-promoted-to-int32 components;
  returns `0.0` if denom `<= 1e-10` or blob-length mismatch (SQL-registered `cbm_cosine_i8`, store.c:563-588).
- **Scoring rule**: **minimum** cosine across all keyword vectors (not average) — "ALL keywords must be
  relevant, not just the average" (store.c:6162-6164 comment; `vs_min_cosine_score`, store.c:6267-6289).
- **No hard similarity threshold/cutoff** — every fetched candidate is scored and returned subject to
  `limit`; confirmed no min-cosine gate anywhere in the vector-search pipeline.
- Candidate pipeline (`cbm_store_vector_search`, store.c:6322-6422): SQL restricts to `label IN
  ('Function','Method','Class')` only (regardless of the `label` param, which is not even read here);
  pre-sorts by cosine against keyword[0] only, over-fetching `limit*5` candidates; re-scores each by true
  min-across-all-keywords cosine in C; re-sorts descending by that true score; trims to `limit` (internal
  default 16 if `limit<=0` at this layer, though the outer tool-level default of 200 normally prevents that
  branch from firing unless the caller explicitly passes `limit: 0` or negative).

### 2.4 Response shapes

**BM25 mode** (mcp.c:1764-1796):
```json
{
  "total": 42,
  "search_mode": "bm25",
  "results": [
    {
      "name": "updateCloudClient",
      "qualified_name": "pkg/foo.updateCloudClient",
      "label": "Function",
      "file_path": "src/foo.go",
      "start_line": 10,
      "end_line": 42,
      "rank": -12.34
    }
  ],
  "has_more": true
}
```
No `offset` echoed. `has_more = total > offset + emitted`.

**Regex/label mode** (mcp.c:1811-1843):
```json
{
  "total": 7,
  "results": [
    {
      "name": "Foo",
      "qualified_name": "pkg/Foo",
      "label": "Class",
      "file_path": "src/foo.go",
      "in_degree": 3,
      "out_degree": 5,
      "connected_names": ["Bar", "Baz"],
      "cyclomatic": 4
    }
  ],
  "has_more": false,
  "hint": "No nodes with this label. Available labels: Function, Method, Class, Interface, Route, Variable, Module, Package, File, Folder."
}
```
- `connected_names` present only if `include_connected=true` and ≥1 found (omitted, not `[]`, otherwise).
- Node `properties_json` scalar keys (string/bool/int/real only — nested values dropped) flattened onto
  the result object.
- `hint` present only when `total==0` (3 wording variants depending on which filters were set).
- Ordering: `ORDER BY name` ascending — **alphabetical, not relevance-ranked**.
- `has_more = total > offset + count`.

**`semantic_results`** (separate top-level array, appended onto the regex-mode response only — never
alongside BM25's response since BM25 short-circuits first):
```json
{
  "semantic_results": [
    { "name": "publishEvent", "qualified_name": "pkg/publishEvent", "label": "Function", "file_path": "src/pkg.go", "score": 0.732 }
  ]
}
```
No `total`/`has_more`/`offset` for this array. Omitted entirely if `semantic_query` absent/empty/zero-hits.
Ordered descending by `score`.

### 2.5 Error cases

- Empty results: not an error, `"results": []`/`"total": 0`, `isError:false`, optional `hint`.
- Bad `relationship` (fails `^[A-Z_]+$` or > 64 chars): `isError:true`, text `"relationship must be
  uppercase letters and underscores"`.
- `semantic_query` present but not an array: `isError:true`, text `"semantic_query must be an array of
  keyword strings, e.g. [\"send\",\"pubsub\",\"publish\"] — not a single string. Split your query into
  individual keywords; each is scored independently via per-keyword min-cosine."`
- Missing `project` key entirely:
  ```json
  { "error": "missing required argument: project", "hint": "Pass the project as the \"project\" argument..." }
  ```
- Unknown/unindexed project:
  ```json
  { "error": "project not found or not indexed", "hint": "Use list_projects...", "available_projects": ["repoA","repoB"], "count": 2 }
  ```
- Project exists but has zero indexed nodes:
  ```json
  { "error": "project not indexed — run index_repository first", "hint": "..." }
  ```
- Error envelope: `{"content":[{"type":"text","text":"<json-or-plain-string>"}],"isError":true}` — no
  `structuredContent` mirror on errors.

---

## 3. `get_graph_schema`

Registration: `src/mcp/mcp.c:429-432`.

### 3.1 Params

| Param | Type | Required |
|---|---|---|
| `project` | string | **yes** |

That is the entire input schema — no pagination, no filters.

### 3.2 Response — dynamic introspection, NOT a static registry

Handler `handle_get_graph_schema` (mcp.c:1446-1514), backed by `get_schema_impl` (store.c:3210-3350). The
response enumerates only labels/edge-types **actually present** in that project's data, ordered by
descending row count (not alphabetical, not a fixed enum):

```json
{
  "node_labels": [
    { "label": "Function", "count": 812, "properties": ["name","qualified_name","file_path","start_line","end_line","complexity","is_entry_point"] },
    { "label": "Class", "count": 140, "properties": ["name","qualified_name","file_path","start_line","end_line"] }
  ],
  "edge_types": [
    { "type": "CALLS", "count": 3021, "properties": ["source_id","target_id"] },
    { "type": "IMPORTS", "count": 480, "properties": ["source_id","target_id"] }
  ],
  "adr_present": false,
  "adr_hint": "No ADR found. Use manage_adr(mode='update') to persist architectural decisions across sessions. Run get_architecture(aspects=['all']) first."
}
```
- `node_labels[].properties`: 5 fixed base columns (`name`,`qualified_name`,`file_path`,`start_line`,
  `end_line`) followed by up to 50 distinct JSON property keys for that label, alphabetical
  (`SCHEMA_MAX_JSON_KEYS = 50`, store.c:3042).
- `edge_types[].properties`: 2 fixed base columns (`source_id`,`target_id`) + up to 50 distinct JSON keys,
  alphabetical.
- `adr_hint` present only when `adr_present==false`.
- Labels/edge-types referenced elsewhere in source as literals that CAN occur (not a closed enum returned
  by this tool, just what implementers should expect data to contain): node labels `Function`, `Method`,
  `Class`, `Interface`, `Type`, `Enum`, `Route`, `Variable`, `Module`, `Package`, `File`, `Folder`,
  `Section`, `Project`; edge types `CALLS`, `USAGE`, `INHERITS`, `IMPLEMENTS`, `HTTP_CALLS`, `ASYNC_CALLS`,
  `DATA_FLOWS`, `CROSS_HTTP_CALLS`, `CROSS_ASYNC_CALLS`, `CROSS_CHANNEL`, `CROSS_GRPC_CALLS`,
  `CROSS_GRAPHQL_CALLS`, `CROSS_TRPC_CALLS`.

### 3.3 Errors

Same envelope/error pattern as `search_graph` §2.5 (`build_no_store_error`/`build_project_list_error`,
mcp.c:1447-1455; `verify_project_indexed`).

---

## 4. `query_graph`

Registration: `src/mcp/mcp.c:376-397`. Handler `handle_query_graph` (mcp.c:2055-2132). Grammar/evaluator:
`src/cypher/cypher.c` (4523 lines) + `src/cypher/cypher.h`.

### 4.1 Params

| Param | Type | Required | Default | Notes |
|---|---|---|---|---|
| `query` | string | **yes** | — | the Cypher query text |
| `project` | string | **yes** | — | resolved via `get_project_arg`, also accepts `project_name`/`project_id`/`projectName` aliases |
| `max_rows` | integer | no | unbounded up to the 100k ceiling (see §4.4) | tool description explicitly states "No offset support" — use `search_graph`'s offset/limit for pagination instead |

### 4.2 Example request / response

```json
{
  "query": "MATCH (f:Function) WHERE f.transitive_loop_depth >= 3 OR f.linear_scan_in_loop >= 1 RETURN f.qualified_name, f.transitive_loop_depth, f.linear_scan_in_loop ORDER BY f.transitive_loop_depth DESC LIMIT 20",
  "project": "myrepo",
  "max_rows": 5000
}
```
Success response (inside the standard MCP envelope from §1):
```json
{
  "columns": ["f.qualified_name", "f.transitive_loop_depth", "f.linear_scan_in_loop"],
  "rows": [
    ["pkg.Foo.bar", "4", "2"],
    ["pkg.Baz.qux", "3", "0"]
  ],
  "total": 2
}
```
**All row cell values are serialized as strings** regardless of underlying numeric/boolean type
(`cbm_cypher_result_t.rows` is `const char ***`; `yyjson_mut_arr_add_str`, mcp.c:2109) — this is a real
port hazard: a Rust implementation returning typed JSON values (numbers as numbers) would NOT be
byte-for-byte compatible unless it deliberately stringifies every cell.
Zero-row success additionally carries `"hint": "Query returned no results. Use get_graph_schema() to see
available labels and edge types."` alongside `columns:[]`, `rows:[]`, `total:0`.

### 4.3 Cypher grammar subset (reconstructed from lexer + recursive-descent parser)

Doc-block example from `cypher.h:7-10`:
```
MATCH (n:Label)-[:TYPE*1..3]->(m:Label {prop: "val"})
WHERE n.name =~ ".*pattern.*" AND m.label = "Function"
RETURN n.name, COUNT(m) AS cnt ORDER BY cnt DESC LIMIT 10
```

Top-level structure (`cbm_parse`, cypher.c:1829-1930):
```
query := [UNWIND expr AS var] [OPTIONAL] MATCH pattern
         (MATCH-chain)*                                  // additional MATCH/OPTIONAL MATCH clauses
         [WHERE expr]
         (post-where: more MATCH/OPTIONAL MATCH)*
         [WITH return-items [ORDER BY ...] [SKIP n] [LIMIT n] [WHERE expr]]
         [RETURN (* | return-items) [ORDER BY ...] [SKIP n] [LIMIT n]]
         [UNION [ALL] query]
```
- A query MUST open with (optional UNWIND, optional OPTIONAL, then) `MATCH` — hard-fails `"expected MATCH"`
  otherwise (cypher.c:1852-1853).
- **Supported clauses**: `MATCH` (incl. `OPTIONAL MATCH`), `WHERE`, `RETURN`, `WITH`, `ORDER BY`, `SKIP`,
  `LIMIT`, `DISTINCT`, `UNION [ALL]`, `UNWIND` (literal-list or variable form only). `UNWIND` works even
  though the MCP tool description text doesn't advertise it.
- **Rejected write clauses** — explicit check-and-error (NOT pure grammar omission; the lexer *recognizes*
  these keywords specifically to produce a helpful rejection message), checked both at query start and
  after WHERE (`unsupported_clause_error`, cypher.c:804-833):

  | Keyword | Exact rejection message |
  |---|---|
  | `CREATE` | `unsupported Cypher feature: CREATE clause (write operations not supported)` |
  | `DELETE` | `unsupported Cypher feature: DELETE clause (write operations not supported)` |
  | `DETACH` | `unsupported Cypher feature: DETACH DELETE (write operations not supported)` |
  | `SET` | `unsupported Cypher feature: SET clause (write operations not supported)` |
  | `REMOVE` | `unsupported Cypher feature: REMOVE clause (write operations not supported)` |
  | `MERGE` | `unsupported Cypher feature: MERGE clause (write operations not supported)` |
  | `YIELD` | `unsupported Cypher feature: YIELD clause` |
  | `CALL` | `unsupported Cypher feature: CALL clause (stored procedures not supported)` |
  | `FOREACH` | `unsupported Cypher feature: FOREACH clause` |
  | `MANDATORY` | `unsupported Cypher feature: MANDATORY MATCH` |
  | `DROP` | `unsupported Cypher feature: DROP (schema operations not supported)` |
  | `CONSTRAINT` | `unsupported Cypher feature: CONSTRAINT (schema operations not supported)` |

**Pattern syntax** (`parse_node` cypher.c:545-595, `parse_rel` cypher.c:704-739, `parse_match_pattern`
cypher.c:1686-1718):
```
pattern   := node (rel node)*                 // chained only — NO comma-separated patterns in one MATCH
node      := '(' [IDENT] [':' label ('|' label)*] ['{' propmap '}'] ')'
propmap   := (IDENT ':' STRING (','?)*)*       // values MUST be string literals — no numeric/bool prop-map values
rel       := ['<'] '-' ['[' [IDENT] [':' TYPE ('|' TYPE)*] ['*' hop_range] ']'] '-' ['>']
hop_range := NUMBER ['..' [NUMBER]] | '..' [NUMBER] | <empty>   // '*' alone = 1..unbounded; 'N' alone = 1..N
```
- Direction: leading `<` and no trailing `>` → inbound; trailing `>` and no leading `<` → outbound; both or
  neither → undirected/"any" (cypher.c:729-736).
- Label alternation `:A|B|C` supported on both node labels and relationship types.
- Variable-length paths `[*]`, `[*3]`, `[*2..5]`, `[*..5]` all supported; max traversal depth capped at
  `CYP_MAX_DEPTH = 10` (cypher.c:25) elsewhere in the engine.
- `EXISTS { (var)-[:TYPE]->() }` — **single-hop only**; multi-hop `EXISTS` patterns are explicitly rejected
  (`"unsupported EXISTS pattern — only the single-hop form '(var)-[:TYPE]->()' is supported"`).
- Multiple comma-separated patterns within ONE `MATCH` are **not supported** — use multiple `MATCH` clauses.

**WHERE clause** — precedence loosest→tightest: `OR > XOR > AND > NOT > atom/parens/condition`:
```
condition := [NOT] IDENT ':' LABEL
           | [NOT] IDENT '.' IDENT comparison_op literal
           | [NOT] IDENT '.' IDENT IS [NOT] NULL
           | [NOT] IDENT '.' IDENT IN '[' literal (',' literal)* ']'
           | [NOT] IDENT
           | [NOT] EXISTS '{' node rel node '}'
comparison_op := '=' | '<>' | '!=' | '=~' | '>=' | '<=' | '>' | '<' | CONTAINS | STARTS WITH | ENDS WITH
```
- `<>` and `!=` are synonyms (both lex to `TOK_NEQ`). `=~` is regex-match.
- Right-hand side must be a **literal** (string/number/true/false) — no variable-to-variable/property
  comparisons.
- `IN [...]` accepts a bracketed literal list.

**RETURN / WITH**:
```
return_clause := RETURN [DISTINCT] ('*' | return_item (',' return_item)*)
                 [ORDER BY order_expr [ASC|DESC]] [SKIP number] [LIMIT number]
return_item    := CASE (WHEN expr THEN literal)+ [ELSE literal] END
                | agg_func '(' [DISTINCT] ('*' | IDENT ['.' IDENT]) ')' [AS alias]
                | str_func '(' IDENT ['.' IDENT] ')' [AS alias]
                | multiarg_func '(' func_arg (',' func_arg)* ')' [AS alias]
                | scalar_func '(' IDENT ['.' IDENT] ')' [AS alias]
                | IDENT ['.' IDENT] [AS alias]
agg_func       := COUNT | SUM | AVG | MIN | MAX | COLLECT
str_func       := toLower | toUpper | toString
scalar_func    := labels | type | id | keys | properties | toInteger | toFloat | toBoolean | size | length | trim | ltrim | rtrim | reverse
multiarg_func  := coalesce | substring | replace | left | right
```
- `DISTINCT` supported both at `RETURN DISTINCT` level and per-aggregate `COUNT(DISTINCT x)`.
- `RETURN *` is **not** a full property dump — it projects a fixed 4-column shape per pattern variable:
  `<var>.name, <var>.qualified_name, <var>.label, <var>.file_path` (for a bound relationship variable, only
  `.type` is filled, the other 3 are empty strings).
- `CASE WHEN <cond> THEN <literal> [WHEN...]* [ELSE <literal>] END` — only the "generic"
  CASE-WHEN-THEN form; no simple-CASE (`CASE x WHEN ...`). `WHEN` condition reuses full WHERE grammar; THEN/
  ELSE values must be literals only.
- `AS` alias supported uniformly on every return-item shape.
- `ORDER BY` accepts a bare `var[.prop]` or an aggregate-call expression, plus `ASC`/`DESC` — cannot order
  by an arbitrary CASE/scalar-func expression.
- `SKIP`/`LIMIT` take a plain integer literal; `limit` internally defaults to sentinel `-1` (distinguishing
  "no LIMIT clause" from an explicit `LIMIT 0`, which must yield zero rows).
- Calling an unsupported function fails loudly with the exact allow-list in the message (not a silent
  blank-column projection):
  `unsupported function '<name>' (supported: count, sum, avg, min, max, collect, toLower, toUpper,
  toString, toInteger, toFloat, toBoolean, size, length, trim, ltrim, rtrim, reverse, labels, type, id,
  keys, properties)`
  `unsupported expression: list indexing/slicing '[...]' is not supported`

### 4.4 The 100k row ceiling — ERRORS, does not silently truncate

`#define CYPHER_RESULT_CEILING 100000` (cypher.c:2589), comment: "Hard ceiling: queries returning more than
this trigger an error instead of data. Prevents accidental multi-GB JSON payloads from unbounded MATCH (n)
RETURN n."

Mechanism (`cbm_cypher_execute`, cypher.c:4443-4504):
1. If caller's `max_rows <= 0` (omitted), it's set to the full ceiling (100000). If caller passes a smaller
   explicit `max_rows`, that smaller cap is used as the active accumulation limit during row-building.
2. After the full result set is built (including UNION legs + dedup), a post-hoc check fires:
   ```c
   if (rb.row_count >= CYPHER_RESULT_CEILING) {
       out->error = "result exceeded 100k rows — use narrower filters or add LIMIT";
       return CBM_NOT_FOUND;
   }
   ```
3. **The whole result is discarded and the call fails** — no partial/truncated payload, no `"truncated":
   true` flag anywhere in the response-building code.
4. Practical consequence: since row-building already respects `max_rows` as an active cap during
   accumulation, this ceiling error only fires when `max_rows` itself was left at/above the 100k default
   (caller passed no smaller `max_rows`) AND the true result would hit exactly the ceiling. If the caller
   passes an explicit smaller `max_rows` (e.g. 500), building simply stops at 500 rows and returns
   successfully — no error, no truncation flag, because the ceiling check compares against the fixed
   constant, not the caller's `max_rows`.
5. This is a *different* mechanism from `search_graph`'s `limit`/`offset`/`has_more` pagination —
   `query_graph` explicitly has "no offset support."

### 4.5 Complexity property names (queryable node properties)

Two write sites: Tier A (per-function, extraction-time) and Tier B (post-pass, interprocedural).

**Tier A** — `build_def_props`, `src/pipeline/pass_parallel.c:443-472`:

| Property (exact JSON key) | Meaning | Scope |
|---|---|---|
| `complexity` | cyclomatic complexity | all definitions |
| `cognitive` | cognitive complexity | Function/Method only |
| `loop_count` | loop count in body | Function/Method only |
| `loop_depth` | max local (non-transitive) nested-loop depth | Function/Method only |
| `self_recursive` | boolean — direct self-call | Function/Method only |
| `param_count` | parameter count | Function/Method only |
| `max_access_depth` | max chained member/property access depth | Function/Method only |
| `linear_scan_in_loop` | count of find/contains/indexOf-style scans inside a loop | Function/Method only |
| `alloc_in_loop` | count of allocations/appends inside a loop | Function/Method only |
| `recursion_in_loop` | boolean — self-call occurs inside a loop | Function/Method only |
| `unguarded_recursion` | boolean — recursion with no conditionally-guarded base case | Function/Method only |
| `lines` | line count | all definitions |
| `is_exported` | boolean | all definitions |
| `is_test` | boolean | all definitions |
| `is_entry_point` | boolean | all definitions |

**Tier B** — `append_complexity_props`, `src/pipeline/pass_complexity.c:79-100` (interprocedural, along
`CALLS` edges, DFS depth-capped at `CBM_TLD_MAX_DEPTH = 256`):

| Property (exact JSON key) | Meaning |
|---|---|
| `transitive_loop_depth` | worst-case nested-loop degree propagated along CALLS edges (upper-bound heuristic, not a proof) |
| `recursive` | boolean — self-recursive OR part of a call-graph cycle (mutual recursion via DFS), OR'd on top of the `self_recursive` seed |

**Naming hazard for the Rust port**: `self_recursive` (Tier A, direct self-call only) is a **different
property** from `recursive` (Tier B, self-recursive OR mutual-recursion-cycle-detected). Do not collapse
these into one boolean.

### 4.6 Error examples

Write-clause rejection (`"CREATE (n:Foo) RETURN n"`):
```json
{ "content": [{ "type": "text", "text": "unsupported Cypher feature: CREATE clause (write operations not supported)" }], "isError": true }
```
100k-ceiling error:
```json
{ "content": [{ "type": "text", "text": "result exceeded 100k rows — use narrower filters or add LIMIT" }], "isError": true }
```
Missing `query` argument (short-circuits before the engine runs at all):
```json
{ "content": [{ "type": "text", "text": "query is required" }], "isError": true }
```
Unknown/unindexed project:
```json
{ "content": [{ "type": "text", "text": "project not found or not indexed" }], "isError": true }
```
or, if resolved but never indexed:
```json
{ "content": [{ "type": "text", "text": "project not indexed — run index_repository first" }], "isError": true }
```
**UNVERIFIED**: whether the project-not-found error text is enriched with the actual list of known project
names (as `search_graph`'s equivalent error does) was not fully confirmed for `query_graph`'s specific error
path (`build_project_list_error`, mcp.c:1173) — worth a follow-up read if exact text parity matters.

---

## 5. `trace_path` (alias: `trace_call_path`)

Registration: `src/mcp/mcp.c:399-418`. Handler `handle_trace_call_path` (mcp.c:2936-3105). Helpers
mcp.c:2656-2934.

**Architectural note**: there is no `target` param — this tool is always a **BFS fan-out from one resolved
root node**, capped by hop-count `depth`, not an A→B shortest-path search between two named nodes.

### 5.1 Params

| Param | Type | Required | Default | Notes |
|---|---|---|---|---|
| `function_name` | string | **yes** | — | name or qualified_name of the root node (resolution: exact `name=` match, else exact `qualified_name=` match — both project-scoped, case-sensitive, no fuzzy matching) |
| `project` | string | **yes** | — | |
| `direction` | enum string | no | `"both"` | exact values `"inbound"` \| `"outbound"` \| `"both"` — not `in`/`out` |
| `depth` | integer | no | **3** (`MCP_DEFAULT_DEPTH`, mcp.c:27) | hop-count cap, root excluded (`hop>0`) |
| `mode` | enum string | no | `"calls"` | `"calls"` \| `"data_flow"` \| `"cross_service"` |
| `parameter_name` | string | no | — | **dead parameter** — read but never used (mcp.c:2942) |
| `edge_types` | string[] | no | mode-derived (see §5.2) | explicit override always wins over the mode default |
| `risk_labels` | boolean | no | `false` | adds a `risk` field per node, see §5.3 |
| `include_tests` | boolean | no | `false` | see §5.4 |

Results are additionally capped by a hardcoded `MCP_BFS_LIMIT = 100` (mcp.c:30) that is **not** reported to
the caller (no truncation flag).

### 5.2 Edge sets per mode

`resolve_trace_edge_types`, mcp.c:2656-2702:

| Mode | Edge types traversed |
|---|---|
| `calls` | `CALLS` |
| `data_flow` | `CALLS`, `DATA_FLOWS` |
| `cross_service` | `HTTP_CALLS`, `ASYNC_CALLS`, `DATA_FLOWS`, `CALLS`, `CROSS_HTTP_CALLS`, `CROSS_ASYNC_CALLS`, `CROSS_CHANNEL`, `CROSS_GRPC_CALLS`, `CROSS_GRAPHQL_CALLS`, `CROSS_TRPC_CALLS` |

### 5.3 Direction, depth, risk labels

- outbound: `WHERE e.source_id = bfs.node_id`; inbound: `WHERE e.target_id = bfs.node_id`; `"both"` runs
  both and returns them under separate `callees`/`callers` response keys.
- `depth` bounds a SQL recursive CTE (`WHERE bfs.hop < depth`).
- `risk_labels=true` adds `risk` per node (store.c:2948-2972): `hop==1 → "CRITICAL"`, `hop==2 → "HIGH"`,
  `hop==3 → "MEDIUM"`, else `"LOW"`.

### 5.4 `include_tests`

Path heuristic (mcp.c:2704-2712): matches `/test`, `test_`, `_test.`, `/tests/`, `/spec/`, `.test.`.
`false` (default) drops matching nodes entirely; `true` keeps them and adds `"is_test": true` (never
emitted as `false` on non-test nodes — the key is simply absent).

### 5.5 Response shape / path encoding

```json
{
  "function": "process_request",
  "direction": "both",
  "mode": "calls",
  "callees": [
    { "name": "validate_input", "qualified_name": "myapp.handlers.validate_input", "hop": 1, "risk": "CRITICAL" }
  ],
  "callers": [
    { "name": "main", "qualified_name": "myapp.main", "hop": 1, "risk": "CRITICAL" }
  ]
}
```
Per-node fields: `name`, `qualified_name`, `hop` (int), `risk` (only if `risk_labels=true`), `is_test` (only
if true), `args` (raw JSON array from the CALLS edge's serialized args — `data_flow` mode only). **There is
no `edges` array and no `file_path`/line fields anywhere in this response** — paths are encoded purely as a
flat list of `{node-fields, hop}` per direction, not as an explicit node/edge chain.
Multiple same-named root candidates are unioned into a single BFS (`bfs_union_same_name`) unless flagged
ambiguous (see §5.6).

### 5.6 Errors

Root not found (mcp.c:3003-3018):
```json
{ "error": "function not found", "function_name": "foo", "hint": "Use search_graph(name_pattern=\".*foo.*\") to find the exact name, then pass it to trace_path." }
```
Ambiguous root (this is `isError: false` — a normal success response carrying a disambiguation prompt, not
a tool error, mcp.c:3878-3910):
```json
{
  "status": "ambiguous",
  "message": "2 matches for \"main\". Pick a qualified_name from suggestions below, or use search_graph(name_pattern=\"...\") to narrow results.",
  "suggestions": [
    { "qualified_name": "myapp.cli.main", "name": "main", "label": "Function", "file_path": "src/cli.c" }
  ]
}
```
Ambiguity trigger (`pick_resolved_node`, mcp.c:2857-2888): top-scored candidates tie, or ≥2 candidates are
"real" callable defs (`end_line > start_line`).

---

## 6. `get_code_snippet`

Registration: `src/mcp/mcp.c:420-427`. Handler mcp.c:4173-4262. Response builder mcp.c:4070-4171.
Qualified-name construction: `src/pipeline/fqn.c`.

### 6.1 Params

| Param | Type | Required | Default |
|---|---|---|---|
| `qualified_name` | string | **yes** | — accepts full QN or a short/suffix name |
| `project` | string | **yes** | — |
| `include_neighbors` | boolean | no | `false` |

### 6.2 Qualified-name format and resolution

Format (`fqn.c:112-135`): dot-joined `project.dir.subdir.name`; Python `__init__`/JS `index` filename stems
are stripped from segments when a symbol name is present. Case-sensitive throughout.

Resolution tiers (mcp.c:4173-4262):
1. **Exact match** on `qualified_name` (store.c:1252-1261).
2. **Suffix match** on miss: `qualified_name LIKE '%.{suffix}' OR qualified_name = {suffix}`
   (store.c:1882-1934) — handles bare short names and partial dotted suffixes.
   - 1 match → resolved, response adds `"match_method": "suffix"`.
   - >1 match → same `pick_resolved_node` disambiguation as §5.6; if still tied → ambiguous response.
   - 0 match → not-found error.

### 6.3 `include_neighbors` payload

When `true` (mcp.c:4131-4136, store.c:2039-2064): adds `caller_names` / `callee_names` — **arrays of short
name strings only** (not qualified names, not code snippets), each omitted entirely if empty (never `[]`).
Considers edge types `CALLS`, `HTTP_CALLS`, `ASYNC_CALLS` (distinct, sorted, capped at hardcoded
`MCP_DEFAULT_LIMIT = 10`).
Independent of this flag, `callers`/`callees` are **always** present as plain **integer counts**
(in/out-degree, `CALLS` edges only — store.c:1938-1967). Note the asymmetry: the always-present counts use
only `CALLS`, while the opt-in neighbor-name arrays use `CALLS + HTTP_CALLS + ASYNC_CALLS`.

### 6.4 Response shape and byte-exactness

```json
{
  "name": "ProcessOrder",
  "qualified_name": "myproj.orders.ProcessOrder",
  "label": "Function",
  "file_path": "/abs/path/to/orders.py",
  "start_line": 10,
  "end_line": 42,
  "source": "def ProcessOrder(...):\n    ...",
  "callers": 3,
  "callees": 5,
  "caller_names": ["main"],
  "callee_names": ["validate", "save"]
}
```
- `file_path` is the **absolute path** when resolvable (not the DB's stored relative path).
- `source` is the raw text, or the literal string `"(source not available)"` on read failure.
- `match_method` field present only when resolved via the suffix tier.
- Any node `properties_json` keys are flattened directly onto the root response object alongside the fixed
  fields.
- **No hash/sha256 field exists anywhere in this tool's response** — despite the parity-harness
  requirement that this tool be "byte-exact, hash-verified," the C baseline itself does not compute or
  return a content hash. A Rust port aiming for hash-verified parity will need to add that hash itself;
  it cannot diff against a baseline hash field that doesn't exist.
- **Read mechanism** (mcp.c:3121-3158, 3953-3985): fresh read from disk by 1-indexed inclusive line range
  via a line-by-line read loop — **not** a stored blob, **not** byte-offset based. Falls back to a
  `SNIPPET_DEFAULT_LINES = 50`-line window when the node lacks a real end line. Output passed through a
  lossy UTF-8 sanitizer (invalid sequences replaced with U+FFFD) — **not guaranteed byte-identical** to the
  file on disk for non-UTF-8 content. (By contrast, `search_code`'s snippet path uses a different, more
  aggressive ASCII-only sanitizer — a real, intentional divergence between the two tools worth preserving
  rather than "fixing" to be consistent.)

### 6.5 Errors

Not found (plain text, `isError:true`):
```
symbol not found. Use search_graph(name_pattern="...") first to discover the exact qualified_name, then pass it to get_code_snippet.
```
Ambiguous (`isError: false`, same shape as §5.6):
```json
{
  "status": "ambiguous",
  "message": "3 matches for \"ProcessOrder\". Pick a qualified_name from suggestions below, or use search_graph(name_pattern=\"...\") to narrow results.",
  "suggestions": [ { "qualified_name": "myproj.orders.legacy.ProcessOrder", "name": "ProcessOrder", "label": "Function", "file_path": "orders/legacy.py" } ]
}
```

---

## 7. `get_architecture`

Registration: `src/mcp/mcp.c:434-449`. Handler `handle_get_architecture` (mcp.c:2325-2651). Valid-aspects
list/validator mcp.c:2256-2270.

### 7.1 Params

| Param | Type | Required | Notes |
|---|---|---|---|
| `project` | string | **yes** | |
| `path` | string | no | directory-prefix scope, applied uniformly across every requested aspect |
| `aspects` | string[] enum | no (default = all) | 13 values: `all, overview, structure, dependencies, routes, languages, packages, entry_points, hotspots, boundaries, layers, file_tree, clusters` |

### 7.2 Response envelope — one flat object, one top-level key per aspect (not nested under an "aspects" wrapper)

| Aspect | Response key | Shape |
|---|---|---|
| `structure` | `node_labels` | `[{label, count}]` |
| `dependencies` | `edge_types` | `[{type, count}]` |
| `languages` | `languages` | `[{language, file_count}]` |
| `packages` | `packages` | `[{name, node_count, fan_in, fan_out}]` — **`fan_in`/`fan_out` are always 0 for this aspect** (UNVERIFIED why — not traced to a bug or an intentional stub) |
| `entry_points` | `entry_points` | `[{name, qualified_name, file}]` |
| `routes` | `routes` | `[{method, path, handler}]`, capped at 20 |
| `hotspots` | `hotspots` | `[{name, qualified_name, fan_in}]` — ranked purely by CALLS in-degree, capped at 10, test files excluded (exact SQL in §7.3) |
| `boundaries` | `boundaries` | `[{from, to, call_count}]` — cross-package CALLS edge counts |
| (always computed, no aspect token gates it) | `services` | `[{from, to, type, count}]` — cross-service HTTP/async links; **the exact builder function was not located in this pass — UNVERIFIED** |
| `layers` | `layers` | `[{name, layer, reason}]` — `layer` ∈ `{entry, api, core, leaf, internal}` via a rule-based classifier on fan_in/fan_out + route/entry-point presence. Exact numeric thresholds for the "core" tier (`ST_MIN_INDEGREE`-style constant) **UNVERIFIED** — not traced to a literal in this pass. |
| `clusters` | `clusters` | `[{id, label, members, cohesion, top_nodes, packages, edge_types}]` — see §7.4 |
| `file_tree` | `file_tree` | **flat array**, not a nested tree: `[{path, type: "dir"|"file", children: int}]` |
| `overview` | (no dedicated key) | meta-token meaning "all aspects except file_tree" |
| `all` | (no dedicated key) | meta-token meaning "every aspect" |

Always present regardless of requested aspects: `project`, `total_nodes`, `total_edges`; additionally, if
`path` is set: `root_total_nodes`, `root_total_edges`, `scoped_total_nodes`, `scoped_total_edges`.

### 7.3 `hotspots` — exact SQL (store.c:3862-3909)

```sql
SELECT n.name, n.qualified_name, COUNT(*) as fan_in
FROM nodes n JOIN edges e ON e.target_id = n.id AND e.type = 'CALLS'
WHERE n.project = ?1 AND n.label IN ('Function','Method')
  AND (json_extract(n.properties,'$.is_test') IS NULL OR json_extract(n.properties,'$.is_test') != 1)
  AND n.file_path NOT LIKE '%test%'
GROUP BY n.id ORDER BY fan_in DESC LIMIT 10
```
No independent rank/score field beyond `fan_in` — array position is the rank.

### 7.4 `clusters` — Leiden community detection (store.c:5084-5481)

- Graph fed to the clustering algorithm: `CALLS` edges only, over `Function`/`Method`/`Class` nodes, capped
  at `CBM_CLUSTER_NODE_CAP = 8000` nodes. Resolution parameter hardcoded to `1.0`.
- Constants: `CBM_CLUSTER_TOP_N = 12`, `CBM_CLUSTER_MAX_TOPNODES = 5`, `CBM_CLUSTER_MAX_PKGS = 5`,
  `CBM_CLUSTER_MIN_MEMBERS = 2` (singleton clusters dropped).
- `cohesion = internal_edges / (internal_edges + boundary_edges)`.
- `top_nodes` ranked by combined in+out degree within the CALLS subgraph.
- `edge_types` field on each cluster is **hardcoded to `["CALLS"]`**, not derived from the actual edges
  present.
- Clusters ranked by descending member count.

### 7.5 Example request

```json
{ "project": "myrepo", "aspects": ["hotspots", "clusters", "file_tree"], "path": "src/services" }
```

### 7.6 Errors

Invalid aspect value (plain text, `isError:true`):
```
Unknown aspect '<bad>'. Valid: all, overview, structure, dependencies, routes, languages, packages, entry_points, hotspots, boundaries, layers, file_tree, clusters.
```
Project not found:
```json
{ "error": "project not found or not indexed", "hint": "...", "available_projects": ["repoA"], "count": 1 }
```

---

## 8. `search_code`

Registration: `src/mcp/mcp.c:451-473`. Handler `handle_search_code` (mcp.c:4925-5175 area).

### 8.1 Params

| Param | Type | Required | Default |
|---|---|---|---|
| `pattern` | string | **yes** | — |
| `project` | string | **yes** | — |
| `file_pattern` | string | no | — (grep `--include` glob) |
| `path_filter` | string | no | — (TRE/POSIX regex on result paths) |
| `mode` | enum `compact\|full\|files` | no | `compact` |
| `context` | integer | no | `0` (compact mode only) |
| `regex` | boolean | no | `false` |
| `limit` | integer | no | `10` (`MCP_DEFAULT_LIMIT`, mcp.c:29) |

### 8.2 Search mechanism — "graph-augmented grep"

Shells out to real `grep` (POSIX: `xargs -0 grep -Hn -E|-F -f <patternfile>`) or PowerShell
`Select-String` on Windows. The pattern is written to a temp file, never inlined on the command line.
`regex=false` → literal match (`grep -F` / `-SimpleMatch`); `true` → extended regex. Multi-word literal
patterns are auto-converted to a `word1.*word2.*...` regex. Hard cap `GREP_MAX_MATCHES = 500` raw hits.
TRE regex engine is used only for `path_filter` matching and up-front regex validation, not for the primary
text scan.

**Match → containing function** (`find_tightest_node`, mcp.c:4682-4695): linear scan of all nodes in the
matched file, picks the **smallest enclosing `[start_line, end_line]`** span containing the hit line. A
hit with no enclosing node goes into `raw_matches` instead of `results`.

### 8.3 Ranking algorithm — exact scores (mcp.c:4299-4326)

```
score = in_degree                                  # live COUNT query over CALLS in-edges
if label in {Function, Method}: score += 10        # SCORE_FUNC
if label == Route:              score += 15        # SCORE_ROUTE
if path contains vendored/|vendor/|node_modules/: score += -50   # SCORE_VENDORED
if path contains test|spec|_test.:                score += -5   # SCORE_TEST
```
Descending `qsort`; ties are unstable (no documented tie-break). **The computed score is never returned to
the caller** — it exists purely to order `results`, not as a response field.

### 8.4 Response shapes by mode

**`files` mode**: `{"files": [...paths...], "directories": {...}, "total_grep_matches", "total_results",
"raw_match_count", "elapsed_ms", "dedup_ratio"?}` — no `results`/`raw_matches` keys at all.

**`compact`/`full` modes**: `{"results": [{node, qualified_name, label, file, start_line, end_line,
in_degree, out_degree, match_lines: [...], context?, context_start?}], "raw_matches": [{file, line,
content}] (capped at 20), "directories", "total_grep_matches", "total_results", "raw_match_count",
"elapsed_ms", "dedup_ratio"?, "warnings"?}`.
- `full` mode replaces `context`/`context_start` with `source` (the full enclosing node body text).
- `compact` mode only adds `context`/`context_start` if the `context` param is `> 0`.

**No matches**: not an error — empty arrays, `"total_results": 0`, `isError: false`.

### 8.5 Example request

```json
{ "project": "myrepo", "pattern": "TODO|FIXME", "regex": true, "mode": "compact", "context": 2, "limit": 25 }
```

### 8.6 Errors

Invalid regex (malformed when `regex=true`):
```
invalid regex pattern (regex=true): check for unbalanced (), [], or {}
```
Missing `pattern`/`project`, unindexed project, unsafe shell characters in `path_filter`/`file_pattern`,
and grep-launch failures each produce distinct plain-text or `build_project_list_error`-shaped errors
(mcp.c:4950-5099) following the same envelope pattern as the other tools.

---

## 9. `index_repository`

**UNVERIFIED — not covered by this extraction pass.** The delegated research agent assigned to this tool
cluster (index_repository, list_projects, delete_project, index_status, detect_changes, manage_adr,
ingest_traces) did not return a report before this document was finalized under the checkpoint discipline
below (see §0 note on the checkpoint commit). The high-level shape from the prior scout digest
(`x06-source-scout-digests.md` §1, row 1) is the only grounding available:

- modes `full`/`moderate`/`fast`
- cross-repo intelligence
- `persistence` param exports `.codebase-memory/graph.db.zst`

None of the exact param names/types/enums, response shape, or error cases have been independently verified
against `src/mcp/mcp.c` in this pass. **Do not implement against the digest summary alone** — re-run this
extraction against `src/mcp/mcp.c` (search for the `index_repository` registration site) and
`src/pipeline/pipeline.c` / `pipeline_incremental.c` / `artifact.c` before writing the Rust handler.

## 10. `list_projects`

**UNVERIFIED — not covered by this extraction pass.** Same caveat as §9. No param/response/ordering
details were independently confirmed. Likely candidates for follow-up: `src/mcp/mcp.c` (grep
`"list_projects"`), and whatever project-registry store backs `resolve_store`/`get_project_arg` (referenced
indirectly by other tools' handlers in §2-§8, e.g. `build_project_list_error`, `mcp.c:1173`).

## 11. `delete_project`

**UNVERIFIED — not covered by this extraction pass.** Same caveat as §9. Expected to take a `project`
param and return a confirmation shape; error case for unknown project should follow the same
`{"error":"project not found or not indexed", ...}` pattern documented in §2.5/§3.3/§7.6, but this was not
independently confirmed for `delete_project`'s specific handler.

## 12. `index_status`

**UNVERIFIED — not covered by this extraction pass.** Same caveat as §9. Expected to report progress
state for an in-flight or completed index run (per `src/mcp/index_supervisor.c`/`.h`, which was in the
delegated file list but not reported on), but no field names were confirmed.

## 13. `detect_changes`

**UNVERIFIED — not covered by this extraction pass.** Same caveat as §9. The prior scout digest states
"git diff → affected symbols + risk classification; base_branch/since" as the shape, and relevant source
files (`src/pipeline/pass_gitdiff.c`, `pass_githistory.c`) were identified for the delegated agent, but no
exact risk-level enum, thresholds, or response shape were independently confirmed in this pass.

## 14. `manage_adr`

**UNVERIFIED — not covered by this extraction pass.** Confirmed only indirectly: `get_graph_schema`'s
response (§3.2) references `manage_adr(mode='update')` in its `adr_hint` text, confirming a `mode` param
exists with at least an `'update'` value, and that ADR presence is tracked per-project. The full
get/update/sections action set and their exact param/response shapes were not independently traced.

## 15. `ingest_traces`

**UNVERIFIED — not covered by this extraction pass.** The prior scout digest states the shape as
`{caller, callee, count}` merging into `CALLS` edges, but the exact input schema, merge semantics
(increment vs. overwrite vs. dedupe-then-sum), and response shape were not independently confirmed against
source in this pass.

## 16. CLI form (`cli <tool> <json>`)

**Partially verified** — confirmed directly from `main.c` (outside the delegated agents' scope) but not
cross-checked against `src/cli/cli.c`'s actual per-tool dispatch/output-formatting code, which was in the
delegated file list for the tool cluster that did not report back:

- Exact usage line (`main.c:203`, echoed at `main.c:495-496`):
  ```
  Usage: codebase-memory-mcp cli [--progress] [--json] <tool_name> [json_args]
  ```
- `cbm_cli_set_version(CBM_VERSION)` is called before CLI dispatch (`main.c:611`), and server startup logs
  `server.start` with the version at `main.c:648` for the MCP-server path — implying the CLI path has its
  own, separate startup/logging sequence rather than sharing the MCP server's stdio loop.
- **UNVERIFIED**: exact exit codes per outcome (success / unknown tool / bad JSON / tool-level error), and
  whether CLI-mode JSON output is identical to the MCP `content[0].text` body or reformatted/unwrapped
  (e.g. does the CLI print the raw inner JSON directly to stdout without the `content`/`isError` envelope,
  or does it preserve it?). `src/cli/cli.c` (4732 lines) was assigned to the non-reporting agent and was
  not independently read in this pass.
- `--progress` flag routes through `src/cli/progress_sink.c` (progress reporting sink) — not traced further.
- `--json` flag presumably toggles CLI output between a human-readable format and raw JSON, mirroring the
  `CBM_LOG_FORMAT` pattern elsewhere in the codebase, but this is an inference, not a confirmed reading —
  **UNVERIFIED**.

## 17. Common error response shape (cross-tool)

Confirmed by direct observation across every tool traced in §2-§8 (`search_graph`, `get_graph_schema`,
`query_graph`, `trace_path`, `get_code_snippet`, `get_architecture`, `search_code`):

- **Not a JSON-RPC 2.0 error object** (no `-32602`-style numeric `code` field observed anywhere in the
  traced tool-level error paths). Every tool-level error is carried as a normal JSON-RPC **result**, using
  the standard MCP tool-result envelope with `"isError": true`:
  ```json
  { "content": [{ "type": "text", "text": "<plain string or small JSON object, tool-specific>" }], "isError": true }
  ```
- The `text` payload is sometimes a bare human-readable string (e.g. `"query is required"`,
  `"relationship must be uppercase letters and underscores"`) and sometimes a small serialized JSON object
  (e.g. the `{"error": "...", "hint": "...", "available_projects": [...], "count": N}` shape used for
  unknown/unindexed-project errors across `search_graph`, `get_graph_schema`, `get_architecture`, and
  implied for the others). There is no single fixed error-object schema across all tools — implementers
  should treat `content[0].text` as "either a plain string or a JSON string," parse-attempt it, and fall
  back to treating it as opaque text on parse failure.
- `structuredContent` is never present alongside an error (`isError: true`) — only success responses get
  the object-mirror convenience field.
- Whether an actual JSON-RPC-level *transport* error (as opposed to a tool-level `isError:true` result) is
  ever used — e.g. for a malformed `tools/call` request itself, versus a tool that ran and failed — was
  **UNVERIFIED** in this pass; all traced examples are tool-level `isError:true` results, not JSON-RPC
  protocol-level error objects.

---

## 18. Checkpoint note

Per the lane's discipline requirement, this document was committed and pushed after 3 of the 4 dispatched
research passes (covering `query_graph`, `trace_path`/`get_code_snippet`/`get_architecture`/`search_code`,
and `search_graph`/`get_graph_schema` — 6 of the 14 tools, plus the shared JSON-RPC envelope/logging
infrastructure) had returned, with the 7th-tool-cluster pass (`index_repository`, `list_projects`,
`delete_project`, `index_status`, `detect_changes`, `manage_adr`, `ingest_traces`, plus the CLI-form and
common-error-shape sections) still outstanding. Sections 9-16 above are therefore intentionally marked
**UNVERIFIED** rather than filled with digest-derived guesses, per the "never guess silently" instruction.
A follow-up extraction pass should target exactly those sections before this doc is treated as complete
parity-floor grounding for X06.2-X06.9 implementers working on the lifecycle/administrative tools.
