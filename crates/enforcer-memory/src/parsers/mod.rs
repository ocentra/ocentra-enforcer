//! Language-agnostic parse result shape, plus the extension-based
//! dispatch that routes a file's content to the right
//! [`crate::languages`] extractor (or the `TextOnly` fallback).
//!
//! [`crate::code_graph`] never touches a tree-sitter tree directly: it
//! only ever sees a [`ParsedFile`], so adding a new language is a
//! change fully contained to `languages/` + the one dispatch arm in
//! [`parse_file`] below.

use crate::languages::{python, rust, typescript};

/// One extracted symbol: a function, type, or test found in a source
/// file. Route/import/call extraction is intentionally modeled
/// separately ([`RouteRef`], [`ImportRef`], [`CallRef`]) because those
/// are edges (relationships to other nodes), not nodes in their own
/// right.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SymbolRef {
    pub name: String,
    pub kind: SymbolKind,
    /// 1-based start line in the source file, for stable ids and for
    /// human-readable "why selected" traces.
    pub line: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolKind {
    Function,
    Type,
    Test,
}

/// An HTTP-style route/endpoint declaration found in source (e.g. an
/// Axum/Actix/Express/FastAPI decorator or macro).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteRef {
    pub method: String,
    pub path: String,
    pub line: usize,
}

/// One import/use statement, module-path as written in source (not yet
/// resolved to a graph node id -- resolution is [`crate::code_graph`]'s
/// job once every file in the repo has been parsed).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportRef {
    pub module_path: String,
    pub line: usize,
}

/// One function-call expression's callee name, as written (unresolved
/// -- same rationale as [`ImportRef`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallRef {
    pub callee: String,
    pub line: usize,
}

/// The language-agnostic result of parsing one source file.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ParsedFile {
    pub symbols: Vec<SymbolRef>,
    pub routes: Vec<RouteRef>,
    pub imports: Vec<ImportRef>,
    pub calls: Vec<CallRef>,
}

/// Which extractor produced (or would produce) a [`ParsedFile`] for a
/// given path -- also doubles as the "supported language" predicate
/// [`crate::code_graph`] uses to decide symbol/route/import/call nodes
/// vs a bare `TextOnly` node.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    Rust,
    TypeScript,
    JavaScript,
    Python,
    ConfigToml,
    ConfigJson,
    ConfigYaml,
    /// Anything else: still indexed as a file node, but with no
    /// structural extraction -- see the workpack's "unsupported files
    /// become TextOnly nodes, never silent skip" hard requirement.
    TextOnly,
}

/// Classify a file purely by its extension. Case-insensitive so
/// `Foo.RS`/`foo.rs` land the same way.
pub fn classify(rel_path: &str) -> Language {
    let ext = rel_path
        .rsplit('.')
        .next()
        .unwrap_or_default()
        .to_lowercase();
    match ext.as_str() {
        "rs" => Language::Rust,
        "ts" | "tsx" | "mts" | "cts" => Language::TypeScript,
        "js" | "jsx" | "mjs" | "cjs" => Language::JavaScript,
        "py" | "pyi" => Language::Python,
        "toml" => Language::ConfigToml,
        "json" => Language::ConfigJson,
        "yml" | "yaml" => Language::ConfigYaml,
        _ => Language::TextOnly,
    }
}

/// Parse `source` per `language`. Returns `None` for languages that
/// have no structural extractor ([`Language::ConfigToml`],
/// [`Language::ConfigJson`], [`Language::ConfigYaml`],
/// [`Language::TextOnly`]) -- callers must still create a file node for
/// those, just with no symbols/routes/imports/calls attached (the
/// `TextOnly`-node fallback).
pub fn parse_file(language: Language, source: &str) -> Option<ParsedFile> {
    match language {
        Language::Rust => Some(rust::parse(source)),
        Language::TypeScript | Language::JavaScript => Some(typescript::parse(source, language)),
        Language::Python => Some(python::parse(source)),
        Language::ConfigToml | Language::ConfigJson | Language::ConfigYaml | Language::TextOnly => {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_recognizes_rust_typescript_python_config() {
        assert_eq!(classify("src/main.rs"), Language::Rust);
        assert_eq!(classify("src/App.tsx"), Language::TypeScript);
        assert_eq!(classify("scripts/build.js"), Language::JavaScript);
        assert_eq!(classify("app/main.py"), Language::Python);
        assert_eq!(classify("Cargo.toml"), Language::ConfigToml);
        assert_eq!(classify("package.json"), Language::ConfigJson);
        assert_eq!(classify("ci.yml"), Language::ConfigYaml);
    }

    #[test]
    fn classify_unknown_extension_is_text_only() {
        assert_eq!(classify("NOTES.qux"), Language::TextOnly);
        assert_eq!(classify("no_extension_at_all"), Language::TextOnly);
    }
}
