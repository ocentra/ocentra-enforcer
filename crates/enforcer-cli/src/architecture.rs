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

/// A rejected architecture language at the CLI boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArchitectureLanguageError {
    /// The supplied language has no architecture-policy implementation.
    UnknownLanguage { raw: String },
}

impl std::fmt::Display for ArchitectureLanguageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownLanguage { raw } => write!(f, "Unknown architecture language: {raw}"),
        }
    }
}

impl std::error::Error for ArchitectureLanguageError {}

impl FromStr for ArchitectureLanguage {
    type Err = ArchitectureLanguageError;

    fn from_str(raw: &str) -> Result<Self, Self::Err> {
        match raw {
            "rust" => Ok(Self::Rust),
            "typescript" | "ts" => Ok(Self::TypeScript),
            other => Err(ArchitectureLanguageError::UnknownLanguage {
                raw: other.to_owned(),
            }),
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
    fn unknown_language_is_an_error() -> Result<(), Box<dyn std::error::Error>> {
        let error = ArchitectureLanguage::from_str("cobol")
            .err()
            .ok_or("must reject cobol")?;
        assert_eq!(error.to_string(), "Unknown architecture language: cobol");
        Ok(())
    }
}
