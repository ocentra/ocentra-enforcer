//! d13 acceptance proof: `rule-version-manifest.json` pins a `Sha256`
//! content hash per rule record, and [`check_registry_drift`] fails closed
//! exactly on the two seeded shapes the workpack's acceptance block
//! requires (content change without a version bump; a hollow version
//! bump with no content change), stays clean on an unchanged registry and
//! a legitimate matched version+hash bump, and the real shipped
//! `rule-version-manifest.json` matches the real shipped baseline
//! registry today (anti-vacuous: this is not fixture-only).

use enforcer_rules::loader::parse_catalog;
use enforcer_rules::registry::RuleRegistry;
use enforcer_rules::version_drift::{
    build_manifest, check_registry_drift, drift_findings, ManifestDrift, RegistryManifest,
};

const BASELINE_CATALOG: &str = include_str!("fixtures/version_drift/baseline_catalog.json");
const BASELINE_MANIFEST: &str = include_str!("fixtures/version_drift/baseline_manifest.json");
const UNCHANGED_CATALOG: &str = include_str!("fixtures/version_drift/unchanged_catalog.json");
const CONTENT_CHANGE_NO_BUMP_CATALOG: &str =
    include_str!("fixtures/version_drift/content_change_no_bump_catalog.json");
const HOLLOW_BUMP_CATALOG: &str = include_str!("fixtures/version_drift/hollow_bump_catalog.json");
const LEGITIMATE_BUMP_CATALOG: &str =
    include_str!("fixtures/version_drift/legitimate_bump_catalog.json");

// The real shipped baseline registry (mirrors `tests/registry_load.rs`'s
// `all_baseline_records`, arc-04's own proof of the same four catalogs) —
// used to prove the real, committed `rule-version-manifest.json` is not
// stale against real production rule records.
const DENY_WALL_JSON: &str = include_str!("../rules/deny-wall.json");
const NO_REEXPORTS_JSON: &str = include_str!("../rules/no-reexports.json");
const OCENTRA_PARENT_POSTURE_JSON: &str = include_str!("../rules/ocentra-parent-posture.json");
const DEFERRED_WORK_GATE_JSON: &str = include_str!("../rules/deferred-work-gate.json");
const REAL_MANIFEST: &str = include_str!("../rule-version-manifest.json");

fn registry_from(catalog_json: &str) -> Result<RuleRegistry, Box<dyn std::error::Error>> {
    let records = parse_catalog(catalog_json, "<fixture>")?;
    Ok(RuleRegistry::from_records(records)?)
}

fn real_baseline_registry() -> Result<RuleRegistry, Box<dyn std::error::Error>> {
    let mut records = parse_catalog(DENY_WALL_JSON, "rules/deny-wall.json")?;
    records.extend(parse_catalog(NO_REEXPORTS_JSON, "rules/no-reexports.json")?);
    records.extend(parse_catalog(
        OCENTRA_PARENT_POSTURE_JSON,
        "rules/ocentra-parent-posture.json",
    )?);
    records.extend(parse_catalog(
        DEFERRED_WORK_GATE_JSON,
        "rules/deferred-work-gate.json",
    )?);
    Ok(RuleRegistry::from_records(records)?)
}

#[test]
fn unchanged_registry_passes_against_the_pinned_manifest() -> Result<(), Box<dyn std::error::Error>>
{
    let manifest = RegistryManifest::parse(BASELINE_MANIFEST)?;
    // Cosmetic-only changes (title/tag rename) relative to the baseline
    // catalog must NOT trip drift.
    let registry = registry_from(UNCHANGED_CATALOG)?;
    let drift = check_registry_drift(&manifest, &registry);
    assert_eq!(drift.len(), 2);
    assert!(
        drift.iter().all(|entry| !entry.outcome.is_drift()),
        "cosmetic-only changes must not be reported as drift: {drift:?}"
    );
    assert!(drift_findings(&drift).is_empty());
    Ok(())
}

#[test]
fn content_change_without_version_bump_fails_closed() -> Result<(), Box<dyn std::error::Error>> {
    let manifest = RegistryManifest::parse(BASELINE_MANIFEST)?;
    let registry = registry_from(CONTENT_CHANGE_NO_BUMP_CATALOG)?;
    let drift = check_registry_drift(&manifest, &registry);

    let vdrift_2 = drift
        .iter()
        .find(|e| e.rule_id.as_str() == "T1-VDRIFT.2")
        .ok_or("expected a T1-VDRIFT.2 drift entry")?;
    assert_eq!(vdrift_2.outcome, ManifestDrift::HashChangedVersionNotBumped);
    assert!(vdrift_2.outcome.is_drift());

    // T1-VDRIFT.1 is untouched in this fixture and must stay clean.
    let vdrift_1 = drift
        .iter()
        .find(|e| e.rule_id.as_str() == "T1-VDRIFT.1")
        .ok_or("expected a T1-VDRIFT.1 drift entry")?;
    assert!(!vdrift_1.outcome.is_drift());

    let findings = drift_findings(&drift);
    assert_eq!(findings.len(), 1, "exactly one rule drifted");
    assert_eq!(findings[0].rule_id.as_str(), "T1-VDRIFT.2");
    assert!(findings[0].detail.contains("Fix:"));
    assert!(findings[0].detail.contains("T1-VDRIFT.2"));
    Ok(())
}

#[test]
fn version_bump_without_content_change_fails_closed() -> Result<(), Box<dyn std::error::Error>> {
    let manifest = RegistryManifest::parse(BASELINE_MANIFEST)?;
    let registry = registry_from(HOLLOW_BUMP_CATALOG)?;
    let drift = check_registry_drift(&manifest, &registry);

    let vdrift_1 = drift
        .iter()
        .find(|e| e.rule_id.as_str() == "T1-VDRIFT.1")
        .ok_or("expected a T1-VDRIFT.1 drift entry")?;
    assert_eq!(vdrift_1.outcome, ManifestDrift::VersionBumpedHashUnchanged);
    assert!(vdrift_1.outcome.is_drift());

    let vdrift_2 = drift
        .iter()
        .find(|e| e.rule_id.as_str() == "T1-VDRIFT.2")
        .ok_or("expected a T1-VDRIFT.2 drift entry")?;
    assert!(!vdrift_2.outcome.is_drift());

    let findings = drift_findings(&drift);
    assert_eq!(findings.len(), 1, "exactly one rule drifted");
    assert_eq!(findings[0].rule_id.as_str(), "T1-VDRIFT.1");
    assert!(findings[0].detail.contains("Fix:"));
    Ok(())
}

#[test]
fn matched_version_and_hash_bump_together_passes() -> Result<(), Box<dyn std::error::Error>> {
    let manifest = RegistryManifest::parse(BASELINE_MANIFEST)?;
    let registry = registry_from(LEGITIMATE_BUMP_CATALOG)?;
    let drift = check_registry_drift(&manifest, &registry);

    let vdrift_2 = drift
        .iter()
        .find(|e| e.rule_id.as_str() == "T1-VDRIFT.2")
        .ok_or("expected a T1-VDRIFT.2 drift entry")?;
    assert_eq!(vdrift_2.outcome, ManifestDrift::HashChangedVersionBumped);
    assert!(!vdrift_2.outcome.is_drift());

    assert!(
        drift.iter().all(|entry| !entry.outcome.is_drift()),
        "a matched version+hash bump must pass cleanly: {drift:?}"
    );
    assert!(drift_findings(&drift).is_empty());
    Ok(())
}

#[test]
fn version_alone_or_hash_alone_never_passes_without_the_other(
) -> Result<(), Box<dyn std::error::Error>> {
    // Restates the two failing acceptance scenarios as one explicit
    // "neither alone passes" assertion: hash-changed-only (no bump) and
    // version-changed-only (hollow bump) both fail; only both together
    // (proven above) passes.
    let manifest = RegistryManifest::parse(BASELINE_MANIFEST)?;

    let hash_only = registry_from(CONTENT_CHANGE_NO_BUMP_CATALOG)?;
    let hash_only_drift = check_registry_drift(&manifest, &hash_only);
    assert!(hash_only_drift.iter().any(|e| e.outcome.is_drift()));

    let version_only = registry_from(HOLLOW_BUMP_CATALOG)?;
    let version_only_drift = check_registry_drift(&manifest, &version_only);
    assert!(version_only_drift.iter().any(|e| e.outcome.is_drift()));

    Ok(())
}

#[test]
fn removing_a_pinned_rule_without_updating_the_manifest_fails_closed(
) -> Result<(), Box<dyn std::error::Error>> {
    let manifest = RegistryManifest::parse(BASELINE_MANIFEST)?;
    // Registry with only one of the two pinned rules — the other was
    // silently dropped from the catalog.
    let records = parse_catalog(BASELINE_CATALOG, "baseline_catalog.json")?;
    let one_record = vec![records
        .into_iter()
        .find(|r| r.rule_id.as_str() == "T1-VDRIFT.1")
        .ok_or("expected T1-VDRIFT.1 in the baseline fixture")?];
    let registry = RuleRegistry::from_records(one_record)?;

    let drift = check_registry_drift(&manifest, &registry);
    let missing = drift
        .iter()
        .find(|e| e.rule_id.as_str() == "T1-VDRIFT.2")
        .ok_or("expected a T1-VDRIFT.2 drift entry")?;
    assert_eq!(missing.outcome, ManifestDrift::MissingFromRegistry);
    assert!(missing.outcome.is_drift());
    Ok(())
}

#[test]
fn manifest_parse_rejects_malformed_manifest_json() {
    assert!(RegistryManifest::parse("{not json").is_err());
    assert!(RegistryManifest::parse(r#"{"schemaVersion":0,"entries":{}}"#).is_err());
}

/// Anti-vacuous: the real, committed `rule-version-manifest.json` matches
/// what [`build_manifest`] computes from the REAL shipped baseline
/// catalogs today. If this fails, the manifest is stale — regenerate it
/// from `build_manifest(&real_baseline_registry, 1)`, do not hand-edit
/// hashes.
#[test]
fn the_real_shipped_manifest_matches_the_real_shipped_baseline_registry(
) -> Result<(), Box<dyn std::error::Error>> {
    let registry = real_baseline_registry()?;
    let recomputed = build_manifest(&registry, 1);
    let pinned = RegistryManifest::parse(REAL_MANIFEST)?;
    assert_eq!(
        pinned, recomputed,
        "rule-version-manifest.json is stale relative to the real shipped baseline catalogs"
    );

    let drift = check_registry_drift(&pinned, &registry);
    assert!(
        drift.iter().all(|entry| !entry.outcome.is_drift()),
        "real shipped registry must currently pass its own pinned manifest: {drift:?}"
    );
    Ok(())
}

/// The real manifest must cover exactly the real baseline registry's rule
/// ids — no orphaned pin, no unpinned record.
#[test]
fn the_real_shipped_manifest_has_no_orphans_or_gaps() -> Result<(), Box<dyn std::error::Error>> {
    let registry = real_baseline_registry()?;
    let pinned = RegistryManifest::parse(REAL_MANIFEST)?;
    let drift = check_registry_drift(&pinned, &registry);
    assert!(
        !drift
            .iter()
            .any(|e| e.outcome == ManifestDrift::MissingFromRegistry),
        "manifest pins a rule id no longer in the real registry"
    );
    assert!(
        !drift
            .iter()
            .any(|e| e.outcome == ManifestDrift::MissingFromManifest),
        "real registry has a rule id with no pinned manifest entry"
    );
    Ok(())
}
