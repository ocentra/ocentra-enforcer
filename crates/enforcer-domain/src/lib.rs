//! `enforcer-domain` — the SINGLE-SOURCE schema crate (arc-02).
//!
//! # Charter
//!
//! **This crate OWNS DTO SHAPE, NOT BEHAVIOR.** It is a serde-only,
//! dependency-light LEAF: branded newtypes and versioned wire records that
//! every other crate in the workspace parses AT ITS BOUNDARY instead of
//! threading raw strings/objects. There is exactly ONE domain crate with
//! modules — never per-feature `*-domain` crates.
//!
//! Rules of this crate:
//! - Every identifier is a branded newtype validating on construction
//!   (fallible `TryFrom`/`FromStr` returning [`boundary::decode_error::DecodeError`]); no public raw-string
//!   constructor exists, so no invalid value can be constructed.
//! - Wire casing is camelCase on MCP/UI-facing types (locked decision).
//! - UI-facing types derive `ts_rs::TS` so arc-24's export bin and
//!   fail-closed drift test regenerate the committed `.ts` from here.
//! - Versioned records carry `schemaVersion` + `eventType` and RIDE the
//!   mechanisms owned elsewhere: the NDJSON sink / hash-chain / redaction
//!   MECHANISM lives in `enforcer-core` (arc-01), the event
//!   envelope/dispatch in `enforcer-events` (arc-25).
//! - No `pub use` barrels: consumers path through the modules directly,
//!   e.g. `enforcer_domain::ids::RuleId`.

pub mod boundary;
pub mod cli_types;
pub mod config_types;
pub mod coordination_types;
pub mod core_types;
pub mod doctrine_profile_types;
pub mod events_types;
pub mod findings;
pub mod harness_types;
pub mod hashes;
pub mod ids;
pub mod install_types;
pub mod language_types;
pub mod mcp_types;
pub mod mechanization_types;
pub mod memory_types;
pub mod paths;
pub mod plan_types;
pub mod proof_types;
pub mod records;
pub mod rules_types;
pub mod run_record;
pub mod scan_types;
pub mod security_types;
pub mod semantic_types;
pub mod severity;
pub mod syntax_types;
pub mod telemetry_types;
pub mod test_doctrine_types;
pub mod ui_logic_coupling_types;
pub mod ui_types;
pub mod xtask_types;
