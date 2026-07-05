//! `enforcer-security` — Track H: money-critical & security-testing
//! `Validator` implementations, per RUST_ARCHITECTURE.md.
//!
//! # Charter
//!
//! This crate is the Rust-native home for the doctrine's Track H
//! validators: financial-correctness / money-handling rules and
//! security-testing-shape rules, keyed to `RuleId`s in `enforcer-rules`
//! and tagged with `ThreatId` (MITRE/OWASP, from `enforcer-domain::ids`)
//! where applicable. It builds on [`enforcer_validator::validator::Validator`]
//! and proves every rule through
//! [`enforcer_validator::harness::run_fixture_parity`], exactly like every
//! other lang/security validator crate in this workspace.
//!
//! It is also the home of the **no-bypass meta-check**
//! ([`rules::no_bypass`]): `enforcer-config` is the single DECLARATIVE
//! control-plane (owner/exempt globs, allow-regex, per-rule toggles,
//! `cfg(test)` skipping) that both the CLI and UI surfaces read. There is
//! never an inline-disable — the enforcer ships NO inline-suppress escape
//! hatch. The no-bypass meta-check bans inline lint-disable /
//! validation-bypass directives (`#[allow(...)]` on enforcer-governed
//! lints, `// eslint-disable`, `# noqa`, `# type: ignore`, `@ts-ignore`,
//! `clippy::allow` on the deny wall, and any ad-hoc suppress comment)
//! wherever they appear in scanned source, across languages — the only
//! legitimate exemption path is a declarative, committed, gated waiver
//! read from `enforcer-config` (owner + reason + ruleId), never an inline
//! comment.
//!
//! Distinct from `enforcer-lang-security` (arc-10): that crate owns the
//! per-family SOURCE-PATTERN security rule family (`SEC-1`, `SEC-2` —
//! dangerous-shape/secret detections). This crate owns the money-critical
//! and no-bypass slice of Track H; the two are disjoint rule families
//! sharing the same `Validator` base.
//!
//! # Crate shape (this workpack: SKELETON only)
//!
//! This workpack (arc-19) stands up the crate skeleton and the no-bypass
//! meta-check ONLY:
//!
//! - [`rules`] is the module-root every feature pack mounts a
//!   `rules::<name>` submodule into.
//! - [`rules::registry`] is the single `Validator`-registration seam:
//!   [`rules::registry::build_all`] enumerates every rule id this crate
//!   owns, paired with its constructed validator. Feature packs add their
//!   rows here as they land their own `src/rules/<name>.rs` — this
//!   skeleton wires the no-bypass meta-check's row only.
//! - [`rules::no_bypass`] is the one fully-implemented validator this
//!   workpack owns.
//!
//! The Track H money-critical (h01-h08) and security-testing (h11)
//! feature packs each own their OWN `src/rules/<name>.rs` module (+ a
//! `src/rules/<name>/` dir if needed) and their own
//! `tests/fixtures/<name>/**` — not this crate's `owns:` set. They `deps:`
//! this workpack and land their `Validator` rows into
//! [`rules::registry::build_all`] once this skeleton exists. Where an
//! irreplaceable engine is needed (symbolic-exec/fuzz/network-scan), a
//! feature pack's validator should route through `enforcer_harness`'s
//! (arc-18) graceful-skip run-adapter seam (e.g.
//! `enforcer_harness::parsers::missing_tool_skip`) rather than an ad-hoc
//! shell-out — this crate does not itself shell out to any external tool.

pub mod cyberskills;
pub mod policy_ingest;
pub mod rules;
