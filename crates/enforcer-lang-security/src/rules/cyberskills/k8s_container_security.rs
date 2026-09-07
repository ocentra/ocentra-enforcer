//! `CYBER-K8S-CONTAINER.1` — supplied-input container-security predicates.
//!
//! This validator covers only deterministic facts in a caller-provided JSON
//! evidence envelope: sensitive Kubernetes audit operations, approved versus
//! observed container drift, and pod configuration associated with escape
//! risk. It does not connect to Kubernetes, Falco, a registry, an image
//! store, a host, a runtime, or any production authority.

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::findings::Finding;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

use crate::boundary::k8s_container_security_manifest::{
    audit_events, audit_reason, drift_fields, escape_indicators, parse, AuditEvent,
    ContainerSecurityManifest, ContainerSnapshot, PodSnapshot,
};

/// Validates static Kubernetes/container-security evidence supplied as JSON.
#[derive(Debug)]
pub struct K8sContainerSecurityValidator {
    rule_id: RuleId,
}

impl K8sContainerSecurityValidator {
    /// Construct the validator with its packet-local, validated rule identity.
    pub fn new() -> Result<Self, DecodeError> {
        Ok(Self {
            // ALLOC-JUSTIFICATION: the validated rule identity owns its boundary text.
            rule_id: RuleId::try_from(String::from("CYBER-K8S-CONTAINER.1"))?,
        })
    }
}

#[derive(Debug)]
struct Observation {
    // BRAND-INVARIANT: a finding location is always a one-based supplied-evidence line.
    line: u32,
    severity: Severity,
    // BRAND-INVARIANT: detail is the validator-owned explanation of one static fact.
    detail: String,
}

fn schema_observation(manifest: &ContainerSecurityManifest) -> Option<Observation> {
    (manifest.schema_version != 1).then(|| Observation {
        line: 1,
        severity: Severity::Error,
        // ALLOC-JUSTIFICATION: the schema value is owned by the finding detail.
        detail: format!(
            "unsupported container-security evidence schemaVersion `{}`; expected `1`",
            manifest.schema_version
        ),
    })
}

fn audit_observations(events: &[AuditEvent]) -> Vec<Observation> {
    events
        .iter()
        .enumerate()
        .filter_map(|(index, event)| {
            audit_reason(event).map(|reason| {
                let request_uri = if event.request_uri.is_empty() {
                    "no request URI"
                } else {
                    event.request_uri.as_str()
                };
                // CAST-JUSTIFICATION: fixture line numbers are bounded by the supplied event count.
                let line = index as u32 + 1;
                // ALLOC-JUSTIFICATION: the finding detail must own supplied event context.
                let detail = format!(
                    "supplied audit evidence records a {reason} for `{}` by `{}` in `{}` ({request_uri}); this is a static classification, not live authorization or compromise proof",
                    event.resource, event.user, event.namespace
                );
                Observation {
                    line,
                    severity: Severity::Warning,
                    detail,
                }
            })
        })
        .collect()
}

fn drift_observations(manifest: &ContainerSecurityManifest) -> Vec<Observation> {
    let observed = manifest
        .observed_containers
        .iter()
        .filter_map(|observed| observed_drift_observation(manifest, observed));
    let missing = manifest
        .approved_containers
        .iter()
        .filter(|approved| {
            !manifest
                .observed_containers
                .iter()
                .any(|candidate| candidate.name == approved.name)
        })
        .map(|approved| {
            // ALLOC-JUSTIFICATION: the finding detail must own the snapshot name.
            Observation {
                line: 1,
                severity: Severity::Warning,
                detail: format!(
                    "approved container `{}` is absent from the supplied observed snapshot; absence is not a runtime disappearance claim",
                    approved.name
                ),
            }
        });
    observed.chain(missing).collect()
}

fn observed_drift_observation(
    manifest: &ContainerSecurityManifest,
    observed: &ContainerSnapshot,
) -> Option<Observation> {
    let approved = manifest
        .approved_containers
        .iter()
        .find(|candidate| candidate.name == observed.name);
    match approved {
        None => {
            // ALLOC-JUSTIFICATION: the finding detail must own the snapshot name.
            Some(Observation {
                line: 1,
                severity: Severity::Warning,
                detail: format!(
                    "observed container `{}` has no matching supplied approved snapshot; drift cannot be resolved from this evidence",
                    observed.name
                ),
            })
        }
        Some(approved) => changed_drift_observation(approved, observed),
    }
}

fn changed_drift_observation(
    approved: &ContainerSnapshot,
    observed: &ContainerSnapshot,
) -> Option<Observation> {
    let fields = drift_fields(approved, observed);
    (!fields.is_empty()).then(|| {
        // ALLOC-JUSTIFICATION: the finding detail must own changed-field names.
        Observation {
            line: 1,
            severity: Severity::Error,
            detail: format!(
                "supplied container snapshot `{}` differs from its approved snapshot in: {}; this is a static comparison, not runtime detection",
                observed.name,
                fields.join(", ")
            ),
        }
    })
}

fn escape_observations(manifest: &ContainerSecurityManifest) -> Vec<Observation> {
    manifest
        .pod_snapshots
        .iter()
        .filter_map(|snapshot: &PodSnapshot| {
            let indicators = escape_indicators(snapshot);
            (!indicators.is_empty()).then(|| {
                // ALLOC-JUSTIFICATION: the finding detail must own supplied snapshot context.
                Observation {
                    line: 1,
                    severity: Severity::Error,
                    detail: format!(
                        "supplied pod snapshot `{}` contains escape-risk configuration: {}; this does not prove an escape occurred",
                        snapshot.name,
                        indicators.join(", ")
                    ),
                }
            })
        })
        .collect()
}

impl Validator for K8sContainerSecurityValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        let malformed = |detail: &str| {
            // ALLOC-JUSTIFICATION: malformed input detail is owned by the emitted finding.
            crate::boundary::finding::from_source(
                (&self.rule_id, Severity::Error),
                "Supplied container-security evidence is malformed",
                format!(
                    "The static evidence envelope could not be decoded: {detail}. No live system was queried."
                ),
                input.file,
                (1, input.source.as_str().lines().next()),
            )
            .into_iter()
            .collect::<Vec<Finding>>()
        };
        let manifest = match parse(input.source.as_str()) {
            Ok(manifest) => manifest,
            Err(error) => {
                // ALLOC-JUSTIFICATION: the decode error text is copied into the finding boundary.
                let detail = error.to_string();
                return malformed(detail.as_str());
            }
        };
        let Some(emit) = crate::boundary::finding::ValidationFindingFactory::new(
            &self.rule_id,
            "Supplied container-security evidence contains a risky fact",
        ) else {
            return Vec::new();
        };
        let events = match audit_events(&manifest) {
            Ok(events) => events,
            Err(error) => {
                // ALLOC-JUSTIFICATION: the audit decode error text is copied into the finding boundary.
                let detail = error.to_string();
                return malformed(detail.as_str());
            }
        };
        schema_observation(&manifest)
            .into_iter()
            .chain(audit_observations(&events))
            .chain(drift_observations(&manifest))
            .chain(escape_observations(&manifest))
            .filter_map(|observation| {
                emit.finding(
                    &input,
                    observation.line,
                    observation.severity,
                    observation.detail,
                )
            })
            .collect()
    }
}
