//! `enforcer-rules` — the structured rule registry (rules-as-data), arc-04.
//!
//! # Charter
//!
//! Rules are STRUCTURED DATA, never prose. Every [`registry::RuleRecord`]
//! carries the full 5-way linkage the plan's doctrine requires: `ruleId <->
//! validator <-> {fail+pass fixtures} <-> doc-anchor <-> tier`. `.md` may
//! still exist as optional human-canonical reading (rendered by the g08
//! rules-&-skills UI explorer); the engine and every AI consumer read the
//! structured record here, never the prose.
//!
//! This crate owns the registry SKELETON:
//! - [`registry`] — the [`registry::RuleRecord`] shape and the loaded,
//!   validated [`registry::RuleRegistry`].
//! - [`loader`] — parse-at-boundary JSON loading (`enforcer-config`
//!   conventions: read bytes, parse, validate, never a partially-loaded
//!   registry escapes as a live value).
//! - [`version_drift`] — d13: detects when a rule record's declared
//!   `version` is out of sync with a change to its validator/fixtures/
//!   doc-anchor, so a rule cannot silently drift from its parity artifacts.
//!
//! It also SHIPS the baseline T1 rule records under `rules/**`: the
//! `[workspace.lints]` deny-wall (rules-as-data mirror of the deny set
//! a01 hard-codes into THIS workspace's root manifest, for checking
//! consumer repos), the `no-reexports` discipline (validator owned by
//! arc-06's `enforcer-lang-rust`, linked here by [`enforcer_domain::ids::RuleId`]),
//! and the `ocentra-parent` posture records (`publicReexportPolicy`,
//! runtime-literal ban, domain-typed serialized fields,
//! `blockedProtocolDependencies`) that PORT that profile's Rust posture as
//! explicit rule DATA. Feature packs that add further rule records own
//! their own `rules/<name>.json` file (disjoint-owns), depending on this
//! crate for the shape.
//!
//! This registry CONSUMES `enforcer-config`'s `sourceShapePolicies` base
//! shape (arc-03) as the substrate for the `ocentra-parent` posture
//! records; it does not redefine or re-home that base shape.
//!
//! No `pub use` barrels (workspace doctrine): consumers path through the
//! modules directly, e.g. `enforcer_rules::registry::RuleRegistry`.

pub mod loader;
pub mod registry;
pub mod rules;
pub mod version_drift;

/// Load-time / structural failure for the rule registry.
#[derive(Debug, thiserror::Error, PartialEq, Eq, Clone)]
pub enum RuleLoadError {
    /// The rule catalog's bytes were not valid JSON, or a record did not
    /// decode into [`registry::RuleRecord`]'s typed shape.
    #[error("rule catalog parse failed at `{path}`: {reason}")]
    Parse {
        /// Source path of the offending catalog file.
        path: String,
        /// Underlying decode/parse reason.
        reason: String,
    },

    /// The catalog file could not be read from disk.
    #[error("failed to read rule catalog `{path}`: {reason}")]
    Io {
        /// Path that failed to read.
        path: String,
        /// Underlying I/O failure description.
        reason: String,
    },

    /// Two records in the same catalog declared the same [`RuleId`]
    /// (`enforcer_domain::ids::RuleId`).
    #[error("duplicate ruleId `{rule_id}` in rule catalog")]
    DuplicateRuleId {
        /// The colliding rule id, verbatim.
        rule_id: String,
    },

    /// A record's linkage fields (validator/fixtures/doc-anchor/title) are
    /// structurally empty or otherwise malformed.
    #[error("malformed rule record `{rule_id}`: {reason}")]
    MalformedRecord {
        /// The offending rule id, verbatim.
        rule_id: String,
        /// Human-readable reason for the rejection.
        reason: String,
    },
}

/// Result alias for `enforcer-rules` load/validate operations.
pub type RuleResult<T> = std::result::Result<T, RuleLoadError>;
