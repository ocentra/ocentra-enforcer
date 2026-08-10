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

pub mod auth_jwt;
pub mod cloud_aws;
pub mod cloud_azure;
pub mod cloud_gcp;
pub mod cloud_security;
pub mod cloud_security_b02;
pub mod cloud_security_b03;
pub mod cmd_injection;
pub mod dependency_confusion;
pub mod docker_daemon;
pub mod dockerfile_hardening;
pub mod fileless_malware;
pub mod github_actions;
pub mod iac_terraform;
pub mod insecure_deser;
pub mod k8s_pod_security;
pub mod k8s_rbac;
pub mod mass_assignment;
pub mod mcp_tool_poisoning;
pub mod net_tls;
pub mod nosql_injection;
pub mod oauth_misconfig;
pub mod path_traversal;
pub mod proto_pollution;
pub mod provider_credentials;
pub mod registry;
pub mod sqli_source;
pub mod ssti;
pub mod tls_verify;
pub mod type_juggle;
pub mod waf_sqli;
pub mod weak_crypto;
pub mod web_cors;
pub mod web_headers;
pub mod web_ssrf;
pub mod websocket_security;
