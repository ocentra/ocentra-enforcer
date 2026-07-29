//! `enforcer-lang-security` — the per-family `Validator` implementations
//! for the security source-pattern rule family (`SEC-1`, `SEC-2`, 22 rules
//! total).
//!
//! # Charter
//!
//! This crate hosts the Rust-native replacement for the ad-hoc
//! `src/source-policy-common-security*.mjs` and
//! `src/generic-common-line-rules.mjs` (`scanSecretLine`) detection logic.
//! It builds on [`enforcer_validator::validator::Validator`] and proves
//! every rule through [`enforcer_validator::harness::run_fixture_parity`].
//!
//! Distinct from `enforcer-security` (Track H money-critical/
//! security-testing validators, a different crate/workpack entirely): this
//! crate is the per-family source-PATTERN validator for the `security`
//! rule family only (dangerous-shape/secret detections), not a general
//! security-testing harness.
//!
//! Two validator families cover the 22 rules (see
//! `docs/plans/enforcer-selfhost-plan/workpacks/arc-10-enforcer-lang-security.md`
//! for the authoritative per-prefix table):
//!
//! - [`rules::secret_scan`] — `common/secret-scan` (`SEC-1.1` inline
//!   secrets, `SEC-1.2` sensitive file paths). Fully self-contained in this
//!   crate; ported from `src/source-policy-common-security-sensitive.mjs`
//!   and `src/generic-common-line-rules.mjs`'s `scanSecretLine`.
//! - [`rules::generic_scanner`] — the security slice (20 rules,
//!   `SEC-2.1`..`SEC-2.20`) of the cross-family `generic-scanner` engine
//!   (plus its `generic-scanner-redaction`/`common/security` siblings,
//!   which this crate treats as the same content-line-scan shape). This
//!   crate owns only the SEC-2 rows of that shared engine's trigger table
//!   (the rule SEMANTICS + fixtures), not the shared engine itself — the
//!   engine and its common/python/typescript partition are owned by arc-09
//!   (`src/generic-common-scanner.mjs` / `generic-scanner-shared.mjs`). No
//!   double-own: arc-09 does not define the SEC-2 rule logic, and this
//!   crate does not re-implement the generic-scanner engine.
//!
//! [`rules::registry`] enumerates every one of the 22 `SEC-*` rule ids with
//! its owning validator and fixture pair, and is the single source the
//! count-parity completeness test in `tests/completeness.rs` walks.

pub(crate) mod boundary;
pub mod rules;
