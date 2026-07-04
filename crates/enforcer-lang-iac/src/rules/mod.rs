//! The three IaC-family validator sub-modules (Terraform, CloudFormation,
//! Kubernetes) plus the shared text-scan primitives and data-driven
//! rule-spec plumbing they build on. See `crate` docs for the per-module
//! rule-id breakdown.

pub mod cloudformation;
pub mod kubernetes;
pub mod registry;
pub mod spec;
pub mod terraform;
pub mod text_scan;
