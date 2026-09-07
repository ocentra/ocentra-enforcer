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
//! - [`waiver`] — a path-and-rule-scoped, typed exception registry. It is
//!   deliberately separate from `enforcer-config`'s project-wide rule
//!   toggles: a waiver can only suppress one known rule for one exact file.
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

pub mod boundary;
#[path = "boundary/cyberskills_disposition.rs"]
pub mod cyberskills_disposition;
pub mod loader;
pub mod registry;
pub mod rules;
pub mod version_drift;
pub mod waiver;

use enforcer_domain::ids::RuleId;
use enforcer_domain::rules_types::{RuleCatalogSource, RuleFailureReason};

/// Load-time / structural failure for the rule registry.
#[derive(Debug, thiserror::Error, PartialEq, Eq, Clone)]
pub enum RuleLoadError {
    /// A JSON boundary value could not convert into its canonical rule type.
    #[error("rule catalog boundary conversion failed: {reason}")]
    Boundary { reason: RuleFailureReason },
    /// The rule catalog's bytes were not valid JSON, or a record did not
    /// decode into [`registry::RuleRecord`]'s typed shape.
    #[error("rule catalog parse failed at `{path}`: {reason}")]
    Parse {
        /// Source path of the offending catalog file.
        path: RuleCatalogSource,
        /// Underlying decode/parse reason.
        reason: RuleFailureReason,
    },

    /// The catalog file could not be read from disk.
    #[error("failed to read rule catalog `{path}`: {reason}")]
    Io {
        /// Path that failed to read.
        path: RuleCatalogSource,
        /// Underlying I/O failure description.
        reason: RuleFailureReason,
    },

    /// Two records in the same catalog declared the same [`RuleId`]
    /// (`enforcer_domain::ids::RuleId`).
    #[error("duplicate ruleId `{rule_id}` in rule catalog")]
    DuplicateRuleId {
        /// The colliding rule id, verbatim.
        rule_id: RuleId,
    },
}

/// Result alias for `enforcer-rules` load/validate operations.
pub type RuleResult<T> = std::result::Result<T, RuleLoadError>;

/// Convert a displayable boundary failure into the canonical non-empty reason.
#[must_use]
pub fn boundary_reason(value: impl std::fmt::Display) -> RuleFailureReason {
    // ALLOC-JUSTIFICATION: the typed failure owns rendered boundary diagnostics.
    let text = value.to_string();
    RuleFailureReason::from_diagnostic(text)
}
