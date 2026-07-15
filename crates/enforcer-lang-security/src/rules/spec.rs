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
use enforcer_domain::findings::Finding;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};
use regex::Regex;

use super::text_scan::{is_comment_only_line, lines};

/// Where a [`RuleSpec`] looks for its pattern.
#[derive(Debug, Clone, Copy)]
pub enum MatchTarget {
    /// Match against each line of file CONTENT.
    Content,
    /// Match against the file's repo-relative PATH once (line 1 in the
    /// resulting [`Finding`], since a path match has no meaningful line).
    Path,
}

/// One rule's static detection spec.
pub struct RuleSpec {
    /// The rule id this spec proves, e.g. `SEC-2.1`.
    pub rule_id: &'static str,
    /// Human title, mirrored into every [`Finding::title`].
    pub title: &'static str,
    /// Occurrence-specific detail, mirrored into every
    /// [`Finding::detail`].
    pub detail: &'static str,
    /// The compiled pattern to search for.
    pub pattern: Regex,
    /// Whether this rule scans file content (line-by-line) or the file
    /// path itself.
    pub target: MatchTarget,
    /// When `true` (the default posture for every rule in this crate — see
    /// `text_scan`'s module doc), a comment-only line is skipped before
    /// content matching. Ignored for [`MatchTarget::Path`].
    pub comment_guard: bool,
    /// An optional second pattern whose PRESENCE on the same line
    /// suppresses the finding — e.g. a placeholder marker
    /// (`example`/`fake`/`<TOKEN>`) that makes an otherwise secret-shaped
    /// line safe, or a `--sarif`/`--json` flag that makes an otherwise
    /// bare tool invocation compliant. Rust's `regex` crate has no
    /// negative lookaround, so this is how every SEC-2 rule whose JS
    /// source used `!SOME_RE.test(line)` as part of its condition is
    /// ported: match `pattern`, then reject the hit if
    /// `suppressed_by_any_of` also matches. `None` when the rule has no
    /// such marker (the common case).
    pub suppressed_by_any_of: Option<Regex>,
}

impl RuleSpec {
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
            let suppressed = self
                .suppressed_by_any_of
                .as_ref()
                .is_some_and(|marker| marker.is_match(line.text));
            if !suppressed && self.pattern.is_match(line.text) {
                findings.push(Finding {
                    rule_id: rule_id.clone(),
                    severity: Severity::Error,
                    title: self.title.to_owned(),
                    detail: self.detail.to_owned(),
                    file: input.file.clone(),
                    line: line.number,
                    snippet: Some(redact_line(line.text)),
                });
            }
        }
        findings
    }

    fn validate_path(&self, input: ValidationInput<'_>, rule_id: &RuleId) -> Vec<Finding> {
        let rel = input.file.as_str();
        if self.pattern.is_match(rel) {
            vec![Finding {
                rule_id: rule_id.clone(),
                severity: Severity::Error,
                title: self.title.to_owned(),
                detail: self.detail.to_owned(),
                file: input.file.clone(),
                line: 1,
                snippet: Some(rel.to_owned()),
            }]
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
pub fn redact_line(text: &str) -> String {
    static REDACT_QUOTED: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    let pattern = REDACT_QUOTED.get_or_init(|| {
        #[allow(clippy::unwrap_used)]
        Regex::new(r#"(['"])[A-Za-z0-9_./+=:@-]{8,}(['"])"#).unwrap()
    });
    pattern
        .replace_all(text.trim(), "$1[REDACTED]$2")
        .into_owned()
}

/// A [`Validator`] wrapper around one [`RuleSpec`] — the adapter every
/// `generic-scanner`-shaped SEC-2 rule registers as its `Validator` impl.
pub struct SpecValidator {
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
    pub fn new(spec: RuleSpec) -> Result<Self, DecodeError> {
        let rule_id: RuleId = spec.rule_id.parse()?;
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
        let redacted = redact_line(r#"token = "abcdef0123456789""#);
        assert!(!redacted.contains("abcdef0123456789"));
        assert!(redacted.contains("[REDACTED]"));
    }

    #[test]
    fn redact_line_leaves_short_quoted_values_alone() {
        let redacted = redact_line(r#"name = "ok""#);
        assert_eq!(redacted, r#"name = "ok""#);
    }
}
