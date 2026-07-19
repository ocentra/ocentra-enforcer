//! The three IaC-family validator sub-modules (Terraform, CloudFormation,
//! Kubernetes) plus the shared text-scan primitives and data-driven
//! rule-spec plumbing they build on. See `crate` docs for the per-module
//! rule-id breakdown.

pub(crate) mod cloudformation;
pub(crate) mod kubernetes;
pub mod registry;
pub(crate) mod spec;
pub(crate) mod terraform;
