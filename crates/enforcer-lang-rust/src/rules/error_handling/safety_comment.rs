//! `RUST-SAFETY-COMMENT` — every `unsafe` block needs a `// SAFETY:`
//! comment immediately above it.
//!
//! `unsafe_code = "forbid"` is the workspace's own lint, but this rule
//! governs CONSUMER repos that have not opted into that forbid-wall: any
//! `unsafe { ... }` block must be preceded by a `// SAFETY:` line comment
//! explaining the invariant the caller is upholding.

use syn::spanned::Spanned;
use syn::visit::{self, Visit};
use syn::ExprUnsafe;

use enforcer_domain::findings::Finding;
use enforcer_domain::ids::RuleId;
use enforcer_domain::paths::RelPath;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

/// The `RUST-SAFETY-COMMENT` `Validator`.
pub struct SafetyCommentValidator {
    rule_id: RuleId,
}

impl SafetyCommentValidator {
    /// Build the validator, parsing its own `RuleId` literal at
    /// construction (parse-at-boundary).
    pub fn new() -> Result<Self, enforcer_domain::boundary::decode_error::DecodeError> {
        Ok(Self {
            rule_id: "RUST-SAFETY-COMMENT".parse()?,
        })
    }
}

impl Validator for SafetyCommentValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let Ok(file) = syn::parse_file(input.source) else {
            return Vec::new();
        };
        // Line-based SAFETY-comment lookup: for each `unsafe { ... }`
        // block's start line, check whether the immediately preceding
        // non-blank source line contains `SAFETY:`. `syn` discards regular
        // comments from its AST, so this rule intentionally cross-checks
        // against the raw source text rather than the parsed tree.
        let lines: Vec<&str> = input.source.lines().collect();
        let mut visitor = Visitor {
            rule_id: self.rule_id.clone(),
            file: input.file.clone(),
            findings: Vec::new(),
            lines,
        };
        visitor.visit_file(&file);
        visitor.findings
    }
}

struct Visitor<'s> {
    rule_id: RuleId,
    file: RelPath,
    findings: Vec<Finding>,
    lines: Vec<&'s str>,
}

impl<'s> Visitor<'s> {
    fn has_safety_comment_above(&self, line_1_based: u32) -> bool {
        // Walk upward through the contiguous `//`-comment block
        // immediately preceding the `unsafe` block (skipping blank lines
        // between the block start and the code line), checking each
        // comment line for `SAFETY:` — a multi-line `// SAFETY: ...` /
        // `// continuation...` block only needs the marker on ONE of its
        // lines, not necessarily the line directly touching `unsafe`.
        let mut idx = line_1_based.saturating_sub(2); // convert to 0-based, step back one
        let mut seen_comment = false;
        loop {
            let Some(text) = self.lines.get(idx as usize) else {
                return false;
            };
            let trimmed = text.trim();
            if trimmed.is_empty() {
                if seen_comment || idx == 0 {
                    return false;
                }
                if idx == 0 {
                    return false;
                }
                idx -= 1;
                continue;
            }
            if !trimmed.starts_with("//") {
                return false;
            }
            if trimmed.contains("SAFETY:") {
                return true;
            }
            seen_comment = true;
            if idx == 0 {
                return false;
            }
            idx -= 1;
        }
    }
}

impl<'ast, 's> Visit<'ast> for Visitor<'s> {
    fn visit_expr_unsafe(&mut self, item: &'ast ExprUnsafe) {
        let line = u32::try_from(item.span().start().line.max(1)).unwrap_or(u32::MAX);
        if !self.has_safety_comment_above(line) {
            self.findings.push(Finding {
                rule_id: self.rule_id.clone(),
                severity: Severity::Error,
                title: "`unsafe` block with no `// SAFETY:` comment".to_owned(),
                detail: "Fix: add a `// SAFETY: ...` comment immediately above this `unsafe` \
                          block explaining the invariant the caller upholds."
                    .to_owned(),
                file: self.file.clone(),
                line,
                snippet: None,
            });
        }
        visit::visit_expr_unsafe(self, item);
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use enforcer_validator::harness::run_fixture_parity;

    use super::SafetyCommentValidator;

    fn manifest_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    #[test]
    fn fires_on_missing_safety_comment_and_silent_when_present(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let validator = SafetyCommentValidator::new()?;
        run_fixture_parity(
            &validator,
            &manifest_dir(),
            "fixtures/safety-comment/fail_unsafe.rs",
            "fixtures/safety-comment/pass_unsafe.rs",
        )?;
        Ok(())
    }

    #[test]
    fn unparseable_source_stays_silent() -> Result<(), Box<dyn std::error::Error>> {
        use enforcer_validator::validator::Validator;
        let validator = SafetyCommentValidator::new()?;
        let file: enforcer_domain::paths::RelPath = "crates/x/src/lib.rs".parse()?;
        let findings = validator.validate(enforcer_validator::validator::ValidationInput {
            file: &file,
            source: "not valid rust {{{",
            scope: enforcer_domain::findings::ScanScope::Files,
        });
        assert!(findings.is_empty());
        Ok(())
    }
}
