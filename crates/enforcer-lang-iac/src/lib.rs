//! `enforcer-lang-iac` — the per-family `Validator` implementations for the
//! infrastructure-as-code rule family (`IAC-1.1` .. `IAC-1.8`, 8 rules
//! total).
//!
//! # Charter
//!
//! This crate hosts the Rust-native replacement for the ad-hoc IaC/
//! config-file detection shapes that lived only inside the `.mjs`
//! generic-scanner. It builds on [`enforcer_validator::validator::Validator`]
//! and proves every rule through
//! [`enforcer_validator::harness::run_fixture_parity`].
//!
//! The enforcer VALIDATES Terraform/CloudFormation/Kubernetes-manifest
//! TEXT from Rust — this crate does not execute `terraform`, `cfn-lint`,
//! `cflint`, or any external engine itself. Where an external engine is
//! irreplaceable (live plan/apply validation, real CFLint rule coverage),
//! this crate leaves that integration to `enforcer-harness`'s
//! graceful-skip adapter seam rather than faking the check here.
//!
//! Three validator families cover the 8 rules:
//!
//! - [`rules::terraform`] — Terraform/HCL shapes (5 rules: IAC-1.1 S3
//!   encryption, IAC-1.2 open ingress, IAC-1.3 hardcoded secrets, IAC-1.6
//!   provider version pin, IAC-1.7 remote-state encryption).
//! - [`rules::cloudformation`] — CloudFormation JSON/YAML template shapes
//!   (2 rules: IAC-1.4 S3 public-access-block, IAC-1.5 IAM wildcard
//!   action+resource).
//! - [`rules::kubernetes`] — Kubernetes manifest shapes (1 rule: IAC-1.8
//!   privileged container).
//!
//! [`rules::registry`] enumerates every one of the 8 `IAC-*` rule ids with
//! its owning validator, and is the single source the count-parity
//! completeness test in `tests/completeness.rs` walks.

pub mod rules;
