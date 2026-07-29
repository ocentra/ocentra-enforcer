//! Typed loader: parse-at-boundary JSON loading of rule catalog files into
//! a validated [`crate::registry::RuleRegistry`]. Mirrors
//! `enforcer-config`'s load convention — read bytes, parse, validate; a
//! partially- or mal-formed catalog never becomes a live registry.

use std::path::Path;

use enforcer_domain::rules_types::{RuleCatalogJson, RuleCatalogSource};

use crate::registry::{RuleRecord, RuleRegistry};
use crate::{RuleLoadError, RuleResult};

/// Parse one JSON document (an array of [`RuleRecord`]) from an in-memory
/// string. The `source` label is used only for error messages.
pub fn parse_catalog(
    raw: &RuleCatalogJson,
    _source: &RuleCatalogSource,
) -> RuleResult<Vec<RuleRecord>> {
    crate::boundary::registry::decode_catalog(raw)
}

/// Load and validate ONE catalog file from disk into standalone records
/// (not yet merged into a registry — see [`load_registry_from_files`] to
/// merge several catalog files into one registry).
pub fn load_catalog_file(path: &Path) -> RuleResult<Vec<RuleRecord>> {
    // ALLOC-JUSTIFICATION: the error/result domain value owns the rendered filesystem source.
    let display = RuleCatalogSource::try_from(path.display().to_string()).map_err(|error| {
        RuleLoadError::Boundary {
            reason: crate::boundary_reason(error),
        }
    })?;
    let raw = std::fs::read_to_string(path).map_err(|e| RuleLoadError::Io {
        // CLONE-JUSTIFICATION: the I/O error owns the source while successful parsing still needs it.
        path: display.clone(),
        reason: crate::boundary_reason(e),
    })?;
    let raw = RuleCatalogJson::try_from(raw).map_err(|error| RuleLoadError::Boundary {
        reason: crate::boundary_reason(error),
    })?;
    parse_catalog(&raw, &display)
}

/// Load every catalog file in `paths`, merge their records, and build the
/// validated registry. Fails closed on the first malformed record, missing
/// file, or duplicate `ruleId` (whether the duplicate spans one file or
/// several).
pub fn load_registry_from_files(paths: &[&Path]) -> RuleResult<RuleRegistry> {
    let mut records = Vec::new();
    for path in paths {
        records.extend(load_catalog_file(path)?);
    }
    RuleRegistry::from_records(records)
}

/// Build a registry directly from already-parsed records (e.g. records
/// embedded via `include_str!` + [`parse_catalog`] at compile time, the
/// pattern `enforcer-config` uses for its embedded profiles).
pub fn load_registry_from_records(records: Vec<RuleRecord>) -> RuleResult<RuleRegistry> {
    RuleRegistry::from_records(records)
}

#[cfg(test)]
mod tests {
    use super::{load_registry_from_files, parse_catalog};
    use enforcer_domain::rules_types::{RuleCatalogJson, RuleCatalogSource};
    use proptest::{prop_assert_eq, proptest};
    use std::io::Write;

    const VALID_CATALOG: &str = r#"[
        {
            "ruleId": "RR-1.1",
            "version": 1,
            "title": "Sample rule",
            "tier": "T1",
            "validator": { "crateName": "enforcer-lang-rust", "path": "sample::SampleValidator" },
            "fixtures": {
                "fail": "crates/enforcer-lang-rust/fixtures/sample/fail.rs",
                "pass": "crates/enforcer-lang-rust/fixtures/sample/pass.rs"
            },
            "docAnchor": "docs/rules/SAMPLE.md#SAMPLE-1",
            "tags": ["rust"]
        }
    ]"#;

    const DUPLICATE_CATALOG: &str = r#"[
        {
            "ruleId": "RR-2.1",
            "version": 1,
            "title": "First",
            "tier": "T1",
            "validator": { "crateName": "c", "path": "p" },
            "fixtures": { "fail": "f", "pass": "p" },
            "docAnchor": "d"
        },
        {
            "ruleId": "RR-2.1",
            "version": 1,
            "title": "Second (duplicate id)",
            "tier": "T1",
            "validator": { "crateName": "c", "path": "p" },
            "fixtures": { "fail": "f", "pass": "p" },
            "docAnchor": "d"
        }
    ]"#;

    proptest! {
        #[test]
        fn parse_catalog_accepts_generated_json_whitespace(
            prefix in "[\\t\\n\\r ]{0,8}",
            suffix in "[\\t\\n\\r ]{0,8}",
        ) {
            let raw = match RuleCatalogJson::try_from(format!("{prefix}{VALID_CATALOG}{suffix}")) {
                Ok(raw) => raw,
                Err(_) => return Err(proptest::test_runner::TestCaseError::fail("catalog wrapper rejected generated JSON")),
            };
            let source = match RuleCatalogSource::try_from("generated catalog".to_owned()) {
                Ok(source) => source,
                Err(_) => return Err(proptest::test_runner::TestCaseError::fail("source wrapper rejected static source")),
            };
            let records = match parse_catalog(&raw, &source) {
                Ok(records) => records,
                Err(_) => return Err(proptest::test_runner::TestCaseError::fail("generated valid catalog did not parse")),
            };
            prop_assert_eq!(records.len(), 1);
        }
    }

    #[test]
    fn parses_a_well_formed_catalog() -> Result<(), Box<dyn std::error::Error>> {
        let raw = RuleCatalogJson::try_from(VALID_CATALOG.to_owned())?;
        let source = RuleCatalogSource::try_from("<inline>".to_owned())?;
        let records = parse_catalog(&raw, &source)?;
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].rule_id.as_str(), "RR-1.1");
        Ok(())
    }

    #[test]
    fn rejects_malformed_json() -> Result<(), Box<dyn std::error::Error>> {
        let raw = RuleCatalogJson::try_from("{not json".to_owned())?;
        let source = RuleCatalogSource::try_from("<inline>".to_owned())?;
        let outcome = parse_catalog(&raw, &source);
        assert!(matches!(outcome, Err(crate::RuleLoadError::Parse { .. })));
        Ok(())
    }

    #[test]
    fn rejects_duplicate_ids_across_the_same_file() -> Result<(), Box<dyn std::error::Error>> {
        let raw = RuleCatalogJson::try_from(DUPLICATE_CATALOG.to_owned())?;
        let source = RuleCatalogSource::try_from("<inline>".to_owned())?;
        let records = parse_catalog(&raw, &source)?;
        let outcome = crate::registry::RuleRegistry::from_records(records);
        assert!(matches!(
            outcome,
            Err(crate::RuleLoadError::DuplicateRuleId { .. })
        ));
        Ok(())
    }

    macro_rules! temp_file {
        ($name:expr, $contents:expr) => {{
            let path = std::env::temp_dir().join(format!(
                "enforcer-rules-loader-test-{}-{}",
                std::process::id(),
                $name.as_str()
            ));
            let mut file = std::fs::File::create(&path)?;
            file.write_all($contents.as_str().as_bytes())?;
            path
        }};
    }

    #[test]
    fn loads_registry_from_disk_files() -> Result<(), Box<dyn std::error::Error>> {
        let path = temp_file!(
            &RuleCatalogSource::try_from("valid.json".to_owned())?,
            &RuleCatalogJson::try_from(VALID_CATALOG.to_owned())?
        );
        let registry = load_registry_from_files(&[path.as_path()])?;
        assert_eq!(registry.iter().count(), 1);
        std::fs::remove_file(&path)?;
        Ok(())
    }

    #[test]
    fn missing_file_fails_closed() {
        let missing = std::env::temp_dir().join("enforcer-rules-loader-test-missing-nope.json");
        let outcome = load_registry_from_files(&[missing.as_path()]);
        assert!(matches!(outcome, Err(crate::RuleLoadError::Io { .. })));
    }

    #[test]
    fn duplicate_across_two_files_fails_closed() -> Result<(), Box<dyn std::error::Error>> {
        let a = temp_file!(
            &RuleCatalogSource::try_from("dup-a.json".to_owned())?,
            &RuleCatalogJson::try_from(VALID_CATALOG.to_owned())?
        );
        // Reuse the same ruleId in a second file to force a cross-file clash.
        let b = temp_file!(
            &RuleCatalogSource::try_from("dup-b.json".to_owned())?,
            &RuleCatalogJson::try_from(VALID_CATALOG.to_owned())?
        );
        let outcome = load_registry_from_files(&[a.as_path(), b.as_path()]);
        assert!(matches!(
            outcome,
            Err(crate::RuleLoadError::DuplicateRuleId { .. })
        ));
        std::fs::remove_file(&a)?;
        std::fs::remove_file(&b)?;
        Ok(())
    }
}
