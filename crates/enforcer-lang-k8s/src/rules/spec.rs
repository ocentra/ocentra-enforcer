//! Typed, data-driven detection specifications for built-in Kubernetes rules.

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::boundary::validation::ValidationSource;
use enforcer_domain::findings::{
    Finding, FindingDetail, FindingLine, FindingSnippet, FindingTitle,
};
use enforcer_domain::ids::{BuiltInK8sRule, RuleId};
use enforcer_domain::severity::Severity;
use enforcer_domain::telemetry_types::SourceLine;
use enforcer_validator::validator::{ValidationInput, Validator};

#[derive(Debug, Clone, Copy)]
enum K8sPattern {
    PrivilegedContainer,
    RunAsRoot,
    PrivilegeEscalation,
    WritableRootFilesystem,
    WildcardVerbComment,
    WildcardVerbDoubleQuoted,
    WildcardVerbSingleQuoted,
    WildcardResourceComment,
    WildcardResourceDoubleQuoted,
    WildcardResourceSingleQuoted,
    EmptyResources,
    EmptyRequests,
    HostNetwork,
    HostPid,
    HostIpc,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PatternPresence {
    Present,
    Absent,
}

impl K8sPattern {
    fn presence_in(self, source: ValidationSource<'_>) -> PatternPresence {
        let pattern = match self {
            Self::PrivilegedContainer => "privileged: true",
            Self::RunAsRoot => "runAsNonRoot: false",
            Self::PrivilegeEscalation => "allowPrivilegeEscalation: true",
            Self::WritableRootFilesystem => "readOnlyRootFilesystem: false",
            Self::WildcardVerbComment => "- \"*\" # verbs",
            Self::WildcardVerbDoubleQuoted => "verbs: [\"*\"]",
            Self::WildcardVerbSingleQuoted => "verbs: ['*']",
            Self::WildcardResourceComment => "- \"*\" # resources",
            Self::WildcardResourceDoubleQuoted => "resources: [\"*\"]",
            Self::WildcardResourceSingleQuoted => "resources: ['*']",
            Self::EmptyResources => "resources: {}",
            Self::EmptyRequests => "requests: {}",
            Self::HostNetwork => "hostNetwork: true",
            Self::HostPid => "hostPID: true",
            Self::HostIpc => "hostIPC: true",
        };
        if source.as_str().contains(pattern) {
            PatternPresence::Present
        } else {
            PatternPresence::Absent
        }
    }
}

/// One built-in Kubernetes rule's static detection plan.
#[derive(Debug, Clone, Copy)]
pub(crate) struct K8sRuleSpec {
    rule: BuiltInK8sRule,
    patterns: &'static [K8sPattern],
}

impl K8sRuleSpec {
    pub(crate) fn build(self) -> Result<K8sValidator, DecodeError> {
        Ok(K8sValidator {
            rule_id: self.rule.id(),
            title: self.rule.finding_title()?,
            patterns: self.patterns,
        })
    }
}

/// Validator backed by one canonical Kubernetes rule specification.
#[derive(Debug)]
pub(crate) struct K8sValidator {
    rule_id: RuleId,
    title: FindingTitle,
    patterns: &'static [K8sPattern],
}

impl Validator for K8sValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        for (index, line) in input.source.as_str().lines().enumerate() {
            let source = ValidationSource::from_text(line);
            if !self
                .patterns
                .iter()
                .any(|pattern| matches!(pattern.presence_in(source), PatternPresence::Present))
            {
                continue;
            }
            let Ok(index) = u32::try_from(index) else {
                return Vec::new();
            };
            let Some(wire_line) = index.checked_add(1) else {
                return Vec::new();
            };
            let Some(wire_line) = std::num::NonZeroU32::new(wire_line) else {
                return Vec::new();
            };
            let source_line = SourceLine::try_new(wire_line);
            let Ok(detail) = FindingDetail::new(format!(
                "matched forbidden Kubernetes manifest shape for `{}`",
                self.rule_id
            )) else {
                return Vec::new();
            };
            let Ok(snippet) = FindingSnippet::new(line.trim().to_owned()) else {
                return Vec::new();
            };
            return vec![Finding {
                rule_id: self.rule_id.clone(),
                severity: Severity::Error,
                title: self.title.clone(),
                detail,
                file: input.file.clone(),
                line: FindingLine::known(source_line),
                snippet: Some(snippet),
            }];
        }
        Vec::new()
    }
}

const PRIVILEGED_CONTAINER: &[K8sPattern] = &[K8sPattern::PrivilegedContainer];
const RUN_AS_ROOT: &[K8sPattern] = &[K8sPattern::RunAsRoot];
const PRIVILEGE_ESCALATION: &[K8sPattern] = &[K8sPattern::PrivilegeEscalation];
const WRITABLE_ROOT_FILESYSTEM: &[K8sPattern] = &[K8sPattern::WritableRootFilesystem];
const WILDCARD_RBAC_VERBS: &[K8sPattern] = &[
    K8sPattern::WildcardVerbComment,
    K8sPattern::WildcardVerbDoubleQuoted,
    K8sPattern::WildcardVerbSingleQuoted,
];
const WILDCARD_RBAC_RESOURCES: &[K8sPattern] = &[
    K8sPattern::WildcardResourceComment,
    K8sPattern::WildcardResourceDoubleQuoted,
    K8sPattern::WildcardResourceSingleQuoted,
];
const EMPTY_RESOURCES: &[K8sPattern] = &[K8sPattern::EmptyResources];
const EMPTY_REQUESTS: &[K8sPattern] = &[K8sPattern::EmptyRequests];
const HOST_NETWORK: &[K8sPattern] = &[K8sPattern::HostNetwork];
const HOST_PROCESS_NAMESPACE: &[K8sPattern] = &[K8sPattern::HostPid, K8sPattern::HostIpc];

pub(crate) const SPECS: &[K8sRuleSpec] = &[
    K8sRuleSpec {
        rule: BuiltInK8sRule::PrivilegedContainer,
        patterns: PRIVILEGED_CONTAINER,
    },
    K8sRuleSpec {
        rule: BuiltInK8sRule::RunAsRoot,
        patterns: RUN_AS_ROOT,
    },
    K8sRuleSpec {
        rule: BuiltInK8sRule::PrivilegeEscalation,
        patterns: PRIVILEGE_ESCALATION,
    },
    K8sRuleSpec {
        rule: BuiltInK8sRule::WritableRootFilesystem,
        patterns: WRITABLE_ROOT_FILESYSTEM,
    },
    K8sRuleSpec {
        rule: BuiltInK8sRule::WildcardRbacVerbs,
        patterns: WILDCARD_RBAC_VERBS,
    },
    K8sRuleSpec {
        rule: BuiltInK8sRule::WildcardRbacResources,
        patterns: WILDCARD_RBAC_RESOURCES,
    },
    K8sRuleSpec {
        rule: BuiltInK8sRule::MissingResourceLimits,
        patterns: EMPTY_RESOURCES,
    },
    K8sRuleSpec {
        rule: BuiltInK8sRule::MissingMemoryRequests,
        patterns: EMPTY_REQUESTS,
    },
    K8sRuleSpec {
        rule: BuiltInK8sRule::HostNetwork,
        patterns: HOST_NETWORK,
    },
    K8sRuleSpec {
        rule: BuiltInK8sRule::HostProcessNamespace,
        patterns: HOST_PROCESS_NAMESPACE,
    },
];
