# x08 — Cross-Harness Worklog (the unified AI work trail)

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Workpack x08 Cross-Harness Worklog`
> Kind: workpack (Track X, cross-cutting)
> owns: `crates/enforcer-coordination/src/worklog.rs`, `crates/enforcer-coordination/src/worklog/**`, `crates/enforcer-cli/src/worklog.rs`, `crates/enforcer-ui/src/worklog/**`, `crates/enforcer-coordination/tests/worklog.rs`, `crates/enforcer-coordination/tests/fixtures/worklog/**`
> deps: arc-16 (ledger read APIs), d04 (telemetry run records), arc-17 (proof read model), c05 (SessionStart event), arc-25 (event spine), g01 (view mount for the UI surface)
> tier: P3 (T1 read-model correctness; the surfaces are additive)
> Read when: claiming x08, or wiring any surface that should emit or render worklog entries.
> Stop rule: read-model ONLY — this pack must not add new write paths to the ledger; it aggregates records that already exist.
> Proves: rows in TEST_PROOF_EXPECTATIONS.md (x08).
<!-- /agent-capsule -->

Sources: [PRODUCT_THESIS](../../PRODUCT_THESIS.md), owner feature request 2026-07-12 (real-world prompt: a
LinkedIn post — "I am running four AI tools in parallel… Four accounts, overlapping chats, half-finished
ideas scattered across all of them. **The tooling multiplied. The tracking did not.**").

## Where we are

Every harness the enforcer is installed into (Codex, Claude, generic MCP consumers, c06–c09 adapters)
already leaves machine-readable traces at the enforcer chokepoint, but nothing aggregates them for a
human:

- **d04 telemetry**: hash-chained NDJSON run records (timestamp, root, tool, scope, outcome) for every
  scan/check/run.
- **arc-16 ledger**: hubs, lanes, presence (with writer/harness identity context), file claims (exact
  paths + operations), mail, closeouts.
- **c05 SessionStart**: a zero-effort "work started here, with this harness" event.
- **arc-17 proof**: what was proven done, when, by which lane.

A person running four AI tools has no unified answer to "what did I work on, where, with which tool,
and how did it end" — the old `.bash_history`-style trail died inside vendor silos. The enforcer is the
only component positioned across ALL of them.

## Where we want to be

`enforcer worklog` answers that question in one place, from records that already exist:

- **Read-model** (`enforcer-coordination/src/worklog.rs`): a `WorklogEntry` projection folding ledger
  events + d04 run records + proof claims into a per-day, per-project, per-harness timeline. Branded
  types throughout; parse-at-boundary for the NDJSON/ledger inputs (boundary module per doctrine).
  Fields: when, project root, harness/agent identity, kind (session-start | scan | check | run | claim |
  mail | proof | closeout), subject (files/rule/lane), outcome.
- **CLI**: `enforcer worklog --root <repo> [--since 2d] [--harness codex|claude|…] [--json]` — the
  modern replacement for the "time, directory, project, previous command" script. Also
  `worklog summary` (per-day rollup: projects touched, tools used, claims opened/closed, proofs).
- **MCP tool**: `ocentra_enforcer_worklog` (same params) so any harness can answer "what was I doing"
  from inside a chat.
- **UI**: `crates/enforcer-ui/src/worklog/**` timeline view mounted via g01 (per-day lanes grouped by
  project, harness badges, click-through to run/proof detail). Honors f04 silent mode.
- **Honesty**: entries carry their source record ids (hash-chain refs) — the trail is tamper-evident,
  and a `coverage` line states which sources were readable vs missing (a09 spirit: never silently thin).

Explicitly OUT of scope: capturing chat content from vendor tools (we log enforcer-observed work, not
conversations); any new ledger write path; cross-machine sync (x06 federation owns that seam).

## Requirement checklist

- [ ] `WorklogEntry` + `WorklogQuery` read-model over arc-16 ledger events (fold, no mutation), with
      boundary module for record decoding.
- [ ] d04 NDJSON run-record ingestion (reuse d04's reader; do not re-parse ad hoc).
- [ ] arc-17 proof-claim ingestion (proof read model API).
- [ ] Harness identity attribution from presence context; `unknown` is an explicit variant, never a
      silent default.
- [ ] CLI `enforcer worklog` + `worklog summary` (human table + `--json`), registered like sibling
      subcommands.
- [ ] MCP tool `ocentra_enforcer_worklog` on the router-consolidated surface.
- [ ] UI timeline view under `src/worklog/**`, g01 view-mount registration, f04-gated.
- [ ] Coverage/honesty line: sources read, records folded, sources unavailable.
- [ ] Fixtures: a synthetic multi-harness day (codex + claude sessions, claims, runs, a proof) with
      golden CLI/JSON output; fail fixture proves tampered chain / missing source is reported, not
      hidden.

## Acceptance and proof

- `cargo test -p enforcer-coordination` worklog suite green: fold correctness, harness attribution,
  ordering, tamper/missing-source honesty, golden summary.
- CLI integration test (enforcer-cli): synthetic fixture day → golden `worklog` and
  `worklog summary --json` output.
- Doctrine gate ZERO on all new files.
- Proof rows in TEST_PROOF_EXPECTATIONS.md flip GREEN with artifact paths.

## Parallel ownership notes

New disjoint files only; one-line additive registrations in `enforcer-coordination/src/lib.rs`,
`enforcer-cli` command table, `enforcer-mcp` tool registry, and the g01 view-mount list. Disjoint from
g06 (hub dashboard renders live hub state; worklog renders the historical human timeline). Reads x06
nothing (worklog is ledger/telemetry/proof only); x06 memory recall may later cite worklog entries —
that seam belongs to x06.
