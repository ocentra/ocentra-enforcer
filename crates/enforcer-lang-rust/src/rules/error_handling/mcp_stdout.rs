//! `RUST-MCP-1.1` — a crate whose source tree carries an `rmcp` dependency
//! must never write to stdout; stdout is the MCP stdio protocol channel,
//! so any `println!`/`print!`/`io::stdout()`/`stdout()` write corrupts the
//! wire. Writes must go to stderr or `tracing` instead.
//!
//! Path-scoped like [`super::layer_domain`]: this validator is only
//! meaningful when run against files inside a crate that depends on
//! `rmcp` (the caller — the orchestrating scan — is responsible for that
//! crate-level gating via `Cargo.toml` dependency inspection, which is
//! outside a single-file [`Validator`]'s contract). This validator itself
//! flags any stdout write in ANY file it is pointed at; callers scope
//! invocation to rmcp-dependent crates.

use syn::visit::{self, Visit};
use syn::{ExprCall, ExprMacro, ExprMethodCall};

use enforcer_domain::findings::Finding;
use enforcer_domain::ids::{BuiltInRustRule, RuleId};
use enforcer_domain::paths::RelPath;
use enforcer_domain::rules_types::RulePredicateResult;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

/// The `RUST-MCP-1.1` `Validator`.
#[derive(Debug)]
pub struct McpStdoutValidator {
    rule_id: RuleId,
}

impl McpStdoutValidator {
    /// Build the validator, parsing its own `RuleId` literal at
    /// construction (parse-at-boundary).
    pub fn new() -> Result<Self, enforcer_domain::boundary::decode_error::DecodeError> {
        Ok(Self {
            rule_id: BuiltInRustRule::McpStdout.id(),
        })
    }
}

impl Validator for McpStdoutValidator {
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

const BANNED_MACROS: &[&str] = &["println", "print"];

fn call_path_is_stdout(expr: &syn::Expr) -> RulePredicateResult {
    let syn::Expr::Path(path_expr) = expr else {
        return RulePredicateResult::NotMatched;
    };
    if path_expr
        .path
        .segments
        .last()
        .is_some_and(|segment| segment.ident == "stdout")
    {
        RulePredicateResult::Matched
    } else {
        RulePredicateResult::NotMatched
    }
}

struct Visitor<'a> {
    rule_id: &'a RuleId,
    file: &'a RelPath,
    findings: Vec<Finding>,
}

impl Visitor<'_> {
    fn push(&mut self, line: enforcer_domain::telemetry_types::SourceLine, what: &str) {
        let Ok(finding) = crate::boundary::finding::from_source(
            (self.rule_id, Severity::Error),
            format!("stdout write (`{what}`) in an MCP-stdio-lane crate"),
            "Fix: stdout is the MCP protocol channel — route this write to stderr \
                      (`eprintln!`) or `tracing` instead.",
            self.file,
            line,
        ) else {
            return;
        };
        self.findings.push(finding);
    }
}

impl<'ast> Visit<'ast> for Visitor<'_> {
    fn visit_expr_macro(&mut self, item: &'ast ExprMacro) {
        if let Some(segment) = item.mac.path.segments.last() {
            if let Some(name) = BANNED_MACROS.iter().find(|name| segment.ident == **name) {
                let line = crate::boundary::finding::source_line(item);
                self.push(line, &format!("{name}!"));
            }
        }
        visit::visit_expr_macro(self, item);
    }

    fn visit_expr_call(&mut self, item: &'ast ExprCall) {
        if call_path_is_stdout(&item.func) == RulePredicateResult::Matched {
            let line = crate::boundary::finding::source_line(item);
            self.push(line, "stdout()");
        }
        visit::visit_expr_call(self, item);
    }

    fn visit_expr_method_call(&mut self, item: &'ast ExprMethodCall) {
        // Catch `io::stdout().write(...)` / `.write_all(...)` chains by
        // checking whether the receiver expression ultimately calls
        // `stdout()` anywhere in its call chain.
        if expr_contains_stdout_call(&item.receiver) == RulePredicateResult::Matched {
            let line = crate::boundary::finding::source_line(item);
            self.push(line, "stdout()...write");
            // The receiver call is already represented by the method-chain
            // finding. Visit only the arguments so the nested `stdout()`
            // call cannot emit a duplicate finding for the same operation.
            for argument in &item.args {
                self.visit_expr(argument);
            }
            return;
        }
        visit::visit_expr_method_call(self, item);
    }
}

fn expr_contains_stdout_call(expr: &syn::Expr) -> RulePredicateResult {
    match expr {
        syn::Expr::Call(call) => call_path_is_stdout(&call.func),
        syn::Expr::MethodCall(call) => expr_contains_stdout_call(&call.receiver),
        _ => RulePredicateResult::NotMatched,
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use enforcer_domain::findings::ScanScope;
    use enforcer_domain::paths::RelPath;
    use enforcer_validator::validator::{ValidationInput, Validator};

    use super::McpStdoutValidator;

    fn manifest_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    fn read_fixture(rel: &str) -> std::io::Result<String> {
        fs::read_to_string(manifest_dir().join(rel))
    }

    #[test]
    fn fires_on_stdout_write() -> Result<(), Box<dyn std::error::Error>> {
        let validator = McpStdoutValidator::new()?;
        let source = read_fixture("fixtures/mcp-stdout/fail_stdout_write.rs")?;
        let file: RelPath =
            crate::boundary::fixture::source_file("crates/enforcer-mcp/src/lib.rs")?;
        let findings = validator.validate(ValidationInput {
            file: &file,
            source: enforcer_domain::boundary::validation::ValidationSource::from_text(&source),
            scope: ScanScope::Files,
        });
        assert_eq!(findings.len(), 1);
        assert!(findings
            .iter()
            .all(|f| f.rule_id.as_str() == "RUST-MCP-1.1"));
        Ok(())
    }

    #[test]
    fn silent_on_stderr_tracing_writes() -> Result<(), Box<dyn std::error::Error>> {
        let validator = McpStdoutValidator::new()?;
        let source = read_fixture("fixtures/mcp-stdout/pass_stderr_tracing.rs")?;
        let file: RelPath =
            crate::boundary::fixture::source_file("crates/enforcer-mcp/src/lib.rs")?;
        let findings = validator.validate(ValidationInput {
            file: &file,
            source: enforcer_domain::boundary::validation::ValidationSource::from_text(&source),
            scope: ScanScope::Files,
        });
        assert!(findings.is_empty());
        Ok(())
    }

    #[test]
    fn malformed_source_stays_silent() -> Result<(), Box<dyn std::error::Error>> {
        let validator = McpStdoutValidator::new()?;
        let file: RelPath =
            crate::boundary::fixture::source_file("crates/enforcer-mcp/src/lib.rs")?;
        let findings = validator.validate(ValidationInput {
            file: &file,
            source: enforcer_domain::boundary::validation::ValidationSource::from_text(
                "malformed rust {{{",
            ),
            scope: ScanScope::Files,
        });
        assert!(findings.is_empty());
        Ok(())
    }
}
