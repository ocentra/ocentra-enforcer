//! Canonical compiled-pattern records created at the regex/text boundary.

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::findings::FindingDetail;
use enforcer_domain::severity::Severity;
use regex::Regex;

/// Static source record decoded into a [`LabelledPattern`].
pub(crate) struct LabelledPatternSource {
    pub(crate) regex: &'static str,
    pub(crate) label: &'static str,
}

/// Static source record decoded into a [`RemediationPattern`].
pub(crate) struct RemediationPatternSource {
    pub(crate) regex: &'static str,
    pub(crate) label: &'static str,
    pub(crate) fix: &'static str,
}

/// Static literal source record decoded at a validator boundary.
pub(crate) struct LabelledLiteralSource {
    pub(crate) literal: &'static str,
    pub(crate) label: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum PatternConfidence {
    Medium,
    High,
    Critical,
}

impl PatternConfidence {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Medium => "medium",
            Self::High => "high",
            Self::Critical => "critical",
        }
    }
}

pub(crate) struct ScoredPatternSource {
    pub(crate) regex: &'static str,
    pub(crate) label: &'static str,
    pub(crate) confidence: PatternConfidence,
}

pub(crate) struct CredentialPattern {
    regex: Regex,
    name: FindingDetail,
    severity: Severity,
}

impl std::fmt::Debug for CredentialPattern {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CredentialPattern")
            .field("name", &self.name)
            .field("severity", &self.severity)
            .field("regex", &"<redacted>")
            .finish()
    }
}

#[derive(Debug)]
pub(crate) struct LabelledSinkPattern<K> {
    regex: Regex,
    label: FindingDetail,
    kind: K,
}

impl<K> LabelledSinkPattern<K> {
    pub(crate) fn compile(
        path: &'static str,
        pattern: &'static str,
        label: &'static str,
        kind: K,
    ) -> Result<Self, DecodeError> {
        Ok(Self {
            regex: crate::boundary::regex::compile(path, pattern)?,
            label: FindingDetail::new(String::from(label))?,
            kind,
        })
    }

    pub(crate) fn regex(&self) -> &Regex {
        &self.regex
    }

    pub(crate) fn label(&self) -> &FindingDetail {
        &self.label
    }

    pub(crate) fn kind(&self) -> &K {
        &self.kind
    }
}

impl CredentialPattern {
    pub(crate) fn compile(
        path: &'static str,
        pattern: &'static str,
        name: &'static str,
        severity: Severity,
    ) -> Result<Self, DecodeError> {
        Ok(Self {
            regex: crate::boundary::regex::compile(path, pattern)?,
            name: FindingDetail::new(String::from(name))?,
            severity,
        })
    }

    pub(crate) fn regex(&self) -> &Regex {
        &self.regex
    }

    pub(crate) fn name(&self) -> &FindingDetail {
        &self.name
    }

    pub(crate) fn severity(&self) -> Severity {
        self.severity
    }
}

/// A compiled detection pattern with canonical explanatory text.
#[derive(Debug)]
pub(crate) struct LabelledPattern {
    regex: Regex,
    label: FindingDetail,
}

#[derive(Debug)]
pub(crate) struct LabelledLiteralPattern {
    regex: Regex,
    literal: FindingDetail,
    label: FindingDetail,
}

impl LabelledLiteralPattern {
    pub(crate) fn compile_source(
        path: &'static str,
        source: &LabelledLiteralSource,
    ) -> Result<Self, DecodeError> {
        let escaped = regex::escape(source.literal);
        let pattern = format!("(?i){escaped}");
        Ok(Self {
            regex: crate::boundary::regex::compile_owned(path, &pattern)?,
            literal: FindingDetail::new(String::from(source.literal))?,
            label: FindingDetail::new(String::from(source.label))?,
        })
    }

    pub(crate) fn regex(&self) -> &Regex {
        &self.regex
    }

    pub(crate) fn literal(&self) -> &FindingDetail {
        &self.literal
    }

    pub(crate) fn label(&self) -> &FindingDetail {
        &self.label
    }
}

#[derive(Debug)]
pub(crate) struct ScoredPattern {
    regex: Regex,
    label: FindingDetail,
    confidence: PatternConfidence,
}

impl ScoredPattern {
    pub(crate) fn compile_source(
        path: &'static str,
        source: &ScoredPatternSource,
    ) -> Result<Self, DecodeError> {
        Ok(Self {
            regex: crate::boundary::regex::compile(path, source.regex)?,
            label: FindingDetail::new(String::from(source.label))?,
            confidence: source.confidence,
        })
    }

    pub(crate) fn regex(&self) -> &Regex {
        &self.regex
    }

    pub(crate) fn label(&self) -> &FindingDetail {
        &self.label
    }

    pub(crate) fn confidence(&self) -> PatternConfidence {
        self.confidence
    }
}

impl LabelledPattern {
    pub(crate) fn compile_source(
        path: &'static str,
        source: &LabelledPatternSource,
    ) -> Result<Self, DecodeError> {
        Self::compile(path, source.regex, source.label)
    }

    pub(crate) fn compile(
        path: &'static str,
        pattern: &'static str,
        label: &'static str,
    ) -> Result<Self, DecodeError> {
        Ok(Self {
            regex: crate::boundary::regex::compile(path, pattern)?,
            label: FindingDetail::new(String::from(label))?,
        })
    }

    pub(crate) fn regex(&self) -> &Regex {
        &self.regex
    }

    pub(crate) fn label(&self) -> &FindingDetail {
        &self.label
    }
}

/// A compiled detection pattern with canonical label and remediation text.
#[derive(Debug)]
pub(crate) struct RemediationPattern {
    regex: Regex,
    label: FindingDetail,
    fix: FindingDetail,
}

impl RemediationPattern {
    pub(crate) fn compile_source(
        path: &'static str,
        source: &RemediationPatternSource,
    ) -> Result<Self, DecodeError> {
        Self::compile(path, source.regex, source.label, source.fix)
    }

    pub(crate) fn compile(
        path: &'static str,
        pattern: &'static str,
        label: &'static str,
        fix: &'static str,
    ) -> Result<Self, DecodeError> {
        Ok(Self {
            regex: crate::boundary::regex::compile(path, pattern)?,
            label: FindingDetail::new(String::from(label))?,
            fix: FindingDetail::new(String::from(fix))?,
        })
    }

    pub(crate) fn regex(&self) -> &Regex {
        &self.regex
    }

    pub(crate) fn label(&self) -> &FindingDetail {
        &self.label
    }

    pub(crate) fn fix(&self) -> &FindingDetail {
        &self.fix
    }
}
