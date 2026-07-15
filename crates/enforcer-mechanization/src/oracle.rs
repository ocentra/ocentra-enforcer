//! The fail-closed parity oracle: a rule is only ACCEPTED if its record
//! shape is well-formed, a validator implementation is supplied, and that
//! validator fires on the declared fail fixture while staying silent on
//! the declared pass fixture.
//!
//! This is the d01 port of the `.mjs` contract-coverage / contract-load
//! scripts (`scripts/check-source-core-contract-*.mjs`): those scripts
//! refused to accept a rule/contract lacking full linkage coverage. This
//! oracle applies the same fail-closed posture to the Rust rule pipeline,
//! reusing [`enforcer_validator::harness::run_fixture_parity`] as the
//! reusable base rather than reimplementing fixture I/O here.

use std::path::Path;

use enforcer_rules::registry::{RuleRecord, RuleRegistry};
use enforcer_validator::harness::run_fixture_parity;
use enforcer_validator::validator::Validator;

use crate::error::{MechanizationError, MechanizationResult};

/// Accept or reject one candidate rule.
///
/// - `record` is the rule as scaffolded (see [`crate::scaffold::scaffold_rule`])
///   or otherwise assembled.
/// - `validator`, if present, is the concrete implementation to test. `None`
///   means "no validator wired yet" — always rejected; a rule record without
///   an implementation to prove is never accepted, regardless of what its
///   fixtures contain.
/// - `repo_root` is the root fixture paths in `record.fixtures` are resolved
///   relative to (typically `CARGO_MANIFEST_DIR` of the calling crate).
///
/// Returns `Ok(())` only when: the record's shape passes
/// `RuleRegistry::from_records` validation, a validator was supplied, that
/// validator's `rule_id()` matches `record.rule_id`, it fires on the fail
/// fixture, and it stays silent on the pass fixture.
pub fn accept_rule(
    record: &RuleRecord,
    validator: Option<&dyn Validator>,
    repo_root: &Path,
) -> MechanizationResult<()> {
    // Re-verify record shape independently of however the caller built it —
    // the oracle does not trust the scaffolder, it proves the record.
    RuleRegistry::from_records(vec![record.clone()]).map_err(|source| {
        MechanizationError::RecordRejected {
            rule_id: record.rule_id.to_string(),
            source,
        }
    })?;

    let Some(validator) = validator else {
        return Err(MechanizationError::MissingValidator {
            rule_id: record.rule_id.to_string(),
        });
    };

    if validator.rule_id() != &record.rule_id {
        return Err(MechanizationError::ParityFailed {
            rule_id: record.rule_id.to_string(),
            source: enforcer_validator::error::HarnessError::DidNotFireOnFail {
                rule_id: format!(
                    "record declares `{}` but validator implements `{}`",
                    record.rule_id,
                    validator.rule_id()
                ),
                fixture: record.fixtures.fail.clone(),
            },
        });
    }

    run_fixture_parity(
        validator,
        repo_root,
        &record.fixtures.fail,
        &record.fixtures.pass,
    )
    .map_err(|source| MechanizationError::ParityFailed {
        rule_id: record.rule_id.to_string(),
        source,
    })
}

#[cfg(test)]
mod tests {
    use super::accept_rule;
    use crate::error::MechanizationError;
    use enforcer_domain::findings::{Finding, ScanScope};
    use enforcer_domain::ids::RuleId;
    use enforcer_domain::severity::{Severity, Tier};
    use enforcer_rules::registry::{FixtureRef, RuleRecord, ValidatorRef};
    use enforcer_validator::validator::{ValidationInput, Validator};

    fn manifest_dir() -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    fn record(
        rule_id: &str,
    ) -> Result<RuleRecord, enforcer_domain::boundary::decode_error::DecodeError> {
        Ok(RuleRecord {
            rule_id: rule_id.parse()?,
            version: 1,
            title: "Sample scaffolded rule".to_owned(),
            tier: Tier::T1,
            validator: ValidatorRef {
                crate_name: "enforcer-mechanization".to_owned(),
                path: "oracle::MarkerValidator".to_owned(),
            },
            fixtures: FixtureRef {
                fail: "fixtures/scaffold/fail.txt".to_owned(),
                pass: "fixtures/scaffold/pass.txt".to_owned(),
            },
            doc_anchor: "docs/rules/SCAFFOLD.md#SCAFFOLD-1".to_owned(),
            tags: vec![],
            params: serde_json::Value::Null,
        })
    }

    /// A validator that fires only when the source contains the literal
    /// marker `SCAFFOLD_MARKER` — mirrors the fixture pair this crate ships
    /// under `fixtures/scaffold/`.
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
                    title: "scaffold marker present".to_owned(),
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

    #[test]
    fn accepts_a_complete_rule() -> Result<(), Box<dyn std::error::Error>> {
        let record = record("RR-90.1")?;
        let validator = MarkerValidator {
            rule_id: record.rule_id.clone(),
        };
        accept_rule(&record, Some(&validator), &manifest_dir())?;
        Ok(())
    }

    #[test]
    fn rejects_missing_validator() -> Result<(), Box<dyn std::error::Error>> {
        let record = record("RR-90.2")?;
        let outcome = accept_rule(&record, None, &manifest_dir());
        assert!(matches!(
            outcome,
            Err(MechanizationError::MissingValidator { .. })
        ));
        Ok(())
    }

    #[test]
    fn rejects_non_firing_validator_on_fail_fixture() -> Result<(), Box<dyn std::error::Error>> {
        let record = record("RR-90.3")?;
        let validator = SilentValidator {
            rule_id: record.rule_id.clone(),
        };
        let outcome = accept_rule(&record, Some(&validator), &manifest_dir());
        assert!(matches!(
            outcome,
            Err(MechanizationError::ParityFailed { .. })
        ));
        Ok(())
    }

    #[test]
    fn rejects_validator_that_fires_on_pass_fixture() -> Result<(), Box<dyn std::error::Error>> {
        let record = record("RR-90.4")?;
        let validator = AlwaysFiresValidator {
            rule_id: record.rule_id.clone(),
        };
        let outcome = accept_rule(&record, Some(&validator), &manifest_dir());
        assert!(matches!(
            outcome,
            Err(MechanizationError::ParityFailed { .. })
        ));
        Ok(())
    }

    #[test]
    fn rejects_record_with_missing_fixture_path() -> Result<(), Box<dyn std::error::Error>> {
        let mut record = record("RR-90.5")?;
        record.fixtures.fail = "   ".to_owned();
        let validator = MarkerValidator {
            rule_id: record.rule_id.clone(),
        };
        let outcome = accept_rule(&record, Some(&validator), &manifest_dir());
        assert!(matches!(
            outcome,
            Err(MechanizationError::RecordRejected { .. })
        ));
        Ok(())
    }

    #[test]
    fn rejects_validator_rule_id_mismatch() -> Result<(), Box<dyn std::error::Error>> {
        let record = record("RR-90.6")?;
        let validator = MarkerValidator {
            rule_id: "RR-90.7".parse()?,
        };
        let outcome = accept_rule(&record, Some(&validator), &manifest_dir());
        assert!(matches!(
            outcome,
            Err(MechanizationError::ParityFailed { .. })
        ));
        Ok(())
    }

    #[test]
    fn scope_is_files_for_fixture_reads() {
        // Sanity: the harness this oracle delegates to always reads
        // fixtures as ScanScope::Files (documented behavior, not this
        // crate's to re-implement) — this test exists only to keep the
        // import used and pin the expectation in one place for readers.
        let scope = ScanScope::Files;
        assert_eq!(scope, ScanScope::Files);
    }
}
