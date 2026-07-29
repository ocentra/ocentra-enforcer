//! BOUNDARY-INVARIANT: this boundary module validates raw wire values and converts only through typed domain contracts.
//! The `enforcer-security` `Validator`-registration seam.
//!
//! [`build_all`] is the single place every rule this crate owns is
//! enumerated, paired with its constructed [`Validator`]. This workpack
//! (arc-19, the crate skeleton) registers only the no-bypass meta-check's
//! row. Feature packs (d18, h01-h08, h11) add their own rows here as they
//! land their `src/rules/<name>.rs` modules â€” this is the seam they
//! extend, not a file they own outright (this file is part of the
//! skeleton `owns:` set per the workpack's Parallel Ownership Notes).

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_validator::validator::Validator;

use crate::cyberskills::frontmatter_lint::SkillFrontmatterValidValidator;

use super::boundary::BoundaryValidator;
use super::economic::EconomicValidator;
use super::economic_invariants::{
    EconomicInvariantPresenceValidator, EconomicInvariantShapeValidator,
};
use super::killswitch::KillSwitchValidator;
use super::money_critical::{MoneyCriticalAnnotatedValidator, MoneyCriticalClassifyValidator};
use super::no_bypass::NoBypassValidator;
use super::required_test_categories::{
    RequiredTestCategoriesMapValidator, RequiredTestCategoriesSevenValidator,
};
use super::rollback::RollbackValidator;
use super::security_test_quality::{
    AssertsRejectionValidator, GlobalMutationValidator, MocksForMoneyLogicValidator,
    NoCrashOnlyValidator, PassIfDeletedValidator, ProtectionRemovedHeuristicValidator,
    ReproducibleSeedValidator, SnapshotOnlyValidator, ThreatQualityScoreValidator,
};
use super::signing::SigningValidator;
use super::threat_test_mapping::{
    ThreatMapNoUnmappedValidator, ThreatMapThreatHasTestValidator, ThreatMapUnitCoverageValidator,
};
use super::time::TimeValidator;

/// One registry row: the rule id this row proves, paired with the
/// constructed [`Validator`] trait object.
pub struct RegistryRow {
    /// The rule id this row proves, e.g. `H00-1.1`.
    pub rule_id: &'static str,
    /// The constructed validator for this rule.
    pub validator: Box<dyn Validator>,
}

/// Build every row this crate currently owns. Fails closed (propagates
/// the first construction error) rather than silently dropping a
/// malformed entry â€” a registry that failed to build completely must not
/// be treated as "loaded".
pub fn build_all() -> Result<Vec<RegistryRow>, DecodeError> {
    let rows = vec![
        RegistryRow {
            rule_id: "H00-1.1",
            validator: Box::new(NoBypassValidator::new()?),
        },
        RegistryRow {
            rule_id: "MONEY-CRIT-CLASSIFY.1",
            validator: Box::new(MoneyCriticalClassifyValidator::new()?),
        },
        RegistryRow {
            rule_id: "MONEY-CRIT-ANNOTATED.1",
            validator: Box::new(MoneyCriticalAnnotatedValidator::new()?),
        },
        RegistryRow {
            rule_id: "THREAT-MAP-UNIT-COVERAGE.1",
            validator: Box::new(ThreatMapUnitCoverageValidator::new()?),
        },
        RegistryRow {
            rule_id: "THREAT-MAP-NO-UNMAPPED.1",
            validator: Box::new(ThreatMapNoUnmappedValidator::new()?),
        },
        RegistryRow {
            rule_id: "THREAT-MAP-THREAT-HAS-TEST.1",
            validator: Box::new(ThreatMapThreatHasTestValidator::new()?),
        },
        RegistryRow {
            rule_id: "ECON-INVARIANT-PRESENCE.1",
            validator: Box::new(EconomicInvariantPresenceValidator::new()?),
        },
        RegistryRow {
            rule_id: "ECON-INVARIANT-SHAPE.1",
            validator: Box::new(EconomicInvariantShapeValidator::new()?),
        },
        RegistryRow {
            rule_id: "MCM-SIGNING.1",
            validator: Box::new(SigningValidator::new()?),
        },
        RegistryRow {
            rule_id: "MCM-TIME.1",
            validator: Box::new(TimeValidator::new()?),
        },
        RegistryRow {
            rule_id: "MCM-BOUNDARY.1",
            validator: Box::new(BoundaryValidator::new()?),
        },
        RegistryRow {
            rule_id: "MCM-KILLSWITCH.1",
            validator: Box::new(KillSwitchValidator::new()?),
        },
        RegistryRow {
            rule_id: "MCM-ECONOMIC.1",
            validator: Box::new(EconomicValidator::new()?),
        },
        RegistryRow {
            rule_id: "MCM-ROLLBACK.1",
            validator: Box::new(RollbackValidator::new()?),
        },
        RegistryRow {
            rule_id: "CYBER-FRONTMATTER.1",
            validator: Box::new(SkillFrontmatterValidValidator::new()?),
        },
        RegistryRow {
            rule_id: "REQ-TESTCAT-SEVEN.1",
            validator: Box::new(RequiredTestCategoriesSevenValidator::new()?),
        },
        RegistryRow {
            rule_id: "REQ-TESTCAT-MAP.1",
            validator: Box::new(RequiredTestCategoriesMapValidator::new()?),
        },
        RegistryRow {
            rule_id: "SEC-TEST-ASSERTS-REJECTION.1",
            validator: Box::new(AssertsRejectionValidator::new()?),
        },
        RegistryRow {
            rule_id: "SEC-TEST-MOCKS-MONEY-LOGIC.1",
            validator: Box::new(MocksForMoneyLogicValidator::new()?),
        },
        RegistryRow {
            rule_id: "SEC-TEST-REPRODUCIBLE-SEED.1",
            validator: Box::new(ReproducibleSeedValidator::new()?),
        },
        RegistryRow {
            rule_id: "SEC-TEST-GLOBAL-MUTATION.1",
            validator: Box::new(GlobalMutationValidator::new()?),
        },
        RegistryRow {
            rule_id: "SEC-TEST-PASS-IF-DELETED.1",
            validator: Box::new(PassIfDeletedValidator::new()?),
        },
        RegistryRow {
            rule_id: "SEC-TEST-NO-CRASH-ONLY.1",
            validator: Box::new(NoCrashOnlyValidator::new()?),
        },
        RegistryRow {
            rule_id: "SEC-TEST-SNAPSHOT-ONLY.1",
            validator: Box::new(SnapshotOnlyValidator::new()?),
        },
        RegistryRow {
            rule_id: "SEC-TEST-THREAT-QUALITY-SCORE.1",
            validator: Box::new(ThreatQualityScoreValidator::new()?),
        },
        RegistryRow {
            rule_id: "SEC-TEST-PROTECTION-REMOVED-HEURISTIC.1",
            validator: Box::new(ProtectionRemovedHeuristicValidator::new()?),
        },
    ];

    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::build_all;

    #[test]
    fn registry_builds_cleanly() -> Result<(), Box<dyn std::error::Error>> {
        let rows = build_all()?;
        assert_eq!(rows.len(), 26);
        assert!(rows.iter().any(|row| row.rule_id == "H00-1.1"));
        assert!(rows
            .iter()
            .any(|row| row.rule_id == "MONEY-CRIT-CLASSIFY.1"));
        assert!(rows
            .iter()
            .any(|row| row.rule_id == "MONEY-CRIT-ANNOTATED.1"));
        assert!(rows
            .iter()
            .any(|row| row.rule_id == "THREAT-MAP-UNIT-COVERAGE.1"));
        assert!(rows
            .iter()
            .any(|row| row.rule_id == "THREAT-MAP-NO-UNMAPPED.1"));
        assert!(rows
            .iter()
            .any(|row| row.rule_id == "THREAT-MAP-THREAT-HAS-TEST.1"));
        assert!(rows
            .iter()
            .any(|row| row.rule_id == "ECON-INVARIANT-PRESENCE.1"));
        assert!(rows
            .iter()
            .any(|row| row.rule_id == "ECON-INVARIANT-SHAPE.1"));
        assert!(rows.iter().any(|row| row.rule_id == "MCM-SIGNING.1"));
        assert!(rows.iter().any(|row| row.rule_id == "MCM-TIME.1"));
        assert!(rows.iter().any(|row| row.rule_id == "MCM-BOUNDARY.1"));
        assert!(rows.iter().any(|row| row.rule_id == "MCM-KILLSWITCH.1"));
        assert!(rows.iter().any(|row| row.rule_id == "MCM-ECONOMIC.1"));
        assert!(rows.iter().any(|row| row.rule_id == "MCM-ROLLBACK.1"));
        assert!(rows.iter().any(|row| row.rule_id == "CYBER-FRONTMATTER.1"));
        assert!(rows.iter().any(|row| row.rule_id == "REQ-TESTCAT-SEVEN.1"));
        assert!(rows.iter().any(|row| row.rule_id == "REQ-TESTCAT-MAP.1"));
        assert!(rows
            .iter()
            .any(|row| row.rule_id == "SEC-TEST-ASSERTS-REJECTION.1"));
        assert!(rows
            .iter()
            .any(|row| row.rule_id == "SEC-TEST-MOCKS-MONEY-LOGIC.1"));
        assert!(rows
            .iter()
            .any(|row| row.rule_id == "SEC-TEST-REPRODUCIBLE-SEED.1"));
        assert!(rows
            .iter()
            .any(|row| row.rule_id == "SEC-TEST-GLOBAL-MUTATION.1"));
        assert!(rows
            .iter()
            .any(|row| row.rule_id == "SEC-TEST-PASS-IF-DELETED.1"));
        assert!(rows
            .iter()
            .any(|row| row.rule_id == "SEC-TEST-NO-CRASH-ONLY.1"));
        assert!(rows
            .iter()
            .any(|row| row.rule_id == "SEC-TEST-SNAPSHOT-ONLY.1"));
        assert!(rows
            .iter()
            .any(|row| row.rule_id == "SEC-TEST-THREAT-QUALITY-SCORE.1"));
        assert!(rows
            .iter()
            .any(|row| row.rule_id == "SEC-TEST-PROTECTION-REMOVED-HEURISTIC.1"));
        Ok(())
    }
}
