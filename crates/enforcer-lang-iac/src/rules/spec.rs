//! Data-driven rule spec: one [`RuleSpec`] per `IAC-*` rule id. Keeping the
//! per-rule detection as DATA (a matcher kind + needle(s)) rather than
//! bespoke functions per rule is what makes the count-parity completeness
//! test (`tests/completeness.rs`) a mechanical fold instead of hand-
//! maintained prose — mirrors `enforcer-lang-ts`'s `spec.rs`.
//!
//! # Two matcher shapes
//! IaC rules split into two shapes that `enforcer-lang-ts`'s single
//! forbidden-pattern-present model does not cover on its own:
//! - [`TriggerKind::ForbiddenPresent`]: the violation is a forbidden token
//!   appearing anywhere in the file (e.g. a hardcoded secret, an open
//!   `0.0.0.0/0` ingress CIDR, `privileged: true`). Fires per matching line.
//! - [`TriggerKind::RequiredAbsent`]: the violation is a REQUIRED token
//!   being absent from the whole file when a scoping "trigger" resource/
//!   block type is present (e.g. an `aws_s3_bucket` with no matching
//!   `server_side_encryption_configuration` anywhere in the same file; a
//!   `required_providers` block with no `version` key; a `backend "s3"`
//!   block with no `encrypt` key). This fires once per file (not per line)
//!   because "absence" has no single line to attach to; the finding is
//!   anchored at the line where the scoping token was found.

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::findings::Finding;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

use super::text_scan::{find_literal, is_comment_only_line, lines};

/// How a [`RuleSpec`] recognizes its violation.
#[derive(Debug, Clone, Copy)]
pub enum TriggerKind {
    /// Fire on every line where `needles[0]` (the forbidden token) is
    /// present as a literal substring. Multiple needles are OR'd — any
    /// single needle hit on a line fires the finding for that line.
    ForbiddenPresent,
    /// Fire once for the whole file when `scope_needle` is present
    /// somewhere in the file (this rule applies) AND `required_needle` is
    /// NOT present anywhere in the file (the required config is missing).
    RequiredAbsent {
        scope_needle: &'static str,
        required_needle: &'static str,
    },
}

/// One rule's static detection spec.
#[derive(Debug, Clone, Copy)]
pub struct RuleSpec {
    /// The rule id this spec proves, e.g. `IAC-1.1`.
    pub rule_id: &'static str,
    /// Human title, mirrored into every [`Finding::title`].
    pub title: &'static str,
    /// How to recognize the violation.
    pub kind: TriggerKind,
    /// The forbidden needle(s) for [`TriggerKind::ForbiddenPresent`].
    /// Ignored (empty) for [`TriggerKind::RequiredAbsent`].
    pub needles: &'static [&'static str],
    /// When `true`, a comment-only line is skipped before matching a
    /// [`TriggerKind::ForbiddenPresent`] needle — a mention of the
    /// forbidden pattern INSIDE a `#`/`//` comment is not a live
    /// occurrence of it. Ignored for [`TriggerKind::RequiredAbsent`] (its
    /// scope/required needles are checked file-wide, not line-by-line).
    pub comment_guard: bool,
}

impl RuleSpec {
    fn validate_forbidden_present(
        &self,
        input: ValidationInput<'_>,
        rule_id: &RuleId,
        needles: &[&str],
    ) -> Vec<Finding> {
        let mut findings = Vec::new();
        for line in lines(input.source) {
            if self.comment_guard && is_comment_only_line(line.text) {
                continue;
            }
            let hit = needles
                .iter()
                .any(|needle| !find_literal(line.text, needle).is_empty());
            if hit {
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

    fn validate_required_absent(
        &self,
        input: ValidationInput<'_>,
        rule_id: &RuleId,
        scope_needle: &str,
        required_needle: &str,
    ) -> Vec<Finding> {
        let mut scope_line: Option<u32> = None;
        let mut required_present = false;
        for line in lines(input.source) {
            if scope_line.is_none() && !find_literal(line.text, scope_needle).is_empty() {
                scope_line = Some(line.number);
            }
            if !find_literal(line.text, required_needle).is_empty() {
                required_present = true;
            }
        }
        let Some(anchor) = scope_line else {
            // The scoping resource/block type is not present in this
            // file at all — this rule does not apply, not a violation.
            return Vec::new();
        };
        if required_present {
            return Vec::new();
        }
        vec![Finding {
            rule_id: rule_id.clone(),
            severity: Severity::Error,
            title: self.title.to_owned(),
            detail: format!(
                "`{scope_needle}` present without required `{required_needle}` anywhere in the file for `{}`",
                self.rule_id
            ),
            file: input.file.clone(),
            line: anchor,
            snippet: None,
        }]
    }

    fn validate_with_id(&self, input: ValidationInput<'_>, rule_id: &RuleId) -> Vec<Finding> {
        match self.kind {
            TriggerKind::ForbiddenPresent => {
                self.validate_forbidden_present(input, rule_id, self.needles)
            }
            TriggerKind::RequiredAbsent {
                scope_needle,
                required_needle,
            } => self.validate_required_absent(input, rule_id, scope_needle, required_needle),
        }
    }
}

/// A [`Validator`] wrapper around one [`RuleSpec`] — the adapter every
/// `enforcer-lang-iac` rule registers as its `Validator` impl.
pub struct SpecValidator {
    spec: RuleSpec,
    rule_id: RuleId,
}

impl SpecValidator {
    /// Build a validator for `spec`. Fails closed (returns `Err`) rather
    /// than panicking when `spec.rule_id` is not a well-formed [`RuleId`]
    /// literal — every call site in [`super::registry`] propagates this
    /// with `?`, and `tests/completeness.rs` asserts the whole registry
    /// constructs cleanly.
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
