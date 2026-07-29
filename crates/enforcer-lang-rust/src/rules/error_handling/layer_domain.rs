//! `RUST-LAYER-1.1` — files under a `domain/` directory must not import a
//! forbidden I/O/framework crate; the domain layer stays pure.
//!
//! Path-scoped: only fires when [`ValidationInput::file`] has a path
//! segment named `domain` (e.g. `src/domain/x.rs`); files outside that
//! layer are untouched regardless of what they import.

use syn::visit::{self, Visit};
use syn::ItemUse;

use enforcer_domain::findings::Finding;
use enforcer_domain::ids::{BuiltInRustRule, RuleId};
use enforcer_domain::paths::RelPath;
use enforcer_domain::rules_types::RulePredicateResult;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

/// Crate/module roots forbidden inside `src/domain/`.
const FORBIDDEN_ROOTS: &[&str] = &[
    "clap", "rmcp", "reqwest", "hyper", "axum", "cli", "commands", "tools", "server",
];

/// The `RUST-LAYER-1.1` `Validator`.
#[derive(Debug)]
pub struct LayerDomainValidator {
    rule_id: RuleId,
}

impl LayerDomainValidator {
    /// Build the validator, parsing its own `RuleId` literal at
    /// construction (parse-at-boundary).
    pub fn new() -> Result<Self, enforcer_domain::boundary::decode_error::DecodeError> {
        Ok(Self {
            rule_id: BuiltInRustRule::LayerDomain.id(),
        })
    }
}

/// True when `path` has a `domain` path segment (case-sensitive, matching
/// the conventional `src/domain/...` layout).
fn is_domain_file(path: &RelPath) -> RulePredicateResult {
    if path.as_str().split('/').any(|segment| segment == "domain") {
        RulePredicateResult::Matched
    } else {
        RulePredicateResult::NotMatched
    }
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

fn forbidden_root(ident: &syn::Ident) -> Option<&'static str> {
    FORBIDDEN_ROOTS
        .iter()
        .copied()
        .find(|candidate| ident == candidate)
}

fn forbidden_pair(root: &syn::Ident, leaf: &syn::Ident) -> Option<(&'static str, &'static str)> {
    FORBIDDEN_TWO_SEGMENT
        .iter()
        .copied()
        .find(|(candidate_root, candidate_leaf)| root == candidate_root && leaf == candidate_leaf)
}

fn use_tree_forbidden(tree: &syn::UseTree, root: Option<&syn::Ident>) -> Option<String> {
    match tree {
        syn::UseTree::Path(use_path) => {
            if let Some(root) = root {
                if let Some((root, leaf)) = forbidden_pair(root, &use_path.ident) {
                    return Some(format!("{root}::{leaf}"));
                }
                return None;
            }
            if let Some(root) = forbidden_root(&use_path.ident) {
                return Some(root.into());
            }
            use_tree_forbidden(&use_path.tree, Some(&use_path.ident))
        }
        syn::UseTree::Name(use_name) => {
            if root.is_none() {
                return forbidden_root(&use_name.ident).map(String::from);
            }
            None
        }
        syn::UseTree::Group(group) => group
            .items
            .iter()
            .find_map(|inner| use_tree_forbidden(inner, root)),
        syn::UseTree::Glob(_) | syn::UseTree::Rename(_) => None,
    }
}

struct Visitor<'a> {
    rule_id: &'a RuleId,
    file: &'a RelPath,
    findings: Vec<Finding>,
}

impl<'ast> Visit<'ast> for Visitor<'_> {
    fn visit_item_use(&mut self, item: &'ast ItemUse) {
        if let Some(forbidden) = use_tree_forbidden(&item.tree, None) {
            let line = crate::boundary::finding::source_line(item);
            let Ok(finding) = crate::boundary::finding::from_source(
                (self.rule_id, Severity::Error),
                format!("forbidden import `{forbidden}` in domain layer"),
                format!(
                    "Fix: remove this `use {forbidden}...` from a `domain/` file; the domain \
                     layer must stay pure of I/O and framework crates. Move the call site to \
                     a `cli`/`commands`/`server` layer that depends on `domain`, not the \
                     reverse."
                ),
                self.file,
                line,
            ) else {
                return;
            };
            self.findings.push(finding);
        }
        visit::visit_item_use(self, item);
    }
}

impl Validator for LayerDomainValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        if is_domain_file(input.file) == RulePredicateResult::NotMatched {
            return Vec::new();
        }
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
        let file: RelPath = crate::boundary::fixture::source_file("crates/x/src/domain/x.rs")?;
        let findings = validator.validate(ValidationInput {
            file: &file,
            source: enforcer_domain::boundary::validation::ValidationSource::from_text(&source),
            scope: ScanScope::Files,
        });
        assert_eq!(findings.len(), 1);
        assert!(findings
            .iter()
            .all(|f| f.rule_id.as_str() == "RUST-LAYER-1.1"));
        Ok(())
    }

    #[test]
    fn silent_on_pure_domain_file() -> Result<(), Box<dyn std::error::Error>> {
        let validator = LayerDomainValidator::new()?;
        let source = read_fixture("fixtures/layer-domain/pass_pure_domain.rs")?;
        let file: RelPath = crate::boundary::fixture::source_file("crates/x/src/domain/x.rs")?;
        let findings = validator.validate(ValidationInput {
            file: &file,
            source: enforcer_domain::boundary::validation::ValidationSource::from_text(&source),
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
        let file: RelPath = crate::boundary::fixture::source_file("crates/x/src/cli/x.rs")?;
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
        let validator = LayerDomainValidator::new()?;
        let file: RelPath = crate::boundary::fixture::source_file("crates/x/src/domain/x.rs")?;
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
