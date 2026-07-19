//! h11 acceptance proof: the `rules/cyberskills.json` catalog (12 records:
//! the frontmatter linter plus the 11 fundamental-logic cyberskills rules)
//! loads into a registry, every record's 5-way linkage resolves (ruleId
//! <-> validator <-> fixtures <-> doc-anchor <-> tier), and every fixture
//! file referenced actually exists on disk (a stronger check than the
//! generic loader's own shape validation, which only asserts
//! non-emptiness of the fixture path STRINGS).

use std::path::{Path, PathBuf};

use enforcer_domain::rules_types::{RuleCatalogJson, RuleCatalogSource};
use enforcer_rules::loader::{load_registry_from_records, parse_catalog};

const CYBERSKILLS_JSON: &str = include_str!("../rules/cyberskills.json");

fn load_catalog() -> Result<Vec<enforcer_rules::registry::RuleRecord>, Box<dyn std::error::Error>> {
    let raw = RuleCatalogJson::try_from(CYBERSKILLS_JSON.to_owned())?;
    let source = RuleCatalogSource::try_from("rules/cyberskills.json".to_owned())?;
    Ok(parse_catalog(&raw, &source)?)
}

fn repo_root() -> Result<PathBuf, Box<dyn std::error::Error>> {
    // CARGO_MANIFEST_DIR is `<repo>/crates/enforcer-rules`.
    Ok(PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()?)
}

#[test]
fn cyberskills_catalog_loads_and_every_record_resolves() -> Result<(), Box<dyn std::error::Error>> {
    let records = load_catalog()?;
    assert_eq!(
        records.len(),
        41,
        "expected 41 native cyberskills rule records"
    );

    let registry = load_registry_from_records(records)?;
    assert_eq!(registry.iter().count(), 41);

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
        "CYBER-K8S-POD.1",
        "CYBER-DOCKER.1",
        "CYBER-SECRET.1",
        "CYBER-AWS.1",
        "CYBER-K8S-RBAC.1",
        "CYBER-GCP.1",
        "CYBER-AUTH-JWT.1",
        "CYBER-CORS.1",
        "CYBER-TLS.1",
        "CYBER-SSRF.1",
        "CYBER-CMD-INJECT.1",
        "CYBER-PATH-TRAVERSAL.1",
        "CYBER-DESERIALIZE.1",
        "CYBER-WEAK-CRYPTO.1",
        "CYBER-TLS-VERIFY.1",
        "CYBER-SQLI-SOURCE.1",
        "CYBER-SSTI.1",
        "CYBER-NOSQL-INJECT.1",
        "CYBER-PROTO-POLLUTION.1",
        "CYBER-GHA.1",
        "CYBER-MASS-ASSIGN.1",
        "CYBER-TYPE-JUGGLE.1",
        "CYBER-OAUTH.1",
        "CYBER-DOCKER-DAEMON.1",
        "CYBER-MCP-POISON.1",
        "CYBER-FILELESS-MALWARE.1",
        "CYBER-FILELESS-TELEMETRY.1",
        "CYBER-FILELESS-REPORT.1",
        "CYBER-WEBSOCKET.1",
    ];

    let root = repo_root()?;
    for id in expected_ids {
        let rule_id = id.parse()?;
        let record = registry
            .get(&rule_id)
            .ok_or_else(|| format!("expected {id} to load"))?;
        assert!(
            record
                .validator
                .crate_name
                .as_str()
                .starts_with("enforcer-"),
            "{id} must name an enforcer-owned validator crate"
        );
        assert!(
            record.validator.path.as_str().starts_with("rules::")
                || record.validator.path.as_str().starts_with("cyberskills::"),
            "{id} must point to a known validator namespace"
        );
        if id.starts_with("CYBER-FILELESS-") {
            assert!(
                record
                    .doc_anchor
                    .as_str()
                    .starts_with("rules/common/fileless-malware.md#"),
                "{id} must route to the product fileless-malware rule documentation"
            );
        } else {
            assert!(
                record
                    .doc_anchor
                    .as_str()
                    .starts_with("docs/plans/enforcer-selfhost-plan/workpacks/h11-"),
                "{id} must retain its h11 workpack anchor"
            );
        }

        let fail_path = root.join(record.fixtures.fail.as_str());
        let pass_path = root.join(record.fixtures.pass.as_str());
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
    let records = load_catalog()?;
    let mut clone_of_first = records.clone();
    clone_of_first.push(records[0].clone());
    assert!(
        matches!(load_registry_from_records(clone_of_first), Err(enforcer_rules::RuleLoadError::DuplicateRuleId { rule_id }) if rule_id.as_str() == "CYBER-FRONTMATTER.1")
    );
    Ok(())
}
