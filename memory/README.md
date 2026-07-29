# memory/ — transitional capture streams (graph destiny: x06)

<!-- agent-capsule -->
> Agent Capsule
> Kind: append-only NDJSON memory streams + schema. The LIVE capture surface until the Rust engine exists.
> Read when: writing a memory record (any lane), seeding recall context, or executing x05/x06 (import source).
> Stop rule: ONE WRITER PER STREAM FILE — you append ONLY to `streams/<your-lane>.ndjson`. Never edit or
> reorder existing lines; supersede via a new record with `supersedes`. Validate against
> `schema/memory-record.schema.json` before appending.
> Graph destiny (L18 rule): records flush -> DuckDB (interim analytics) -> x06 `enforcer-memory` graph +
> vector index. After x05 `lesson import` + x06 land, these files stop being source-of-truth and become a
> generated view. Ids are permanent; the graph keeps them.
<!-- /agent-capsule -->

## Why
Owner requirement (2026-07-04): every interaction is a memory update — user preferences, decisions, lessons,
observations — captured NOW as structured data so the finished system boots with a real dataset instead of
starting amnesiac. The Rust engine (x05 capture -> x06 graph/recall) takes over this job; until then, every
agent writes NDJSON here and the orchestrator reports back what was updated ("lesson learnt updated, user pref
updated") each time it happens.

## Rules
1. **One writer per file**: `streams/primary.ndjson` (orchestrator), `streams/<lane>.ndjson` (workers) —
   append-only per-writer streams cannot merge-conflict across worktrees (same pattern as hub event streams).
2. **Schema-valid or it doesn't land**: `schema/memory-record.schema.json` (v1). camelCase, branded-id-shaped,
   mirrors x05 `LessonRecord`/x06 `MemoryNode` so import is a mapping, not a rewrite.
3. **Append-only**: never edit a line; supersede with a new record.
4. **d22/L18 size caps apply**: a stream at ~500 lines rolls to `streams/<writer>-NN.ndjson`.
5. **Redaction before any export**: user-attributed records stay `personal` tier until the x06 consent gates exist.
