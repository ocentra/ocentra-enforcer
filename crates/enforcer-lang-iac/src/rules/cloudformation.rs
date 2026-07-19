//! CloudFormation built-in IaC detection specifications.

use enforcer_domain::ids::BuiltInIacRule;

use super::spec::{CommentPolicy, RuleSpec, TriggerKind};
use crate::boundary::source_text::IacPattern;

const WILDCARD_ACTION: &[IacPattern] = &[IacPattern::WildcardAction];

pub(crate) const SPECS: &[RuleSpec] = &[
    RuleSpec {
        rule: BuiltInIacRule::CloudFormationPublicAccess,
        kind: TriggerKind::RequiredAbsent {
            scope: IacPattern::CloudFormationS3Bucket,
            required: IacPattern::PublicAccessBlock,
        },
        patterns: &[],
        comments: CommentPolicy::Include,
    },
    RuleSpec {
        rule: BuiltInIacRule::CloudFormationWildcardIam,
        kind: TriggerKind::ForbiddenPresent,
        patterns: WILDCARD_ACTION,
        comments: CommentPolicy::Include,
    },
];
