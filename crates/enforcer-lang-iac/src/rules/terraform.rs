//! Terraform/HCL built-in IaC detection specifications.

use enforcer_domain::ids::BuiltInIacRule;

use super::spec::{CommentPolicy, RuleSpec, TriggerKind};
use crate::boundary::source_text::IacPattern;

const OPEN_INGRESS: &[IacPattern] = &[IacPattern::OpenIngress];
const HARDCODED_SECRETS: &[IacPattern] = &[
    IacPattern::AccessKeyId,
    IacPattern::SecretAccessKey,
    IacPattern::PasswordAssignment,
];

pub(crate) const SPECS: &[RuleSpec] = &[
    RuleSpec {
        rule: BuiltInIacRule::TerraformS3Encryption,
        kind: TriggerKind::RequiredAbsent {
            scope: IacPattern::S3Bucket,
            required: IacPattern::ServerSideEncryption,
        },
        patterns: &[],
        comments: CommentPolicy::Include,
    },
    RuleSpec {
        rule: BuiltInIacRule::TerraformOpenIngress,
        kind: TriggerKind::ForbiddenPresent,
        patterns: OPEN_INGRESS,
        comments: CommentPolicy::Ignore,
    },
    RuleSpec {
        rule: BuiltInIacRule::TerraformHardcodedSecrets,
        kind: TriggerKind::ForbiddenPresent,
        patterns: HARDCODED_SECRETS,
        comments: CommentPolicy::Ignore,
    },
    RuleSpec {
        rule: BuiltInIacRule::TerraformProviderVersion,
        kind: TriggerKind::RequiredAbsent {
            scope: IacPattern::RequiredProviders,
            required: IacPattern::Version,
        },
        patterns: &[],
        comments: CommentPolicy::Include,
    },
    RuleSpec {
        rule: BuiltInIacRule::TerraformRemoteStateEncryption,
        kind: TriggerKind::RequiredAbsent {
            scope: IacPattern::S3Backend,
            required: IacPattern::Encrypt,
        },
        patterns: &[],
        comments: CommentPolicy::Include,
    },
];
