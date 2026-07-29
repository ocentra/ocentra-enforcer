//! POLICY-SPEC-INGESTION (h08): map an arbitrary project's security/testing
//! spec doc into a mechanized [`spec::MechanizedProfile`] — generalizing
//! "target repo owns policy" per `RUST_ARCHITECTURE.md`.
//!
//! # Pipeline
//!
//! 1. [`parse::parse_spec`] turns raw `.mdc` text into a typed
//!    [`spec::PolicySpec`] (parse-at-boundary; [`error::PolicyIngestError`]
//!    on malformed input — never a silent default).
//! 2. [`map::map_to_profile`] maps that spec against the
//!    [`backing::BackedRuleCatalog`] snapshot to produce a
//!    [`spec::MechanizedProfile`] plus [`enforcer_domain::findings::Finding`]s
//!    for every asserted-but-unbacked rule.
//!
//! # The honesty seam
//!
//! Backed rules — a `RuleId` with a real mechanized
//! [`enforcer_validator::validator::Validator`] already registered in this
//! crate — become ENABLED profile rows (`backed: true`). Rules the source
//! spec asserts but that have no such validator are never silently
//! accepted as enforced: they stay in the profile as visibly `backed:
//! false` rows AND produce a structured `Finding` flagging them for
//! mechanization, to be fed into d01's rule-scaffold engine (and tracked
//! by d08). This is deliberate — an un-backed asserted rule that looked
//! identical to an enabled one would report enforcement that does not
//! exist; this module makes the gap visible instead of erasing it.
//!
//! This module does not itself define new Track H rule ids or validators
//! — it consumes the crate's already-registered rule ids by string only
//! (see [`backing::BackedRuleCatalog::track_h_snapshot`]) and never opens
//! or edits `src/rules/<name>.rs` / `src/rules/registry.rs` (the shared
//! arc-19 seam those feature packs own).

#[path = "boundary/policy_ingest/backing.rs"]
pub mod backing;
#[path = "boundary/policy_ingest/error.rs"]
pub mod error;
#[path = "boundary/policy_ingest/map.rs"]
pub mod map;
#[path = "boundary/policy_ingest/parse.rs"]
pub mod parse;
#[path = "boundary/policy_ingest/spec.rs"]
pub mod spec;
