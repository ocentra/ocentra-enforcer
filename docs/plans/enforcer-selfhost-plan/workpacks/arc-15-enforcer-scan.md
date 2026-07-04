# arc-15 Crate enforcer-scan

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Crate enforcer-scan`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `crates/enforcer-scan/Cargo.toml`, `crates/enforcer-scan/src/lib.rs`, `crates/enforcer-scan/src/engine.rs`, `crates/enforcer-scan/src/scope.rs`, `crates/enforcer-scan/src/walk.rs`, `crates/enforcer-scan/tests/**` (SKELETON only — feature modules owned by siblings: `src/modes.rs` f01, `src/router/**` f05, `src/rules/baseline_ratchet.rs` d02)
- deps: `arc-01`, `arc-02`, `arc-03`, `arc-04`, `arc-05`, `arc-06`, `arc-07`, `arc-08`, `arc-09`, `arc-10`, `arc-11`, `arc-12`, `arc-13`, `arc-25`
- tier: `P1`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md), [RUST_ARCHITECTURE](../RUST_ARCHITECTURE.md).

## Where We Are
The scan engine, the detect-and-route router (f05), and the scan modes (f01) live in `scripts/rust-rules-scan-*.mjs` / `rust-rules-source-scan.mjs` / `src/*scan*.mjs` as serial JS with ad hoc routing. There is no parallel Rust scan engine unifying the validator families.

## Where We Want To Be
`enforcer-scan` is the parallel scan engine per RUST_ARCHITECTURE.md: rayon-based CPU-bound fan-out over files, the detect-and-route router (f05) that dispatches each file to the right language-family validators, and the scan modes (f01). It produces `enforcer-domain` `Report`s aggregating findings from all family crates + literal-scan. Per the consumer-contract borrow it exposes a **tri-modal scope resolver** (`src/scope.rs`): `<paths...>` explicit paths | `--base <sha> --head <sha>` git-diff range | `--all` whole-tree — resolving to a canonical `ScanScope` (`enforcer-domain` newtype), Windows-first (argv-quoting + backslash normalization), with NO override flag. It walks the tree with **ignored-segments** filtering (`src/walk.rs`: skip `target/`, `.git/`, vendored/generated dirs, and `enforcer-config` owner/exempt globs) and an **idempotency guard** so re-scanning the same scope yields a byte-identical `Report` (deterministic ordering; parallel and serial runs agree). It also hosts **d02 baseline-ratchet** (`src/rules/baseline_ratchet.rs`): a monotonic baseline so the violation count can only decrease — new violations fail closed, fixed ones ratchet the baseline down.

## Requirement Checklist
- [ ] Implement the parallel scan engine (rayon fan-out) per RUST_ARCHITECTURE.md, consuming the `Validator`s from arc-06..12 and the scored scanner from arc-13.
- [ ] Implement the detect-and-route router (f05, `src/router/**`): classify each path/scope and route it to the correct language-family validators.
- [ ] Implement the scan modes (f01, `src/modes.rs`) and emit an `enforcer-domain` `Report` aggregating all findings/violations with correct tiers/severity.
- [ ] Implement the **tri-modal scope resolver** (`src/scope.rs`): `<paths...>` | `--base <sha> --head <sha>` | `--all` -> a canonical `ScanScope`, Windows-first (argv-quoting + backslash normalization), no override flag.
- [ ] Implement the **ignored-segments walk** (`src/walk.rs`): skip `target/`/`.git/`/vendored/generated dirs + `enforcer-config` owner/exempt globs while walking the resolved scope.
- [ ] Implement the **idempotency guard**: deterministic finding ordering so re-scanning the same scope yields a byte-identical `Report`, and parallel/serial runs agree.
- [ ] Implement **d02 baseline-ratchet** (`src/rules/baseline_ratchet.rs`): a monotonic baseline where new violations fail closed and fixed ones ratchet the recorded baseline down.
- [ ] Port the `.mjs` scan-engine + args + classification + routing logic (`scripts/rust-rules-scan-*.mjs`, `rust-rules-source-classification.mjs`, `rust-rules-source-scan.mjs`) to Rust.
- [ ] `cargo test -p enforcer-scan` passes: fixture repo trees route correctly (fail fixture: a planted violation is found and routed to the right family; pass fixture: a clean tree produces an empty report); each scope mode resolves to the expected file set (paths / base..head diff / all); the walk skips ignored segments; an idempotency test proves two runs over the same scope produce byte-identical `Report`s and parallel==serial; d02 baseline-ratchet fails closed on a new violation and ratchets down on a fixed one.
- [ ] Clean `cargo clippy` / `cargo fmt --check`.

## Acceptance And Proof
Tier P1. Proof row asserts `cargo test -p enforcer-scan` exits 0 — routing + fan-out produce the expected `Report` on fail/pass fixture trees; the tri-modal scope resolver yields the right file set per mode; the walk excludes ignored segments; the idempotency guard proves byte-identical `Report`s across repeated runs and parallel==serial; d02 baseline-ratchet fails closed on a new violation and ratchets down on a fix. Record the artifact path in TEST_PROOF_EXPECTATIONS.md.

## Parallel Ownership Notes
arc-15 owns the crate SKELETON + baseline of `enforcer-scan`: `Cargo.toml` (`[lints] workspace = true`), `src/lib.rs`, the rayon engine (`src/engine.rs`), scan modes (f01, `src/modes.rs`), the tri-modal scope resolver (`src/scope.rs`), the ignored-segments walk + idempotency guard (`src/walk.rs`), the detect-and-route router (f05, `src/router/**`), and d02 baseline-ratchet (`src/rules/baseline_ratchet.rs`), plus `tests/**`. Deps the full validator-family frontier (arc-06..13) plus foundation and arc-03 (`enforcer-config`, for owner/exempt globs the walk honors); it is the integration point above them. Parallel-safe with arc-16 (coordination), arc-18 (harness) — disjoint crate trees. Precedes arc-17 (proof) and the surfaces.

Parallel-ownership boundary (disjoint-owns model): scan-feature packs that add scan rules or router adapters own SPECIFIC files under this crate — a scan-rule pack owns `crates/enforcer-scan/src/rules/<name>.rs` (+ `tests/fixtures/<name>/**`), a router-adapter pack owns a specific `src/router/<name>.rs`, and each `deps: arc-15` so it lands after this skeleton, engine, and router root exist. They do NOT own the whole crate. f01/f05/d02 are hosted in THIS skeleton, not spun out as feature packs. Keep owns DISJOINT by file; sequence by `deps:`.
