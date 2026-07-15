//! c10 — CI integration and binary bootstrap: the enforcer's OWN release
//! pipeline, the reusable `enforcer-scan` composite GitHub Action, the
//! portable `install.sh`/`install.ps1` scripts, and the optional npm
//! wrapper package. RUST_ARCHITECTURE.md, "CI integration for CONSUMER
//! projects (c10)" (binding) is this module's charter; see also the
//! workpack at
//! `docs/plans/enforcer-selfhost-plan/workpacks/c10-ci-integration-and-binary-bootstrap.md`.
//!
//! # Producer vs. consumer
//!
//! This repo's OWN CI builds and PUBLISHES the release
//! ([`release_pipeline`]); every consumer project's CI only ever POINTS
//! AT it, via one of [`installer_scripts`] (the universal path),
//! [`github_action`] (GitHub-specific, adds caching), or [`npm_wrapper`]
//! (optional, for consumers already wired the Node-centric way). Disjoint
//! from [`crate::emitters::consumer_ci`] (c07), which writes a
//! CONSUMER's OWN per-repo workflow set (codeql/secret-scan/sbom/etc.),
//! never a release-pipeline or Action-definition file.

pub mod branch_protection;
pub mod boundary {
    pub mod branch_protection;
}
pub mod branch_protection_domain;
pub mod github_action;
pub mod installer_scripts;
pub mod npm_wrapper;
pub mod release_pipeline;
