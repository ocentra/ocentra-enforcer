//! `enforcer-ui` — UI server / Tauri backend skeleton (arc-24, Track G).
//!
//! # Charter
//!
//! Per `RUST_ARCHITECTURE.md` ("Presentation only is TS"), the enforcer's
//! desktop cockpit is Tauri: a Rust backend (this crate) plus a TS/web
//! frontend under [`frontend/`](../frontend) — the ONLY TS surface left in
//! the product, and it is presentation-only. No business logic lives in
//! TS: the backend calls straight into `enforcer-scan`/`enforcer-mcp` and
//! renders their `enforcer-domain::findings::Report` output into a UI
//! payload shape at the Rust boundary; the frontend just displays it.
//!
//! This workpack (arc-24) lays the crate SKELETON only:
//! - [`payload`] — renders an `enforcer-domain::findings::Report` into the
//!   [`payload::UiReportPayload`] the frontend consumes, plus the
//!   empty-state and malformed-request rejection paths. Fully implemented
//!   here (not a mount point) because it is the one seam every Track G
//!   feature pack needs before any of them can render anything.
//! - [`ts_export`] — the Rust->TS type-generation pipeline: calls
//!   `ts_rs::TS::export_all_to` for every UI-facing `enforcer-domain`/
//!   [`payload`] type. This workpack's own acceptance criteria require a
//!   PROVEN fail-closed drift test to exist and pass now, so arc-24 lays
//!   a working `src/bin/export_ts.rs` + `tests/ts_drift.rs` +
//!   `tests/cross_lang_roundtrip.rs` on top of this module -- per
//!   `WORKPACK_INDEX.md` those two file paths are formally g05's `owns:`
//!   (settings' golden-config-diff work extends them further), so g05
//!   inherits a working pipeline/drift-test/round-trip harness rather
//!   than building one from scratch, exactly as this workpack's Track G
//!   preamble in `TEST_PROOF_EXPECTATIONS.md` describes.
//! - Mount points for the Track G feature packs (g01..g08). Per
//!   `WORKPACK_INDEX.md`, g02/g03/g04/g05/g06/g08 each own a
//!   `crates/enforcer-ui/src/<pack>/` DIRECTORY (this workpack creates
//!   only each directory's `mod.rs` seam, empty of behavior); g07 owns
//!   `src/security/*` + `tests/ui_security/**`; g01 owns the single flat
//!   file `src/serve.rs` (this workpack creates only the minimal
//!   served-HTML fallback + view-mount registry smoke-tested by its own
//!   proof row -- g01 replaces/extends the rest).
//!
//! No `pub use` barrels (workspace doctrine): consumers path through the
//! modules directly, e.g. `enforcer_ui::payload::render_report`.

/// g01 (Tauri shell + served HTML fallback). This workpack lays the
/// self-contained headless-served-HTML smoke path; the Tauri desktop
/// shell wiring itself is g01's to fill.
pub mod serve;

/// g02 (report view: violation-matrix completeness, grouping, silent-mode
/// suppression). Mount point only — g02 fills this module.
pub mod report;

/// g03 (waiver-honesty actions: named waivers, never silent suppression).
/// Mount point only — g03 fills this module.
pub mod actions;

/// g04 (run-dispatch: fix-intent schema at the boundary, ledger write via
/// arc-16 `enforcer-coordination`). Mount point only — g04 fills this
/// module.
pub mod run_dispatch;

/// g05 (settings / config control-plane; writes routed through arc-23
/// c-track adapters). Mount point only — g05 fills this module.
pub mod settings;

/// g06 (live lane/claim/lease/mail hub panel, read-only view over arc-16
/// materialized state). Mount point only — g06 fills this module.
pub mod hub;

/// g07 (UI-security surface). Mount point only — g07 fills this module.
pub mod security;

/// g08 (rules-&-skills explorer: the human-canonical `.md` browsed by a
/// human while the AI still reads the structured rule). Mount point
/// only — g08 fills this module.
pub mod explorer;

/// Renders `enforcer-domain::findings::Report` into the UI data model at
/// the Rust boundary (arc-24-owned; the seam every Track G pack renders
/// through). Frontend types are DERIVED from this module + `enforcer-
/// domain` via [`ts_export`].
pub mod payload;

/// The Rust->TS type-generation pipeline: `ts_rs::TS::export_all_to` over
/// every UI-facing type, feeding the committed `frontend/src/bindings/`
/// output and the fail-closed drift test.
pub mod ts_export;
