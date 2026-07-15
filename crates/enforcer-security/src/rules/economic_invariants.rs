//! `ECON-INVARIANT-PRESENCE.1` + `ECON-INVARIANT-SHAPE.1` (both T1) — the
//! economic/logic invariant property-suite family (h05, §2.3/§4/§8.3 of
//! the ingested money-critical/security-testing spec).
//!
//! Doctrine (§2.3): a money-critical unit (as classified by h01, consumed
//! here read-only, never redefined) MUST guarantee ten economic/logic
//! invariants, each proven by a property-based test that REFUTES the bad
//! outcome over generated inputs — a single hand-picked literal case does
//! not discharge the obligation. A settlement module can otherwise ship
//! with zero property coverage of e.g. "failure != reward" and still pass.
//!
//! # The ten invariants (§2.3, mapped to G1-G6)
//!
//! `same-request-twice != more-value` (idempotency), `failure != reward`,
//! `retry != mutation`, `partial-failure != profit`, `order != advantage`,
//! `attacker-cost >= system-cost`, `compensation idempotent+replay-safe`,
//! `time-assumptions fail-closed`, `backend-never-signs-unverifiable`,
//! `emergency-reduces-blast`.
//!
//! # Two validators, one INVARIANT_SUITE record shape
//!
//! - [`EconomicInvariantPresenceValidator`] (`ECON-INVARIANT-PRESENCE.1`):
//!   for a unit listed in the h01-shaped `moneyCriticalUnits` manifest
//!   snapshot, every one of the ten required invariant ids must have a
//!   corresponding entry in that unit's `properties` map — any missing
//!   invariant is a `Finding`. A unit with all ten present is clean
//!   regardless of shape (that is [`EconomicInvariantShapeValidator`]'s
//!   concern). A unit absent from `moneyCriticalUnits` requires no suite at
//!   all (non-money-critical scope, per the workpack's `plain_util`
//!   representative fixture).
//! - [`EconomicInvariantShapeValidator`] (`ECON-INVARIANT-SHAPE.1`): every
//!   invariant entry that IS present must carry `"shape": "property"` — a
//!   generator-driven refutation (`fc.assert(fc.property(...))` for TS/JS,
//!   `proptest!`/`quickcheck` for Rust, Hypothesis `@given` for Python).
//!   An entry recorded as `"shape": "single_case"` (one hand-picked literal
//!   assertion, not a property) is flagged as non-property shape, even
//!   though the invariant is nominally "present".
//!
//! GENERIC across any value system (fiat, Stripe, an internal ledger, or
//! the optional crypto/Anchor instance) — the fixtures below use a
//! deliberately neutral `credit_balance`/`settlement` vocabulary, never a
//! crypto-only one.
//!
//! # INVARIANT_SUITE wire shape
//!
//! ```jsonc
//! {
//!   "moneyCriticalUnits": ["settlement"],
//!   "units": [
//!     {
//!       "unit": "settlement",
//!       "properties": {
//!         "failure-not-reward": { "shape": "property", "test": "prop_failure_not_reward" },
//!         "idempotent-replay": { "shape": "single_case", "test": "single_case_replay_once" }
//!       }
//!     }
//!   ]
//! }
//! ```
//!
//! `moneyCriticalUnits` is the h01-shaped money-critical manifest SNAPSHOT
//! this fixture format carries inline (a real pipeline wiring would
//! resolve this list from h01's `#[money_critical(registered)]` scan
//! instead; this record format lets the presence/shape check be exercised
//! standalone against a fixture, exactly like the h03 representative
//! triples this module follows).

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::findings::Finding;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

/// The ten required economic/logic invariant ids from §2.3, in the order
/// the spec enumerates them. Stable, short, kebab-case ids — these are the
/// keys a unit's `properties` map is checked against, not display strings.
const REQUIRED_INVARIANTS: &[&str] = &[
    "idempotent-replay",
    "failure-not-reward",
    "retry-not-mutation",
    "partial-failure-not-profit",
    "order-not-advantage",
    "attacker-cost-not-below-system-cost",
    "compensation-idempotent-replay-safe",
    "time-assumptions-fail-closed",
    "backend-never-signs-unverifiable",
    "emergency-reduces-blast",
];

/// One invariant entry's shape: `"property"` (generator-driven refutation)
/// or `"single_case"` (a single hand-picked literal assertion — does not
/// discharge the property obligation).
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
enum PropertyShape {
    Property,
    SingleCase,
}

/// One invariant's recorded coverage: its assertion shape and (for finding
/// detail only) the test identifier that backs it.
#[derive(Debug, Clone, serde::Deserialize)]
struct PropertyEntry {
    shape: PropertyShape,
    #[serde(default)]
    test: String,
}

/// One `units` entry: a unit name paired with whichever of the ten
/// required invariants it currently records, keyed by invariant id.
#[derive(Debug, Clone, serde::Deserialize)]
struct UnitEntry {
    unit: String,
    #[serde(default)]
    properties: std::collections::BTreeMap<String, PropertyEntry>,
}

/// The whole INVARIANT_SUITE record: the h01-shaped money-critical
/// manifest snapshot (`moneyCriticalUnits`) and the per-unit invariant
/// coverage ledger (`units`).
#[derive(Debug, Clone, serde::Deserialize)]
struct InvariantSuite {
    #[serde(rename = "moneyCriticalUnits", default)]
    money_critical_units: Vec<String>,
    #[serde(default)]
    units: Vec<UnitEntry>,
}

/// Parse `source` as an INVARIANT_SUITE record. Unparseable/non-JSON
/// source is not this validator family's concern (mirrors h01/h03's
/// "unparseable source stays silent" contract) — returns `None` rather
/// than a `Finding`.
fn parse_invariant_suite(source: &str) -> Option<InvariantSuite> {
    serde_json::from_str(source).ok()
}

/// The `units` entry for `unit_name`, if this record's manifest actually
/// requires a suite for it (i.e. `unit_name` is present in
/// `moneyCriticalUnits`). A unit outside `moneyCriticalUnits` needs no
/// suite at all, regardless of whether a stray `units` entry exists for it.
fn scoped_unit<'a>(map: &'a InvariantSuite, unit_name: &str) -> Option<&'a UnitEntry> {
    map.units.iter().find(|entry| entry.unit == unit_name)
}

/// `ECON-INVARIANT-PRESENCE.1` — T1 per-unit invariant-presence gate
/// (§2.3/§4/§8.3).
///
/// Every unit listed under `moneyCriticalUnits` must carry all ten
/// required invariant ids in its `properties` map (scoped from the
/// `units` entry, if any — a money-critical unit with no `units` entry at
/// all is missing every invariant). Any missing invariant is a `Finding`
/// naming that invariant id; a unit with all ten present is clean under
/// THIS validator (shape correctness is
/// [`EconomicInvariantShapeValidator`]'s job). A unit absent from
/// `moneyCriticalUnits` is out of scope entirely — no suite required.
pub struct EconomicInvariantPresenceValidator {
    rule_id: RuleId,
}

impl EconomicInvariantPresenceValidator {
    /// Build the validator, parsing its own `RuleId` literal at
    /// construction (parse-at-boundary).
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: "ECON-INVARIANT-PRESENCE.1".parse()?,
        })
    }
}

impl Validator for EconomicInvariantPresenceValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let Some(suite) = parse_invariant_suite(input.source) else {
            return Vec::new();
        };

        let mut findings = Vec::new();
        for (index, unit_name) in suite.money_critical_units.iter().enumerate() {
            let present: std::collections::BTreeSet<&str> = scoped_unit(&suite, unit_name)
                .map(|entry| entry.properties.keys().map(String::as_str).collect())
                .unwrap_or_default();

            let missing: Vec<&str> = REQUIRED_INVARIANTS
                .iter()
                .copied()
                .filter(|invariant| !present.contains(invariant))
                .collect();

            if missing.is_empty() {
                continue;
            }

            findings.push(Finding {
                rule_id: self.rule_id.clone(),
                severity: Severity::Error,
                title: "money-critical unit is missing required economic invariant properties \
                        (T1)"
                    .to_owned(),
                detail: format!(
                    "unit `{unit_name}` is missing the invariant property test(s): {}. Doctrine \
                     (§2.3/§4/§8.3): every money-critical unit MUST guarantee all ten \
                     economic/logic invariants, each proven by a property-based test. Fix: add a \
                     generator-driven property test for each missing invariant to this unit's \
                     INVARIANT_SUITE `properties` entry.",
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

/// `ECON-INVARIANT-SHAPE.1` — T1 assertion-shape gate (§2.3/§4/§8.3).
///
/// Every invariant entry that IS present in a money-critical unit's
/// `properties` map must carry `"shape": "property"` — a generator-driven
/// refutation of the bad outcome. An entry recorded `"shape":
/// "single_case"` (a single hand-picked literal assertion) is flagged,
/// independent of [`EconomicInvariantPresenceValidator`]'s presence check
/// (an invariant can be "present" by key but still fail this shape gate).
pub struct EconomicInvariantShapeValidator {
    rule_id: RuleId,
}

impl EconomicInvariantShapeValidator {
    /// Build the validator, parsing its own `RuleId` literal at
    /// construction (parse-at-boundary).
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            rule_id: "ECON-INVARIANT-SHAPE.1".parse()?,
        })
    }
}

impl Validator for EconomicInvariantShapeValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let Some(suite) = parse_invariant_suite(input.source) else {
            return Vec::new();
        };

        let mut findings = Vec::new();
        for (index, unit) in suite.units.iter().enumerate() {
            // Only units actually in scope (h01-classified) are checked —
            // a stray `units` entry for a non-money-critical unit is not
            // this validator's concern.
            if !suite
                .money_critical_units
                .iter()
                .any(|classified| classified == &unit.unit)
            {
                continue;
            }

            let non_property: Vec<(&str, &str)> = unit
                .properties
                .iter()
                .filter(|(_, entry)| entry.shape == PropertyShape::SingleCase)
                .map(|(invariant, entry)| (invariant.as_str(), entry.test.as_str()))
                .collect();

            if non_property.is_empty() {
                continue;
            }

            let named = non_property
                .iter()
                .map(|(invariant, test)| format!("{invariant} (test: {test})"))
                .collect::<Vec<_>>()
                .join(", ");

            findings.push(Finding {
                rule_id: self.rule_id.clone(),
                severity: Severity::Error,
                title: "invariant test is a single literal case, not a property refutation (T1)"
                    .to_owned(),
                detail: format!(
                    "unit `{}` records the invariant propert{} {named} with shape \
                     `single_case` — a single hand-picked assertion, not a generator-driven \
                     refutation. Doctrine (§4/§8.3): the property MUST be a `fc.assert(fc.property(...))` \
                     (TS/JS), `proptest!`/`quickcheck` (Rust), or Hypothesis `@given` (Python) \
                     refutation over generated inputs, never a single literal case. Fix: rewrite \
                     the test as a property that refutes the bad outcome across generated inputs.",
                    unit.unit,
                    if non_property.len() == 1 { "y" } else { "ies" },
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

    use super::{EconomicInvariantPresenceValidator, EconomicInvariantShapeValidator};

    fn manifest_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    fn rel(path: &str) -> Result<RelPath, Box<dyn std::error::Error>> {
        Ok(path.parse()?)
    }

    #[test]
    fn presence_fixture_parity_failure_not_reward() -> Result<(), Box<dyn std::error::Error>> {
        // Representative triple from the workpack: a suite covering nine
        // invariants but missing `failure-not-reward` -> flagged; the full
        // ten-invariant suite -> clean.
        let validator = EconomicInvariantPresenceValidator::new()?;
        run_fixture_parity(
            &validator,
            &manifest_dir(),
            "tests/fixtures/economic_invariants/failure_not_reward/bad/missing_failure_not_reward.json",
            "tests/fixtures/economic_invariants/full_suite/good/full_suite.json",
        )?;

        let bad_source = std::fs::read_to_string(manifest_dir().join(
            "tests/fixtures/economic_invariants/failure_not_reward/bad/missing_failure_not_reward.json",
        ))?;
        let file = rel("crates/x/invariant-suite.json")?;
        let bad_findings = validator.validate(ValidationInput {
            file: &file,
            source: &bad_source,
            scope: ScanScope::Files,
        });
        assert_eq!(bad_findings.len(), 1);
        assert!(bad_findings[0].detail.contains("failure-not-reward"));
        Ok(())
    }

    #[test]
    fn presence_fixture_parity_attacker_cost() -> Result<(), Box<dyn std::error::Error>> {
        let validator = EconomicInvariantPresenceValidator::new()?;
        run_fixture_parity(
            &validator,
            &manifest_dir(),
            "tests/fixtures/economic_invariants/attacker_cost/bad/cost_no_property.json",
            "tests/fixtures/economic_invariants/attacker_cost/good/cost_property.json",
        )?;
        Ok(())
    }

    #[test]
    fn presence_fixture_parity_compensation() -> Result<(), Box<dyn std::error::Error>> {
        let validator = EconomicInvariantPresenceValidator::new()?;
        run_fixture_parity(
            &validator,
            &manifest_dir(),
            "tests/fixtures/economic_invariants/compensation/bad/compensation_missing.json",
            "tests/fixtures/economic_invariants/compensation/good/compensation_property.json",
        )?;
        Ok(())
    }

    #[test]
    fn presence_stays_clean_for_non_money_critical_unit() -> Result<(), Box<dyn std::error::Error>>
    {
        // `plain_util` is not in `moneyCriticalUnits` -> no suite required
        // at all, regardless of `properties` being entirely absent.
        let validator = EconomicInvariantPresenceValidator::new()?;
        let source = std::fs::read_to_string(
            manifest_dir()
                .join("tests/fixtures/economic_invariants/plain_util/good/plain_util.json"),
        )?;
        let file = rel("crates/x/invariant-suite.json")?;
        let findings = validator.validate(ValidationInput {
            file: &file,
            source: &source,
            scope: ScanScope::Files,
        });
        assert!(findings.is_empty());
        Ok(())
    }

    #[test]
    fn shape_fixture_parity_idempotency() -> Result<(), Box<dyn std::error::Error>> {
        // Representative triple: a single hand-picked replay assertion
        // (not a property) -> flagged as non-property shape; the
        // `fc.assert(fc.property(...))` refutation -> clean.
        let validator = EconomicInvariantShapeValidator::new()?;
        run_fixture_parity(
            &validator,
            &manifest_dir(),
            "tests/fixtures/economic_invariants/idempotency/bad/single_case.json",
            "tests/fixtures/economic_invariants/idempotency/good/property.json",
        )?;

        let bad_source = std::fs::read_to_string(
            manifest_dir()
                .join("tests/fixtures/economic_invariants/idempotency/bad/single_case.json"),
        )?;
        let file = rel("crates/x/invariant-suite.json")?;
        let bad_findings = validator.validate(ValidationInput {
            file: &file,
            source: &bad_source,
            scope: ScanScope::Files,
        });
        assert_eq!(bad_findings.len(), 1);
        assert!(bad_findings[0].detail.contains("single_case"));
        Ok(())
    }

    #[test]
    fn shape_ignores_units_outside_money_critical_scope() -> Result<(), Box<dyn std::error::Error>>
    {
        let validator = EconomicInvariantShapeValidator::new()?;
        let file = rel("crates/x/invariant-suite.json")?;
        let source = r#"{
            "moneyCriticalUnits": [],
            "units": [
                { "unit": "plain_util", "properties": {
                    "idempotent-replay": { "shape": "single_case", "test": "one_off" }
                }}
            ]
        }"#;
        let findings = validator.validate(ValidationInput {
            file: &file,
            source,
            scope: ScanScope::Files,
        });
        assert!(findings.is_empty());
        Ok(())
    }

    #[test]
    fn full_suite_stays_clean_across_both_validators() -> Result<(), Box<dyn std::error::Error>> {
        let source = std::fs::read_to_string(
            manifest_dir()
                .join("tests/fixtures/economic_invariants/full_suite/good/full_suite.json"),
        )?;
        let file = rel("crates/x/invariant-suite.json")?;
        for validator in [
            Box::new(EconomicInvariantPresenceValidator::new()?) as Box<dyn Validator>,
            Box::new(EconomicInvariantShapeValidator::new()?) as Box<dyn Validator>,
        ] {
            let findings = validator.validate(ValidationInput {
                file: &file,
                source: &source,
                scope: ScanScope::Files,
            });
            assert!(
                findings.is_empty(),
                "rule {} must stay clean on the full ten-invariant fixture: {findings:#?}",
                validator.rule_id()
            );
        }
        Ok(())
    }

    #[test]
    fn unparseable_source_stays_silent_for_both_validators(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let file = rel("crates/x/invariant-suite.json")?;
        let presence = EconomicInvariantPresenceValidator::new()?;
        let shape = EconomicInvariantShapeValidator::new()?;
        let source = "this is not valid json {{{";
        assert!(presence
            .validate(ValidationInput {
                file: &file,
                source,
                scope: ScanScope::Files,
            })
            .is_empty());
        assert!(shape
            .validate(ValidationInput {
                file: &file,
                source,
                scope: ScanScope::Files,
            })
            .is_empty());
        Ok(())
    }
}
