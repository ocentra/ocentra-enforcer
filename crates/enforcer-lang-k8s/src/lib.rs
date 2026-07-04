//! `enforcer-lang-k8s` — the per-family `Validator` implementations for
//! the Kubernetes-manifest rule family (`K8S-1` .. `K8S-4`, 10 rules
//! total).
//!
//! # Charter
//!
//! This crate hosts the Rust-native detection logic for insecure
//! Kubernetes manifest shapes: pod/container security-context
//! misconfiguration, over-broad RBAC grants, missing resource limits, and
//! host-namespace/network escapes. It builds on
//! [`enforcer_validator::validator::Validator`] and proves every rule
//! through [`enforcer_validator::harness::run_fixture_parity`], reusing
//! [`enforcer_lang_common::pattern::PatternValidator`] — the shared
//! `generic-scanner` line/keyword engine arc-09 single-owns — rather than
//! duplicating a second copy of that engine (the same reuse posture the
//! `SEC-2` family takes on `enforcer-lang-common`'s engine).
//!
//! The enforcer VALIDATES Kubernetes YAML manifests from Rust — this crate
//! does not run `kubectl`/talk to a live cluster; it inspects manifest
//! TEXT for known-insecure key/value shapes (`privileged: true`,
//! `hostNetwork: true`, wildcard RBAC verbs, empty `resources: {}`), the
//! same literal-key-scan posture the TS/common families' own
//! `generic-scanner` slices take on their respective source languages.
//!
//! Four rule families cover the 10 rules (see [`rules::spec::SPECS`] for
//! the authoritative per-rule table):
//!
//! - `K8S-1` — pod/container security-context shapes (privileged
//!   containers, root execution, privilege escalation, writable root fs).
//! - `K8S-2` — RBAC shapes (wildcard verbs/resources).
//! - `K8S-3` — resource-limit shapes (missing limits/requests).
//! - `K8S-4` — host-namespace/network shapes (`hostNetwork`, `hostPID`,
//!   `hostIPC`).
//!
//! [`rules::registry`] enumerates every `K8S-*` rule id with its owning
//! validator, and is the single source `tests/completeness.rs`' count-parity
//! completeness test walks.

pub mod rules;
