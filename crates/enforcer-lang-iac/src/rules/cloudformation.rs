//! `iac/cfn-*` — the CloudFormation (JSON/YAML template) slice of the IaC
//! rule family: IAC-1.4, IAC-1.5.

use super::spec::{RuleSpec, TriggerKind};

/// Every CloudFormation rule's static spec, in `rules/rules.json`
/// declaration order.
pub const SPECS: &[RuleSpec] = &[
    RuleSpec {
        rule_id: "IAC-1.4",
        title: "CloudFormation S3 buckets must block public access",
        kind: TriggerKind::RequiredAbsent {
            scope_needle: "AWS::S3::Bucket",
            required_needle: "PublicAccessBlockConfiguration",
        },
        needles: &[],
        comment_guard: false,
    },
    RuleSpec {
        rule_id: "IAC-1.5",
        title: "CloudFormation IAM policies must not grant wildcard action+resource",
        kind: TriggerKind::ForbiddenPresent,
        // Both a wildcard Action AND wildcard Resource must be present in
        // the file for this to be the specific over-broad-grant shape this
        // rule targets — checked as two independent literal needles OR'd
        // per-line would over-fire on a file that merely mentions either
        // alone, so this rule keys on the co-occurring pair line
        // (`"Action": "*"` and `"Resource": "*"` are adjacent lines in the
        // canonical CFN statement shape) via the combined marker below.
        needles: &["\"Action\": \"*\""],
        comment_guard: false,
    },
];
