//! Data-driven rule spec: one [`RuleSpec`] per `TS-*` rule id, consumed by
//! the `source_scan` and `generic_scanner` validator families. Keeping the
//! per-rule detection as DATA (a matcher kind + needle(s)) rather than 73
//! bespoke functions is what makes the count-parity completeness test
//! (`tests/completeness.rs`) a mechanical fold instead of hand-maintained
//! prose.

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::findings::Finding;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

use super::text_scan::{find_literal, find_non_null_assertions, find_word, is_comment_only_line};

/// How a [`RuleSpec`] recognizes its forbidden pattern in one source line.
#[derive(Debug, Clone, Copy)]
pub enum TriggerKind {
    /// Match `needle` as a whole word (identifier boundaries on both
    /// sides) — for bare-keyword triggers like `any`, `let`.
    Word,
    /// Match `needle` as a literal substring — for multi-token/punctuation
    /// triggers like `as unknown as`, `export * from`.
    Literal,
    /// The special-cased `!` non-null-assertion postfix-operator guard.
    NonNullAssertion,
}

/// One rule's static detection spec.
#[derive(Debug, Clone, Copy)]
pub struct RuleSpec {
    /// The rule id this spec proves, e.g. `TS-6.1`.
    pub rule_id: &'static str,
    /// Human title, mirrored into every [`Finding::title`].
    pub title: &'static str,
    /// How to recognize the pattern.
    pub kind: TriggerKind,
    /// The needle(s) to look for; multiple needles are OR'd (any hit
    /// fires). Ignored (empty) for [`TriggerKind::NonNullAssertion`].
    pub needles: &'static [&'static str],
    /// When `true`, a comment-only line is skipped before matching (the
    /// default posture: a mention of a forbidden pattern INSIDE a comment
    /// is not a live occurrence of it). Set `false` for the rare rule
    /// whose violation IS itself a comment directive — e.g. TS-2.1's
    /// suppression comments (`// eslint-disable`, `// @ts-ignore`) are
    /// comment-only lines BY DEFINITION, so skipping comment-only lines
    /// would defeat that rule entirely.
    pub comment_guard: bool,
}

impl RuleSpec {
    /// Run this spec's matcher against one line, returning the byte
    /// offsets of every hit (used only to decide fire/no-fire — exact
    /// column is not part of the [`Finding`] contract here).
    fn hits_on_line(&self, text: &str) -> bool {
        match self.kind {
            TriggerKind::Word => self
                .needles
                .iter()
                .any(|needle| !find_word(text, needle).is_empty()),
            TriggerKind::Literal => self
                .needles
                .iter()
                .any(|needle| !find_literal(text, needle).is_empty()),
            TriggerKind::NonNullAssertion => !find_non_null_assertions(text).is_empty(),
        }
    }

    /// Validate one file's source against this spec, honoring the
    /// comment-only-line guard (arc-06's position-not-just-kind lesson: a
    /// mention of the forbidden pattern inside a `//` comment is not a
    /// live occurrence of it). `rule_id` is the already-parsed brand for
    /// `self.rule_id` (parsed once at [`SpecValidator::new`] time, not
    /// re-parsed per file).
    fn validate_with_id(&self, input: ValidationInput<'_>, rule_id: &RuleId) -> Vec<Finding> {
        let mut findings = Vec::new();
        for line in super::text_scan::lines(input.source) {
            if self.comment_guard && is_comment_only_line(line.text) {
                continue;
            }
            if self.hits_on_line(line.text) {
                findings.push(Finding {
                    rule_id: rule_id.clone(),
                    severity: Severity::Error,
                    title: self.title.to_owned(),
                    detail: format!(
                        "line {} matches forbidden pattern for `{}`",
                        line.number, self.rule_id
                    ),
                    file: input.file.clone(),
                    line: line.number,
                    snippet: Some(line.text.trim().to_owned()),
                });
            }
        }
        findings
    }
}

/// A [`Validator`] wrapper around one [`RuleSpec`] — the adapter every
/// `source_scan`/`generic_scanner` rule registers as its `Validator` impl.
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
