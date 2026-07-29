//! Typed validators for the ten built-in Kubernetes manifest rules.
//!
//! The crate detects insecure pod and container security contexts,
//! over-broad RBAC grants, missing resource declarations, and host namespace
//! access. It validates manifest source text and does not contact a live
//! Kubernetes cluster.
//!
//! The rule families are:
//!
//! - `K8S-1`: pod and container security contexts.
//! - `K8S-2`: wildcard RBAC verbs and resources.
//! - `K8S-3`: resource limits and memory requests.
//! - `K8S-4`: host network and process namespaces.
//!
//! [`rules::registry`] is the public validator registry. Canonical built-in
//! identities live in [`enforcer_domain::ids::BuiltInK8sRule`].

pub mod rules;
