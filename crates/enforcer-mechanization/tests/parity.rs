//! Integration test for the whole-registry `parity` sweep
//! ([`enforcer_mechanization::parity::ParityOracle`]), exercised against
//! on-disk fixtures under `tests/fixtures/parity/**` rather than the
//! crate-local unit tests in `src/parity.rs` (which use inline fixture
//! strings and the crate's own `fixtures/scaffold/` pair).
//!
//! Named oracle proof for TEST_PROOF_EXPECTATIONS.md's d01 row: seeds each
//! of the five parity legs missing in turn (record/doc-anchor/validator/
//! fixtures/detection) and asserts the sweep fails closed on each one, plus
//! one fully-wired case proving the sweep is clean when all five agree.

use std::collections::BTreeSet;

use enforcer_domain::findings::Finding;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::{Severity, Tier};
use enforcer_mechanization::parity::{ParityOracle, ValidatorLookup};
use enforcer_rules::registry::{FixtureRef, RuleRecord, RuleRegistry, ValidatorRef};
use enforcer_validator::validator::{ValidationInput, Validator};

fn manifest_dir() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn wired_record(
    rule_id: &str,
) -> Result<RuleRecord, enforcer_domain::boundary::decode_error::DecodeError> {
    Ok(RuleRecord {
        rule_id: rule_id.parse()?,
        version: 1,
        title: "Integration parity sample rule".to_owned(),
        tier: Tier::T1,
        validator: ValidatorRef {
            crate_name: "enforcer-mechanization".to_owned(),
            path: "parity_it::MarkerValidator".to_owned(),
        },
        fixtures: FixtureRef {
            fail: "tests/fixtures/parity/registry/fail.txt".to_owned(),
            pass: "tests/fixtures/parity/registry/pass.txt".to_owned(),
        },
        doc_anchor: "tests/fixtures/parity/docs/SAMPLE.md#SAMPLE-ANCHOR".to_owned(),
        tags: vec![],
        params: serde_json::Value::Null,
    })
}

struct MarkerValidator {
    rule_id: RuleId,
}

impl Validator for MarkerValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        if input.source.contains("SCAFFOLD_MARKER") {
            vec![Finding {
                rule_id: self.rule_id.clone(),
                severity: Severity::Error,
                title: "marker present".to_owned(),
                detail: "found SCAFFOLD_MARKER".to_owned(),
                file: input.file.clone(),
                line: 1,
                snippet: None,
            }]
        } else {
            Vec::new()
        }
    }
}

struct SingleLookup<'a>(&'a RuleId, &'a dyn Validator);

impl ValidatorLookup for SingleLookup<'_> {
    fn resolve(&self, rule_id: &RuleId) -> Option<&dyn Validator> {
        if rule_id == self.0 {
            Some(self.1)
        } else {
            None
        }
    }
}

struct EmptyLookup;

impl ValidatorLookup for EmptyLookup {
    fn resolve(&self, _rule_id: &RuleId) -> Option<&dyn Validator> {
        None
    }
}

/// Positive case: all five legs (ruleId, doc anchor, validator, fail
/// fixture, pass fixture) agree — the sweep must be clean.
#[test]
fn full_five_way_chain_passes_clean() -> Result<(), Box<dyn std::error::Error>> {
    let record = wired_record("RR-88.1")?;
    let rule_id = record.rule_id.clone();
    let registry = RuleRegistry::from_records(vec![record])?;
    let validator = MarkerValidator {
        rule_id: rule_id.clone(),
    };
    let lookup = SingleLookup(&rule_id, &validator);
    let oracle = ParityOracle::new(&registry, &manifest_dir(), BTreeSet::new());
    assert!(oracle.sweep(&lookup).is_empty());
    Ok(())
}

/// Leg 1 missing: no registry record at all for a claimed rule id (the
/// reverse/orphan direction) — fails closed.
#[test]
fn missing_registry_record_leg_fails_closed() -> Result<(), Box<dyn std::error::Error>> {
    let registry = RuleRegistry::from_records(vec![])?;
    let orphan: RuleId = "RR-88.2".parse()?;
    let mut orphans = BTreeSet::new();
    orphans.insert(orphan);
    let oracle = ParityOracle::new(&registry, &manifest_dir(), orphans);
    let findings = oracle.sweep(&EmptyLookup);
    assert_eq!(findings.len(), 1);
    assert!(findings[0].title.contains("orphan"));
    Ok(())
}

/// Leg 2 missing: dangling doc anchor — the file/fragment does not resolve.
#[test]
fn missing_doc_anchor_leg_fails_closed() -> Result<(), Box<dyn std::error::Error>> {
    let mut record = wired_record("RR-88.3")?;
    record.doc_anchor = "tests/fixtures/parity/docs/DOES-NOT-EXIST.md#NOPE".to_owned();
    let rule_id = record.rule_id.clone();
    let registry = RuleRegistry::from_records(vec![record])?;
    let validator = MarkerValidator {
        rule_id: rule_id.clone(),
    };
    let lookup = SingleLookup(&rule_id, &validator);
    let oracle = ParityOracle::new(&registry, &manifest_dir(), BTreeSet::new());
    let findings = oracle.sweep(&lookup);
    assert_eq!(findings.len(), 1);
    assert!(findings[0].detail.contains("does not resolve"));
    Ok(())
}

/// Leg 3 missing: no validator wired for the rule id at all.
#[test]
fn missing_validator_leg_fails_closed() -> Result<(), Box<dyn std::error::Error>> {
    let record = wired_record("RR-88.4")?;
    let registry = RuleRegistry::from_records(vec![record])?;
    let oracle = ParityOracle::new(&registry, &manifest_dir(), BTreeSet::new());
    let findings = oracle.sweep(&EmptyLookup);
    assert_eq!(findings.len(), 1);
    assert!(findings[0].detail.contains("no validator wired"));
    Ok(())
}

/// Leg 4 missing: the fail fixture path does not exist on disk.
#[test]
fn missing_fail_fixture_leg_fails_closed() -> Result<(), Box<dyn std::error::Error>> {
    let mut record = wired_record("RR-88.5")?;
    record.fixtures.fail = "tests/fixtures/parity/registry/does-not-exist.txt".to_owned();
    let rule_id = record.rule_id.clone();
    let registry = RuleRegistry::from_records(vec![record])?;
    let validator = MarkerValidator {
        rule_id: rule_id.clone(),
    };
    let lookup = SingleLookup(&rule_id, &validator);
    let oracle = ParityOracle::new(&registry, &manifest_dir(), BTreeSet::new());
    let findings = oracle.sweep(&lookup);
    assert_eq!(findings.len(), 1);
    Ok(())
}

/// Leg 4b: the pass fixture path does not exist on disk (same leg,
/// opposite fixture — kept as a distinct row since the workpack calls out
/// "missing fixture" without specifying which side).
#[test]
fn missing_pass_fixture_leg_fails_closed() -> Result<(), Box<dyn std::error::Error>> {
    let mut record = wired_record("RR-88.6")?;
    record.fixtures.pass = "tests/fixtures/parity/registry/does-not-exist.txt".to_owned();
    let rule_id = record.rule_id.clone();
    let registry = RuleRegistry::from_records(vec![record])?;
    let validator = MarkerValidator {
        rule_id: rule_id.clone(),
    };
    let lookup = SingleLookup(&rule_id, &validator);
    let oracle = ParityOracle::new(&registry, &manifest_dir(), BTreeSet::new());
    let findings = oracle.sweep(&lookup);
    assert_eq!(findings.len(), 1);
    Ok(())
}

/// Leg 5: the "detection" leg — a validator IS wired but does not actually
/// fire on the fail fixture (the classic "silent/broken validator" case
/// the doctrine calls out).
#[test]
fn validator_does_not_fire_on_fail_fixture_fails_closed() -> Result<(), Box<dyn std::error::Error>>
{
    struct SilentValidator {
        rule_id: RuleId,
    }
    impl Validator for SilentValidator {
        fn rule_id(&self) -> &RuleId {
            &self.rule_id
        }
        fn validate(&self, _input: ValidationInput<'_>) -> Vec<Finding> {
            Vec::new()
        }
    }

    let record = wired_record("RR-88.7")?;
    let rule_id = record.rule_id.clone();
    let registry = RuleRegistry::from_records(vec![record])?;
    let validator = SilentValidator {
        rule_id: rule_id.clone(),
    };
    let lookup = SingleLookup(&rule_id, &validator);
    let oracle = ParityOracle::new(&registry, &manifest_dir(), BTreeSet::new());
    let findings = oracle.sweep(&lookup);
    assert_eq!(findings.len(), 1);
    Ok(())
}

/// Companion detection-leg case: a validator fires on the PASS fixture
/// too (over-eager/broken the other direction) — also fails closed.
#[test]
fn validator_fires_on_pass_fixture_fails_closed() -> Result<(), Box<dyn std::error::Error>> {
    struct AlwaysFiresValidator {
        rule_id: RuleId,
    }
    impl Validator for AlwaysFiresValidator {
        fn rule_id(&self) -> &RuleId {
            &self.rule_id
        }
        fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
            vec![Finding {
                rule_id: self.rule_id.clone(),
                severity: Severity::Error,
                title: "always fires".to_owned(),
                detail: "broken validator".to_owned(),
                file: input.file.clone(),
                line: 1,
                snippet: None,
            }]
        }
    }

    let record = wired_record("RR-88.8")?;
    let rule_id = record.rule_id.clone();
    let registry = RuleRegistry::from_records(vec![record])?;
    let validator = AlwaysFiresValidator {
        rule_id: rule_id.clone(),
    };
    let lookup = SingleLookup(&rule_id, &validator);
    let oracle = ParityOracle::new(&registry, &manifest_dir(), BTreeSet::new());
    let findings = oracle.sweep(&lookup);
    assert_eq!(findings.len(), 1);
    Ok(())
}

/// T3 label-presence gate, exercised at integration level: a T3 record
/// missing the verbatim label fails closed even though it carries no
/// fixtures/validator requirement.
#[test]
fn t3_record_without_label_fails_closed() -> Result<(), Box<dyn std::error::Error>> {
    let mut record = wired_record("RR-88.9")?;
    record.tier = Tier::T3;
    record.tags = vec!["no-label-here".to_owned()];
    let registry = RuleRegistry::from_records(vec![record])?;
    let oracle = ParityOracle::new(&registry, &manifest_dir(), BTreeSet::new());
    let findings = oracle.sweep(&EmptyLookup);
    assert_eq!(findings.len(), 1);
    assert!(findings[0].detail.contains("mandatory verbatim label"));
    Ok(())
}
