//! g05 — settings / config control-plane, mounted into the arc-24 shell's
//! `settings` view slug (see [`crate::serve::VIEW_MOUNTS`]).
//!
//! **Ownership**: g05 owns `crates/enforcer-ui/src/settings/*`,
//! `crates/enforcer-ui/src/bin/export_ts.rs`, and
//! `crates/enforcer-ui/tests/ts_drift.rs` per `WORKPACK_INDEX.md`. The
//! type-gen bin + drift test were laid WORKING by arc-24 (this pack
//! inherits, not builds, that machinery); this module is the actual
//! settings *behavior* arc-24 left as a mount-point seam.
//!
//! # Read/write surface
//! - [`read`] renders the human control-plane view (discovered per-project
//!   native ties + declarative [`enforcer_config::policy::Policy`]) purely
//!   from the arc-03 typed load API (`enforcer_config::project_tie::
//!   load_project_tie`) — no hardcoded defaults, no raw file peeking.
//! - [`write`] is the ONLY mutation route: every write goes through the
//!   typed [`enforcer_config::project_tie::ProjectConfig`] model, validated
//!   via [`enforcer_config::project_tie::ResolvedProjectTie::resolve`]
//!   BEFORE any byte touches disk (fail-closed on a malformed waiver), then
//!   serialized once through `serde_json` — never a raw string/regex edit
//!   of the config file. Kept in its own module (distinct from [`read`],
//!   which is GET-only and mutates nothing) so g07's same-origin/CSRF
//!   guard layer can wrap exactly this route without touching the read
//!   path.
//!
//! Proof row: `proof/ui/g05-settings.json` (`settings-config-writes` +
//! `ts_drift`) per `TEST_PROOF_EXPECTATIONS.md`.

pub mod read;
pub mod write;
