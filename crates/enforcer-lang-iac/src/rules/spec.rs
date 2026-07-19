//! Typed, data-driven detection specifications for built-in IaC rules.

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::boundary::validation::ValidationSource;
use enforcer_domain::findings::{Finding, FindingTitle};
use enforcer_domain::ids::{BuiltInIacRule, RuleId};
use enforcer_validator::validator::{ValidationInput, Validator};

use crate::boundary::source_text::{
    is_comment_only_line, lines, CommentLine, IacPattern, PatternPresence,
};

/// Whether comment-only lines participate in a literal match.
#[derive(Debug, Clone, Copy)]
pub(crate) enum CommentPolicy {
    Include,
    Ignore,
}

/// How a rule recognizes a violation.
#[derive(Debug, Clone, Copy)]
pub(crate) enum TriggerKind {
    ForbiddenPresent,
    RequiredAbsent {
        scope: IacPattern,
        required: IacPattern,
    },
}

#[derive(Debug, Clone, Copy)]
struct RequiredPatterns {
    scope: IacPattern,
    required: IacPattern,
}

/// One built-in rule's static detection plan.
#[derive(Debug, Clone, Copy)]
pub(crate) struct RuleSpec {
    pub(crate) rule: BuiltInIacRule,
    pub(crate) kind: TriggerKind,
    pub(crate) patterns: &'static [IacPattern],
    pub(crate) comments: CommentPolicy,
}

impl RuleSpec {
    fn validate_forbidden_present(
        &self,
        input: ValidationInput<'_>,
        rule_id: &RuleId,
        title: &FindingTitle,
    ) -> Vec<Finding> {
        let mut findings = Vec::new();
        for line in lines(input.source) {
            if matches!(self.comments, CommentPolicy::Ignore)
                && matches!(is_comment_only_line(line.text), CommentLine::Yes)
            {
                continue;
            }
            if self
                .patterns
                .iter()
                .any(|pattern| matches!(pattern.presence_in(line.text), PatternPresence::Present))
            {
                findings.extend(iac_finding!(
                    rule_id,
                    title,
                    format!(
                        "line {} matches forbidden pattern for `{rule_id}`",
                        line.number
                    ),
                    input.file,
                    line.number,
                    Some(line.text),
                ));
            }
        }
        findings
    }

    fn validate_required_absent(
        &self,
        input: ValidationInput<'_>,
        rule_id: &RuleId,
        title: &FindingTitle,
        patterns: RequiredPatterns,
    ) -> Vec<Finding> {
        let RequiredPatterns { scope, required } = patterns;
        let mut scope_line = None;
        let mut required_present = false;
        for line in lines(input.source) {
            if scope_line.is_none()
                && matches!(scope.presence_in(line.text), PatternPresence::Present)
            {
                scope_line = Some(line.number);
            }
            if matches!(required.presence_in(line.text), PatternPresence::Present) {
                required_present = true;
            }
        }
        let Some(anchor) = scope_line else {
            return Vec::new();
        };
        if required_present {
            return Vec::new();
        }
        iac_finding!(
            rule_id,
            title,
            format!("required IaC configuration is absent for `{rule_id}`"),
            input.file,
            anchor,
            Option::<ValidationSource<'_>>::None,
        )
        .into_iter()
        .collect()
    }

    fn validate(
        &self,
        input: ValidationInput<'_>,
        rule_id: &RuleId,
        title: &FindingTitle,
    ) -> Vec<Finding> {
        match self.kind {
            TriggerKind::ForbiddenPresent => self.validate_forbidden_present(input, rule_id, title),
            TriggerKind::RequiredAbsent { scope, required } => self.validate_required_absent(
                input,
                rule_id,
                title,
                RequiredPatterns { scope, required },
            ),
        }
    }
}

/// Validator backed by one canonical built-in IaC specification.
#[derive(Debug)]
pub(crate) struct SpecValidator {
    spec: RuleSpec,
    rule_id: RuleId,
    title: FindingTitle,
}

impl SpecValidator {
    pub(crate) fn new(spec: RuleSpec) -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: spec.rule.id(),
            title: spec.rule.finding_title()?,
            spec,
        })
    }
}

impl Validator for SpecValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        self.spec.validate(input, &self.rule_id, &self.title)
    }
}
