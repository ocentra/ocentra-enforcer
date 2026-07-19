//! `cargo test -p enforcer-security` — h08 POLICY-SPEC-INGESTION proof.
//!
//! Three named tests per the workpack's Acceptance And Proof section:
//! `policy_ingest_mapping` (T1 ingest-mapping equality), `policy_ingest_unbacked`
//! (T2 unbacked-rule flag, never silent accept), and `profile_shape`
//! (the committed neutral profile deserializes into the typed record).
//! Plus a fourth malformed-input boundary test.

use std::collections::BTreeSet;
use std::path::PathBuf;

use enforcer_domain::config_types::ConfigProfileName;
use enforcer_domain::ids::RuleId;
use enforcer_domain::paths::RelPath;
use enforcer_domain::severity::Tier;
use enforcer_security::policy_ingest::backing::BackedRuleCatalog;
use enforcer_security::policy_ingest::error::PolicyIngestError;
use enforcer_security::policy_ingest::map::map_to_profile;
use enforcer_security::policy_ingest::parse::parse_spec;
use enforcer_security::policy_ingest::spec::{MechanizedProfile, MechanizedProfileDto};
use proptest::{collection, prop_assert, prop_assert_eq, proptest};

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read_fixture(rel: &str) -> Result<String, Box<dyn std::error::Error>> {
    Ok(std::fs::read_to_string(manifest_dir().join(rel))?)
}

fn fixture_rel_path(rel: &str) -> Result<RelPath, Box<dyn std::error::Error>> {
    Ok(rel.parse()?)
}

#[test]
fn policy_ingest_mapping() -> Result<(), Box<dyn std::error::Error>> {
    // T1: ingesting the reference spec yields a profile whose
    // required-test-categories + invariants equal the spec's §3 + §2.3
    // set, and every asserted rule that IS backed becomes an ENABLED row.
    let path = "tests/fixtures/policy_ingest/good/ingest_reference_spec.mdc";
    let source = read_fixture(path)?;
    let file = fixture_rel_path(path)?;

    let parsed = parse_spec(path, &source)?;
    let catalog = BackedRuleCatalog::track_h_snapshot()?;
    let profile_name = ConfigProfileName::try_new("money-critical-security".to_owned())?;
    let (profile, findings) = map_to_profile(profile_name, &parsed, &catalog, &file);

    assert_eq!(
        profile.required_test_categories, parsed.required_test_categories,
        "profile required-test-categories must equal the spec's §3 set (mapping equality)"
    );
    assert_eq!(
        profile.invariants, parsed.invariants,
        "profile invariants must equal the spec's §2.3 set (mapping equality)"
    );

    // Every rule this fixture asserts (MCM-SIGNING.1, MCM-TIME.1,
    // MCM-BOUNDARY.1, MCM-KILLSWITCH.1, MCM-ECONOMIC.1, MCM-ROLLBACK.1) is
    // backed by the Track H snapshot -> all rows enabled, zero findings.
    assert_eq!(profile.rules.len(), 6);
    assert!(
        profile.rules.iter().all(|row| row.backed),
        "every asserted rule in the reference fixture is backed: {:?}",
        profile.rules
    );
    assert!(
        findings.is_empty(),
        "a fully-backed spec must not emit any unbacked-rule findings: {findings:#?}"
    );

    let rule_ids: BTreeSet<&str> = profile
        .rules
        .iter()
        .map(|row| row.rule_id.as_str())
        .collect();
    let expected_rule_ids = BTreeSet::from([
        "MCM-SIGNING.1",
        "MCM-TIME.1",
        "MCM-BOUNDARY.1",
        "MCM-KILLSWITCH.1",
        "MCM-ECONOMIC.1",
        "MCM-ROLLBACK.1",
    ]);
    assert_eq!(
        rule_ids, expected_rule_ids,
        "reference fixture must map to exactly its six expected rule rows"
    );

    Ok(())
}

#[test]
fn backed_rule_catalog_brands_raw_ids_at_boundary() -> Result<(), Box<dyn std::error::Error>> {
    let rule_id = RuleId::try_from(String::from("MCM-SIGNING.1"))?;
    assert!(matches!(
        BackedRuleCatalog::from_raw_ids(["MCM-SIGNING.1"]),
        Ok(catalog) if catalog.is_backed(&rule_id)
    ));
    assert!(matches!(
        BackedRuleCatalog::from_raw_ids(["not a rule id"]),
        Err(error) if error.path == "ruleId"
    ));
    Ok(())
}

#[test]
fn policy_ingest_unbacked() -> Result<(), Box<dyn std::error::Error>> {
    // T2: an ingested spec asserting a rule with no mechanized backing
    // emits a Finding flagging it for mechanization (feeds d01/d08) --
    // never a silent accept-as-enforced.
    let path = "tests/fixtures/policy_ingest/bad/ingest_unbacked_rule.mdc";
    let source = read_fixture(path)?;
    let file = fixture_rel_path(path)?;

    let parsed = parse_spec(path, &source)?;
    let catalog = BackedRuleCatalog::track_h_snapshot()?;
    let profile_name = ConfigProfileName::try_new("money-critical-security".to_owned())?;
    let (profile, findings) = map_to_profile(profile_name, &parsed, &catalog, &file);

    // Two asserted rules: MCM-SIGNING.1 (backed) and
    // MCM-NOT-YET-MECHANIZED.1 (unbacked).
    assert_eq!(profile.rules.len(), 2);
    let unbacked = profile.unbacked_rule_ids();
    assert_eq!(unbacked.len(), 1);
    assert_eq!(
        unbacked,
        BTreeSet::from(["MCM-NOT-YET-MECHANIZED.1"]),
        "only the deliberately unbacked fixture rule may be flagged"
    );

    let backed_row = profile
        .rules
        .iter()
        .find(|row| row.rule_id.as_str() == "MCM-SIGNING.1")
        .ok_or("MCM-SIGNING.1 row present")?;
    assert!(
        backed_row.backed,
        "MCM-SIGNING.1 has real backing and must be enabled"
    );

    // Exactly one Finding, naming the unbacked rule, never silently
    // dropped or treated as enabled.
    assert_eq!(findings.len(), 1);
    let finding = &findings[0];
    assert_eq!(finding.rule_id.as_str(), "MCM-NOT-YET-MECHANIZED.1");
    assert_eq!(
        finding.severity,
        enforcer_domain::severity::Severity::Warning
    );
    assert_eq!(
        finding.title.as_str(),
        "policy spec asserts a rule with no mechanized backing (flagged, not enabled)"
    );
    assert_eq!(finding.snippet, None);

    Ok(())
}

#[test]
fn policy_ingest_malformed_input_is_typed_boundary_error() -> Result<(), Box<dyn std::error::Error>>
{
    // fail: `bad/malformed.mdc` has zero recognizable `## Section`
    // headers -> a typed PolicyIngestError, not a silent empty default.
    let path = "tests/fixtures/policy_ingest/bad/malformed.mdc";
    let source = read_fixture(path)?;
    let result = parse_spec(path, &source);
    assert!(matches!(result, Err(PolicyIngestError::NoSections { .. })));
    Ok(())
}

#[test]
fn policy_ingest_conflicting_severity_is_rejected() {
    let source = "## Rules\n- MCM-SIGNING.1 (T1)\n- MCM-SIGNING.1 (T2)\n";
    let result = parse_spec("inline-conflict", source);
    assert!(matches!(
        result,
        Err(PolicyIngestError::ConflictingSeverity { .. })
    ));
}

#[test]
fn policy_ingest_malformed_rule_entry_is_rejected() {
    let source = "## Rules\n- not-a-well-formed-line-without-tier\n";
    let result = parse_spec("inline-malformed", source);
    assert!(matches!(
        result,
        Err(PolicyIngestError::MalformedEntry { .. })
    ));
}

proptest! {
    #[test]
    fn parse_spec_preserves_first_category_occurrence(
        categories in collection::vec("[A-Za-z0-9_-]{1,16}", 0..32)
    ) {
        let mut source = String::from("## Required test categories\n");
        for category in &categories {
            source.push_str("- ");
            source.push_str(category);
            source.push('\n');
        }

        let mut expected = Vec::new();
        for category in categories {
            if !expected.contains(&category) {
                expected.push(category);
            }
        }

        match parse_spec("generated-categories", &source) {
            Ok(parsed) => prop_assert_eq!(
                parsed
                    .required_test_categories
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>(),
                expected
            ),
            Err(error) => prop_assert!(false, "generated category document failed: {error}"),
        }
    }
}

#[test]
fn profile_shape() -> Result<(), Box<dyn std::error::Error>> {
    // The committed neutral profile deserializes into the typed
    // MechanizedProfileDto record: rule ids + severities (tiers) +
    // categories present, and the name carries no product/company/game
    // branding.
    let repo_root = manifest_dir()
        .parent()
        .and_then(|p| p.parent())
        .ok_or("enforcer-security should be nested two levels under the workspace root")?
        .to_path_buf();
    let profile_path = repo_root.join("profiles/money-critical-security.json");
    let raw = std::fs::read_to_string(&profile_path)?;
    let profile_dto: MechanizedProfileDto = serde_json::from_str(&raw)?;
    let profile = MechanizedProfile::try_from(profile_dto)?;
    let encoded = serde_json::to_string(&MechanizedProfileDto::from(&profile))?;
    let round_trip_dto: MechanizedProfileDto = serde_json::from_str(&encoded)?;
    let round_trip = MechanizedProfile::try_from(round_trip_dto)?;
    assert_eq!(round_trip, profile);

    assert_eq!(profile.profile_name.as_str(), "money-critical-security");
    assert_eq!(profile.required_test_categories.len(), 20);
    assert_eq!(profile.invariants.len(), 10);
    assert_eq!(profile.rules.len(), 13);
    assert!(profile.rules.iter().any(|row| row.tier == Tier::T1));
    assert!(profile.rules.iter().any(|row| row.tier == Tier::T2));

    let branding_terms = ["solana-labs", "anchor-lang", "ocentra", "simpro"];
    let lower = raw.to_lowercase();
    for term in branding_terms {
        assert_eq!(
            lower.matches(term).count(),
            0,
            "profile must carry no product/company branding: found `{term}`"
        );
    }

    Ok(())
}
