//! h11 acceptance proof: the `rules/cyberskills.json` catalog (12 records:
//! the frontmatter linter plus the 11 fundamental-logic cyberskills rules)
//! loads into a registry, every record's 5-way linkage resolves (ruleId
//! <-> validator <-> fixtures <-> doc-anchor <-> tier), and every fixture
//! file referenced actually exists on disk (a stronger check than the
//! generic loader's own shape validation, which only asserts
//! non-emptiness of the fixture path STRINGS).

use std::path::{Path, PathBuf};

use enforcer_rules::loader::{load_registry_from_records, parse_catalog};

const CYBERSKILLS_JSON: &str = include_str!("../rules/cyberskills.json");

fn repo_root() -> Result<PathBuf, Box<dyn std::error::Error>> {
    // CARGO_MANIFEST_DIR is `<repo>/crates/enforcer-rules`.
    Ok(PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()?)
}

#[test]
fn cyberskills_catalog_loads_and_every_record_resolves() -> Result<(), Box<dyn std::error::Error>> {
    let records = parse_catalog(CYBERSKILLS_JSON, "rules/cyberskills.json")?;
    assert_eq!(
        records.len(),
        12,
        "expected 12 h11 cyberskills rule records"
    );

    let registry = load_registry_from_records(records)?;
    assert_eq!(registry.len(), 12);

    let expected_ids = [
        "CYBER-FRONTMATTER.1",
        "CYBER-IAC-S3-SSE.1",
        "CYBER-IAC-IAM-WILDCARD.1",
        "CYBER-IAC-SG-SSH.1",
        "CYBER-AZURE-BLOB-PUBLIC.1",
        "CYBER-AZURE-HTTPS.1",
        "CYBER-AZURE-TLS12.1",
        "CYBER-HEADERS-HSTS.1",
        "CYBER-HEADERS-CSP.1",
        "CYBER-COOKIE-SECURE.1",
        "CYBER-DEPCONFUSION.1",
        "CYBER-WAF-SQLI.1",
    ];

    let root = repo_root()?;
    for id in expected_ids {
        let rule_id = id.parse()?;
        let record = registry
            .get(&rule_id)
            .ok_or_else(|| format!("expected {id} to load"))?;
        assert!(!record.validator.crate_name.is_empty());
        assert!(!record.validator.path.is_empty());
        assert!(!record.doc_anchor.is_empty());

        let fail_path = root.join(&record.fixtures.fail);
        let pass_path = root.join(&record.fixtures.pass);
        assert!(
            Path::new(&fail_path).is_file(),
            "fail fixture missing on disk for {id}: {}",
            fail_path.display()
        );
        assert!(
            Path::new(&pass_path).is_file(),
            "pass fixture missing on disk for {id}: {}",
            pass_path.display()
        );
    }

    // T2 scored rule is tagged correctly.
    let waf_sqli = registry
        .get(&"CYBER-WAF-SQLI.1".parse()?)
        .ok_or("expected CYBER-WAF-SQLI.1")?;
    assert_eq!(waf_sqli.tier, enforcer_domain::severity::Tier::T2);

    Ok(())
}

#[test]
fn cyberskills_catalog_has_no_duplicate_or_malformed_records(
) -> Result<(), Box<dyn std::error::Error>> {
    let records = parse_catalog(CYBERSKILLS_JSON, "rules/cyberskills.json")?;
    let mut clone_of_first = records.clone();
    clone_of_first.push(records[0].clone());
    assert!(load_registry_from_records(clone_of_first).is_err());
    Ok(())
}
