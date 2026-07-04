//! g05 mount point — settings / config control-plane (writes routed
//! through arc-23 c-track adapters).
//!
//! **Ownership**: g05 owns `crates/enforcer-ui/src/settings/*`,
//! `crates/enforcer-ui/src/bin/export_ts.rs`, and
//! `crates/enforcer-ui/tests/ts_drift.rs` per `WORKPACK_INDEX.md`. This
//! workpack (arc-24) creates this `mod.rs` seam plus a WORKING
//! `export_ts.rs` bin + `ts_drift.rs` drift test + `cross_lang_
//! roundtrip.rs` harness (arc-24's own acceptance criteria require a
//! proven fail-closed drift test now, see [`crate::ts_export`]) — g05
//! inherits these rather than building them from scratch, and extends
//! `ts_drift.rs` with its own settings-specific golden-config-diff
//! scenarios. No settings *behavior* lands here from this workpack.
//!
//! Proof row: `proof/ui/g05-settings.json` (`settings-config-writes` +
//! `ts_drift`) per `TEST_PROOF_EXPECTATIONS.md`. Not proved by this
//! workpack (this workpack proves the drift/round-trip machinery only,
//! not g05's settings-write behavior).
