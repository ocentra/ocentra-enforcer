//! `CYBER-COMPLIANCE-MANIFEST.01` - CP09 supplied compliance capability.
//!
//! BOUNDARY-INVARIANT: the validator checks only caller-supplied offline
//! control, risk, authorization, privacy, and safeguard manifests. It does
//! not calculate a regulatory finding from a live system or contact a
//! framework owner, assessor, regulator, GRC service, or production system.
// NEGATIVE-TEST: crates/enforcer-lang-security/tests/cyberskills_compliance_governance_manifest_b01.rs
// ROUNDTRIP-TEST: crates/enforcer-lang-security/tests/cyberskills_compliance_governance_manifest_b01.rs

use std::collections::BTreeSet;

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::findings::Finding;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

use crate::boundary::compliance_governance_b01_manifest_wire::{
    parse, ControlWire, EvidenceWire, InformationTypeWire, ManifestWire, ProcessingActivityWire,
    RecordWire, RiskWire, SafeguardWire,
};

const RULE_ID: &str = "CYBER-COMPLIANCE-MANIFEST.01";
const CMMC_SKILL: &str = "achieving-cmmc-level-2-compliance";
const RISK_SKILL: &str = "conducting-cyber-risk-assessment-with-nist-800-30";
const RMF_SKILL: &str = "executing-nist-rmf-authorization-to-operate";
const GDPR_SKILL: &str = "implementing-gdpr-data-protection-controls";
const HIPAA_SKILL: &str = "implementing-hipaa-security-rule-safeguards";
const STATIC_SCOPE: &str = "scope:offline-authorized-static-only";

#[derive(Clone, Copy)]
struct SkillId(&'static str);

#[derive(Clone, Copy)]
struct ManifestText<'a>(&'a str);

#[derive(Clone, Copy)]
struct ReferenceText<'a>(&'a str);

#[derive(Clone, Copy)]
struct RiskText<'a>(&'a str);

#[derive(Clone, Copy)]
struct ImpactText<'a>(&'a str);

#[derive(Clone, Copy)]
struct TextList<'a>(&'a [String]);

#[derive(Clone, Copy)]
struct ControlWeight(u8);

#[derive(Clone, Copy)]
struct ImpactRank(usize);

#[derive(Clone, Copy, PartialEq, Eq)]
struct RiskLabel(&'static str);

#[derive(Clone, Copy)]
struct Predicate(bool);

#[derive(Clone, Copy)]
struct Score(i32);

#[derive(Clone, Copy)]
struct RmfCategory(&'static str);

const RISK_LEVELS: [RiskLabel; 5] = [
    RiskLabel("Very Low"),
    RiskLabel("Low"),
    RiskLabel("Moderate"),
    RiskLabel("High"),
    RiskLabel("Very High"),
];
const RISK_MATRIX: [[RiskLabel; 5]; 5] = [
    [
        RiskLabel("Very Low"),
        RiskLabel("Very Low"),
        RiskLabel("Very Low"),
        RiskLabel("Low"),
        RiskLabel("Low"),
    ],
    [
        RiskLabel("Very Low"),
        RiskLabel("Low"),
        RiskLabel("Low"),
        RiskLabel("Low"),
        RiskLabel("Moderate"),
    ],
    [
        RiskLabel("Very Low"),
        RiskLabel("Low"),
        RiskLabel("Moderate"),
        RiskLabel("Moderate"),
        RiskLabel("High"),
    ],
    [
        RiskLabel("Very Low"),
        RiskLabel("Low"),
        RiskLabel("Moderate"),
        RiskLabel("High"),
        RiskLabel("Very High"),
    ],
    [
        RiskLabel("Very Low"),
        RiskLabel("Low"),
        RiskLabel("Moderate"),
        RiskLabel("High"),
        RiskLabel("Very High"),
    ],
];

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

fn control_deduction(control: &ControlWire) -> Option<Score> {
    let weight = control.weight.map(ControlWeight);
    let weighted = weight.is_some_and(|value| matches!(value.0, 1 | 3 | 5));
    let partial_valid = weight
        .zip(control.partial_deduction)
        .is_some_and(|(value, deduction)| deduction <= value.0);
    let not_met_score = weight.map(|value| Score(i32::from(value.0)));
    let partial_score = control
        .partial_deduction
        .map(|deduction| Score(i32::from(deduction)));
    [
        (
            control.status == "na" && weight.is_none() && control.partial_deduction.is_none(),
            Some(Score(0)),
        ),
        (
            control.status == "met" && weighted && control.partial_deduction.is_none(),
            Some(Score(0)),
        ),
        (
            control.status == "not_met" && weighted && control.partial_deduction.is_none(),
            not_met_score,
        ),
        (
            control.status == "partial" && weighted && partial_valid,
            partial_score,
        ),
    ]
    .into_iter()
    .find_map(|(valid, value)| valid.then_some(value))
    .flatten()
}

fn cmmc_score(controls: &[ControlWire]) -> Option<Score> {
    let score = (!controls.is_empty())
        .then(|| {
            let mut ids = BTreeSet::new();
            controls.iter().try_fold(110, |score, control| {
                let valid_identity = text_is_present(ManifestText(&control.id)).0
                    && text_is_present(ManifestText(&control.family)).0
                    && ids.insert(control.id.as_str());
                valid_identity
                    .then(|| control_deduction(control).map(|deduction| score - deduction.0))
                    .flatten()
            })
        })
        .flatten()?;
    (score >= 0).then_some(Score(score))
}

fn risk_level(likelihood: RiskText<'_>, impact: RiskText<'_>) -> Option<RiskLabel> {
    let likelihood = RISK_LEVELS
        .iter()
        .position(|value| value.0 == likelihood.0)?;
    let impact = RISK_LEVELS.iter().position(|value| value.0 == impact.0)?;
    RISK_MATRIX.get(likelihood)?.get(impact).copied()
}

fn valid_risk(risk: &RiskWire) -> Predicate {
    Predicate(
        text_is_present(ManifestText(&risk.id)).0
            && text_is_present(ManifestText(&risk.threat_event)).0
            && text_is_present(ManifestText(&risk.asset)).0
            && risk_level(RiskText(&risk.likelihood), RiskText(&risk.impact))
                .is_some_and(|level| level.0 == risk.risk_level),
    )
}

fn impact_rank(value: ImpactText<'_>) -> Option<ImpactRank> {
    ["Low", "Moderate", "High"]
        .iter()
        .position(|candidate| *candidate == value.0)
        .map(ImpactRank)
}

fn rmf_categorization(information_types: &[InformationTypeWire]) -> Option<RmfCategory> {
    let valid = information_types.iter().all(|information| {
        text_is_present(ManifestText(&information.name)).0
            && impact_rank(ImpactText(&information.confidentiality)).is_some()
            && impact_rank(ImpactText(&information.integrity)).is_some()
            && impact_rank(ImpactText(&information.availability)).is_some()
    });
    let highest = information_types
        .iter()
        .flat_map(|information| {
            [
                impact_rank(ImpactText(&information.confidentiality)),
                impact_rank(ImpactText(&information.integrity)),
                impact_rank(ImpactText(&information.availability)),
            ]
        })
        .flatten()
        .max_by_key(|rank| rank.0)?;
    valid
        .then(|| ["Low", "Moderate", "High"].get(highest.0).copied())
        .flatten()
        .map(RmfCategory)
}

fn valid_list(values: TextList<'_>) -> Predicate {
    Predicate(
        !values.0.is_empty()
            && values
                .0
                .iter()
                .all(|value| text_is_present(ManifestText(value)).0),
    )
}

fn valid_processing_activity(activity: &ProcessingActivityWire) -> Predicate {
    Predicate(
        text_is_present(ManifestText(&activity.activity_id)).0
            && text_is_present(ManifestText(&activity.purpose)).0
            && text_is_present(ManifestText(&activity.lawful_basis)).0
            && valid_list(TextList(&activity.data_categories)).0
            && valid_list(TextList(&activity.data_subjects)).0
            && valid_list(TextList(&activity.recipients)).0
            && text_is_present(ManifestText(&activity.retention_period)).0
            && valid_list(TextList(&activity.security_measures)).0
            && valid_list(TextList(&activity.international_transfers)).0,
    )
}

fn valid_gdpr(record: &RecordWire) -> Predicate {
    Predicate(
        record.controls.is_none()
            && record.score.is_none()
            && record.risk_items.is_none()
            && record.information_types.is_none()
            && record.categorization.is_none()
            && record.safeguards.is_none()
            && record
                .processing_activities
                .as_deref()
                .is_some_and(|activities| {
                    !activities.is_empty()
                        && activities
                            .iter()
                            .all(|activity| valid_processing_activity(activity).0)
                })
            && record
                .data_subject_requests
                .as_deref()
                .is_some_and(|requests| {
                    !requests.is_empty()
                        && requests.iter().all(|request| {
                            text_is_present(ManifestText(&request.id)).0
                                && text_is_present(ManifestText(&request.request_type)).0
                                && text_is_present(ManifestText(&request.received_date)).0
                                && text_is_present(ManifestText(&request.deadline)).0
                                && text_is_present(ManifestText(&request.status)).0
                        })
                })
            && record.breach_records.as_deref().is_some_and(|breaches| {
                !breaches.is_empty()
                    && breaches.iter().all(|breach| {
                        text_is_present(ManifestText(&breach.id)).0
                            && text_is_present(ManifestText(&breach.detected_at)).0
                            && text_is_present(ManifestText(&breach.severity)).0
                            && (!breach.authority_notified
                                || breach.notification_hours.is_some_and(|hours| hours <= 72))
                            && (!breach.subjects_notified || breach.subjects_affected > 0)
                    })
            }),
    )
}

fn valid_safeguard(safeguard: &SafeguardWire) -> Predicate {
    let status_valid = matches!(safeguard.status.as_str(), "implemented" | "partial" | "gap");
    let requirement_valid = matches!(safeguard.requirement.as_str(), "required" | "addressable");
    let addressable_gap_is_documented = safeguard.requirement != "addressable"
        || safeguard.status == "implemented"
        || safeguard.alternative_documented == Some(true);
    Predicate(
        text_is_present(ManifestText(&safeguard.id)).0
            && text_is_present(ManifestText(&safeguard.section)).0
            && text_is_present(ManifestText(&safeguard.name)).0
            && status_valid
            && requirement_valid
            && addressable_gap_is_documented,
    )
}

fn valid_record(record: &RecordWire) -> Predicate {
    let Some(expected) = [
        ("cmmc-controls", SkillId(CMMC_SKILL)),
        ("nist-800-30-risk", SkillId(RISK_SKILL)),
        ("nist-rmf-authorization", SkillId(RMF_SKILL)),
        ("gdpr-data-protection", SkillId(GDPR_SKILL)),
        ("hipaa-safeguards", SkillId(HIPAA_SKILL)),
    ]
    .into_iter()
    .find(|(kind, _)| *kind == record.kind.as_str())
    .map(|(_, skill)| skill) else {
        return Predicate(false);
    };
    let semantic = [
        (
            record.kind == "cmmc-controls",
            record
                .controls
                .as_deref()
                .and_then(cmmc_score)
                .is_some_and(|score| record.score == Some(score.0)),
        ),
        (
            record.kind == "nist-800-30-risk",
            record.risk_items.as_deref().is_some_and(|risks| {
                !risks.is_empty() && risks.iter().all(|risk| valid_risk(risk).0)
            }),
        ),
        (
            record.kind == "nist-rmf-authorization",
            record
                .information_types
                .as_deref()
                .and_then(rmf_categorization)
                .is_some_and(|category| record.categorization.as_deref() == Some(category.0)),
        ),
        (record.kind == "gdpr-data-protection", valid_gdpr(record).0),
        (
            record.kind == "hipaa-safeguards",
            record.safeguards.as_deref().is_some_and(|safeguards| {
                !safeguards.is_empty()
                    && safeguards
                        .iter()
                        .all(|safeguard| valid_safeguard(safeguard).0)
            }),
        ),
    ]
    .into_iter()
    .find(|(selected, _)| *selected)
    .map(|(_, valid)| valid)
    .unwrap_or(false);
    Predicate(
        record.skill_id.as_deref() == Some(expected.0)
            && !record.refs.is_empty()
            && record
                .refs
                .iter()
                .all(|reference| reference_is_valid(ReferenceText(reference)).0)
            && semantic,
    )
}

fn valid_manifest(manifest: &ManifestWire) -> Predicate {
    let expected: BTreeSet<&str> = [
        "cmmc-controls",
        "nist-800-30-risk",
        "nist-rmf-authorization",
        "gdpr-data-protection",
        "hipaa-safeguards",
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

/// Native validator for supplied B01 compliance-governance manifests.
#[derive(Debug)]
pub struct ComplianceGovernanceManifestB01Validator {
    rule_id: RuleId,
}

impl ComplianceGovernanceManifestB01Validator {
    /// Construct the deterministic B01 compliance-manifest validator.
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            // ALLOC-JUSTIFICATION: RuleId owns the canonical rule identity across findings.
            rule_id: RuleId::try_from(RULE_ID.to_owned())?,
        })
    }
}

impl Validator for ComplianceGovernanceManifestB01Validator {
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
            "Compliance-governance B01 manifest predicate failed",
            "The supplied manifest is malformed or missing a typed control, risk, authorization, privacy, or safeguard relationship. This is a static schema finding only; no framework, assessor, regulator, GRC, personal-data, healthcare, production, or compliance outcome was evaluated.",
            input.file,
            (1, input.source.as_str().lines().next()),
        )
        .into_iter()
        .collect()
    }
}
