//! `iac/terraform-*` — the Terraform (HCL) slice of the IaC rule family:
//! IAC-1.1, IAC-1.2, IAC-1.3, IAC-1.6, IAC-1.7.

use super::spec::{RuleSpec, TriggerKind};

/// Every Terraform rule's static spec, in `rules/rules.json` declaration
/// order.
pub const SPECS: &[RuleSpec] = &[
    RuleSpec {
        rule_id: "IAC-1.1",
        title: "Terraform S3 buckets must enable server-side encryption",
        kind: TriggerKind::RequiredAbsent {
            scope_needle: "aws_s3_bucket",
            required_needle: "server_side_encryption_configuration",
        },
        needles: &[],
        comment_guard: false,
    },
    RuleSpec {
        rule_id: "IAC-1.2",
        title: "Terraform security groups must not allow unrestricted ingress",
        kind: TriggerKind::ForbiddenPresent,
        needles: &["0.0.0.0/0"],
        comment_guard: true,
    },
    RuleSpec {
        rule_id: "IAC-1.3",
        title: "Terraform resources must not hardcode secrets or credentials",
        kind: TriggerKind::ForbiddenPresent,
        needles: &["aws_access_key_id", "aws_secret_access_key", "password ="],
        comment_guard: true,
    },
    RuleSpec {
        rule_id: "IAC-1.6",
        title: "Terraform provider blocks must pin an exact version",
        kind: TriggerKind::RequiredAbsent {
            scope_needle: "required_providers",
            required_needle: "version",
        },
        needles: &[],
        comment_guard: false,
    },
    RuleSpec {
        rule_id: "IAC-1.7",
        title: "Terraform remote state backends must enable encryption",
        kind: TriggerKind::RequiredAbsent {
            scope_needle: "backend \"s3\"",
            required_needle: "encrypt",
        },
        needles: &[],
        comment_guard: false,
    },
];
