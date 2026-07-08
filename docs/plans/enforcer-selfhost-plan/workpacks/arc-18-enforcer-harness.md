# arc-18 Crate enforcer-harness

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Crate enforcer-harness`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `crates/enforcer-harness/**` (run-adapters/parsers module + the run-storage/retention module + their tests)
- deps: `arc-01`, `arc-02`
- tier: `P1`

> Scope note: arc-18 has TWO halves. (A) run-adapter PARSING: shell native tools, parse output into `enforcer-domain` diagnostics, compact + graceful-skip. (B) run-STORAGE: persist each run under `.enforce/runs/<id>/`, maintain `.enforce/db/ingest-manifest.json` + `duckdb-status.json`, run the retention/prune engine, and expose read/query/reset APIs that back 6 MCP tools (run_status/diagnostics/last_failure/artifact/prune_runs/reset_runs) + the `runs` CLI. Both halves live in `src/harness.mjs` today and both are owned here. Half B is the source of AUDIT_FINDINGS WAVE 2 gaps G1/G2/G3 and WAVE 4 (harness run-store MCP tools).

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md), [RUST_ARCHITECTURE](../RUST_ARCHITECTURE.md).

## Where We Are
Two halves live in `src/*.mjs` today, neither ported to Rust:

- Run-adapter PARSING: adapters that run native tools (cargo, tsc, ruff, dart, CFLint, ...) and compact their output into diagnostics live in `src/harness-parsers-*.mjs` and related `.mjs`.
- Run-STORAGE + retention (`src/harness.mjs`): every completed run is persisted under `.enforce/runs/<runId>/` with `raw/stdout.log`, `raw/stderr.log`, `diagnostics.ndjson`, `events.ndjson`, and `summary.json`; a run index/manifest is written to `.enforce/db/ingest-manifest.json`; a `.enforce/db/duckdb-status.json` is stamped (mode `optional`, `available:false`, pointing at a reserved `db/harness.duckdb`) because `DEFAULT_HARNESS_CONFIG.store = 'ndjson-duckdb'`. A retention/prune engine (`pruneRuns`) enforces `maxRuns` (50), `maxRunsPerTool` (20), `maxFailedRuns` (20), `pruneAfterDays` (14) with keep/pin logic, and runs on every `run` write. Read/query APIs `listRuns`/`runSummary`/`runDiagnostics`/`lastFailure`/`readArtifact` plus `resetRuns`/`pruneRuns` back the MCP tools run_status/prune_runs/reset_runs/last_failure/artifact and the `runs` CLI. Secret redaction (`SECRET_REDACTION_PATTERNS` / `redactSecrets`) is applied to stdout/stderr/artifact bytes. Storage reads a **dual root**: `candidateStorageRoots` reads both `.enforce` and legacy `.ocentra-enforcer` (`LEGACY_STORAGE_DIR`) and dedupes by `runId` — dropping this loses pre-migration run history.

There is no Rust crate providing either half.

## Where We Want To Be
`enforcer-harness` is the Rust native-tool run crate per RUST_ARCHITECTURE.md, covering both halves:

- PARSING: shells out to native tools (cargo/tsc/ruff/dart/CFLint/...), parses their output into `enforcer-domain` diagnostics, produces compact diagnostics, and is the graceful-skip seam where an external engine is irreplaceable (report skip, do not hard-fail).
- STORAGE: persists each run to the `.enforce/runs/<runId>/{raw/stdout.log,raw/stderr.log,diagnostics.ndjson,events.ndjson,summary.json}` layout, maintains `.enforce/db/ingest-manifest.json` (append-or-replace by runId) and stamps `.enforce/db/duckdb-status.json`; runs the retention/prune engine on write and via explicit prune/reset; and exposes `listRuns`/`runSummary`/`runDiagnostics`/`lastFailure`/`readArtifact`/`resetRuns`/`pruneRuns` as the query surface behind the run-store MCP tools and `runs` CLI. It consumes `enforcer-core` redaction (a single shared redactor) rather than re-inlining the pattern list, and reads the legacy `.ocentra-enforcer` root alongside `.enforce` (dedupe by runId) so existing installs keep their run history — coordinated with arc-23 install/migration.

  The `ndjson-duckdb` store: NDJSON is authoritative; DuckDB ingestion is an OPTIONAL seam (`duckdb-status.json` = `mode:optional, available:false` until duckdb is present). The Rust port must preserve this seam — either port an optional duckdb ingestion path or defer it while still stamping `duckdb-status.json` and keeping NDJSON authoritative. State the chosen posture (port-or-defer) in the crate.

## Requirement Checklist

### Half A — run-adapter parsing
- [ ] Implement run-adapters per RUST_ARCHITECTURE.md for the native tools (cargo/tsc/ruff/dart/CFLint...), each parsing tool output into `enforcer-domain` findings/diagnostics.
- [ ] Implement compact diagnostics (the condensed output format) and graceful-skip when a tool is absent (report skip, do not hard-fail) per the distribution doctrine.
- [ ] Port the `.mjs` harness-parser logic (`src/harness-parsers-*.mjs`) to Rust.
- [ ] Fixture proof: canned tool-output samples parse to the expected diagnostics — fail fixture (a real error line -> finding) PASSES only when the finding is produced; pass fixture (clean output -> none) FAILS if any finding is emitted; missing-tool case yields a graceful skip (not a hard-fail).

### Half B — run storage layout [G1]
- [ ] Persist each run to `.enforce/runs/<runId>/` with exactly `raw/stdout.log`, `raw/stderr.log`, `diagnostics.ndjson`, `events.ndjson`, `summary.json`; `summary.json` carries the artifact-relative paths + `storage.root` + retention summary. Fixture: after a recorded run, all five files exist with the expected relative paths in `summary.json`; a fail fixture asserts a run missing any of the five is rejected/repaired.
- [ ] Maintain `.enforce/db/ingest-manifest.json` as the run index (append-or-replace by `runId`; entries carry tool/status/crateName/packageName/domain/tags/duckdb/summaryPath/ingestedAt). Fixture: two runs -> two manifest entries; re-recording the same runId replaces (not duplicates) its entry.
- [ ] Apply `enforcer-core` redaction to stdout/stderr/artifact bytes before write/return (do NOT re-inline the pattern list). Fixture: a seeded secret in tool output is `[REDACTED]` in `raw/stdout.log` and in `readArtifact` output; a control fixture with no secret is byte-preserved.

### Half B — retention/prune engine [G1]
- [ ] Implement the prune engine honoring `maxRuns`/`maxRunsPerTool`/`maxFailedRuns`/`pruneAfterDays` with keep/pin logic (nullable = unlimited), invoked on every run write AND via explicit `pruneRuns`. Fixture: with `maxRuns=2`, recording 3 runs prunes the oldest and `summary.pruned` lists it; a pinned/kept run and a within-`maxFailedRuns` failed run survive prune; a run older than `pruneAfterDays` is removed while a fresh one is kept.
- [ ] Implement `resetRuns` (clear the run store). Fixture: after reset, `listRuns` is empty and the run directories are gone.

### Half B — query surface (backs 6 MCP tools + `runs` CLI) [G1, WAVE 4]
- [ ] Implement `listRuns`/`runSummary`/`runDiagnostics`/`lastFailure`/`readArtifact` with the query filters (runId/status/tool/limit). Fixture: `listRuns` returns newest-first; `lastFailure` returns the most recent `status==failed` run + its top-N diagnostics; `runDiagnostics` reads `diagnostics.ndjson` for a run; `readArtifact` returns redacted, byte-capped (`maxArtifactBytes`) content. These back run_status/run_status/prune_runs/reset_runs/last_failure/artifact + `diagnostics` MCP tools and the `runs` CLI — parity asserted so none silently drop.

### Half B — ndjson-duckdb store seam [G2]
- [ ] Preserve the `ndjson-duckdb` store contract: NDJSON authoritative; stamp `.enforce/db/duckdb-status.json` (`mode:optional`, `available:false`, `database: db/harness.duckdb`, detail string) on write. Port-or-defer the duckdb ingestion path and STATE the chosen posture in the crate. Fixture: after a run, `duckdb-status.json` exists with `available:false` and NDJSON diagnostics are complete/authoritative without duckdb installed.

### Half B — legacy dual-read migration [G3]
- [ ] Read both `.enforce` and legacy `.ocentra-enforcer` storage roots and dedupe by `runId` (legacy read-only, `.enforce` authoritative for writes). Coordinate with arc-23 install/migration. Fixture: a run present only under `.ocentra-enforcer` is surfaced by `listRuns`; a runId present in both roots appears once; dropping the legacy root is a fail fixture (lost run history).

### Cross-cutting
- [ ] `cargo test -p enforcer-harness` passes with all fail/pass fixtures above (parsing + storage + retention + query + seam + legacy).
- [ ] Clean `cargo clippy` / `cargo fmt --check`.

## Acceptance And Proof
Tier P1. Proof row asserts `cargo test -p enforcer-harness` exits 0 across BOTH halves — canned tool-output parsing (fail/pass) + graceful-skip, AND run-storage layout, retention/prune, query surface (listRuns/runSummary/runDiagnostics/lastFailure/readArtifact/resetRuns/pruneRuns), ndjson-duckdb seam, and legacy dual-read all proven by fail/pass fixtures. Record the artifact path in TEST_PROOF_EXPECTATIONS.md.

## Parallel Ownership Notes
Owns only `crates/enforcer-harness/**` — both the parser module and the run-storage/retention module plus their tests. Deps arc-01/02 only, so it can proceed early in parallel with the validator track. Parallel-safe with arc-15/arc-16/arc-17 — disjoint crate trees. Coordination seams (no shared files): consumes `enforcer-core` redaction (arc-01/02); the run-store query surface is CONSUMED by the MCP crate (arc-21) and the CLI `runs` subcommand (arc-22) — expose stable function signatures, they wire to them. The legacy `.ocentra-enforcer` -> `.enforce` dual-read is COORDINATED with arc-23 install/migration — arc-18 owns the read/dedupe logic, arc-23 owns install-time copy/move.
