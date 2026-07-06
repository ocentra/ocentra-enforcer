# x06 Baseline Tool Schemas — exact wire contracts extracted from codebase-memory-mcp (C source)

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `x06-baseline-tool-schemas`
> Kind: baseline-schema extraction (sonnet-tier, 2026-07-05, completed in a second sonnet-tier pass same
> day). Ground-truth wire contracts for **all 14 of 14** MCP tools codebase-memory-mcp exposes plus the
> JSON-RPC envelope/transport, extracted directly from its pure-C source (shallow clone of
> `DeusData/codebase-memory-mcp`, tag/HEAD `0.8.1` per `server.json`), so enforcer-memory implementers stop
> guessing param names, defaults, response shapes, and algorithmic constants.
> Read when: implementing any X06.2-X06.9 MCP tool handler, building the parity harness in
> `MEMORY_RETRIEVAL_PARITY_HARNESS.md`, or writing a Rust struct/serializer that must byte-match a
> codebase-memory-mcp response shape.
> Stop rule: this doc describes the BASELINE (C) system only. It does not prescribe what enforcer-memory
> must do — see `MEMORY_RETRIEVAL_OWNER_INTENT.md` / `MEMORY_RETRIEVAL_DECISIONS.md` for binding choices,
> and `x06-source-scout-digests.md` §1 for the high-level parity floor. Where this doc's fine-grained
> findings correct or refine that digest (e.g. semantic search DOES use a bundled neural embedding, not
> pure-lexical; BM25's true default limit is 100 not 200; `ingest_traces` is a STUB that never merges into
> CALLS edges; `detect_changes` has NO risk classification at all, `risk_labels` belongs to `trace_path`
> only), the correction is called out inline.
> Proves: nothing by itself — this is a reference extraction, not a proof artifact. Verify at point of
> copy; every claim below is cited to a `file:line` in the cloned C source. Anything not independently
> re-derivable from source in the time available is marked **UNVERIFIED**.
> Status: 14/14 tools verified (§2-§15), envelope/transport verified (§1), CLI form verified (§16), common
> error shape verified (§17). Remaining UNVERIFIED items are narrow and listed at point of use (see the
> per-section notes) — none block X06.2-X06.9 implementation.
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

- **Transport**: stdio, **dual-framing** — confirmed directly from the event loop `cbm_mcp_server_run`
  (`src/mcp/mcp.c:6137-6214`). Each iteration reads one line via `cbm_getline` (mcp.c:6182); if that line
  starts with the literal `Content-Length:` (mcp.c:6196, `strncmp(line, "Content-Length:",
  SLEN("Content-Length:"))`), the server switches to **LSP-style framing** for that message —
  `handle_content_length_frame` (mcp.c:6041-6070) skips the blank line(s) between header and body, `fread`s
  exactly `content_len` bytes as the body, and replies with `Content-Length: %zu\r\n\r\n%s` (mcp.c:6066, no
  trailing newline after the body). Otherwise — the common case for MCP stdio clients — the line itself
  **is** the JSON-RPC message (**newline-delimited JSON**, no framing header at all): `cbm_mcp_server_handle`
  is called directly on the trimmed line (mcp.c:6204), and the reply is written as `"%s\n"` (mcp.c:6206), a
  bare trailing newline. So: newline-delimited-JSON is the primary/default path (matches the MCP spec's
  stdio transport), and `Content-Length`-framing is accepted as a secondary, LSP-compatible mode
  auto-detected per-message by peeking the first line — not a global negotiated setting. A message longer
  than `MCP_DEFAULT_LIMIT * 1MB` declared in a `Content-Length` header is silently ignored (mcp.c:6198,
  `content_len > 0 && content_len <= MCP_DEFAULT_LIMIT * CBM_SZ_1K * CBM_SZ_1K` guards the call).
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
- **`tools/list` response shape** — confirmed by direct read of `cbm_mcp_tools_list_range`
  (mcp.c:534-570) and its caller `cbm_mcp_tools_list_page` (mcp.c:623-626, wired to the `tools/list` method
  at mcp.c:5970-5971):
  ```json
  { "tools": [ { "name": "index_repository", "title": "Index repository", "description": "...", "inputSchema": {...}, "outputSchema": {"type":"object","additionalProperties":true} } ], "nextCursor": "14" }
  ```
  - Each tool object has exactly 4 keys: `name`, `title`, `description`, `inputSchema` (mcp_add_tool_def,
    mcp.c:522-532) plus a **uniform, non-informative** `outputSchema` — every tool gets the same literal
    `{"type":"object","additionalProperties":true}` (`MCP_TOOL_OUTPUT_SCHEMA`, mcp.c:508), not a per-tool
    schema. No `"$schema"` draft-version key anywhere in the tool literals (confirmed by inspecting every
    registration literal in §2-§9 of this doc) — bare JSON-Schema-like objects, no draft pragma.
  - **Paginated**, not a single flat dump: page size `MCP_TOOLS_PAGE_SIZE` (constant referenced at
    mcp.c:624; 14 tools total fit in one page in practice for this version's tool count, `TOOL_COUNT =
    sizeof(TOOLS)/sizeof(TOOLS[0])`, mcp.c:506). `nextCursor` (a stringified integer offset) is included
    only when more tools remain past the current page (`end < TOOL_COUNT`, mcp.c:561-565); a client resumes
    by echoing that string back as `params.cursor` on the next `tools/list` call
    (`mcp_tools_cursor_offset`, mcp.c:591-621 — non-numeric or negative cursors reset to offset 0; a cursor
    `> TOOL_COUNT` clamps to `TOOL_COUNT`, i.e. an empty final page).
- **`initialize` handshake / server info** — confirmed, `cbm_mcp_initialize_response` (mcp.c:639-680),
  wired at mcp.c:5963-5967:
  ```json
  {
    "protocolVersion": "2025-11-25",
    "serverInfo": { "name": "codebase-memory-mcp", "version": "0.8.1" },
    "capabilities": { "tools": { "listChanged": false } }
  }
  ```
  - `serverInfo.name` is the **hardcoded literal** `"codebase-memory-mcp"` (mcp.c:667), not the
    registry-qualified `server.json` name (`io.github.DeusData/codebase-memory-mcp`) — do not conflate the
    two when porting.
  - `serverInfo.version` comes from `cbm_cli_get_version()` (mcp.c:668), which returns the binary's compiled
    `CBM_VERSION` (== `0.8.1` for this clone) — not read from `server.json` at runtime.
  - **Protocol version negotiation**: server supports 4 versions, newest first (mcp.c:630-637):
    `2025-11-25`, `2025-06-18`, `2025-03-26`, `2024-11-05`. If the client's requested `protocolVersion`
    (from `initialize` params) exactly matches one of these, it is echoed back; otherwise the server's
    newest (`2025-11-25`) is returned regardless of what the client asked for (mcp.c:642-658) — there is no
    hard rejection/error path for an unsupported version, just a silent fallback to latest.
  - `capabilities` advertises only `tools` (with `listChanged: false`, i.e. no dynamic tool-list-changed
    notifications) — no `resources`, `prompts`, or `logging` capability keys are added.
  - Side effects fired on `initialize` (mcp.c:5963-5967, before the response is even built/returned):
    `start_update_check(srv)`, `detect_session(srv)`, `maybe_auto_index(srv)` — session-root detection and
    a possible auto-index kick off as a side effect of the handshake itself, not of any tool call.
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

Registration/schema literal: `src/mcp/mcp.c:315-337`. Dispatch: `mcp.c:5632-5634`. Handler
`handle_index_repository`, `mcp.c:3741-3877`. Cross-repo-intelligence sub-handler `handle_cross_repo_mode`,
`mcp.c:3201-3258`. Persistence artifact mechanics: `src/pipeline/artifact.c` (756 lines).

### 9.1 Params (from the registered `inputSchema` literal, mcp.c:320-337)

| Param | Type | Required | Default | Notes |
|---|---|---|---|---|
| `repo_path` | string | **yes** | — | canonicalized if it exists (`canonicalize_repo_path_if_exists`, mcp.c:3763); path separators normalized (`cbm_normalize_path_sep`, mcp.c:3755) |
| `mode` | enum string | no | `"full"` | `"full"` \| `"moderate"` \| `"fast"` \| `"cross-repo-intelligence"` — see §9.2 |
| `target_projects` | string[] | required **only** for `mode="cross-repo-intelligence"` | — | project names to cross-link against; `["*"]` means all indexed projects (handled inside `cbm_cross_repo_match`, not specially in the MCP layer) |
| `name` | string | no | derived from `repo_path` (`cbm_pipeline_project_name`) | overrides the derived project name; rejected with `"invalid project name"` if it fails `cbm_pipeline_set_project_name`'s validation (mcp.c:3789-3794) |
| `persistence` | boolean | no | `false` | see §9.4 |

### 9.2 Mode semantics — exact, from the enum comments and pipeline gating (`src/pipeline/pipeline.h:36-40`)

```c
CBM_MODE_FULL = 0,     /* Full: everything including SIMILAR_TO + SEMANTICALLY_RELATED */
CBM_MODE_MODERATE = 1, /* Moderate: fast discovery + SIMILAR_TO + SEMANTICALLY_RELATED */
CBM_MODE_FAST = 2,     /* Fast: skip non-essential files, no similarity/semantic edges */
```
Confirmed gating call sites:
- `cbm_set_macro_extraction(p->mode == CBM_MODE_FULL)` (pipeline.c:1228) — C/C++ `#define` Macro nodes are
  extracted **only** in `full` mode (moderate/fast skip them entirely; comment notes macros are ≈49% of
  nodes on macro-dense repos like the Linux kernel, hence the gate).
- Git-history computation (`run_githistory`, pipeline.c:1104-1143) is skipped **only** in `fast` mode
  (`if (p->mode != CBM_MODE_FAST) { ...run githistory... } else { log pass.skip reason=fast_mode }`,
  pipeline.c:1113-1126) — so `full` and `moderate` both compute git history; only `fast` omits it.
- `moderate_only` pass flag (`pipeline.c:718`): passes tagged `moderate_only` are skipped **only** when
  `p->mode == CBM_MODE_FAST` — i.e. `moderate` mode runs the same "moderate-tagged" passes as `full`; only
  `fast` mode drops them. This matches the tool docstring's "moderate: filtered files + similarity/semantic.
  fast: filtered files, no similarity/semantic" (mcp.c:325-327) — `full` vs `moderate` differ only in the
  macro-extraction gate and file-discovery filtering (not independently traced further — file-list filtering
  itself lives in the discovery pass, not read in this cluster); `fast` is the one that drops
  similarity/semantic edges and git-history.
- `mode="cross-repo-intelligence"` **short-circuits before any of the above** — `handle_index_repository`
  detects it first (mcp.c:3765-3771) and dispatches straight to `handle_cross_repo_mode`, which never
  touches the extraction pipeline at all; it only matches Routes/Channels across already-indexed projects
  (`cbm_cross_repo_match`) to create `CROSS_HTTP_CALLS`/`CROSS_ASYNC_CALLS`/`CROSS_CHANNEL`/
  `CROSS_GRPC_CALLS`/`CROSS_GRAPHQL_CALLS`/`CROSS_TRPC_CALLS` edges — see §9.5 for its response shape.
  Per the tool description (mcp.c:317-319), target projects must already have "fresh indexes" — this mode
  does no extraction of its own.

### 9.3 Response shape — normal indexing (`full`/`moderate`/`fast`)

Success (`rc == 0` from `cbm_pipeline_run`, mcp.c:3849-3859, fields assembled by `build_index_success_response`, mcp.c:3379-3469):
```json
{
  "project": "myrepo",
  "status": "indexed",
  "nodes": 1204,
  "edges": 3980,
  "expected_nodes": 1204,
  "expected_edges": 3980,
  "adr_present": false,
  "adr_hint": "Project indexed. Consider creating an Architecture Decision Record: ...",
  "artifact_present": false,
  "excluded": { "dirs": ["node_modules", "vendor"], "count": 2, "truncated": false },
  "skipped_count": 0
}
```
- `status`: `"indexed"` on a clean run, **`"degraded"`** (still `isError:false`, not a failure) when the
  persisted node count falls below `cbm_dump_verify_min_ratio()` of the expected count (durability-loss
  detection after a hard-killed sibling process, mcp.c:3400-3419) — in that case `hint` explains and
  recommends re-running.
  On pipeline failure (`rc != 0`): `"status": "error"`, plus a `hint` recommending `mode='fast'` for
  diagnosis (mcp.c:3860-3865) — **and** the overall tool result is `isError:true` (mcp.c:3874,
  `rc != 0`).
- `expected_nodes`/`expected_edges` only present when the pipeline tracked committed counts
  (`cbm_pipeline_get_committed_counts`, mcp.c:3390); omitted (`exp_nodes < 0`) otherwise.
- `excluded` object present only when ≥1 subtree was skipped during discovery (dirs like
  `node_modules`/`vendor`), capped at 25 entries with `count`/`truncated` (`add_excluded_summary`,
  mcp.c:3278-3295, `INDEX_EXCLUDED_DIR_CAP = 25`).
- `skipped_count` is **always present** (0 on a clean run); when >0, a `skipped` object
  (`{files:[{path,reason,phase}] (<=50), count, truncated}`) plus a `logfile` path are added
  (`add_skipped_summary`, mcp.c:3309-3332, `INDEX_SKIPPED_FILE_CAP = 50`). The FULL uncapped skip list is
  always written to a per-run logfile (`write_skip_logfile`, mcp.c:3338-3357) at
  `$CBM_INDEX_LOG` (env override) or `<cache_dir>/logs/<project>-<unix_epoch>.log` — **only when there were
  skips** (no logfile on a clean run).
- `adr_present`/`adr_hint`: same semantics as `get_graph_schema`'s (§3.2); `adr_hint` omitted when
  `degraded` (mcp.c:3452, `!degraded` guards it) or when an ADR already exists.
- `artifact_present`: whether a persistence artifact (`.codebase-memory/graph.db.zst`) already exists for
  `repo_path` (`cbm_artifact_exists`, mcp.c:3460) — reflects the artifact's state **after** this run, not
  whether persistence was requested this call. `artifact_hint` is added only when **both**
  `persistence=true` on this call **and** the artifact now exists (mcp.c:3462-3466).
- No `timing`/`elapsed_ms` field anywhere in the normal-mode response (contrast with
  `cross-repo-intelligence`'s response in §9.5, which does carry `elapsed_ms`).

### 9.4 Persistence artifact — exact mechanics

- Trigger: `persistence=true` → `cbm_pipeline_set_persistence(p, true)` (mcp.c:3796) before
  `cbm_pipeline_run`; the actual export call site is inside the pipeline's dump-and-persist stage (not
  re-traced line-by-line in this pass — `dump_and_persist_hashes`, `pipeline.c:1182`, calls into
  `src/pipeline/artifact.c`'s `cbm_artifact_export`).
- **Location/filename**: `<repo_path>/.codebase-memory/graph.db.zst` (`CBM_ARTIFACT_FILENAME =
  "graph.db.zst"`, `artifact.h:21`), plus a sidecar `<repo_path>/.codebase-memory/artifact.json`
  (`CBM_ARTIFACT_META = "artifact.json"`, `artifact.h:22`) holding metadata (schema version, commit hash —
  `cbm_artifact_commit` reads `artifact.json`'s `"commit"` key, artifact.c:721-753). Schema version constant
  `CBM_ARTIFACT_SCHEMA_VERSION = 2` (artifact.h:19); `cbm_artifact_exists` (artifact.c:701-717) requires the
  `.zst` file to be non-empty **and** the sidecar's version to be `<= 2`.
- **Export mechanism** (`cbm_artifact_export`, artifact.c:482-… ; doc-comment at artifact.c:1-6): `VACUUM
  INTO` a temp copy of the live SQLite db, optionally (quality=`CBM_ARTIFACT_BEST`) **drop all indexes**
  from that temp copy and re-`VACUUM` for better compression, then zstd-compress (`ART_ZSTD_FAST=3` or
  `ART_ZSTD_BEST=9`, artifact.c:11-12) and write the `.zst` file into a freshly-`mkdir -p`'d
  `.codebase-memory/` directory (permissions `0755`, `ART_DIR_PERMS`, artifact.c:10,497).
- **Bootstrap-on-index** (`try_artifact_bootstrap`, mcp.c:3261-3268, called unconditionally at the top of
  `handle_index_repository` at mcp.c:3801 — independent of the `persistence` param on *this* call): if no
  local `.db` file exists yet for the derived project name AND an artifact already exists at `repo_path`,
  the artifact is imported (decompressed + written into the cache dir) **before** the fresh index run,
  giving teammates a fast-path bootstrap from a committed artifact instead of full re-indexing. This runs
  regardless of whether `persistence=true` was passed on this specific call — it triggers purely on
  "artifact exists but no local db yet."
- Import mechanism (doc-comment, artifact.c:1-6): decompress → write to cache dir → open (SQLite
  auto-creates missing indexes) → integrity check.

### 9.5 `cross-repo-intelligence` mode response (`handle_cross_repo_mode`, mcp.c:3201-3258)

```json
{
  "status": "success",
  "mode": "cross-repo-intelligence",
  "project": "myrepo",
  "projects_scanned": 3,
  "cross_http_calls": 12,
  "cross_async_calls": 4,
  "cross_channel": 0,
  "cross_grpc_calls": 0,
  "cross_graphql_calls": 0,
  "cross_trpc_calls": 0,
  "total_cross_edges": 16,
  "elapsed_ms": 842.5
}
```
`total_cross_edges` is the sum of the six typed counts. `elapsed_ms` is a real (float), unlike anything in
the normal-mode response. Error case (`target_projects` missing/empty/non-array, mcp.c:3211-3218):
```json
{ "error": "target_projects is required for cross-repo-intelligence mode. Use [\"*\"] for all projects. Run list_projects to see available." }
```

### 9.6 Errors

- `repo_path` missing: plain text `"repo_path is required"`, `isError:true` (mcp.c:3757-3761).
- `name` override invalid: plain text `"invalid project name"` (mcp.c:3789-3794).
- Pipeline construction failure: `"failed to create pipeline"` (mcp.c:3784-3788).
- Supervisor-worker crash/hang path (`cbm_index_supervisor_should_wrap`, mcp.c:3745-3750, when the index
  runs in an isolated worker subprocess and that worker crashes or hangs) produces a **distinct** response
  shape via `build_worker_failure_response` (mcp.c:3475-3498), `isError:true`:
  ```json
  { "status": "error", "outcome": "hang", "hint": "Indexing worker timed out (a file made no progress). The worker was terminated and the server survived. Re-run to retry.", "repo_path": "..." }
  ```
  (`outcome` is `"hang"` or the crash variant per `cbm_proc_outcome_str`; exact string values of every
  `cbm_proc_outcome_t` enum member beyond "hang" were **not independently enumerated in this pass** —
  UNVERIFIED, would require reading `index_supervisor.c/.h` in full.)

---

## 10. `list_projects`

Registration: `src/mcp/mcp.c:475-476` (`{"type":"object","properties":{}}` — no params at all, no
`required`). Dispatch `mcp.c:5606-5608`. Handler `handle_list_projects`, `mcp.c:1369-1424`. Per-entry
builder `build_project_json_entry`, `mcp.c:1327-1365`.

### 10.1 How projects are discovered — no central registry

Each project **is** a single `.db` file — `list_projects` scans the cache directory
(`cache_dir`, mcp.c:961-968: `cbm_resolve_cache_dir()`, falling back to `cbm_tmpdir()`) for files matching
`is_project_db_file` (mcp.c:1256-1263: filename ends in `.db`, length `>= MCP_MIN_DB_NAME`, and is not `_...`
or the literal `:memory:`). For each candidate file, the project's **name** is read from inside the db
itself (`db_internal_project_name`, mcp.c:1267-1291 — opens query-mode, requires exactly one row in the
`projects` table with a non-empty name; ghost/empty/corrupt dbs are silently skipped, not listed) — the
filename is not trusted as the project name (comment at mcp.c:1334-1338 explains this guards against a
renamed/copied `.db` file reporting stale node/edge counts under the wrong key).

### 10.2 Response shape

```json
{
  "projects": [
    {
      "name": "myrepo",
      "root_path": "/home/user/code/myrepo",
      "git": { "is_git": true, "is_worktree": false, "is_detached": false, "root_exists": true, "worktree_root": "...", "git_dir": "...", "git_common_dir": "...", "canonical_root": "...", "branch": "main", "branch_slug": "main", "head_sha": "abc123...", "base_sha": "..." },
      "nodes": 1204,
      "edges": 3980,
      "size_bytes": 5242880
    }
  ]
}
```
- No ordering guarantee is imposed by the handler itself — order follows `cbm_readdir`'s OS-level directory
  iteration order (not sorted by name/size/mtime).
- `git` sub-object is added by the same `add_git_context_json` helper used by `index_status` (§12) —
  identical shape in both tools (mcp.c:1150-1169).
- `hint` key (`"No projects indexed. Call index_repository(repo_path=...) first."`) is added only when the
  `projects` array is empty (mcp.c:1413-1416) — **not an error**, `isError:false`.
- Cache-directory-unreadable error (`opendir` failure, mcp.c:1383-1391):
  ```json
  { "error": "cannot read cache directory: <path>", "hint": "Check directory permissions or run index_repository first." }
  ```
  This is the one path in this tool that IS `isError:true`.

---

## 11. `delete_project`

Registration: `src/mcp/mcp.c:477-479` (`project` string, required). Dispatch `mcp.c:5621-5623`. Handler
`handle_delete_project`, `mcp.c:2178-2250`.

### 11.1 What's deleted on disk — exact

Only the project's own `.db` file plus its SQLite WAL/SHM sidecars: `<cache_dir>/<project>.db`,
`<cache_dir>/<project>.db-wal`, `<cache_dir>/<project>.db-shm` (`project_db_path` mcp.c:971-980 +
`cbm_unlink` calls at mcp.c:2212-2214). **Does not touch**:
- the persistence artifact (`.codebase-memory/graph.db.zst` inside the *repo*, not the cache dir) — entirely
  separate storage, untouched by this tool;
- the ADR content (also stored inside the same `.db` file's `project_summaries` table per §14, so it IS
  deleted as a side effect of deleting the `.db` — but there's no separate/independent ADR deletion path).

Side effects before the delete (mcp.c:2185-2229):
1. If the target project is the server's currently-cached open store, it is closed first
   (`cbm_store_close`) and `srv->current_project` is cleared — avoids deleting a file with an open handle.
2. `cbm_pipeline_lock()`/`unlock()` wraps the delete so it can't race an in-progress `index_repository` run
   on the same project.
3. If a filesystem watcher is active for the project, `cbm_watcher_unwatch(srv->watcher, name)` stops it.
4. `cbm_mem_collect()` returns freed allocator pages to the OS after closing the database.

### 11.2 Response shape

```json
{ "project": "myrepo", "status": "deleted" }
```
- `status` values: `"deleted"` (unlink succeeded), `"delete_failed"` (unlink failed on an existing file —
  adds `"error": "<strerror text>"`, `isError:true`), or `"not_found"` (no `.db` file existed for that
  project name at all — **this is also `isError:true`**, mcp.c:2206-2224: `is_error` is set `true`
  whenever `exists` was false, distinct from the "delete succeeded" happy path).
- **Note**: unlike `search_graph`/`get_graph_schema`/etc., the unknown-project error here is a **flat**
  `{"project":..., "status":"not_found"}` object, NOT the `{"error":"project not found or not
  indexed",...,"available_projects":[...]}` shape documented in §2.5/§3.3/§7.6 — `delete_project` never
  calls `build_project_list_error`/`build_no_store_error` at all. This is a real, verified divergence from
  the other tools' error-shape convention, not an oversight in this doc.
- Missing `project` argument entirely: plain text `"project is required"`, `isError:true` (mcp.c:2180-2182)
  — this one IS a plain string, not JSON.

---

## 12. `index_status`

Registration: `src/mcp/mcp.c:481-483` (`project` string, required). Dispatch `mcp.c:5618-5620`. Handler
`handle_index_status`, `mcp.c:2134-2175`.

### 12.1 Response shape

```json
{
  "project": "myrepo",
  "nodes": 1204,
  "edges": 3980,
  "status": "ready",
  "root_path": "/home/user/code/myrepo",
  "git": { "is_git": true, "...": "...same shape as §10.2/§3.2" }
}
```
- `status`: exactly two values — `"ready"` (`nodes > 0`) or `"empty"` (`nodes == 0`, mcp.c:2149). **There is
  no in-progress/"indexing" status value at all** — this tool has no visibility into an in-flight
  `index_repository` run; it only reports the current on-disk node/edge counts and a binary
  ready/empty classification. (The supervised-worker crash/hang machinery in §9.6 is the closest thing to
  "in-progress" state, and it's not surfaced here — a caller polling `index_status` mid-run would just see
  whatever partial counts happen to be committed so far, with no explicit "still running" signal.)
- When `nodes == 0`, a `hint` is added: `"Project is empty. Re-run index_repository(repo_path=...) to
  populate."` (mcp.c:2159-2163).
- `root_path`/`git` are populated only if `cbm_store_get_project` succeeds (mcp.c:2151-2158); note this
  reuses `add_git_context_json`, the exact same helper as §10.2.
- If `get_project_arg` cannot resolve a `project` at all (missing key, no fallback), response is instead the
  degenerate `{"status": "no_project"}` (mcp.c:2164-2166) — **not an error**, `isError:false`.
- Unknown/unindexed project (the store can't even be opened): standard `REQUIRE_STORE` macro fires →
  `build_no_store_error` → the same `{"error":"project not found or not indexed", "hint":..., ,
  "available_projects":[...], "count":N}` shape as §2.5/§3.3/§7.6 (mcp.c:2136-2137, the `REQUIRE_STORE`
  macro at mcp.c:1216-1225).

---

## 13. `detect_changes`

Registration: `src/mcp/mcp.c:485-491`. Dispatch `mcp.c:5641-5643`. Handler `handle_detect_changes`,
`mcp.c:5215-5383`. Per-file symbol lookup `detect_add_impacted_symbols`, `mcp.c:5197-5213`.

### 13.1 Params

| Param | Type | Required | Default | Notes |
|---|---|---|---|---|
| `project` | string | **yes** | — | |
| `scope` | string | no | `"symbols"` (implicit — see below) | `"symbols"` or `"impact"` → include `impacted_symbols`; any other value (e.g. `"files"`) → files only |
| `depth` | integer | no | `2` (`MCP_DEFAULT_BFS_DEPTH`, mcp.c:27) | clamped to `cbm_mcp_max_depth()` (default **15**, override via `CBM_MCP_MAX_DEPTH` env var, `foundation/limits.h:43-44`) via the same `clamp_mcp_depth` helper `trace_path` uses (§5) — **but see the correction below: this param is echoed, not actually used to bound anything** |
| `base_branch` | string | no | `"main"` | validated against shell-metacharacter injection (`cbm_validate_shell_arg`) before being interpolated into a `git diff`/`git -C` shell command |
| `since` | string | no | — | if non-empty, **takes precedence over and replaces `base_branch`** entirely (mcp.c:5226-5236) — routed through the identical `<ref>...HEAD` three-dot diff. So `since` is not a separate mechanism, it's a `base_branch` alias with priority. |

**Correction vs. the tool's own registered description** (mcp.c:488-489, `"since": "Git ref or tag to
compare from ... Diffs <ref>...HEAD"`) — confirmed accurate for the diff semantics, but note `since`
literally overwrites `base_branch` in the handler rather than being a distinct code path.

### 13.2 Change detection mechanism — three merged git sources

Exact shell command (mcp.c:5278-5293, POSIX and Windows variants), merging three sources so nothing is
missed:
1. `git -C <root> diff --name-only <base>...HEAD` — committed changes vs. the merge-base with `base_branch`.
2. `git -C <root> diff --name-only` — unstaged tracked-file changes.
3. `git --no-optional-locks -C <root> status --porcelain --untracked-files=normal` — untracked + staged-new
   files (invisible to `git diff`; comment at mcp.c:5273-5276 notes this was previously missed, causing new
   files to not appear until a manual re-index, issue #520).
Porcelain's 2-char status-code prefix (`"?? path"`, `"A  path"`) is stripped; for a rename line
(`"R  old -> new"`) only the destination path (after ` -> `) is kept (mcp.c:5330-5344).

### 13.3 Risk classification — **VERIFIED ABSENT, not merely unverified**

**This tool has NO risk classification of any kind.** Grepped the full handler and its helpers
(`mcp.c:5194-5383`) and `src/pipeline/pass_gitdiff.c` for `risk`/`CRITICAL`/`HIGH`/`MEDIUM`/`LOW` — zero
matches. The prior scout digest's "risk classification" characterization for `detect_changes` is **wrong**
for this version (`0.8.1`): the CRITICAL/HIGH/MEDIUM/LOW risk-label scheme that exists in this codebase
belongs exclusively to **`trace_path`'s** `risk_labels` param (§5.3 of this doc — hop-distance-based:
hop1=CRITICAL, hop2=HIGH, hop3=MEDIUM, else LOW). `detect_changes` only reports raw changed files +
(optionally) the symbols defined in those files — no severity/impact scoring whatsoever. Implementers should
not port a risk-level enum for this tool; if that capability is wanted for parity, it does not exist in the
baseline and must be original design.

### 13.4 The `depth` param is a **dead/cosmetic parameter** here

Traced `detect_add_impacted_symbols` (mcp.c:5197-5213) in full: it calls `cbm_store_find_nodes_by_file`
for each changed file and adds every non-File/Folder/Project-labeled node defined in that file. **There is
no BFS/graph-traversal step at all in this handler** — `depth` is read, clamped, and only ever written back
into the response as `"depth": depth` (mcp.c:5371); it does not bound anything (contrast with `trace_path`,
where the structurally-identical `clamp_mcp_depth` call genuinely gates a recursive CTE's hop count). This
mirrors the `search_graph` mode-interaction and `query_graph` ceiling discrepancies already flagged
elsewhere in this doc as real, cited divergences between docstring/naming and actual behavior — not a
doc error to "fix."

### 13.5 Response shape

```json
{
  "changed_files": ["src/foo.py", "src/bar.py"],
  "changed_count": 2,
  "impacted_symbols": [
    { "name": "process_order", "label": "Function", "file": "src/foo.py" }
  ],
  "depth": 2
}
```
- `impacted_symbols` is present (possibly `[]`) regardless of `scope`; it is simply left unpopulated (stays
  `[]`) when `scope` is anything other than `"symbols"`/`"impact"` (`want_symbols` gate, mcp.c:5224,
  5352-5354) — the key itself is not omitted.
- No `total`/`has_more`/pagination fields — this tool has none.

### 13.6 Errors

- `base_branch` contains shell metacharacters: plain text `"base_branch contains invalid characters"`,
  `isError:true` (mcp.c:5243-5248).
- Project not found/no store: `build_no_store_error` shape, same as §2.5 (mcp.c:5250-5259, via
  `get_project_root`).
- Project path fails validation: `"project path contains invalid characters"` (mcp.c:5261-5267).
- `git` executable missing / popen failure: `"git diff failed: cannot execute command (<strerror>). Check
  that git is installed."` (mcp.c:5295-5306).
- `git diff` exits non-zero AND zero files were found (likely bad `base_branch`/ref):
  ```json
  { "changed_files": [], "changed_count": 0, "impacted_symbols": [], "depth": 2, "hint": "git diff exited with status <N>. Check that branch '<base_branch>' exists." }
  ```
  `isError:true` in this specific case (mcp.c:5358-5366) — note this is the one error case that still
  carries the full normal response shape (arrays present, just empty) rather than a bare error string/object.

---

## 14. `manage_adr`

Registration: `src/mcp/mcp.c:493-497`. Dispatch `mcp.c:5644-5646`. Handler `handle_manage_adr`,
`mcp.c:5459-5563`. Section-lister `adr_list_sections_from_content`, `mcp.c:5389-5414`. Legacy-file migration
`adr_read_legacy_file`, `mcp.c:5419-5449`.

### 14.1 Params

| Param | Type | Required | Default | Notes |
|---|---|---|---|---|
| `project` | string | **yes** | — | |
| `mode` | enum string | no | `"get"` | `"get"` \| `"update"` \| `"sections"` — `mode_str` defaults to `heap_strdup("get")` when absent (mcp.c:5464-5466); the handler ALSO silently accepts the undocumented alias `"store"` as a synonym for `"update"` (mcp.c:5529 — `strcmp(mode_str,"update")==0 \|\| strcmp(mode_str,"store")==0` — not in the registered enum literal, real but hidden extra value) |
| `content` | string | required for `mode="update"`/`"store"` only | — | the full ADR markdown body — **whole-document replace, not a diff/merge** (see §14.3) |
| `sections` | string[] | registered in the schema (mcp.c:496) but **never read anywhere in the handler** | — | **dead/unused parameter** — confirmed by grepping the full handler body for `"sections"` as an arg key: absent. `mode="sections"` derives its section list by parsing the *stored* ADR content's markdown headers, not from this input array. |

### 14.2 Storage location — SQLite store, not a file (with legacy-file migration)

ADRs live in the **same SQLite `.db` file** as the rest of the project graph, in a `project_summaries`
table accessed via `cbm_store_adr_get`/`cbm_store_adr_store` (comment at mcp.c:5468-5470: "the SAME backend
the UI `/api/adr` endpoints use — so writes via the MCP tool and the UI are visible to each other," issue
#256). `resolve_store` normally opens projects **read-only**; `manage_adr` is called out as "the only
`resolve_store` caller that WRITES" (mcp.c:5482-5489) — it explicitly opens a second, dedicated read-write
handle to the same `.db` file path via `cbm_store_open_path` when the project is file-backed, and uses the
already-writable handle directly only for the in-memory/embedded case.

**One-time legacy migration** (mcp.c:5506-5522, every call, any mode): if the store has no ADR row yet, the
handler checks for an old-style file at `<repo_root>/.codebase-memory/adr.md` (`adr_read_legacy_file`,
mcp.c:5419-5449 — plain `fopen`/`fread`, no special encoding handling) and, if found and non-empty, imports
its content into the store via `cbm_store_adr_store` before proceeding with the requested mode. This makes
`manage_adr` implicitly stateful/mutating even on a plain `mode="get"` call for a project with a legacy
file and no store-backed ADR yet.

### 14.3 Content format

Freeform markdown string — no schema/validation on `content` beyond being present. The `ADR_EMPTY_HINT`
constant (mcp.c:5451-5457) advertises a **6-section convention** (not enforced): `PURPOSE`, `STACK`,
`ARCHITECTURE`, `PATTERNS`, `TRADEOFFS`, `PHILOSOPHY`, as `##`-level markdown headers. `mode="update"`/
`"store"` **replaces the entire stored content wholesale** (`cbm_store_adr_store` — one full-text field, no
append/merge/diff semantics of any kind).

### 14.4 Response shapes by mode

**`update`/`store`** (content provided):
```json
{ "status": "updated" }
```
or, on a store-layer write failure: `{ "status": "write_error" }`, `isError:true` (mcp.c:5529-5535).

**`sections`** — parses `#`-prefixed lines (any heading level, not just `##`) out of the *currently stored*
content (mcp.c:5389-5413, trims trailing `\r`, truncates any single header line at 1023 chars):
```json
{ "sections": ["## PURPOSE", "## STACK"] }
```
If there is no stored ADR at all, `adr_list_sections_from_content` is called with `content=NULL`, which
degenerates to `p = NULL` in the walking loop and the function immediately falls through to an empty
`"sections": []` — **no explicit "no ADR" hint is added in `sections` mode** (contrast with `get` mode
below, which does add one).

**`get`** (default mode):
```json
{ "content": "## PURPOSE\n...full markdown..." }
```
or, when no ADR exists (and no legacy file to migrate):
```json
{ "content": "", "status": "no_adr", "adr_hint": "No ADR yet. Create one with manage_adr(mode='update', content='## PURPOSE\\n...\\n\\n## STACK\\n...\\n\\n## ARCHITECTURE\\n...\\n\\n## PATTERNS\\n...\\n\\n## TRADEOFFS\\n...\\n\\n## PHILOSOPHY\\n...'). For guided creation: explore the codebase with get_architecture, then draft and store. Sections: PURPOSE, STACK, ARCHITECTURE, PATTERNS, TRADEOFFS, PHILOSOPHY." }
```
This is `isError:false` in every mode/branch traced — `manage_adr` has no "ADR not found" error path, only
the `no_adr`-status success response above.

### 14.5 Errors

Only one error path found: no store resolvable for `project` → `build_no_store_error` shape, same as §2.5
(mcp.c:5471-5480). No project-argument-missing-specific text was found distinct from that generic path.

---

## 15. `ingest_traces`

Registration: `src/mcp/mcp.c:499-503`. Dispatch `mcp.c:5647-5649`. Handler `handle_ingest_traces`,
`mcp.c:5567-5597`.

### 15.1 Params (from the registered schema — validated only at the JSON-Schema level, not semantically)

| Param | Type | Required |
|---|---|---|
| `traces` | array of `{caller: string, callee: string, count: integer}` objects (`"additionalProperties":false`) | **yes** |
| `project` | string | **yes** |

### 15.2 **VERIFIED: this tool is an unimplemented stub — it does NOT merge anything into CALLS edges**

Read the full handler body (mcp.c:5567-5597) line by line. It does exactly this:
1. Parse `args` as JSON, look up the `"traces"` key.
2. If present and is a JSON array, record its length as `trace_count`. **The `caller`/`callee`/`count`
   fields of each element are never read, never validated, never touched.**
3. `(void)srv;` — the function takes the server handle and **discards it immediately**; no store lookup,
   no `resolve_store`, no `project` validation despite `project` being a required schema param.
4. Build and return:
   ```json
   { "status": "accepted", "traces_received": 3, "note": "Runtime edge creation from traces not yet implemented" }
   ```
   `isError:false` unconditionally, even if `args` fails to parse as JSON at all (in which case
   `trace_count` simply stays 0 and the same "accepted" shape is returned).

There is **no merge semantics to document** — not sum, not replace, not dedupe — because no CALLS-edge
write of any kind occurs. `grep`'d `src/traces/traces.c` (142 lines) as a sanity check: it implements OTLP
*resource-attribute* extraction helpers (`cbm_extract_service_name`, `cbm_extract_path_from_url`) used
elsewhere for auto-detecting service names from OpenTelemetry data during indexing — an unrelated feature,
not the backing implementation for this MCP tool. **The prior scout digest's characterization ("merges into
CALLS edges") describes a feature that does not exist in `0.8.1`** — likely an intended/planned feature
inferred from the tool's name and schema shape, not from its actual behavior. Implementers targeting true
baseline parity should replicate the stub (accept + count + advisory note), not invent merge semantics; if
the product intent is for enforcer-memory to actually implement trace-based edge enrichment, that is new
design work with no C baseline to port.

### 15.3 Errors

None found — every code path in the handler returns the same `isError:false` "accepted" shape regardless of
malformed input, missing `project`, or an empty/absent `traces` array (`trace_count` just reports as `0`).

---

## 16. CLI form (`cli <tool> <json>`)

Confirmed directly from `src/main.c`'s `run_cli` (main.c:329-487) — the full function was read in this pass
end-to-end (this lives in `main.c`, not `src/cli/cli.c`; `cli.c`'s 4732 lines implement the flag-parsing
helpers `run_cli` calls into — `cbm_cli_build_args_json`, `cbm_cli_print_tool_help` — not the top-level
dispatch loop itself, which is in `main.c`).

### 16.1 Usage and argument resolution

```
Usage: codebase-memory-mcp cli [--progress] [--json] <tool_name> [json_args]
```
(`CLI_USAGE`, main.c:203; printed to **stderr** with exit code `1` when `argc < 1` at either check,
main.c:330-333, 369-372).

Flags `--progress` and `--json` are stripped from argv before tool-name resolution (`cli_strip_flag`,
main.c:237-249) — order-independent, can appear anywhere in the arg list. `--help`/`-h` anywhere after the
tool name short-circuits to `cbm_cli_print_tool_help(tool_name)` and returns **0** on success, or prints
`"error: unknown tool '<name>'"` to stderr and returns **1** if the tool name is unrecognized
(main.c:380-388) — this happens before any server/store work.

**Argument-source precedence** (main.c:390-444), first match wins:
1. `--args-file <path>` — slurp the file's bytes as the JSON args string; missing path arg or unreadable
   file → stderr error, exit **1**.
2. Raw JSON positional arg (`cli <tool> '{"k":"v"}'`) — detected by the first non-whitespace byte being `{`
   (`cli_first_nonspace_is_brace`). **Deprecated**: prints a stderr-only warning ("will be removed in a
   future release; use flags... `--args-file`... or piped stdin") but still works.
3. Flag form (`cli <tool> --flag value --bare-bool`) — first remaining arg starts with `--` → routed through
   `cbm_cli_build_args_json` (in `cli.c`, not independently re-traced in this pass beyond its call site) to
   synthesize the JSON args object from CLI flags. Build failure → stderr `"error: <msg>"`, exit **1**.
4. Piped stdin (`cli <tool> < args.json`) — only when stdin is not a TTY (`cli_isatty(0)` false) AND none of
   the above matched; empty stdin falls back to `"{}"`.
5. Otherwise: bare `"{}"`.

### 16.2 Output format — envelope IS unwrapped by default; `--json` restores it

**This resolves the doc's prior open question**: CLI output is **not** identical to the raw MCP tool result
by default. `cli_print_mcp_result` (main.c:208-234) parses the tool's `{"content":[...],"isError":...}`
envelope and prints **only** `content[0].text` — to **stdout** if `isError` is false, to **stderr** if
`isError` is true (main.c:226-227) — never the surrounding envelope JSON. If the envelope itself fails to
parse as JSON (defensive fallback), it prints the raw `result` string verbatim to stdout instead
(main.c:210-212, 229).
Passing `--json` (main.c:473-474) **bypasses `cli_print_mcp_result` entirely** and prints the full raw
envelope (`{"content":[{"type":"text","text":"..."}],"isError":...}`, exactly the MCP wire shape) straight
to stdout — confirming the doc's prior inference about `--json` toggling a raw-vs-formatted mode was
directionally correct, though the actual mechanism is "skip envelope-unwrapping," not a text-vs-JSON
log-format toggle analogous to `CBM_LOG_FORMAT`.

### 16.3 Exit codes — exact and complete

| Outcome | Exit code | Source |
|---|---|---|
| `argc < 1` (no tool name given at all) | **1** | main.c:330-333 |
| `--help`/`-h` on a known tool | **0** | main.c:386 |
| `--help`/`-h` on an unknown tool | **1** | main.c:382-385 |
| `--args-file` missing its path arg, or file unreadable | **1** | main.c:404-413 |
| Flag-form arg synthesis fails (`cbm_cli_build_args_json` returns NULL) | **1** | main.c:428-432 |
| Server construction fails (`cbm_mcp_server_new` returns NULL) | **1** | main.c:451-457 |
| Tool ran, result `isError:false` (includes an **unknown tool name** reaching `cbm_mcp_handle_tool`, which
  returns a normal `isError:true` text result — see next row, NOT this one) | **0** | main.c:460, 476 (`cli_print_mcp_result` returns 0 when `!is_error`) |
| Tool ran, result `isError:true` (covers **every** tool-level error documented in §2-§15 of this doc,
  INCLUDING an unrecognized tool name — `cbm_mcp_handle_tool`'s fallthrough `"unknown tool: <name>"`,
  mcp.c:5650-5652, is itself an `isError:true` result) | **1** | main.c:476, via `cli_print_mcp_result`'s
  `return is_error ? SKIP_ONE : 0` (`SKIP_ONE` = 1) |
| `--json` flag set (raw envelope printed) | **0** always — the exit code stays its default-initialized
  `0` (main.c:460) and is **never set from the envelope's `isError`** in this branch (main.c:473-474 just
  `printf`s and does not touch `exit_code`) | main.c:460, 473-474 |

**Real, citable divergence worth flagging for the Rust port**: passing `--json` silently loses the
otherwise-correct `isError`→exit-code mapping — a caller scripting against `--json` output must parse the
envelope's `"isError"` field itself rather than trusting the process exit code, whereas the default
(non-`--json`) mode DOES thread `isError` through to the exit code correctly.

### 16.4 Supervised-worker interaction

`--index-worker` and `--response-out <path>` flags (main.c:342-344, stripped before tool dispatch, not part
of the public CLI surface documented in `CLI_USAGE`) mark this CLI invocation as a supervised index worker
subprocess: `cbm_index_set_worker_role` records the role, and after `cbm_mcp_handle_tool` returns, the full
result string is additionally written to the `--response-out` file (main.c:465-472) before the normal
stdout/stderr printing happens — this is the mechanism `handle_index_repository`'s supervisor-wrap path
(§9.6) uses to get the child's result back to the parent process. Not relevant to a direct CLI user, but
relevant if the Rust port needs to replicate the crash/hang-isolation architecture.

### 16.5 `--progress`

Routes through `cbm_progress_sink_init(stderr)`/`cbm_progress_sink_fini()` (main.c:446-448, 482-484) — a
progress-reporting sink initialized on stderr for the duration of the tool call. Internal implementation
(`src/cli/progress_sink.c`) was not traced in this pass — **UNVERIFIED** what specific progress events it
emits or their format; only its stderr-attachment and init/fini lifecycle around the single tool call are
confirmed.

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
- **Resolved**: real JSON-RPC-level *protocol* errors ARE used, but only for the request envelope itself,
  never for tool-call outcomes. Two confirmed sites in the main dispatch loop (`mcp.c:5940-6018`):
  - Unparseable JSON-RPC request: `cbm_jsonrpc_format_error(0, JSONRPC_PARSE_ERROR, "Parse error")`
    (mcp.c:5940-5941) — a standard numeric-`code` JSON-RPC error object (`JSONRPC_PARSE_ERROR` is the
    conventional `-32700`; the exact numeric value was not re-derived from its `#define` site in this pass
    but the constant name follows the JSON-RPC 2.0 spec's reserved code).
  - Unknown `method` (e.g. neither `initialize`/`ping`/`tools/list`/`tools/call`): a real
    `{"code":<JSONRPC_METHOD_NOT_FOUND>,"message":"Method not found"}` error object echoing the original
    request `id` (mcp.c:6001-6010, comment references issue #253 for the id-echo fix).
  So the split is: **transport/protocol-level failures** (bad JSON, unknown RPC method) → genuine numeric
  JSON-RPC error objects; **tool-level failures** (bad args, project not found, etc., i.e. everything in
  §2-§15 of this doc) → always a normal `isError:true` tool **result**, never promoted to a JSON-RPC error.
  `notifications/cancelled` (a notification, no `id`) is handled specially and produces no response at all
  (mcp.c:5945-5956).

---

## 18. Checkpoint note

**Pass 1** (2026-07-05, sonnet-tier) committed and pushed this document after 3 of 4 dispatched research
passes returned (covering `query_graph`, `trace_path`/`get_code_snippet`/`get_architecture`/`search_code`,
and `search_graph`/`get_graph_schema` — 6 of 14 tools, plus the shared JSON-RPC envelope/logging
infrastructure), with the 7th-tool-cluster pass (`index_repository`, `list_projects`, `delete_project`,
`index_status`, `detect_changes`, `manage_adr`, `ingest_traces`, plus the CLI-form and common-error-shape
sections) left intentionally **UNVERIFIED** rather than filled with digest-derived guesses.

**Pass 2** (2026-07-05, sonnet-tier, same day — this pass) completed the remaining 7 tools (§9-§15), the
envelope/transport gaps in §1 (stdio framing, `tools/list` wrapping, `initialize` handshake), the CLI form
(§16), and the JSON-RPC-vs-tool-error split in §17, all re-cloned from the same `DeusData/codebase-memory-mcp`
tag `v0.8.1` and cited to `file:line`. Two of the prior scout digest's characterizations were found to be
**factually wrong for this version** and are called out explicitly rather than silently corrected: (a)
`ingest_traces` does not merge anything into `CALLS` edges — it is an unimplemented stub that only echoes an
accepted-count (§15.2); (b) `detect_changes` has no risk-classification scheme at all — the CRITICAL/HIGH/
MEDIUM/LOW risk-label scheme belongs solely to `trace_path`'s `risk_labels` param (§13.3). A few narrower
items remain genuinely unverified and are flagged in place rather than gathered here — see §9.6 (exact
`cbm_proc_outcome_t` string values beyond `"hang"`), §16.5 (`progress_sink.c`'s internal event format), and
§0's long-standing `token_vectors` population question. None of these block X06.2-X06.9 implementation; all
14 tools now have complete, cited param/response/error documentation.
