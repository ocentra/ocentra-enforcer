//! The detect-and-route router (f05): classifies each walked path and
//! dispatches it to the correct language-family
//! [`enforcer_validator::validator::Validator`] set.
//!
//! **SKELETON BOUNDARY**: arc-15 owns this module root
//! (`src/router/mod.rs`) — the [`LanguageFamily`] classification enum and
//! [`classify`], the minimal by-extension router every family dispatch
//! sits on top of. Deeper per-family adapters (a dedicated
//! `src/router/<name>.rs` per family, richer content-sniffing beyond
//! extension, config-driven root/exempt overrides) are owned by f05's own
//! feature packs, each `deps: arc-15`, landing as sibling files under this
//! same directory — they do NOT replace this root, they extend it.
//!
//! [`crate::engine`] is the only current caller of [`classify`]; it wires
//! each [`LanguageFamily`] to the family crate's registry that is already
//! landed (arc-06..13). A family with no wired registry yet (or a path
//! that classifies to [`LanguageFamily::Unknown`]) is routed to no
//! validators — not an error, not a silent full-repo default, just zero
//! findings from zero applicable validators, exactly like a genuinely
//! clean file.

use enforcer_domain::paths::RelPath;
use enforcer_domain::scan_types::LanguageFamily;

pub mod detect;
pub mod identity;
pub mod native_tie;
pub mod plan;
pub mod scope;

/// Classify a repo-relative path into its [`LanguageFamily`] by extension.
/// Pure and total: every path maps to exactly one family (falling back to
/// [`LanguageFamily::Unknown`] rather than erroring on an unrecognized
/// extension).
pub fn classify(path: &RelPath) -> LanguageFamily {
    let lower = path.as_str().to_ascii_lowercase();
    let ext = lower.rsplit('.').next().unwrap_or("");
    match ext {
        "rs" => LanguageFamily::Rust,
        "ts" | "tsx" | "js" | "jsx" | "mjs" | "cjs" => LanguageFamily::TypeScript,
        "py" => LanguageFamily::Python,
        "tf" => LanguageFamily::Terraform,
        "yaml" | "yml" | "json" => LanguageFamily::YamlOrConfig,
        _ => LanguageFamily::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::classify;
    use enforcer_domain::scan_types::LanguageFamily;
    use std::str::FromStr;

    fn rel(literal: &str) -> Result<enforcer_domain::paths::RelPath, Box<dyn std::error::Error>> {
        Ok(enforcer_domain::paths::RelPath::from_str(literal)?)
    }

    #[test]
    fn classifies_known_extensions() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(classify(&rel("src/lib.rs")?), LanguageFamily::Rust);
        assert_eq!(classify(&rel("src/app.tsx")?), LanguageFamily::TypeScript);
        assert_eq!(classify(&rel("scripts/run.py")?), LanguageFamily::Python);
        assert_eq!(classify(&rel("infra/main.tf")?), LanguageFamily::Terraform);
        assert_eq!(
            classify(&rel("k8s/pod.yaml")?),
            LanguageFamily::YamlOrConfig
        );
        Ok(())
    }

    #[test]
    fn classifies_unknown_extension_as_unknown_not_error() -> Result<(), Box<dyn std::error::Error>>
    {
        assert_eq!(classify(&rel("README.md")?), LanguageFamily::Unknown);
        assert_eq!(classify(&rel("Dockerfile")?), LanguageFamily::Unknown);
        Ok(())
    }

    #[test]
    fn classification_is_case_insensitive() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(classify(&rel("src/LIB.RS")?), LanguageFamily::Rust);
        Ok(())
    }
}
