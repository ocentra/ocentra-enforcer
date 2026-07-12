//! arc-09 acceptance proof: every validator this crate registers fires on
//! its fail fixture and stays silent on its pass fixture
//! (`enforcer_validator::harness::run_fixture_parity`), AND the
//! count-parity assertion the workpack requires — every `language ==
//! "common"` `RuleId` in the legacy `rules/rules.json` catalog, MINUS the
//! SEC-2 family delegated to `enforcer-lang-security` (arc-10), has a
//! registered validator here. Missing or extra `RuleId`s fail this test.

use std::collections::BTreeSet;
use std::path::PathBuf;

use enforcer_lang_common::port_platform::DeclaredScope;
use enforcer_lang_common::registry;
use enforcer_validator::harness::run_fixture_parity;

/// The legacy rule catalog, read once per test binary. Path is relative to
/// this crate's manifest dir (`crates/enforcer-lang-common`) up to the
/// workspace root's `rules/rules.json` — the same catalog `enforcer-rules`
/// will eventually load structured records from; this test reads it
/// directly (raw JSON) because no `enforcer-rules` `RuleRecord`s exist yet
/// for the `common` family (only `deny-wall`/`no-reexports`/
/// `ocentra-parent-posture` have landed) and the workpack's count-parity
/// requirement is specifically "reads `rules.json`".
const RULES_JSON: &str = include_str!("../../../rules/rules.json");

/// Minimal decode: only the fields this test needs (`id`, `language`) from
/// the legacy catalog's `{ rules: [...] }` envelope. Deliberately not the
/// full `enforcer-rules::registry::RuleRecord` shape (that crate's catalogs
/// are a distinct, not-yet-populated concern) — this is reading the LEGACY
/// `.mjs`-era catalog format directly, one field at a time, on purpose.
#[derive(serde::Deserialize)]
struct LegacyRuleCatalog {
    rules: Vec<LegacyRuleEntry>,
}

#[derive(serde::Deserialize)]
struct LegacyRuleEntry {
    id: String,
    language: String,
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn family_of(rule_id: &str) -> String {
    // e.g. "ARCH-1.11" -> "arch-1"; "PORT-1.1" -> "port-1".
    let prefix = rule_id.split('.').next().unwrap_or(rule_id);
    prefix.to_ascii_lowercase()
}

fn fixture_paths(rule_id: &str) -> (String, String) {
    let family = family_of(rule_id);
    let id_lower = rule_id.to_ascii_lowercase();
    (
        format!("fixtures/{family}/{id_lower}/fail.txt"),
        format!("fixtures/{family}/{id_lower}/pass.txt"),
    )
}

/// Every rule id this crate owns, per `rules.json`: `language == "common"`
/// minus the SEC-2 family (delegated to arc-10 per the workpack's SEC-2
/// decision).
fn expected_common_minus_sec2_ids() -> Result<BTreeSet<String>, Box<dyn std::error::Error>> {
    let catalog: LegacyRuleCatalog = serde_json::from_str(RULES_JSON)?;
    Ok(catalog
        .rules
        .into_iter()
        .filter(|e| e.language == "common" && !e.id.starts_with("SEC-2"))
        .map(|e| e.id)
        .collect())
}

#[test]
fn every_registered_validator_fires_on_fail_and_is_silent_on_pass(
) -> Result<(), Box<dyn std::error::Error>> {
    let validators = registry::all(DeclaredScope::Undeclared);
    let repo_root = manifest_dir();
    let mut failures = Vec::new();
    for validator in &validators {
        let rule_id = validator.rule_id().to_string();
        let (fail_path, pass_path) = fixture_paths(&rule_id);
        if let Err(err) = run_fixture_parity(validator.as_ref(), &repo_root, &fail_path, &pass_path)
        {
            failures.push(format!("{rule_id}: {err}"));
        }
    }
    assert!(
        failures.is_empty(),
        "{} validator(s) failed fixture parity:\n{}",
        failures.len(),
        failures.join("\n")
    );
    Ok(())
}

#[test]
fn count_parity_against_rules_json_language_common_minus_sec2(
) -> Result<(), Box<dyn std::error::Error>> {
    let expected = expected_common_minus_sec2_ids()?;
    let validators = registry::all(DeclaredScope::Undeclared);
    let registered: BTreeSet<String> = validators.iter().map(|v| v.rule_id().to_string()).collect();

    let missing: Vec<&String> = expected.difference(&registered).collect();
    let extra: Vec<&String> = registered.difference(&expected).collect();

    assert!(
        missing.is_empty() && extra.is_empty(),
        "count-parity mismatch — missing (in rules.json, not registered): {missing:?}; \
         extra (registered, not in rules.json's common-minus-SEC-2 set): {extra:?}"
    );
    assert_eq!(
        expected.len(),
        250,
        "expected 270 - 20 SEC-2 = 250 common rules"
    );
    assert_eq!(registered.len(), 250);
    Ok(())
}

#[test]
fn every_expected_rule_has_both_fixture_files_on_disk() -> Result<(), Box<dyn std::error::Error>> {
    let expected = expected_common_minus_sec2_ids()?;
    let repo_root = manifest_dir();
    let mut missing = Vec::new();
    for rule_id in &expected {
        let (fail_path, pass_path) = fixture_paths(rule_id);
        if !repo_root.join(&fail_path).is_file() {
            missing.push(fail_path);
        }
        if !repo_root.join(&pass_path).is_file() {
            missing.push(pass_path);
        }
    }
    assert!(missing.is_empty(), "missing fixture files: {missing:?}");
    Ok(())
}
