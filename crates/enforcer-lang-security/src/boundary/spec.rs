//! Data-driven rule spec: one [`RuleSpec`] per `SEC-2.*` rule id, consumed
//! by the [`super::generic_scanner`] validator family. Keeping the
//! per-rule detection as DATA (a compiled regex + a path/content matcher
//! kind) rather than 20 bespoke functions is what makes the count-parity
//! completeness test (`tests/completeness.rs`) a mechanical fold instead
//! of hand-maintained prose, and mirrors `enforcer-lang-ts`'s
//! `rules::spec` shape for the sibling `generic-scanner` slice.
//!
//! Unlike the TS slice (whose `generic-scanner` rows are literal
//! keyword/punctuation triggers), every SEC-2 pattern ported from
//! `src/generic-scanner-shared.mjs`'s `COMMON_SECRET_RULES` table and
//! `src/generic-common-line-rules.mjs`'s `scanSecretLine` is itself a
//! regex (token shapes, high-entropy assignments, path patterns) — so
//! [`RuleSpec::pattern`] is a compiled [`regex::Regex`], not a plain
//! needle string.

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::findings::{Finding, FindingDetail, FindingTitle};
use enforcer_domain::ids::{BuiltInSecurityRule, RuleId};
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};
use regex::Regex;

use crate::boundary::text_scan::{is_command_like_line, is_comment_only_line, lines};

/// Where a [`RuleSpec`] looks for its pattern.
#[derive(Debug, Clone, Copy)]
pub(crate) enum MatchTarget {
    /// Match against each line of file CONTENT.
    Content,
    /// Match against the file's repo-relative PATH once (line 1 in the
    /// resulting [`Finding`], since a path match has no meaningful line).
    Path,
}

/// Matching posture kept separate from rule identity and presentation.
pub(crate) struct RuleBehavior {
    target: MatchTarget,
    comment_guard: bool,
    command_guard: bool,
    suppressed_by_any_of: Option<Regex>,
}

impl RuleBehavior {
    pub(crate) const fn content() -> Self {
        Self {
            target: MatchTarget::Content,
            comment_guard: true,
            command_guard: false,
            suppressed_by_any_of: None,
        }
    }

    pub(crate) const fn unguarded_content() -> Self {
        Self {
            target: MatchTarget::Content,
            comment_guard: false,
            command_guard: false,
            suppressed_by_any_of: None,
        }
    }

    /// Build a command-tooling rule. These rules are defined by the shared
    /// scanner's `isCommandLikeLine` context, so a rule's own regex literal
    /// (for example `compile(r"...trufflehog...")`) is not treated as a
    /// command invocation during self-scan.
    pub(crate) fn command_suppressed(pattern: Regex) -> Self {
        Self {
            target: MatchTarget::Content,
            comment_guard: true,
            command_guard: true,
            suppressed_by_any_of: Some(pattern),
        }
    }

    pub(crate) fn unguarded_content_suppressed(pattern: Regex) -> Self {
        Self {
            target: MatchTarget::Content,
            comment_guard: false,
            command_guard: false,
            suppressed_by_any_of: Some(pattern),
        }
    }

    pub(crate) const fn path() -> Self {
        Self {
            target: MatchTarget::Path,
            comment_guard: false,
            command_guard: false,
            suppressed_by_any_of: None,
        }
    }
}

/// One rule's static detection spec.
pub(crate) struct RuleSpec {
    /// The rule id this spec proves, e.g. `SEC-2.1`.
    rule_id: RuleId,
    /// Human title, mirrored into every [`Finding::title`].
    title: FindingTitle,
    /// Occurrence-specific detail, mirrored into every
    /// [`Finding::detail`].
    detail: FindingDetail,
    /// The compiled pattern to search for.
    pattern: Regex,
    /// Whether this rule scans file content (line-by-line) or the file
    /// path itself.
    target: MatchTarget,
    /// When `true` (the default posture for every rule in this crate — see
    /// `text_scan`'s module doc), a comment-only line is skipped before
    /// content matching. Ignored for [`MatchTarget::Path`].
    comment_guard: bool,
    /// When `true`, content matching is limited to command-like lines as
    /// classified by the shared generic scanner.
    command_guard: bool,
    /// An optional second pattern whose PRESENCE on the same line
    /// suppresses the finding — e.g. an explicit safe-value marker
    /// (`example`/`<TOKEN>`) that makes an otherwise secret-shaped
    /// line safe, or a `--sarif`/`--json` flag that makes an otherwise
    /// bare tool invocation compliant. Rust's `regex` crate has no
    /// negative lookaround, so this is how every SEC-2 rule whose JS
    /// source used `!SOME_RE.test(line)` as part of its condition is
    /// ported: match `pattern`, then reject the hit if
    /// `suppressed_by_any_of` also matches. `None` when the rule has no
    /// such marker (the common case).
    suppressed_by_any_of: Option<Regex>,
}

impl RuleSpec {
    pub(crate) fn new(
        rule: BuiltInSecurityRule,
        title: &'static str,
        detail: &'static str,
        pattern: Regex,
        behavior: RuleBehavior,
    ) -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: rule.id(),
            title: FindingTitle::new(String::from(title))?,
            detail: FindingDetail::new(String::from(detail))?,
            pattern,
            target: behavior.target,
            comment_guard: behavior.comment_guard,
            command_guard: behavior.command_guard,
            suppressed_by_any_of: behavior.suppressed_by_any_of,
        })
    }

    #[cfg(test)]
    pub(crate) fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    /// Validate one file's source/path against this spec, honoring the
    /// comment-only-line guard for content-target specs.
    fn validate_with_id(&self, input: ValidationInput<'_>, rule_id: &RuleId) -> Vec<Finding> {
        match self.target {
            MatchTarget::Content => self.validate_content(input, rule_id),
            MatchTarget::Path => self.validate_path(input, rule_id),
        }
    }

    fn validate_content(&self, input: ValidationInput<'_>, rule_id: &RuleId) -> Vec<Finding> {
        let mut findings = Vec::new();
        for line in lines(input.source) {
            if self.comment_guard && is_comment_only_line(line.text) {
                continue;
            }
            if self.command_guard && !is_command_like_line(line.text.as_str()) {
                continue;
            }
            let suppressed = self
                .suppressed_by_any_of
                .as_ref()
                .is_some_and(|marker| marker.is_match(line.text.as_str()));
            if !suppressed && self.pattern.is_match(line.text.as_str()) {
                findings.extend(crate::boundary::finding::from_owned_source(
                    (rule_id, Severity::Error),
                    self.title.as_str(),
                    self.detail.as_str(),
                    input.file,
                    (line.number, Some(redact_line(line.text.as_str()))),
                ));
            }
        }
        findings
    }

    fn validate_path(&self, input: ValidationInput<'_>, rule_id: &RuleId) -> Vec<Finding> {
        let rel = input.file.as_str();
        if self.pattern.is_match(rel) {
            crate::boundary::finding::from_source(
                (rule_id, Severity::Error),
                self.title.as_str(),
                self.detail.as_str(),
                input.file,
                (1, Some(rel)),
            )
            .into_iter()
            .collect()
        } else {
            Vec::new()
        }
    }
}

/// Redact any quoted secret-looking value before it lands in a
/// [`Finding::snippet`] — findings must never carry a raw secret value
/// (SEC-2.15's own charter: "Secret diagnostics must redact matched
/// values"), even for the OTHER 21 rules in this crate that report a
/// finding pointing AT a secret-shaped line.
pub(crate) fn redact_line(text: &str) -> String {
    static REDACT_QUOTED: std::sync::OnceLock<Result<Regex, regex::Error>> =
        std::sync::OnceLock::new();
    let Ok(pattern) =
        REDACT_QUOTED.get_or_init(|| Regex::new(r#"(['"])[A-Za-z0-9_./+=:@-]{8,}(['"])"#))
    else {
        return String::from(text.trim());
    };
    pattern
        .replace_all(text.trim(), "$1[REDACTED]$2")
        .into_owned()
}

/// A [`Validator`] wrapper around one [`RuleSpec`] — the adapter every
/// `generic-scanner`-shaped SEC-2 rule registers as its `Validator` impl.
pub(crate) struct SpecValidator {
    spec: RuleSpec,
    rule_id: RuleId,
}

impl SpecValidator {
    /// Build a validator for `spec`. Fails closed (returns `Err`) rather
    /// than panicking when `spec.rule_id` is not a well-formed [`RuleId`]
    /// literal — every call site in [`super::registry`] propagates this
    /// with `?`, and `tests/completeness.rs` asserts the whole registry
    /// constructs cleanly, so a malformed literal fails the build's tests
    /// instead of surfacing as a runtime panic.
    pub(crate) fn new(spec: RuleSpec) -> Result<Self, DecodeError> {
        let rule_id = spec.rule_id.clone();
        Ok(Self { spec, rule_id })
    }
}

impl Validator for SpecValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        self.spec.validate_with_id(input, &self.rule_id)
    }
}

#[cfg(test)]
mod tests {
    use super::redact_line;

    #[test]
    fn redact_line_masks_quoted_secret_shaped_values() {
        let input = ["token", " = ", r#""abcdef0123456789""#].concat();
        let redacted = redact_line(&input);
        assert!(!redacted.contains("abcdef0123456789"));
        assert!(redacted.contains("[REDACTED]"));
    }

    #[test]
    fn redact_line_leaves_short_quoted_values_alone() {
        let redacted = redact_line(r#"name = "ok""#);
        assert_eq!(redacted, r#"name = "ok""#);
    }
}
