//! BOUNDARY-INVARIANT: this boundary module validates raw wire values and converts only through typed domain contracts.
//! `THREAT-MAP-UNIT-COVERAGE.1` + `THREAT-MAP-NO-UNMAPPED.1` +
//! `THREAT-MAP-THREAT-HAS-TEST.1` (all T1) â€” the threat/invariant/test
//! mapping-completeness family (h03, Â§0.5 + Â§8.5 of the ingested
//! money-critical/security-testing spec).
//!
//! Doctrine (Â§0.5): "unmapped logic is forbidden logic" â€” every
//! money-critical unit (as classified by h01, consumed here read-only,
//! never redefined) must map to at least one threat, one invariant, one
//! property test, one concurrency test, and one replay test; and every
//! threat declared anywhere in the map must itself be backed by at least
//! one test, or the threat model is incomplete. This module implements a
//! `Validator` over a typed `THREAT_MAP` record (deserialized at the
//! boundary into branded newtypes â€” [`ThreatId`], [`RuleId`] â€” never bare
//! `String`) rather than any heuristic AST scan; the money-critical
//! classification signal itself is h01's, this module only asserts the
//! mapping GRAPH is complete for whatever units + threats a THREAT_MAP
//! record declares.
//!
//! # Three validators, one THREAT_MAP shape
//!
//! - [`ThreatMapUnitCoverageValidator`] (`THREAT-MAP-UNIT-COVERAGE.1`):
//!   every unit listed under `units` must carry at least one threat, at
//!   least one invariant, at least one property test, at least one
//!   concurrency test, and at least one replay test â€” any missing edge on
//!   any listed unit is a `Finding`.
//! - [`ThreatMapNoUnmappedValidator`] (`THREAT-MAP-NO-UNMAPPED.1`): every
//!   entry in the h01-shaped `moneyCriticalUnits` manifest snapshot must have a
//!   corresponding entry under `units` â€” an h01-classified unit absent
//!   from the map entirely is "unmapped logic", flagged regardless of
//!   whether the units that ARE present are fully mapped.
//! - [`ThreatMapThreatHasTestValidator`] (`THREAT-MAP-THREAT-HAS-TEST.1`):
//!   every threat declared under the record's `threats` list must itself
//!   carry at least one test id in its own `tests` array â€” a threat with
//!   zero tests means the threat model itself is incomplete, independent
//!   of whether any unit happens to reference that threat.
//!
//! GENERIC across any value system (fiat, Stripe, an internal ledger, or
//! the optional crypto/Anchor instance) â€” the fixtures below use a
//! deliberately neutral `credit_balance`/`sign_payment` vocabulary, never a
//! crypto-only one.
//!
//! # THREAT_MAP wire shape
//!
//! ```jsonc
//! {
//!   "moneyCriticalUnits": ["credit_balance", "sign_payment"],
//!   "units": [
//!     {
//!       "unit": "credit_balance",
//!       "threats": ["T1565.001"],
//!       "invariants": ["balance-non-negative"],
//!       "tests": {
//!         "property": ["prop_credit_balance_conserves_total"],
//!         "concurrency": ["conc_credit_balance_no_lost_update"],
//!         "replay": ["replay_credit_balance_idempotent"]
//!       }
//!     }
//!   ],
//!   "threats": [
//!     { "threatId": "T1565.001", "tests": ["prop_credit_balance_conserves_total"] }
//!   ]
//! }
//! ```
//!
//! `moneyCriticalUnits` is the h01-shaped money-critical manifest SNAPSHOT this
//! fixture format carries inline (a real pipeline wiring would resolve this
//! list from h01's `#[money_critical(registered)]` scan instead; this
//! record format lets the mapping-completeness check be exercised
//! standalone against a fixture, exactly like the other h03 representative
//! triples in the workpack).

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::findings::Finding;
use enforcer_domain::ids::{RuleId, ThreatId};
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

/// One unit's mapped tests, split by required test kind.
#[derive(Debug, Clone, serde::Deserialize)]
struct UnitTests {
    // DEFAULT-JUSTIFICATION: an omitted property-test list means no property
    // evidence was declared and is reported by the validator.
    #[serde(default)]
    property: Vec<String>,
    // DEFAULT-JUSTIFICATION: an omitted concurrency-test list means no
    // concurrency evidence was declared and is reported by the validator.
    #[serde(default)]
    concurrency: Vec<String>,
    // DEFAULT-JUSTIFICATION: an omitted replay-test list means no replay
    // evidence was declared and is reported by the validator.
    #[serde(default)]
    replay: Vec<String>,
}

/// One `units` entry: a unit name paired with its threat/invariant/test
/// mapping edges. `threats` is parsed as [`ThreatId`] at the boundary
/// (parse-at-boundary doctrine) â€” a malformed threat id fails record
/// deserialization rather than being silently treated as unmapped.
#[derive(Debug, Clone, serde::Deserialize)]
struct UnitEntry {
    unit: String,
    // DEFAULT-JUSTIFICATION: omitted threats represent no declared mapping
    // edges and are reported by the validator.
    #[serde(default)]
    threats: Vec<ThreatId>,
    // DEFAULT-JUSTIFICATION: omitted invariants represent no declared mapping
    // edges and are reported by the validator.
    #[serde(default)]
    invariants: Vec<String>,
    // DEFAULT-JUSTIFICATION: an omitted tests object represents no test
    // evidence and is reported by the validator.
    #[serde(default)]
    tests: Option<UnitTests>,
}

/// One `threats` entry: a declared threat id paired with the test ids that
/// back it.
#[derive(Debug, Clone, serde::Deserialize)]
struct ThreatEntry {
    #[serde(rename = "threatId")]
    threat_id: ThreatId,
    // DEFAULT-JUSTIFICATION: an omitted test list means the threat has no
    // backing evidence and is reported by the validator.
    #[serde(default)]
    tests: Vec<String>,
}

/// The whole THREAT_MAP record: the h01-shaped money-critical manifest
/// snapshot (`moneyCriticalUnits`), the per-unit mapping graph (`units`), and
/// the declared-threat completeness ledger (`threats`).
#[derive(Debug, Clone, serde::Deserialize)]
struct ThreatMap {
    // DEFAULT-JUSTIFICATION: an absent manifest snapshot represents zero
    // declared money-critical units and is diagnosed by the validator.
    #[serde(rename = "moneyCriticalUnits", default)]
    money_critical_units: Vec<String>,
    // DEFAULT-JUSTIFICATION: an absent unit ledger represents zero mapped
    // units and is diagnosed by the validator.
    #[serde(default)]
    units: Vec<UnitEntry>,
    // DEFAULT-JUSTIFICATION: an absent threat ledger represents zero declared
    // threats and is diagnosed by the validator.
    #[serde(default)]
    threats: Vec<ThreatEntry>,
}

/// Parse `source` as a THREAT_MAP record. Unparseable/non-JSON source is
/// not this validator family's concern (mirrors h01's "unparseable source
/// stays silent" contract) â€” returns `None` rather than a `Finding`.
fn parse_threat_map(source: &str) -> Option<ThreatMap> {
    serde_json::from_str(source).ok()
}

/// The five required test-kind edges for `THREAT-MAP-UNIT-COVERAGE.1`,
/// each paired with the human-readable label used in finding details.
fn missing_edges(unit: &UnitEntry) -> Vec<&'static str> {
    let mut missing = Vec::new();
    if unit.threats.is_empty() {
        missing.push("threat");
    }
    if unit.invariants.is_empty() {
        missing.push("invariant");
    }
    let tests = unit.tests.as_ref();
    if tests.is_none_or(|t| t.property.is_empty()) {
        missing.push("property test");
    }
    if tests.is_none_or(|t| t.concurrency.is_empty()) {
        missing.push("concurrency test");
    }
    if tests.is_none_or(|t| t.replay.is_empty()) {
        missing.push("replay test");
    }
    missing
}

/// `THREAT-MAP-UNIT-COVERAGE.1` â€” T1 per-unit mapping completeness (Â§8.5).
///
/// Every unit listed under `units` must carry at least one threat, at
/// least one invariant, at least one property test, at least one
/// concurrency test, and at least one replay test. This validator only
/// inspects units already present in the map â€” a unit
/// entirely absent from the map is [`ThreatMapNoUnmappedValidator`]'s
/// concern, not this one's.
pub struct ThreatMapUnitCoverageValidator {
    rule_id: RuleId,
}

impl ThreatMapUnitCoverageValidator {
    /// Build the validator, parsing its own `RuleId` literal at
    /// construction (parse-at-boundary).
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: "THREAT-MAP-UNIT-COVERAGE.1".parse()?,
        })
    }
}

impl Validator for ThreatMapUnitCoverageValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let Some(map) = parse_threat_map(input.source.as_str()) else {
            return Vec::new();
        };

        let mut findings = Vec::new();
        for (index, unit) in map.units.iter().enumerate() {
            let missing = missing_edges(unit);
            if missing.is_empty() {
                continue;
            }
            findings.extend(canonical_finding! {
                rule_id: self.rule_id.clone(),
                severity: Severity::Error,
                title: "money-critical unit has an incomplete threat/test mapping (T1)".to_owned(),
                detail: format!(
                    "unit `{}` is missing: {}. Doctrine (Â§8.5): every money-critical unit must \
                     map to >=1 threat, >=1 invariant, >=1 property test, >=1 concurrency test, \
                     and >=1 replay test. Fix: add the missing edge(s) to this unit's THREAT_MAP \
                     entry.",
                    unit.unit,
                    missing.join(", ")
                ),
                file: input.file.clone(),
                line: u32::try_from(index).unwrap_or(u32::MAX).saturating_add(1),
                snippet: None,
            });
        }
        findings
    }
}

/// `THREAT-MAP-NO-UNMAPPED.1` â€” T1 unmapped-logic-forbidden gate (Â§0.5).
///
/// Any unit named in the h01-shaped `moneyCriticalUnits` manifest snapshot
/// that has no corresponding `units` entry is "unmapped logic" and is
/// flagged, independent of whether the units that ARE present are fully
/// mapped (that completeness check is
/// [`ThreatMapUnitCoverageValidator`]'s job).
pub struct ThreatMapNoUnmappedValidator {
    rule_id: RuleId,
}

impl ThreatMapNoUnmappedValidator {
    /// Build the validator, parsing its own `RuleId` literal at
    /// construction (parse-at-boundary).
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: "THREAT-MAP-NO-UNMAPPED.1".parse()?,
        })
    }
}

impl Validator for ThreatMapNoUnmappedValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let Some(map) = parse_threat_map(input.source.as_str()) else {
            return Vec::new();
        };

        let mut findings = Vec::new();
        for (index, classified_unit) in map.money_critical_units.iter().enumerate() {
            let is_mapped = map.units.iter().any(|entry| &entry.unit == classified_unit);
            if is_mapped {
                continue;
            }
            findings.extend(canonical_finding! {
                rule_id: self.rule_id.clone(),
                severity: Severity::Error,
                title: "money-critical unit absent from THREAT_MAP (T1)".to_owned(),
                detail: format!(
                    "unit `{classified_unit}` is classified money-critical but has no entry in \
                     THREAT_MAP `units`. Doctrine (Â§0.5): unmapped logic is forbidden logic â€” \
                     every h01-classified unit MUST have a THREAT_MAP entry. Fix: add a `units` \
                     entry for `{classified_unit}` mapping it to its threats, invariants, and \
                     tests."
                ),
                file: input.file.clone(),
                line: u32::try_from(index).unwrap_or(u32::MAX).saturating_add(1),
                snippet: None,
            });
        }
        findings
    }
}

/// `THREAT-MAP-THREAT-HAS-TEST.1` â€” T1 declared-threat completeness gate
/// (Â§0.5).
///
/// Any threat declared under the record's `threats` list with zero
/// associated test ids means the threat model itself is incomplete â€” the
/// threat is named but nothing proves the system resists it. This is
/// independent of whether any `units` entry happens to reference that
/// threat id.
pub struct ThreatMapThreatHasTestValidator {
    rule_id: RuleId,
}

impl ThreatMapThreatHasTestValidator {
    /// Build the validator, parsing its own `RuleId` literal at
    /// construction (parse-at-boundary).
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: "THREAT-MAP-THREAT-HAS-TEST.1".parse()?,
        })
    }
}

impl Validator for ThreatMapThreatHasTestValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let Some(map) = parse_threat_map(input.source.as_str()) else {
            return Vec::new();
        };

        let mut findings = Vec::new();
        for (index, threat) in map.threats.iter().enumerate() {
            if !threat.tests.is_empty() {
                continue;
            }
            findings.extend(canonical_finding! {
                rule_id: self.rule_id.clone(),
                severity: Severity::Error,
                title: "declared threat has zero backing tests (T1, incomplete threat model)"
                    .to_owned(),
                detail: format!(
                    "threat `{}` is declared in THREAT_MAP `threats` with an empty `tests` list. \
                     Doctrine (Â§0.5): a declared threat with zero tests means the threat model is \
                     incomplete â€” naming a threat proves nothing without a test that would fail \
                     if the corresponding protection were removed. Fix: add >=1 test id to this \
                     threat's `tests` array.",
                    threat.threat_id.as_str()
                ),
                file: input.file.clone(),
                line: u32::try_from(index).unwrap_or(u32::MAX).saturating_add(1),
                snippet: None,
            });
        }
        findings
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use enforcer_domain::findings::ScanScope;
    use enforcer_domain::paths::RelPath;
    use enforcer_validator::harness::run_fixture_parity;
    use enforcer_validator::validator::{ValidationInput, Validator};

    use super::{
        ThreatMapNoUnmappedValidator, ThreatMapThreatHasTestValidator,
        ThreatMapUnitCoverageValidator,
    };

    fn manifest_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    fn rel(path: &str) -> Result<RelPath, Box<dyn std::error::Error>> {
        Ok(path.parse()?)
    }

    #[test]
    fn threat_map_no_unmapped_fixture_parity() -> Result<(), Box<dyn std::error::Error>> {
        let validator = ThreatMapNoUnmappedValidator::new()?;
        run_fixture_parity(
            &validator,
            &enforcer_domain::paths::RepoRoot::try_from(manifest_dir().as_path())?,
            &"tests/fixtures/threat_test_mapping/unmapped/bad/unit_absent.json".parse()?,
            &"tests/fixtures/threat_test_mapping/unmapped/good/full_mapping.json".parse()?,
        )?;
        Ok(())
    }

    #[test]
    fn threat_map_unit_coverage_fixture_parity() -> Result<(), Box<dyn std::error::Error>> {
        // Representative triple from the workpack: `mapped/bad/partial_mapping.json`
        // has a unit with a threat and an invariant and a property test but
        // NO concurrency/replay test â€” a genuine coverage gap, distinct
        // from `unmapped/bad/unit_absent.json` (a unit missing entirely,
        // which is `THREAT-MAP-NO-UNMAPPED.1`'s fail case, not this rule's).
        let validator = ThreatMapUnitCoverageValidator::new()?;
        run_fixture_parity(
            &validator,
            &enforcer_domain::paths::RepoRoot::try_from(manifest_dir().as_path())?,
            &"tests/fixtures/threat_test_mapping/mapped/bad/partial_mapping.json".parse()?,
            &"tests/fixtures/threat_test_mapping/mapped/good/full_mapping.json".parse()?,
        )?;

        // The bad fixture's single finding must name every missing edge.
        let bad_source = std::fs::read_to_string(
            manifest_dir()
                .join("tests/fixtures/threat_test_mapping/mapped/bad/partial_mapping.json"),
        )?;
        let file = rel("crates/x/threat-map.json")?;
        let bad_findings = validator.validate(ValidationInput {
            file: &file,
            source: enforcer_domain::boundary::validation::ValidationSource::from_text(&bad_source),
            scope: ScanScope::Files,
        });
        assert_eq!(bad_findings.len(), 1);
        assert!(bad_findings[0]
            .detail
            .as_str()
            .contains("missing: concurrency test, replay test"));
        Ok(())
    }

    #[test]
    fn threat_map_threat_has_test_fixture_parity() -> Result<(), Box<dyn std::error::Error>> {
        let validator = ThreatMapThreatHasTestValidator::new()?;
        run_fixture_parity(
            &validator,
            &enforcer_domain::paths::RepoRoot::try_from(manifest_dir().as_path())?,
            &"tests/fixtures/threat_test_mapping/incomplete/bad/threat_zero_tests.json".parse()?,
            &"tests/fixtures/threat_test_mapping/incomplete/good/threat_with_test.json".parse()?,
        )?;
        Ok(())
    }

    #[test]
    fn unparseable_source_stays_silent_for_all_three_validators(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let file = rel("crates/x/threat-map.json")?;
        let coverage = ThreatMapUnitCoverageValidator::new()?;
        let unmapped = ThreatMapNoUnmappedValidator::new()?;
        let threat_has_test = ThreatMapThreatHasTestValidator::new()?;
        let source = "this is not valid json {{{";
        assert!(coverage
            .validate(ValidationInput {
                file: &file,
                source: enforcer_domain::boundary::validation::ValidationSource::from_text(source),
                scope: ScanScope::Files,
            })
            .is_empty());
        assert!(unmapped
            .validate(ValidationInput {
                file: &file,
                source: enforcer_domain::boundary::validation::ValidationSource::from_text(source),
                scope: ScanScope::Files,
            })
            .is_empty());
        assert!(threat_has_test
            .validate(ValidationInput {
                file: &file,
                source: enforcer_domain::boundary::validation::ValidationSource::from_text(source),
                scope: ScanScope::Files,
            })
            .is_empty());
        Ok(())
    }

    #[test]
    fn full_mapping_stays_clean_across_all_three_validators(
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Full 5-way-mapped record with all threats backed by tests must
        // stay clean across every h03 validator simultaneously.
        let source = std::fs::read_to_string(
            manifest_dir().join("tests/fixtures/threat_test_mapping/mapped/good/full_mapping.json"),
        )?;
        let file = rel("crates/x/threat-map.json")?;
        for validator in [
            Box::new(ThreatMapUnitCoverageValidator::new()?) as Box<dyn Validator>,
            Box::new(ThreatMapNoUnmappedValidator::new()?) as Box<dyn Validator>,
            Box::new(ThreatMapThreatHasTestValidator::new()?) as Box<dyn Validator>,
        ] {
            let findings = validator.validate(ValidationInput {
                file: &file,
                source: enforcer_domain::boundary::validation::ValidationSource::from_text(&source),
                scope: ScanScope::Files,
            });
            assert!(
                findings.is_empty(),
                "rule {} must stay clean on the fully-mapped fixture: {findings:#?}",
                validator.rule_id()
            );
        }
        Ok(())
    }
}
