//! Typed loader: parse-at-boundary JSON loading of rule catalog files into
//! a validated [`crate::registry::RuleRegistry`]. Mirrors
//! `enforcer-config`'s load convention — read bytes, parse, validate; a
//! partially- or mal-formed catalog never becomes a live registry.

use std::path::Path;

use crate::registry::{RuleRecord, RuleRegistry};
use crate::{RuleLoadError, RuleResult};

/// Parse one JSON document (an array of [`RuleRecord`]) from an in-memory
/// string. The `source` label is used only for error messages.
pub fn parse_catalog(raw: &str, source: &str) -> RuleResult<Vec<RuleRecord>> {
    serde_json::from_str::<Vec<RuleRecord>>(raw).map_err(|e| RuleLoadError::Parse {
        path: source.to_owned(),
        reason: e.to_string(),
    })
}

/// Load and validate ONE catalog file from disk into standalone records
/// (not yet merged into a registry — see [`load_registry_from_files`] to
/// merge several catalog files into one registry).
pub fn load_catalog_file(path: &Path) -> RuleResult<Vec<RuleRecord>> {
    let display = path.display().to_string();
    let raw = std::fs::read_to_string(path).map_err(|e| RuleLoadError::Io {
        path: display.clone(),
        reason: e.to_string(),
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

    #[test]
    fn parses_a_well_formed_catalog() -> Result<(), Box<dyn std::error::Error>> {
        let records = parse_catalog(VALID_CATALOG, "<inline>")?;
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].rule_id.as_str(), "RR-1.1");
        Ok(())
    }

    #[test]
    fn rejects_malformed_json() {
        let outcome = parse_catalog("{not json", "<inline>");
        assert!(outcome.is_err());
    }

    #[test]
    fn rejects_duplicate_ids_across_the_same_file() -> Result<(), Box<dyn std::error::Error>> {
        let records = parse_catalog(DUPLICATE_CATALOG, "<inline>")?;
        let outcome = crate::registry::RuleRegistry::from_records(records);
        assert!(outcome.is_err());
        Ok(())
    }

    fn temp_file(name: &str, contents: &str) -> Result<std::path::PathBuf, std::io::Error> {
        let path = std::env::temp_dir().join(format!(
            "enforcer-rules-loader-test-{}-{}",
            std::process::id(),
            name
        ));
        let mut f = std::fs::File::create(&path)?;
        f.write_all(contents.as_bytes())?;
        Ok(path)
    }

    #[test]
    fn loads_registry_from_disk_files() -> Result<(), Box<dyn std::error::Error>> {
        let path = temp_file("valid.json", VALID_CATALOG)?;
        let registry = load_registry_from_files(&[path.as_path()])?;
        assert_eq!(registry.len(), 1);
        std::fs::remove_file(&path)?;
        Ok(())
    }

    #[test]
    fn missing_file_fails_closed() {
        let missing = std::env::temp_dir().join("enforcer-rules-loader-test-missing-nope.json");
        let outcome = load_registry_from_files(&[missing.as_path()]);
        assert!(outcome.is_err());
    }

    #[test]
    fn duplicate_across_two_files_fails_closed() -> Result<(), Box<dyn std::error::Error>> {
        let a = temp_file("dup-a.json", VALID_CATALOG)?;
        // Reuse the same ruleId in a second file to force a cross-file clash.
        let b = temp_file("dup-b.json", VALID_CATALOG)?;
        let outcome = load_registry_from_files(&[a.as_path(), b.as_path()]);
        assert!(outcome.is_err());
        std::fs::remove_file(&a)?;
        std::fs::remove_file(&b)?;
        Ok(())
    }
}
