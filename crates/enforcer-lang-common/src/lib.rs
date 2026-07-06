//! `enforcer-lang-common` — the common-family `Validator` crate (arc-09).
//!
//! # Charter
//!
//! This crate implements the `Validator` (from `enforcer-validator`) for
//! every `RuleId` in `rules/rules.json` whose `language` field is
//! `"common"`, EXCEPT the `SEC-2` family (20 rules), which is semantically
//! owned by `enforcer-lang-security` (arc-10) per the workpack's explicit
//! SEC-2 decision — arc-10 depends on this crate only for the shared
//! `generic-scanner` engine ([`pattern`]), not for SEC-2's rule bodies.
//! This crate's count-parity set is therefore 249 of the 269
//! `language==common` rules (269 − 20 SEC-2).
//!
//! Rule families covered (see [`families`] for the full per-prefix list,
//! and the workpack's "Rule inventory (per-prefix)" table for the
//! authoritative mapping): governance/manifest, docs/policy,
//! contract-family dispatch/registry surfaces (`PROOF-1`, `MCP-1`,
//! `SCAN-1/2`, `HAR-1/2` — rule families that validate an arbitrary TARGET
//! REPO's proof registry / MCP surface / harness wiring, distinct from and
//! not "covered by" the runtime crates that implement those subsystems),
//! CI/repo governance, source-shape/architecture boundaries, waiver policy,
//! test-quality, and the shared `generic-scanner` line/pattern engine.
//!
//! # Structure
//!
//! - [`pattern`] — the generic keyword/marker-based detection engine
//!   ([`pattern::PatternValidator`]) that the vast majority of common-family
//!   rules are built on, matching the dominant "line/keyword scan" shape of
//!   the ported `.mjs` sources (`source-policy-common*.mjs`, `check-*.mjs`).
//! - [`port_platform`] — the one bespoke, non-pattern validator: `PORT-1.1`,
//!   which needs `enforcer-config`'s resolved `supportedPlatforms` to scope
//!   its check rather than a fixed marker list.
//! - [`families`] — one module per `RuleId` prefix (`ARCH-1`, `CI-1`, ...),
//!   each building its slice of [`pattern::PatternValidator`]s.
//! - [`rules`] — bespoke non-`PatternValidator` common-family rules that
//!   need more than a literal-marker scan (e.g. `DEFER-1.1`'s structured
//!   annotation grammar), one module per rule, mirroring
//!   [`port_platform`]'s existing bespoke-validator precedent.
//! - [`registry`] — [`registry::all`], the single entry point returning
//!   every validator this crate owns (the arc-09 count-parity set).
//! - [`rules`] — NEW-mechanism rule families scaffolded via d01 that sit
//!   OUTSIDE the legacy `rules.json` `language==common` count-parity set:
//!   [`rules::fsm`] (d16, FSM transition validity — ADBP_GAPS.md rows
//!   41-50), [`rules::size_shape`] (d22, size/shape caps — ADBP_GAPS.md
//!   rows 91-94), and [`rules::resilience`] (d10, resilience auditor —
//!   ADBP_GAPS.md rows 82-85, `d20-resilience-obligations` folded in).
//!   Disjoint files/tests/fixtures from [`families`]/[`registry`]; not
//!   included in [`registry::all`]'s count and not part of
//!   `tests/parity.rs`'s legacy-catalog assertion.
//!
//! No `pub use` barrels (workspace doctrine): consumers path through the
//! modules directly, e.g. `enforcer_lang_common::registry::all`.

pub mod error;
pub mod families;
pub mod pattern;
pub mod port_platform;
pub mod registry;
pub mod rules;
