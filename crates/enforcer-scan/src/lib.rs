//! `enforcer-scan` — the CONVERGENCE pack (arc-15): the parallel scan
//! engine that routes every landed language family into one
//! `enforcer-domain::Report` surface.
//!
//! # Charter
//!
//! Where the legacy `.mjs` engine lived as serial `scripts/rust-rules-
//! scan-*.mjs` / `rust-rules-source-scan.mjs` / `rust-rules-source-
//! classification.mjs` with ad hoc routing, this crate is the Rust
//! replacement: a `rayon`-based CPU-bound fan-out over files
//! ([`engine`]), the detect-and-route router that dispatches each file to
//! the right language-family [`enforcer_validator::validator::Validator`]s
//! ([`router`], f05), and the scan modes ([`modes`], f01). It aggregates
//! every family crate's findings (arc-06..12 lang validators, arc-13's
//! scored literal scanner) into one [`enforcer_domain::findings::Report`].
//!
//! Three seams this skeleton lays but does not fill (owned by sibling
//! feature packs, sequenced by `deps: arc-15`):
//! - [`modes`] — scan-mode selection/orchestration (f01).
//! - [`router`] — per-path classification + family dispatch (f05).
//! - [`rules::baseline_ratchet`] — the monotonic violation-count baseline
//!   (d02).
//!
//! This skeleton owns and fully implements:
//! - [`scope`] — the **tri-modal scope resolver**: `<paths...>` | `--base
//!   <sha> --head <sha>` | `--all` → a canonical `ScanScope`. Windows-first
//!   (argv-quoting + backslash normalization via `enforcer_core::platform`),
//!   with NO override flag — exactly one of the three modes is ever active.
//! - [`walk`] — the **ignored-segments walk**: skips `target/`, `.git/`,
//!   vendored/generated dirs, and `enforcer-config`'s
//!   `ignoreDirs`/`ignoreFileGlobs` while walking a resolved scope, plus
//!   the **idempotency guard**: deterministic finding/file ordering so
//!   re-scanning the same scope yields a byte-identical `Report`, and
//!   parallel and serial runs agree.
//! - [`engine`] — the rayon fan-out itself: walks a scope, routes each file
//!   through the currently-wired family registries, and folds every
//!   family's findings into one `Report`.
//! - [`outcome`] — the anti-silent-skip primitive (a09): every dispatch
//!   decision is an explicit `Outcome::Ran { .. } | Outcome::Skipped {
//!   reason }` with a guaranteed-non-empty reason, so a validator that ran
//!   on nothing cannot look identical to one that ran and passed.
//! - [`coverage`] — the scan-coverage accounting (a09): aggregates
//!   per-target `Outcome`s into ran/skipped counts + a skip-reason list,
//!   and hard-fails (`Coverage::require_nonzero_ran`) when the total
//!   ran-count is zero — a scan that checked nothing is never a clean
//!   pass.
//!
//! No `pub use` barrels (workspace doctrine): consumers path through the
//! modules directly, e.g. `enforcer_scan::engine::scan`.

pub mod ai_rule_index;
pub mod architecture_policy;
pub mod boundary;
pub mod cargo_workspace_policy;
pub mod coverage;
pub mod docs_completeness;
pub mod doctor;
pub mod engine;
pub mod generated_artifacts;
pub mod import_boundaries;
pub mod modes;
pub mod mutation_risk;
pub mod onboard;
pub mod outcome;
pub mod router;
pub mod rules;
pub mod sbom_policy;
pub mod scope;
pub mod single_source_contracts;
pub mod source_shape;
pub mod string_boundaries;
pub mod test_doctrine;
pub mod ui_logic_coupling;
pub mod walk;
