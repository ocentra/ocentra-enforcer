# arc-24 Crate enforcer-ui

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Crate enforcer-ui`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `crates/enforcer-ui/**`
- deps: `arc-01`, `arc-02`, `arc-15`, `arc-21`
- tier: `P1`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md), [RUST_ARCHITECTURE](../RUST_ARCHITECTURE.md).

## Where We Are
There is no desktop/served UI. Track G is specced as Tauri (Rust backend + TS/web frontend). Per doctrine, the ONLY remaining TS in the product is this UI frontend (presentation only); no business logic lives in TS, and its types must be DERIVED from `enforcer-domain` rather than hand-written — nothing yet generates or drift-guards them.

## Where We Want To Be
`enforcer-ui` is the Track G crate per RUST_ARCHITECTURE.md: it stands up the crate SKELETON — the UI server / Tauri backend (Rust) serving a self-contained HTML fallback for headless use and backing the Tauri desktop app, plus the Rust->TS type-generation pipeline. The frontend is TS/web living under `crates/enforcer-ui/frontend/` (the sole TS in the product, presentation only); the backend calls `enforcer-scan`/`enforcer-mcp` — no business logic in TS. Frontend types are DERIVED from `enforcer-domain` via `#[derive(ts_rs::TS)]` -> an export bin/xtask -> committed generated `.ts`, guarded by a fail-closed `cargo test` drift test (byte-compare committed vs freshly-emitted). It hosts the Track G feature modules (g01-g08).

## Requirement Checklist
- [ ] Stand up the `enforcer-ui` crate skeleton per RUST_ARCHITECTURE.md: a UI server that exposes scan/report data (via `enforcer-scan`/`enforcer-mcp`), a served self-contained HTML fallback for headless mode, and the Tauri backend (Rust commands) wiring the desktop app to the engine crates.
- [ ] Implement the Rust->TS type-generation pipeline: `#[derive(ts_rs::TS)]` on the `enforcer-domain` DTOs the frontend consumes, an export bin/xtask that emits the committed generated `.ts` under `crates/enforcer-ui/frontend/`, and a FAIL-CLOSED `cargo test` drift test that byte-compares the committed `.ts` against a fresh emit (stale/hand-edited types fail the build). camelCase wire casing.
- [ ] Render `enforcer-domain` `Report`s (findings/violations/tiers) into the UI data model at the Rust boundary; the frontend only presents. The TS/web frontend is the ONLY TS in the product and holds no business logic.
- [ ] Provide a cross-language fixture round-trip test: a fixture `Report` serialized by the Rust backend deserializes in the frontend type model (and back) without loss — proving the derived TS types match the Rust wire contract across the language boundary.
- [ ] `cargo test -p enforcer-ui` passes with fail/pass fixtures: the backend renders a fixture `Report` into the expected UI payload (pass), an empty/clean report yields the empty-state payload (pass), a malformed request is rejected (fail fixture), the ts_rs drift test passes on committed types and FAILS on a mutated domain type, and the cross-lang round-trip holds. Frontend build (if present) type-checks (frontend-only — the allowed Tauri TS surface; non-binding on the Rust engine dogfood).
- [ ] Clean `cargo clippy` / `cargo fmt --check` (obey `[workspace.lints]`; no `pub use` barrels).

## Acceptance And Proof
Tier P1. Proof row asserts `cargo test -p enforcer-ui` exits 0 — backend report-to-payload rendering proven with fail/pass fixtures, the served-HTML fallback smoke test passes, the fail-closed ts_rs drift test passes (and is proven to fail on a domain-type mutation), and the cross-lang fixture round-trip holds. Record the artifact path in TEST_PROOF_EXPECTATIONS.md.

## Parallel Ownership Notes
Owns the `enforcer-ui` crate SKELETON: `crates/enforcer-ui/Cargo.toml`, `src/lib.rs`, the UI-server/Tauri-backend root, the `ts_rs` export bin/xtask + fail-closed drift test + cross-lang round-trip harness, and the bundled `frontend/` scaffold (the sole TS in the product — presentation only, types derived not hand-written). Deps arc-01/02/15/21.

Parallel Ownership Note (disjoint feature packs): the Track G feature packs each own SPECIFIC modules under this crate — g01 `src/serve.rs` (Tauri shell + served HTML fallback); g02 `src/report/`; g03 `src/actions/`; g04 `src/run_dispatch/` (deps arc-16); g05 `src/settings/` (config control-plane, writes routed through arc-23 c-track adapters); g06 `src/hub/` (live lane/claim/lease/mail panel); g07 `src/security/`; g08 `src/explorer/` (rules-&-skills explorer, where the human-canonical `.md` is browsed — the AI still reads the structured rule) — each with its own frontend assets and fixtures. They own their modules, NOT the whole crate; they `deps:` arc-24 and are sequenced after this skeleton. owns stay DISJOINT BY FILE. Parallel-safe with arc-23 (install) — disjoint crate trees. Last surface; presentation only.
