//! Decode embedded TypeScript rule rows and execute their lexical matchers.

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::boundary::validation::ValidationSource;
use enforcer_domain::findings::{Finding, FindingTitle};
use enforcer_domain::ids::RuleId;
use enforcer_domain::paths::RelPath;
use enforcer_domain::scan_types::LiteralFileRole;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

use crate::boundary::finding::{from_source, SourceFinding};
use crate::boundary::rule_spec::{RawRuleSpec, TriggerKind};
use crate::boundary::source_text::{
    find_literal, find_non_null_assertions, find_word, source_line_role, SourceLineRole,
};

#[derive(Debug, Clone, Copy)]
enum CommentPolicy {
    SkipCommentOnly,
    InspectComments,
}

#[derive(Debug, Clone, Copy)]
enum Matcher {
    Word(&'static [&'static str]),
    Literal(&'static [&'static str]),
    NonNullAssertion,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LineMatch {
    Hit,
    Clean,
}

#[derive(Debug, Clone)]
struct RuleSpec {
    rule_id: RuleId,
    title: FindingTitle,
    matcher: Matcher,
    comment_policy: CommentPolicy,
}

impl RuleSpec {
    fn decode(raw: RawRuleSpec) -> Result<Self, DecodeError> {
        let matcher = match raw.kind {
            TriggerKind::Word => Matcher::Word(raw.needles),
            TriggerKind::Literal => Matcher::Literal(raw.needles),
            TriggerKind::NonNullAssertion => Matcher::NonNullAssertion,
        };
        let comment_policy = if raw.comment_guard {
            CommentPolicy::SkipCommentOnly
        } else {
            CommentPolicy::InspectComments
        };
        Ok(Self {
            rule_id: crate::boundary::rule_spec::decode_rule_id(raw.rule_id)?,
            title: crate::boundary::rule_spec::decode_finding_title(raw.title)?,
            matcher,
            comment_policy,
        })
    }

    fn match_line(&self, text: ValidationSource<'_>) -> LineMatch {
        let text = text.as_str();
        let matched = match self.matcher {
            Matcher::Word(needles) => needles
                .iter()
                .any(|needle| !find_word(text, needle).is_empty()),
            Matcher::Literal(needles) => needles
                .iter()
                .any(|needle| !find_literal(text, needle).is_empty()),
            Matcher::NonNullAssertion => !find_non_null_assertions(text).is_empty(),
        };
        if matched {
            LineMatch::Hit
        } else {
            LineMatch::Clean
        }
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        if self.rule_id.as_str() == "TS-6.19"
            && matches!(classify_decoder_path(input.file), LiteralFileRole::Boundary)
        {
            return Vec::new();
        }
        let mut findings = Vec::new();
        for line in crate::boundary::source_text::lines(input.source) {
            if matches!(self.comment_policy, CommentPolicy::SkipCommentOnly)
                && source_line_role(line.text) == SourceLineRole::CommentOnly
            {
                continue;
            }
            if self.match_line(line.text) == LineMatch::Hit {
                findings.extend(from_source(
                    &self.rule_id,
                    input.file,
                    SourceFinding {
                        severity: Severity::Error,
                        title: self.title.as_str(),
                        detail: format!(
                            "line {} matches forbidden pattern for `{}`",
                            line.number, self.rule_id
                        ),
                        line: line.number,
                        snippet: Some(line.text.as_str().trim()),
                    },
                ));
            }
        }
        findings
    }
}

/// TS-6.19 is a boundary rule: JSON decoding is expected in tooling,
/// integration, schema, and decoder modules. Paths arrive from the Windows
/// walker with either separator spelling, so normalize before classifying.
fn classify_decoder_path(path: &RelPath) -> LiteralFileRole {
    let normalized = path.as_str();
    if normalized.contains("/integration/") {
        return LiteralFileRole::Boundary;
    }
    if normalized.split('/').any(|segment| {
        matches!(
            segment,
            "schema"
                | "schemas"
                | "decoder"
                | "decoders"
                | "codec"
                | "codecs"
                | "boundary"
                | "boundaries"
                | "adapter"
                | "adapters"
                | "transport"
                | "serde"
        )
    }) {
        LiteralFileRole::Boundary
    } else {
        LiteralFileRole::Domain
    }
}

#[derive(Debug)]
/// Validator adapter around one decoded embedded rule row.
pub(crate) struct SpecValidator {
    spec: RuleSpec,
}

impl SpecValidator {
    /// Decode one embedded rule row into its validated execution form.
    pub(crate) fn new(raw: RawRuleSpec) -> Result<Self, DecodeError> {
        Ok(Self {
            spec: RuleSpec::decode(raw)?,
        })
    }
}

impl Validator for SpecValidator {
    fn rule_id(&self) -> &RuleId {
        &self.spec.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        self.spec.validate(input)
    }
}

#[cfg(test)]
mod tests {
    use enforcer_domain::boundary::decode_error::DecodeError;
    use enforcer_domain::paths::RelPath;
    use enforcer_domain::scan_types::LiteralFileRole;

    use super::classify_decoder_path;

    #[test]
    fn decoder_boundary_paths_accept_windows_and_posix_separators() -> Result<(), DecodeError> {
        let integration = RelPath::try_from(String::from(
            "crates/enforcer-literal-scan/integration/ocentra-literal-scan.mjs",
        ))?;
        let boundary = RelPath::try_from(String::from(
            "crates\\enforcer-literal-scan\\src\\boundary\\json_wire.mjs",
        ))?;
        assert_eq!(
            classify_decoder_path(&integration),
            LiteralFileRole::Boundary
        );
        assert_eq!(classify_decoder_path(&boundary), LiteralFileRole::Boundary);
        Ok(())
    }

    #[test]
    fn product_domain_paths_are_not_decoder_boundaries() -> Result<(), DecodeError> {
        let domain = RelPath::try_from(String::from(
            "crates/enforcer-literal-scan/src/domain/literal_policy.mjs",
        ))?;
        assert_eq!(classify_decoder_path(&domain), LiteralFileRole::Domain);
        Ok(())
    }

    #[test]
    fn rel_path_try_from_rejects_invalid_input() {
        assert!(RelPath::try_from(String::new()).is_err());
        assert!(RelPath::try_from(String::from("../escape.mjs")).is_err());
    }
}
