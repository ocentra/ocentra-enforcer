//! `RUST-LAYER-1.1` — files under a `domain/` directory must not import a
//! forbidden I/O/framework crate; the domain layer stays pure.
//!
//! Path-scoped: only fires when [`ValidationInput::file`] has a path
//! segment named `domain` (e.g. `src/domain/x.rs`); files outside that
//! layer are untouched regardless of what they import.

use syn::spanned::Spanned;
use syn::visit::{self, Visit};
use syn::ItemUse;

use enforcer_domain::findings::Finding;
use enforcer_domain::ids::RuleId;
use enforcer_domain::paths::RelPath;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

/// Crate/module roots forbidden inside `src/domain/`.
const FORBIDDEN_ROOTS: &[&str] = &[
    "clap", "rmcp", "reqwest", "hyper", "axum", "cli", "commands", "tools", "server",
];

/// The `RUST-LAYER-1.1` `Validator`.
pub struct LayerDomainValidator {
    rule_id: RuleId,
}

impl LayerDomainValidator {
    /// Build the validator, parsing its own `RuleId` literal at
    /// construction (parse-at-boundary).
    pub fn new() -> Result<Self, enforcer_core::error::DecodeError> {
        Ok(Self {
            rule_id: "RUST-LAYER-1.1".parse()?,
        })
    }
}

/// True when `path` has a `domain` path segment (case-sensitive, matching
/// the conventional `src/domain/...` layout).
fn is_domain_file(path: &RelPath) -> bool {
    path.as_str().split('/').any(|segment| segment == "domain")
}

/// `tokio::net` / `tokio::fs` / `std::process` / `std::io` are two-segment
/// forbidden paths; check the leading two segments in addition to the
/// single-segment [`FORBIDDEN_ROOTS`] check.
const FORBIDDEN_TWO_SEGMENT: &[(&str, &str)] = &[
    ("tokio", "net"),
    ("tokio", "fs"),
    ("std", "process"),
    ("std", "io"),
];

fn use_tree_forbidden(tree: &syn::UseTree, prefix: &[String]) -> Option<String> {
    match tree {
        syn::UseTree::Path(use_path) => {
            let mut next_prefix = prefix.to_vec();
            next_prefix.push(use_path.ident.to_string());
            match next_prefix.as_slice() {
                [root] if FORBIDDEN_ROOTS.contains(&root.as_str()) => {
                    return Some(next_prefix.join("::"));
                }
                [root, leaf] if FORBIDDEN_TWO_SEGMENT.contains(&(root.as_str(), leaf.as_str())) => {
                    return Some(format!("{root}::{leaf}"));
                }
                _ => {}
            }
            use_tree_forbidden(&use_path.tree, &next_prefix)
        }
        syn::UseTree::Name(use_name) => {
            let mut next_prefix = prefix.to_vec();
            next_prefix.push(use_name.ident.to_string());
            if let [root] = next_prefix.as_slice() {
                if FORBIDDEN_ROOTS.contains(&root.as_str()) {
                    return Some(next_prefix.join("::"));
                }
            }
            None
        }
        syn::UseTree::Group(group) => group
            .items
            .iter()
            .find_map(|inner| use_tree_forbidden(inner, prefix)),
        syn::UseTree::Glob(_) | syn::UseTree::Rename(_) => None,
    }
}

struct Visitor {
    rule_id: RuleId,
    file: RelPath,
    findings: Vec<Finding>,
}

impl<'ast> Visit<'ast> for Visitor {
    fn visit_item_use(&mut self, item: &'ast ItemUse) {
        if let Some(forbidden) = use_tree_forbidden(&item.tree, &[]) {
            let line = u32::try_from(item.span().start().line.max(1)).unwrap_or(u32::MAX);
            self.findings.push(Finding {
                rule_id: self.rule_id.clone(),
                severity: Severity::Error,
                title: format!("forbidden import `{forbidden}` in domain layer"),
                detail: format!(
                    "Fix: remove this `use {forbidden}...` from a `domain/` file; the domain \
                     layer must stay pure of I/O and framework crates. Move the call site to \
                     a `cli`/`commands`/`server` layer that depends on `domain`, not the \
                     reverse."
                ),
                file: self.file.clone(),
                line,
                snippet: None,
            });
        }
        visit::visit_item_use(self, item);
    }
}

impl Validator for LayerDomainValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        if !is_domain_file(input.file) {
            return Vec::new();
        }
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

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use enforcer_domain::findings::ScanScope;
    use enforcer_domain::paths::RelPath;
    use enforcer_validator::validator::{ValidationInput, Validator};

    use super::LayerDomainValidator;

    fn manifest_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    fn read_fixture(rel: &str) -> std::io::Result<String> {
        fs::read_to_string(manifest_dir().join(rel))
    }

    #[test]
    fn fires_on_forbidden_import_in_domain_file() -> Result<(), Box<dyn std::error::Error>> {
        let validator = LayerDomainValidator::new()?;
        let source = read_fixture("fixtures/layer-domain/fail_forbidden_import.rs")?;
        let file: RelPath = "crates/x/src/domain/x.rs".parse()?;
        let findings = validator.validate(ValidationInput {
            file: &file,
            source: &source,
            scope: ScanScope::Files,
        });
        assert!(!findings.is_empty());
        assert!(findings
            .iter()
            .all(|f| f.rule_id.as_str() == "RUST-LAYER-1.1"));
        Ok(())
    }

    #[test]
    fn silent_on_pure_domain_file() -> Result<(), Box<dyn std::error::Error>> {
        let validator = LayerDomainValidator::new()?;
        let source = read_fixture("fixtures/layer-domain/pass_pure_domain.rs")?;
        let file: RelPath = "crates/x/src/domain/x.rs".parse()?;
        let findings = validator.validate(ValidationInput {
            file: &file,
            source: &source,
            scope: ScanScope::Files,
        });
        assert!(findings.is_empty());
        Ok(())
    }

    #[test]
    fn silent_outside_domain_layer_even_with_forbidden_import(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let validator = LayerDomainValidator::new()?;
        let source = read_fixture("fixtures/layer-domain/fail_forbidden_import.rs")?;
        let file: RelPath = "crates/x/src/cli/x.rs".parse()?;
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
        let validator = LayerDomainValidator::new()?;
        let file: RelPath = "crates/x/src/domain/x.rs".parse()?;
        let findings = validator.validate(ValidationInput {
            file: &file,
            source: "not valid rust {{{",
            scope: ScanScope::Files,
        });
        assert!(findings.is_empty());
        Ok(())
    }
}
