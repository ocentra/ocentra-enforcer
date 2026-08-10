//! External text and source observations decoded into canonical security values.
//!
//! BOUNDARY-INVARIANT: untrusted external representations are decoded here;
//! policy validators consume only the resulting typed observations.
//! Malformed-input rejection has negative coverage beside each fallible decoder.

pub(crate) mod blockchain_security_manifest_wire;
pub(crate) mod cloud_azure;
pub(crate) mod dependency_manifest;
pub(crate) mod dockerfile;
pub(crate) mod fileless;
pub(crate) mod finding;
#[cfg(test)]
pub(crate) mod fixture;
pub(crate) mod json;
pub(crate) mod k8s_pod;
pub(crate) mod k8s_rbac;
pub(crate) mod pattern;
pub(crate) mod regex;
pub(crate) mod source_predicates;
pub(crate) mod spec;
pub(crate) mod terraform;
pub(crate) mod text_scan;
pub(crate) mod web_headers;
