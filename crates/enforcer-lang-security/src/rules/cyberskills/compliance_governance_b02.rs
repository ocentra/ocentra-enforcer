//! `CYBER-COMPLIANCE-MANIFEST.02` - CP09 supplied compliance capability.
//!
//! BOUNDARY-INVARIANT: the validator checks only caller-supplied offline
//! control, documentation, vendor-risk, maturity, and audit-evidence
//! manifests. It never connects to a framework owner, assessor, vendor,
//! cloud provider, endpoint, payment system, or production service.
// NEGATIVE-TEST: crates/enforcer-lang-security/tests/cyberskills_compliance_governance_manifest_b02.rs
// ROUNDTRIP-TEST: crates/enforcer-lang-security/tests/cyberskills_compliance_governance_manifest_b02.rs

use std::collections::BTreeSet;

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::findings::Finding;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

use crate::boundary::compliance_governance_b02_manifest_wire::{
    ControlWire, DocumentWire, EvidenceItemWire, EvidenceWire, ManifestWire, MaturityWire,
    ReadinessWire, RecordWire, RiskWire, VendorProfileWire, parse,
};

const RULE_ID: &str = "CYBER-COMPLIANCE-MANIFEST.02";
const ISO_SKILL: &str = "implementing-iso-27001-information-security-management";
const PCI_SKILL: &str = "implementing-pci-dss-compliance-controls";
const VENDOR_SKILL: &str = "managing-third-party-vendor-risk";
const NIST_SKILL: &str = "performing-nist-csf-maturity-assessment";
const SOC2_SKILL: &str = "performing-soc2-type2-audit-preparation";
const STATIC_SCOPE: &str = "scope:offline-authorized-static-only";
const SKILL_IDS: [(&str, &str); 5] = [
    ("iso-27001", ISO_SKILL),
    ("pci-dss", PCI_SKILL),
    ("vendor-risk", VENDOR_SKILL),
    ("nist-csf", NIST_SKILL),
    ("soc2-type2", SOC2_SKILL),
];

#[derive(Clone, Copy)]
struct ManifestText<'a>(&'a str);

#[derive(Clone, Copy)]
struct ReferenceText<'a>(&'a str);

#[derive(Clone, Copy)]
struct Predicate(bool);

#[derive(Clone, Copy)]
struct VendorRiskScore(u8);

#[derive(Clone, Copy, PartialEq, Eq)]
enum VendorTier {
    Critical,
    High,
    Moderate,
    Low,
}

fn text_is_present(value: ManifestText<'_>) -> Predicate {
    let value = value.0;
    Predicate(!value.trim().is_empty() && value.len() <= 512 && !value.contains('\0'))
}

fn reference_is_valid(value: ReferenceText<'_>) -> Predicate {
    let value = value.0;
    Predicate(value.split_once(':').is_some_and(|(kind, identifier)| {
        text_is_present(ManifestText(kind)).0
            && text_is_present(ManifestText(identifier)).0
            && !value.chars().any(char::is_whitespace)
    }))
}

fn evidence_is_valid(evidence: &[EvidenceWire]) -> Predicate {
    let mut seen = BTreeSet::new();
    Predicate(
        !evidence.is_empty()
            && evidence.iter().all(|entry| {
                text_is_present(ManifestText(&entry.kind)).0
                    && reference_is_valid(ReferenceText(&entry.reference)).0
                    && seen.insert(format!("{}:{}", entry.kind, entry.reference))
            }),
    )
}

fn control_is_valid(control: &ControlWire) -> Predicate {
    let status_valid = matches!(
        control.status.as_str(),
        "implemented" | "partial" | "not_implemented" | "excluded"
    );
    let evidence_valid = control
        .evidence_reference
        .as_deref()
        .is_some_and(|reference| reference_is_valid(ReferenceText(reference)).0);
    let evidence_required = control.status != "implemented" || evidence_valid;
    let exclusion_documented = control.status != "excluded"
        || control
            .justification
            .as_deref()
            .is_some_and(|justification| text_is_present(ManifestText(justification)).0);
    Predicate(
        text_is_present(ManifestText(&control.id)).0
            && text_is_present(ManifestText(&control.family)).0
            && text_is_present(ManifestText(&control.requirement)).0
            && status_valid
            && evidence_required
            && exclusion_documented,
    )
}

fn documents_are_valid(documents: &[DocumentWire]) -> Predicate {
    let mut seen = BTreeSet::new();
    Predicate(
        !documents.is_empty()
            && documents.iter().all(|document| {
                let status_valid = matches!(
                    document.status.as_str(),
                    "present" | "missing" | "outdated" | "not_applicable"
                );
                let review_valid = document
                    .last_reviewed
                    .as_deref()
                    .is_none_or(|date| text_is_present(ManifestText(date)).0);
                text_is_present(ManifestText(&document.id)).0
                    && text_is_present(ManifestText(&document.title)).0
                    && status_valid
                    && review_valid
                    && seen.insert(document.id.as_str())
            }),
    )
}

fn risks_are_valid(risks: &[RiskWire]) -> Predicate {
    let mut seen = BTreeSet::new();
    Predicate(
        !risks.is_empty()
            && risks.iter().all(|risk| {
                let level_valid = matches!(
                    risk.level.as_str(),
                    "critical" | "high" | "moderate" | "low"
                );
                let treatment_valid = matches!(
                    risk.treatment.as_str(),
                    "mitigate" | "transfer" | "avoid" | "accept"
                );
                text_is_present(ManifestText(&risk.id)).0
                    && text_is_present(ManifestText(&risk.owner)).0
                    && level_valid
                    && treatment_valid
                    && seen.insert(risk.id.as_str())
            }),
    )
}

fn iso_is_valid(record: &RecordWire) -> Predicate {
    Predicate(
        record.controls.as_deref().is_some_and(|controls| {
            !controls.is_empty() && controls.iter().all(|c| control_is_valid(c).0)
        }) && record
            .documents
            .as_deref()
            .is_some_and(|documents| documents_are_valid(documents).0)
            && record
                .risks
                .as_deref()
                .is_some_and(|risks| risks_are_valid(risks).0)
            && record.vendor_profiles.is_none()
            && record.maturity_items.is_none()
            && record.evidence_items.is_none()
            && record.readiness.is_none(),
    )
}

fn pci_is_valid(record: &RecordWire) -> Predicate {
    Predicate(
        record.controls.as_deref().is_some_and(|controls| {
            !controls.is_empty() && controls.iter().all(|c| control_is_valid(c).0)
        }) && record
            .evidence_items
            .as_deref()
            .is_some_and(|items| evidence_items_are_valid(items).0)
            && record.documents.is_none()
            && record.risks.is_none()
            && record.vendor_profiles.is_none()
            && record.maturity_items.is_none()
            && record.readiness.is_none(),
    )
}

fn vendor_points(profile: &VendorProfileWire) -> Option<VendorRiskScore> {
    let data = [
        ("regulated", 4),
        ("confidential", 3),
        ("internal", 1),
        ("public", 0),
    ]
    .into_iter()
    .find_map(|(value, points)| (profile.data_sensitivity == value).then_some(points));
    let access = [("system", 4), ("network", 3), ("physical", 2), ("none", 0)]
        .into_iter()
        .find_map(|(value, points)| (profile.access == value).then_some(points));
    let criticality = [("high", 4), ("medium", 2), ("low", 1)]
        .into_iter()
        .find_map(|(value, points)| (profile.criticality == value).then_some(points));
    let integration = [("deep", 2), ("moderate", 1), ("none", 0)]
        .into_iter()
        .find_map(|(value, points)| (profile.integration == value).then_some(points));
    Some(VendorRiskScore(
        data?
            + access?
            + criticality?
            + integration?
            + u8::from(profile.regulated_scope) * 2
            + u8::from(profile.concentration),
    ))
}

fn vendor_profiles_are_valid(profiles: &[VendorProfileWire]) -> Predicate {
    let mut seen = BTreeSet::new();
    Predicate(
        !profiles.is_empty()
            && profiles.iter().all(|profile| {
                let evidence_valid = !profile.evidence.is_empty()
                    && profile
                        .evidence
                        .iter()
                        .all(|reference| reference_is_valid(ReferenceText(reference)).0);
                let declared_tier = [
                    ("Critical", VendorTier::Critical),
                    ("High", VendorTier::High),
                    ("Moderate", VendorTier::Moderate),
                    ("Low", VendorTier::Low),
                ]
                .into_iter()
                .find_map(|(label, tier)| (profile.tier == label).then_some(tier));
                let score_valid =
                    vendor_points(profile)
                        .zip(declared_tier)
                        .is_some_and(|(score, tier)| {
                            let calculated_tier = [
                                (13, VendorTier::Critical),
                                (9, VendorTier::High),
                                (5, VendorTier::Moderate),
                                (0, VendorTier::Low),
                            ]
                            .into_iter()
                            .find_map(|(minimum, tier)| (score.0 >= minimum).then_some(tier))
                            .unwrap_or(VendorTier::Low);
                            score.0 == profile.risk_score && calculated_tier == tier
                        });
                text_is_present(ManifestText(&profile.vendor_id)).0
                    && evidence_valid
                    && score_valid
                    && seen.insert(profile.vendor_id.as_str())
            }),
    )
}

fn maturity_items_are_valid(items: &[MaturityWire]) -> Predicate {
    const CATEGORIES: [&str; 21] = [
        "ID.AM", "ID.BE", "ID.GV", "ID.RA", "ID.RM", "ID.SC", "PR.AC", "PR.AT", "PR.DS", "PR.IP",
        "PR.MA", "PR.PT", "DE.AE", "DE.CM", "DE.DP", "RS.RP", "RS.CO", "RS.AN", "RS.MI", "RS.IM",
        "RC.RP",
    ];
    let mut seen = BTreeSet::new();
    Predicate(
        !items.is_empty()
            && items.iter().all(|item| {
                let category_valid = CATEGORIES.contains(&item.category.as_str());
                let score_valid = (1..=4).contains(&item.score)
                    && (1..=4).contains(&item.target)
                    && item.target >= item.score;
                let evidence_valid = item
                    .evidence_reference
                    .as_deref()
                    .is_none_or(|reference| reference_is_valid(ReferenceText(reference)).0);
                category_valid
                    && score_valid
                    && evidence_valid
                    && seen.insert(item.category.as_str())
            }),
    )
}

fn evidence_items_are_valid(items: &[EvidenceItemWire]) -> Predicate {
    let mut seen = BTreeSet::new();
    Predicate(
        !items.is_empty()
            && items.iter().all(|item| {
                let status_valid =
                    matches!(item.status.as_str(), "collected" | "pending" | "missing");
                text_is_present(ManifestText(&item.id)).0
                    && text_is_present(ManifestText(&item.control_id)).0
                    && text_is_present(ManifestText(&item.period_start)).0
                    && text_is_present(ManifestText(&item.period_end)).0
                    && item.period_start <= item.period_end
                    && reference_is_valid(ReferenceText(&item.reference)).0
                    && status_valid
                    && seen.insert(item.id.as_str())
            }),
    )
}

fn readiness_is_valid(items: &[ReadinessWire]) -> Predicate {
    let mut seen = BTreeSet::new();
    Predicate(
        !items.is_empty()
            && items.iter().all(|item| {
                text_is_present(ManifestText(&item.id)).0
                    && text_is_present(ManifestText(&item.area)).0
                    && text_is_present(ManifestText(&item.owner)).0
                    && seen.insert(item.id.as_str())
            }),
    )
}

fn valid_record(record: &RecordWire) -> Predicate {
    let expected_id = SKILL_IDS
        .iter()
        .find_map(|(kind, skill)| (*kind == record.kind).then_some(*skill));
    let common = expected_id.is_some_and(|skill| record.skill_id.as_deref() == Some(skill))
        && !record.refs.is_empty()
        && record
            .refs
            .iter()
            .all(|reference| reference_is_valid(ReferenceText(reference)).0);
    let semantic = [
        ("iso-27001", iso_is_valid as fn(&RecordWire) -> Predicate),
        ("pci-dss", pci_is_valid as fn(&RecordWire) -> Predicate),
        (
            "vendor-risk",
            (|record: &RecordWire| {
                Predicate(
                    record
                        .vendor_profiles
                        .as_deref()
                        .is_some_and(|profiles| vendor_profiles_are_valid(profiles).0)
                        && record.controls.is_none()
                        && record.documents.is_none()
                        && record.risks.is_none()
                        && record.maturity_items.is_none()
                        && record.evidence_items.is_none()
                        && record.readiness.is_none(),
                )
            }) as fn(&RecordWire) -> Predicate,
        ),
        (
            "nist-csf",
            (|record: &RecordWire| {
                Predicate(
                    record
                        .maturity_items
                        .as_deref()
                        .is_some_and(|items| maturity_items_are_valid(items).0)
                        && record.controls.is_none()
                        && record.documents.is_none()
                        && record.risks.is_none()
                        && record.vendor_profiles.is_none()
                        && record.evidence_items.is_none()
                        && record.readiness.is_none(),
                )
            }) as fn(&RecordWire) -> Predicate,
        ),
        (
            "soc2-type2",
            (|record: &RecordWire| {
                Predicate(
                    record.controls.as_deref().is_some_and(|controls| {
                        !controls.is_empty() && controls.iter().all(|c| control_is_valid(c).0)
                    }) && record
                        .evidence_items
                        .as_deref()
                        .is_some_and(|items| evidence_items_are_valid(items).0)
                        && record
                            .readiness
                            .as_deref()
                            .is_some_and(|items| readiness_is_valid(items).0)
                        && record.documents.is_none()
                        && record.risks.is_none()
                        && record.vendor_profiles.is_none()
                        && record.maturity_items.is_none(),
                )
            }) as fn(&RecordWire) -> Predicate,
        ),
    ]
    .into_iter()
    .find_map(|(kind, validator)| (kind == record.kind.as_str()).then_some(validator(record)))
    .unwrap_or(Predicate(false));
    Predicate(common && semantic.0)
}

fn valid_manifest(manifest: &ManifestWire) -> Predicate {
    let expected: BTreeSet<&str> = [
        "iso-27001",
        "pci-dss",
        "vendor-risk",
        "nist-csf",
        "soc2-type2",
    ]
    .into_iter()
    .collect();
    let actual: BTreeSet<&str> = manifest
        .records
        .iter()
        .map(|record| record.kind.as_str())
        .collect();
    Predicate(
        manifest.schema_version == 1
            && text_is_present(ManifestText(&manifest.bundle_id)).0
            && text_is_present(ManifestText(&manifest.owner)).0
            && manifest.scope == STATIC_SCOPE
            && evidence_is_valid(&manifest.evidence).0
            && actual == expected
            && manifest.records.len() == expected.len()
            && manifest.records.iter().all(|record| valid_record(record).0),
    )
}

/// Native validator for supplied B02 compliance-governance manifests.
#[derive(Debug)]
pub struct ComplianceGovernanceManifestB02Validator {
    rule_id: RuleId,
}

impl ComplianceGovernanceManifestB02Validator {
    /// Construct the deterministic B02 compliance-manifest validator.
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            // ALLOC-JUSTIFICATION: RuleId owns the canonical rule identity across findings.
            rule_id: RuleId::try_from(RULE_ID.to_owned())?,
        })
    }
}

impl Validator for ComplianceGovernanceManifestB02Validator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let valid = parse(input.source.as_str()).is_ok_and(|manifest| valid_manifest(&manifest).0);
        if valid {
            return Vec::new();
        }
        crate::boundary::finding::from_source(
            (&self.rule_id, Severity::Error),
            "Compliance-governance B02 manifest predicate failed",
            "The supplied manifest is malformed or missing a typed framework, vendor-risk, maturity, control, or evidence relationship. This is a static schema finding only; no framework, auditor, vendor, cloud, endpoint, payment, production, or compliance outcome was evaluated.",
            input.file,
            (1, input.source.as_str().lines().next()),
        )
        .into_iter()
        .collect()
    }
}
