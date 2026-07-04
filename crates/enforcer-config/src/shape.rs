//! `sourceShapePolicies` — the per-root/per-extension source-shape budget.
//!
//! This is a first-class config field [G4]: arc-04 (rules) consumes the
//! resolved budgets to run the source-shape check; the honest per-file
//! tuning + dishonest-bump distinction lives in a08, which references this
//! shape but does not own it.

use serde::{Deserialize, Serialize};

/// The language/file kind a [`SourceShapePolicy`] applies to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SourceShapeKind {
    /// TypeScript/JavaScript family sources.
    Typescript,
    /// Rust sources.
    Rust,
    /// Python sources.
    Python,
    /// Cross-language / generic sources.
    Common,
}

/// One source-shape budget entry: applies to files under `roots` matching
/// `extensions`. Every limit field is `Option<usize>` — a policy sets only
/// the dimensions relevant to its `kind` (e.g. a rust entry sets
/// `max_types`, not `max_classes`); an absent limit means unbounded, never
/// zero.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceShapePolicy {
    /// Root directories (repo-relative globs) this policy applies to.
    pub roots: Vec<String>,
    /// File extensions (with leading dot, e.g. `.rs`) this policy applies
    /// to.
    pub extensions: Vec<String>,
    /// The language/file kind this policy budgets for.
    pub kind: SourceShapeKind,
    /// Max class count per file (typescript-shaped budgets).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_classes: Option<usize>,
    /// Max public export count per file.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_exports: Option<usize>,
    /// Max function count per file.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_functions: Option<usize>,
    /// Max lines per function.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_function_lines: Option<usize>,
    /// Max lines per file.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_lines: Option<usize>,
    /// Max type count per file (rust-shaped budgets: structs+enums+traits).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_types: Option<usize>,
}

#[cfg(test)]
mod tests {
    use super::{SourceShapeKind, SourceShapePolicy};

    #[test]
    fn omitted_limit_deserializes_to_none_not_zero() -> Result<(), serde_json::Error> {
        let json = r#"{
            "roots": ["Tools", "tools"],
            "extensions": [".rs"],
            "kind": "rust",
            "maxFunctionLines": 80,
            "maxFunctions": 18,
            "maxLines": 1000,
            "maxTypes": 24
        }"#;
        let policy: SourceShapePolicy = serde_json::from_str(json)?;
        assert_eq!(policy.kind, SourceShapeKind::Rust);
        assert_eq!(policy.max_classes, None);
        assert_eq!(policy.max_exports, None);
        assert_eq!(policy.max_types, Some(24));
        Ok(())
    }

    #[test]
    fn round_trips_through_serialize_deserialize() -> Result<(), serde_json::Error> {
        let policy = SourceShapePolicy {
            roots: vec!["src".to_owned()],
            extensions: vec![".ts".to_owned()],
            kind: SourceShapeKind::Typescript,
            max_classes: Some(1),
            max_exports: Some(35),
            max_functions: Some(30),
            max_function_lines: Some(80),
            max_lines: Some(1000),
            max_types: None,
        };
        let wire = serde_json::to_string(&policy)?;
        let back: SourceShapePolicy = serde_json::from_str(&wire)?;
        assert_eq!(policy, back);
        Ok(())
    }
}
