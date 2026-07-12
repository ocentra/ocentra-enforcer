//! h11 — cyberskills corpus fundamental-logic rules, reimplemented as
//! native Rust `Validator`s (no CLI subprocess). Rule content is harvested
//! from `vendor/anthropic-cybersecurity-skills/` per
//! `docs/plans/enforcer-selfhost-plan/workpacks/h11-cyberskills-corpus-to-rust-rules.md`'s
//! harvest-target list; each submodule documents its own vendor source
//! path.
//!
//! Boundary (workpack "Boundary honored"): only the (a)/(b)
//! fundamental-logic cluster (regex tables, boolean field predicates,
//! manifest parsers, a scored WAF-log matcher) is ported here. The
//! irreplaceable-engine (d) skills (symbolic execution, fuzzing, network
//! scan, binary/memory forensics) are NOT reimplemented in this module —
//! they are deferred to the optional `h12` adapter pack. No CLI subprocess
//! is introduced anywhere in this module.

pub mod cloud_aws;
pub mod cloud_azure;
pub mod dependency_confusion;
pub mod dockerfile_hardening;
pub mod iac_terraform;
pub mod k8s_pod_security;
pub mod provider_credentials;
pub mod registry;
pub mod waf_sqli;
pub mod web_headers;
