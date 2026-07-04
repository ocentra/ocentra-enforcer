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

use syn::spanned::Spanned;
use syn::visit::{self, Visit};
use syn::{ExprCall, ExprMacro, ExprMethodCall};

use enforcer_domain::findings::Finding;
use enforcer_domain::ids::RuleId;
use enforcer_domain::paths::RelPath;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

/// The `RUST-MCP-1.1` `Validator`.
pub struct McpStdoutValidator {
    rule_id: RuleId,
}

impl McpStdoutValidator {
    /// Build the validator, parsing its own `RuleId` literal at
    /// construction (parse-at-boundary).
    pub fn new() -> Result<Self, enforcer_core::error::DecodeError> {
        Ok(Self {
            rule_id: "RUST-MCP-1.1".parse()?,
        })
    }
}

impl Validator for McpStdoutValidator {
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

const BANNED_MACROS: &[&str] = &["println", "print"];

fn call_path_is_stdout(expr: &syn::Expr) -> bool {
    let syn::Expr::Path(path_expr) = expr else {
        return false;
    };
    path_expr
        .path
        .segments
        .last()
        .is_some_and(|segment| segment.ident == "stdout")
}

struct Visitor {
    rule_id: RuleId,
    file: RelPath,
    findings: Vec<Finding>,
}

impl Visitor {
    fn push(&mut self, line: u32, what: &str) {
        self.findings.push(Finding {
            rule_id: self.rule_id.clone(),
            severity: Severity::Error,
            title: format!("stdout write (`{what}`) in an MCP-stdio-lane crate"),
            detail: "Fix: stdout is the MCP protocol channel — route this write to stderr \
                      (`eprintln!`) or `tracing` instead."
                .to_owned(),
            file: self.file.clone(),
            line,
            snippet: None,
        });
    }
}

impl<'ast> Visit<'ast> for Visitor {
    fn visit_expr_macro(&mut self, item: &'ast ExprMacro) {
        if let Some(segment) = item.mac.path.segments.last() {
            let name = segment.ident.to_string();
            if BANNED_MACROS.contains(&name.as_str()) {
                let line = u32::try_from(item.span().start().line.max(1)).unwrap_or(u32::MAX);
                self.push(line, &format!("{name}!"));
            }
        }
        visit::visit_expr_macro(self, item);
    }

    fn visit_expr_call(&mut self, item: &'ast ExprCall) {
        if call_path_is_stdout(&item.func) {
            let line = u32::try_from(item.span().start().line.max(1)).unwrap_or(u32::MAX);
            self.push(line, "stdout()");
        }
        visit::visit_expr_call(self, item);
    }

    fn visit_expr_method_call(&mut self, item: &'ast ExprMethodCall) {
        // Catch `io::stdout().write(...)` / `.write_all(...)` chains by
        // checking whether the receiver expression ultimately calls
        // `stdout()` anywhere in its call chain.
        if expr_contains_stdout_call(&item.receiver) {
            let line = u32::try_from(item.span().start().line.max(1)).unwrap_or(u32::MAX);
            self.push(line, "stdout()...write");
        }
        visit::visit_expr_method_call(self, item);
    }
}

fn expr_contains_stdout_call(expr: &syn::Expr) -> bool {
    match expr {
        syn::Expr::Call(call) => call_path_is_stdout(&call.func),
        syn::Expr::MethodCall(call) => expr_contains_stdout_call(&call.receiver),
        _ => false,
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
        let file: RelPath = "crates/enforcer-mcp/src/lib.rs".parse()?;
        let findings = validator.validate(ValidationInput {
            file: &file,
            source: &source,
            scope: ScanScope::Files,
        });
        assert!(!findings.is_empty());
        assert!(findings
            .iter()
            .all(|f| f.rule_id.as_str() == "RUST-MCP-1.1"));
        Ok(())
    }

    #[test]
    fn silent_on_stderr_tracing_writes() -> Result<(), Box<dyn std::error::Error>> {
        let validator = McpStdoutValidator::new()?;
        let source = read_fixture("fixtures/mcp-stdout/pass_stderr_tracing.rs")?;
        let file: RelPath = "crates/enforcer-mcp/src/lib.rs".parse()?;
        let findings = validator.validate(ValidationInput {
            file: &file,
            source: &source,
            scope: ScanScope::Files,
        });
        assert!(findings.is_empty());
        Ok(())
    }

    #[test]
    fn unparseable_source_stays_silent() -> Result<(), Box<dyn std::error::Error>> {
        let validator = McpStdoutValidator::new()?;
        let file: RelPath = "crates/enforcer-mcp/src/lib.rs".parse()?;
        let findings = validator.validate(ValidationInput {
            file: &file,
            source: "not valid rust {{{",
            scope: ScanScope::Files,
        });
        assert!(findings.is_empty());
        Ok(())
    }
}
