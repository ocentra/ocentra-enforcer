//! `RUST-ERR-CONTEXT` (T2) — a bare `?` on a fallible standard-library I/O
//! call (`std::fs`/`std::io` free functions) inside a `commands/`-lane
//! function should carry `.with_context(...)` so the propagated error
//! names what operation failed, not just the underlying I/O error text.
//!
//! Scope: flags `<fs-call>(...)?` where the `?` operand is a direct call
//! to a function whose last path segment is a known `std::fs` read/write
//! helper (`read_to_string`, `read`, `write`, `create`, `remove_file`,
//! etc.) with no `.with_context`/`.context` method call wrapping it.
//! Non-blocking (T2/`Severity::Warning`): scored discipline, not a hard
//! compile-time gate.

use syn::visit::{self, Visit};
use syn::{Expr, ExprTry};

use enforcer_domain::findings::Finding;
use enforcer_domain::ids::{BuiltInRustRule, RuleId};
use enforcer_domain::paths::RelPath;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

const FS_IO_FNS: &[&str] = &[
    "read_to_string",
    "read",
    "write",
    "create",
    "remove_file",
    "rename",
    "copy",
    "create_dir",
    "create_dir_all",
    "remove_dir",
    "remove_dir_all",
];

/// The `RUST-ERR-CONTEXT` `Validator`.
#[derive(Debug)]
pub struct ErrContextValidator {
    rule_id: RuleId,
}

impl ErrContextValidator {
    /// Build the validator, parsing its own `RuleId` literal at
    /// construction (parse-at-boundary).
    pub fn new() -> Result<Self, enforcer_domain::boundary::decode_error::DecodeError> {
        Ok(Self {
            rule_id: BuiltInRustRule::ErrContext.id(),
        })
    }
}

impl Validator for ErrContextValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let Ok(file) = syn::parse_file(input.source.as_str()) else {
            return Vec::new();
        };
        let mut visitor = Visitor {
            rule_id: &self.rule_id,
            file: input.file,
            findings: Vec::new(),
        };
        visitor.visit_file(&file);
        visitor.findings
    }
}

fn is_bare_fs_io_call(expr: &Expr) -> Option<String> {
    let Expr::Call(call) = expr else {
        return None;
    };
    let Expr::Path(path_expr) = call.func.as_ref() else {
        return None;
    };
    let segment = path_expr.path.segments.last()?;
    FS_IO_FNS
        .iter()
        .find(|name| segment.ident == **name)
        .map(|name| format!("fs::{name}"))
}

struct Visitor<'a> {
    rule_id: &'a RuleId,
    file: &'a RelPath,
    findings: Vec<Finding>,
}

impl<'ast> Visit<'ast> for Visitor<'_> {
    fn visit_expr_try(&mut self, item: &'ast ExprTry) {
        if let Some(call_name) = is_bare_fs_io_call(&item.expr) {
            let line = crate::boundary::finding::source_line(item);
            let Ok(finding) = crate::boundary::finding::from_source(
                (self.rule_id, Severity::Warning),
                format!("bare `{call_name}(...)?` with no `.with_context`"),
                format!(
                    "Fix: attach `.with_context(|| format!(\"...\"))?` to this `{call_name}` \
                     call so the propagated error names the operation, not just the \
                     underlying I/O error text."
                ),
                self.file,
                line,
            ) else {
                return;
            };
            self.findings.push(finding);
        }
        visit::visit_expr_try(self, item);
    }
}

#[cfg(test)]
mod tests {

    use crate::boundary::fixture::run_fixture_parity;

    use super::ErrContextValidator;

    #[test]
    fn fires_on_bare_question_and_silent_with_context() -> Result<(), Box<dyn std::error::Error>> {
        let validator = ErrContextValidator::new()?;
        run_fixture_parity(
            &validator,
            "fixtures/err-context/fail_bare_question.rs",
            "fixtures/err-context/pass_with_context.rs",
        )?;
        Ok(())
    }

    #[test]
    fn malformed_source_stays_silent() -> Result<(), Box<dyn std::error::Error>> {
        use enforcer_validator::validator::Validator;
        let validator = ErrContextValidator::new()?;
        let file: enforcer_domain::paths::RelPath =
            crate::boundary::fixture::source_file("crates/x/src/lib.rs")?;
        let findings = validator.validate(enforcer_validator::validator::ValidationInput {
            file: &file,
            source: enforcer_domain::boundary::validation::ValidationSource::from_text(
                "malformed rust {{{",
            ),
            scope: enforcer_domain::findings::ScanScope::Files,
        });
        assert!(findings.is_empty());
        Ok(())
    }
}
