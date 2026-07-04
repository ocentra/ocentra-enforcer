//! The `architecture check --language <lang> --scope <..>` CLI seam onto
//! the `architecture-policy`/`import-boundaries` named-check family
//! (`src/cli-checks.mjs` port). Both named checks are the same surface
//! `enforcer-mcp`'s registry tracks for check-enum parity; this module
//! only owns the CLI routing, not the checks themselves.
//!
//! # What is real today
//! `import-boundaries` has a landed validator
//! (`enforcer_lang_ts::rules::import_boundaries::ImportBoundariesValidator`).
//! `architecture-policy` exists only as a named-check STRING in
//! `enforcer-mcp`'s registry (parity bookkeeping) -- no backing validator
//! has landed in any crate yet. [`crate::commands::run_architecture`]
//! routes the former to a real scan and reports the latter through the
//! internal-error exit class with a diagnostic naming the gap, rather
//! than silently no-op'ing (per the workpack's explicit requirement that
//! an unimplemented named check never masquerade as "0 findings, clean").

use std::str::FromStr;

/// Language family `architecture check --language` selects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchitectureLanguage {
    Rust,
    TypeScript,
}

impl FromStr for ArchitectureLanguage {
    type Err = String;

    fn from_str(raw: &str) -> Result<Self, String> {
        match raw {
            "rust" => Ok(Self::Rust),
            "typescript" | "ts" => Ok(Self::TypeScript),
            other => Err(format!("Unknown architecture language: {other}")),
        }
    }
}

impl ArchitectureLanguage {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Rust => "rust",
            Self::TypeScript => "typescript",
        }
    }
}

impl std::fmt::Display for ArchitectureLanguage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl clap::ValueEnum for ArchitectureLanguage {
    fn value_variants<'a>() -> &'a [Self] {
        &[Self::Rust, Self::TypeScript]
    }

    fn to_possible_value(&self) -> Option<clap::builder::PossibleValue> {
        Some(clap::builder::PossibleValue::new(self.as_str()))
    }
}

#[cfg(test)]
mod tests {
    use super::ArchitectureLanguage;
    use std::str::FromStr;

    #[test]
    fn rust_and_typescript_parse() {
        assert_eq!(
            ArchitectureLanguage::from_str("rust"),
            Ok(ArchitectureLanguage::Rust)
        );
        assert_eq!(
            ArchitectureLanguage::from_str("typescript"),
            Ok(ArchitectureLanguage::TypeScript)
        );
    }

    #[test]
    fn unknown_language_is_an_error() {
        assert!(ArchitectureLanguage::from_str("cobol").is_err());
    }
}
