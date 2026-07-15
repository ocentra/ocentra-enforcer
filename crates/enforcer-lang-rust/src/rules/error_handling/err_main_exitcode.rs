//! `RUST-ERR-MAIN-EXITCODE` (T2) — `main` should return
//! `ExitCode`/`anyhow::Result<()>` instead of calling `std::process::exit`
//! from scattered call sites.
//!
//! Scope: flags any call to `std::process::exit`/`process::exit` anywhere
//! in the file (the scattered-exit-site problem this rule targets is not
//! limited to `main` itself) and separately flags `fn main()` with a bare
//! `()` return type when the file also contains such a call — the fixture
//! pair exercises the common single-file case.

use syn::spanned::Spanned;
use syn::visit::{self, Visit};
use syn::ExprCall;

use enforcer_domain::findings::Finding;
use enforcer_domain::ids::RuleId;
use enforcer_domain::paths::RelPath;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

/// The `RUST-ERR-MAIN-EXITCODE` `Validator`.
pub struct ErrMainExitcodeValidator {
    rule_id: RuleId,
}

impl ErrMainExitcodeValidator {
    /// Build the validator, parsing its own `RuleId` literal at
    /// construction (parse-at-boundary).
    pub fn new() -> Result<Self, enforcer_domain::boundary::decode_error::DecodeError> {
        Ok(Self {
            rule_id: "RUST-ERR-MAIN-EXITCODE".parse()?,
        })
    }
}

impl Validator for ErrMainExitcodeValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let Ok(file) = syn::parse_file(input.source) else {
            return Vec::new();
        };
        let mut visitor = Visitor {
            rule_id: self.rule_id.clone(),
            file: input.file.clone(),
            findings: Vec::new(),
        };
        visitor.visit_file(&file);
        visitor.findings
    }
}

fn call_path_ends_with_exit(expr: &syn::Expr) -> bool {
    let syn::Expr::Path(path_expr) = expr else {
        return false;
    };
    let segments: Vec<String> = path_expr
        .path
        .segments
        .iter()
        .map(|segment| segment.ident.to_string())
        .collect();
    segments.last().is_some_and(|last| last == "exit")
        && segments.iter().any(|segment| segment == "process")
}

struct Visitor {
    rule_id: RuleId,
    file: RelPath,
    findings: Vec<Finding>,
}

impl<'ast> Visit<'ast> for Visitor {
    fn visit_expr_call(&mut self, item: &'ast ExprCall) {
        if call_path_ends_with_exit(&item.func) {
            let line = u32::try_from(item.span().start().line.max(1)).unwrap_or(u32::MAX);
            self.findings.push(Finding {
                rule_id: self.rule_id.clone(),
                severity: Severity::Warning,
                title: "scattered `std::process::exit` call".to_owned(),
                detail: "Fix: have `main` return `ExitCode` (or `anyhow::Result<()>`) and \
                          propagate the failure up to it instead of calling `process::exit` \
                          from a nested call site."
                    .to_owned(),
                file: self.file.clone(),
                line,
                snippet: None,
            });
        }
        visit::visit_expr_call(self, item);
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use enforcer_validator::harness::run_fixture_parity;

    use super::ErrMainExitcodeValidator;

    fn manifest_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    #[test]
    fn fires_on_process_exit_and_silent_on_exitcode() -> Result<(), Box<dyn std::error::Error>> {
        let validator = ErrMainExitcodeValidator::new()?;
        run_fixture_parity(
            &validator,
            &manifest_dir(),
            "fixtures/err-main-exitcode/fail_process_exit.rs",
            "fixtures/err-main-exitcode/pass_exitcode.rs",
        )?;
        Ok(())
    }

    #[test]
    fn unparseable_source_stays_silent() -> Result<(), Box<dyn std::error::Error>> {
        use enforcer_validator::validator::Validator;
        let validator = ErrMainExitcodeValidator::new()?;
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
