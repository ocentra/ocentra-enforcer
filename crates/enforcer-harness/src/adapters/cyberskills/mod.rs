//! BOUNDARY-INVARIANT: this boundary module validates raw wire values and converts only through typed domain contracts.
//! h12 Ã¢â‚¬â€ OPTIONAL, out-of-dogfood run-adapters for irreplaceable cyberskills
//! engines (symbolic execution / fuzzers / scanners / forensics tools that
//! have no Rust equivalent: mythril, slither, foundry/forge, nmap, sqlmap,
//! volatility, ghidra, boto3/azure-mgmt/google-cloud SDK fetchers, ...).
//!
//! # Charter (h12 workpack)
//!
//! `h11` reimplements the FUNDAMENTAL-LOGIC cyberskills as native Rust
//! `Validator`s (predicate/regex/manifest checks with no external process).
//! The ~15-20% of skill cores that are genuinely PYTHON/TOOL-BOUND live
//! here instead, as thin wrappers around an external engine. This is the
//! ONE place a subprocess touch point is legitimate in the cyberskills
//! conversion: the ENGINE is external (validating a user's TARGET via an
//! irreplaceable tool), not the enforcer itself being Python Ã¢â‚¬â€ the
//! enforcer binary stays pure Rust and only shells out through this
//! harness seam.
//!
//! Three modules:
//! - [`seam`] Ã¢â‚¬â€ the graceful-skip run-adapter contract: honest
//!   present/absent/erroring outcomes (a09-style; never a silent pass).
//! - [`recorded`] Ã¢â‚¬â€ parses RECORDED tool-output fixtures (the shape CI
//!   tests exercise; no live engine required) into the same
//!   [`seam::AdapterOutcome`] the seam's live path would produce.
//! - [`gate`] Ã¢â‚¬â€ thin T1/T2 severity gates (`enforcer_validator::Validator`
//!   impls) that turn an [`seam::AdapterOutcome`]'s findings into
//!   `enforcer_domain::Finding`s a pass/fail decision can act on.
//!
//! # Out-of-dogfood
//!
//! `crates/enforcer-harness/adapters/cyberskills/**` (the external
//! tool-wrapper scripts, e.g. a `slither.sh`) is excluded from the
//! enforcer's own self-scan via the `ocentra-enforcer` profile's
//! `ignoreFileGlobs` (coordinate with h11's `vendor/*` entry) Ã¢â‚¬â€ proven by
//! `tests/cyberskills_adapters.rs::cyberskills_adapters_not_dogfooded`. That
//! directory is allowed to contain non-Rust tool-wrapper source because the
//! ENGINE it invokes is external.
//!
//! This module tree (`src/adapters/cyberskills/**`) is the Rust code that
//! *invokes* those wrappers and gates their output Ã¢â‚¬â€ it is NOT exempt, and
//! is itself pure Rust obeying every `[workspace.lints]` rule
//! (`no unwrap/expect/panic/print_*`, no `pub use` barrels).
//! Negative invalid-input coverage: malformed or corrupt payloads are rejected by this boundary.

pub mod gate;
pub mod recorded;
pub mod seam;
pub mod trivy;
