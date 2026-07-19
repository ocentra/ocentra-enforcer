use enforcer_core::error::Result;
use enforcer_domain::ids::RuleId;
use enforcer_domain::paths::RelPath;
use enforcer_domain::proof_types::{ClaimViolationCode, ProofId, ProofRunId};
use enforcer_proof::boundary::read_model::ProjectProofSnapshotDto;
use enforcer_proof::boundary::read_model_claim::ProjectClaimSummaryDto;
use enforcer_proof::boundary::read_model_journal::ProjectJournalSummaryDto;
use enforcer_proof::boundary::read_model_run::{ProjectProofRunSummaryDto, ProjectRunArtifactsDto};
use enforcer_proof::claim::{AcceptedProofEnvelope, ClaimEnvelope, ClaimViolationEnvelope};
use enforcer_proof::envelope::{
    ArtifactRecordEnvelope, AttestationDigestEnvelope, AttestationEnvelope,
    AttestationPredicateEnvelope, AttestationSubjectEnvelope, ExportBundleEnvelope,
    ExportRunRowEnvelope, GitStateEnvelope, ProofRunEnvelope, RetentionPolicyEnvelope,
};
use enforcer_proof::harness::{
    ManifestRowEnvelope, ProofDefinitionEnvelope, ProofDiagnosticEnvelope, ProofRegistryEnvelope,
    RouteRequest,
};
use enforcer_proof::journal::JournalRecordEnvelope;
use enforcer_proof::legacy_import::{LegacyArtifactEnvelope, LegacyBundleEnvelope};
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::{json, Value};
use std::fmt::Debug;

fn assert_round_trip<T>(wire: Value) -> Result<()>
where
    T: Debug + PartialEq + Serialize + DeserializeOwned,
{
    let decoded: T = serde_json::from_value(wire)?;
    let encoded = serde_json::to_value(&decoded)?;
    let decoded_again: T = serde_json::from_value(encoded.clone())?;
    assert_eq!(decoded_again, decoded);
    assert_eq!(serde_json::to_value(decoded_again)?, encoded);
    Ok(())
}

#[test]
fn proof_external_dtos_round_trip_through_their_canonical_wire_shapes() -> Result<()> {
    let digest = format!("sha256:{}", "0".repeat(64));
    let git = json!({"commit":"abcdef0", "branch":"rust-build", "dirty":false});
    let artifact = json!({
        "name":"report.json", "path":"reports/report.json", "sha256":digest,
        "byteLength":42
    });
    let run = json!({
        "schemaVersion":1, "proofId":"PROOF-1", "runId":"run-1", "title":"Proof",
        "capability":"local", "git":git, "status":"passed", "exitCode":0,
        "startedAt":"2026-07-17T00:00:00Z", "endedAt":"2026-07-17T00:00:01Z",
        "command":["cargo","test"], "diagnosticCount":0, "pinned":false,
        "artifacts":[artifact], "claimsProved":["tests pass"], "claimsNotProved":[]
    });
    let violation = json!({
        "proofId":"PROOF-1", "code":"missing-artifact", "message":"missing report",
        "severity":"error"
    });
    let accepted = json!({
        "proofId":"PROOF-1", "runId":"run-1", "status":"passed", "commit":"abcdef0"
    });
    let claim = json!({
        "claimId":"claim-1", "prReady":false, "proofIds":["PROOF-1"],
        "currentGit":git, "accepted":[accepted], "violations":[violation]
    });
    let definition = json!({
        "id":"PROOF-1", "title":"Proof", "family":"command", "severity":"error",
        "appliesTo":["workspace"], "triggers":[], "languages":["rust"],
        "capabilities":["local"], "collector":"command", "docs":[],
        "commands":[["cargo","test"]], "requiredArtifacts":["report.json"],
        "requiredPaths":["Cargo.toml"], "requiredForPrReady":true,
        "claimsProved":["tests pass"], "claimsNotProved":[], "ciSupport":true,
        "deviceSupport":false
    });
    let diagnostic = json!({
        "runId":"run-1", "proofId":"PROOF-1", "severity":"error",
        "ruleId":"PROOF-1", "message":"failed", "file":"Cargo.toml", "line":1
    });
    let journal = json!({
        "schemaVersion":1, "eventType":"proof-finished", "runId":"run-1",
        "proofId":"PROOF-1", "timestamp":"2026-07-17T00:00:01Z",
        "payload":{"status":"passed"}
    });
    let legacy_artifact = json!({
        "path":"legacy/report.json", "sha256":digest, "byteLength":42, "status":"passed"
    });
    let attestation_digest = json!({"gitCommit":"abcdef0"});
    let attestation_subject = json!({"name":"PROOF-1", "digest":attestation_digest});
    let attestation_predicate = json!({
        "runId":"run-1", "status":"passed", "startedAt":"2026-07-17T00:00:00Z",
        "endedAt":"2026-07-17T00:00:01Z", "capability":"local"
    });
    let export_row = json!({
        "runId":"run-1", "proofId":"PROOF-1", "status":"passed",
        "startedAt":"2026-07-17T00:00:00Z", "endedAt":"2026-07-17T00:00:01Z",
        "commit":"abcdef0", "pinned":false
    });
    let artifacts = json!({"declared":1, "present":1, "missing":0, "totalBytes":42});
    let run_summary = json!({
        "path":".enforce/proofs/runs/run-1/proof-run.json", "proofRun":run,
        "freshness":"current", "artifacts":artifacts, "parseError":null
    });
    let journal_summary = json!({
        "path":".enforce/proofs/events.ndjson", "state":"verified", "recordCount":1,
        "latestEventType":"proof-finished", "latestProofId":"PROOF-1",
        "latestTimestamp":"2026-07-17T00:00:01Z", "error":null
    });
    let claim_summary = json!({
        "registryPath":"proofs.json", "state":"blocked", "requiredProofIds":["PROOF-1"],
        "claim":claim, "error":null
    });

    assert_round_trip::<GitStateEnvelope>(git.clone())?;
    assert_round_trip::<ArtifactRecordEnvelope>(artifact)?;
    assert_round_trip::<RetentionPolicyEnvelope>(json!({
        "maxRunsPerProof":20, "maxFailedRuns":20, "maxArtifactBytes":52428800,
        "pruneAfterDays":14, "pinPrReadyDays":30
    }))?;
    assert_round_trip::<ProofRunEnvelope>(run.clone())?;
    let proof_run: ProofRunEnvelope = serde_json::from_value(run)?;
    let extracted_proof_id: ProofId = proof_run.into();
    assert_eq!(extracted_proof_id.as_str(), "PROOF-1");
    assert_round_trip::<AttestationEnvelope>(json!({
        "_type":"https://in-toto.io/Statement/v1", "subject":[attestation_subject],
        "predicateType":"https://ocentra.dev/attestations/proof-run/v1",
        "predicate":attestation_predicate
    }))?;
    assert_round_trip::<AttestationSubjectEnvelope>(attestation_subject)?;
    assert_round_trip::<AttestationDigestEnvelope>(attestation_digest)?;
    assert_round_trip::<AttestationPredicateEnvelope>(attestation_predicate)?;
    assert_round_trip::<ExportRunRowEnvelope>(export_row.clone())?;
    assert_round_trip::<ExportBundleEnvelope>(json!({
        "schemaVersion":1, "generatedAt":"2026-07-17T00:00:02Z",
        "runs":[export_row], "note":"manifest only"
    }))?;
    assert_round_trip::<ClaimViolationEnvelope>(violation.clone())?;
    let violation: ClaimViolationEnvelope = serde_json::from_value(violation)?;
    let extracted_code: ClaimViolationCode = violation.into();
    assert_eq!(extracted_code, ClaimViolationCode::MissingArtifact);
    assert_round_trip::<AcceptedProofEnvelope>(accepted)?;
    assert_round_trip::<ClaimEnvelope>(claim)?;
    assert_round_trip::<JournalRecordEnvelope>(journal)?;
    assert_round_trip::<LegacyArtifactEnvelope>(legacy_artifact.clone())?;
    let legacy_artifact: LegacyArtifactEnvelope = serde_json::from_value(legacy_artifact)?;
    let extracted_path: RelPath = legacy_artifact.clone().into();
    assert_eq!(extracted_path.as_str(), "legacy/report.json");
    assert_round_trip::<LegacyBundleEnvelope>(json!({
        "artifacts":[legacy_artifact], "failedArtifacts":[],
        "claimsProved":["tests pass"], "claimsNotProved":[]
    }))?;
    assert_round_trip::<ProofDefinitionEnvelope>(definition.clone())?;
    assert_round_trip::<ProofRegistryEnvelope>(json!({
        "schemaVersion":1, "productName":"Ocentra Enforcer", "proofs":[definition]
    }))?;
    assert_round_trip::<ProofDiagnosticEnvelope>(diagnostic.clone())?;
    let diagnostic: ProofDiagnosticEnvelope = serde_json::from_value(diagnostic)?;
    let extracted_rule_id: RuleId = diagnostic.into();
    assert_eq!(extracted_rule_id.as_str(), "PROOF-1");
    let manifest_wire = json!({
        "runId":"run-1", "proofId":"PROOF-1", "status":"passed",
        "startedAt":"2026-07-17T00:00:00Z", "pinned":false
    });
    assert_round_trip::<ManifestRowEnvelope>(manifest_wire.clone())?;
    let manifest: ManifestRowEnvelope = serde_json::from_value(manifest_wire)?;
    let extracted_run_id: ProofRunId = manifest.into();
    assert_eq!(extracted_run_id.as_str(), "run-1");
    let route = RouteRequest {
        proof_id: Some("PROOF-1".parse()?),
        files: vec![],
        plan: None,
        capability: None,
        scope: None,
    };
    let extracted_route_id: Option<ProofId> = route.into();
    assert_eq!(
        extracted_route_id.as_ref().map(ProofId::as_str),
        Some("PROOF-1")
    );
    assert_round_trip::<ProjectRunArtifactsDto>(artifacts)?;
    assert_round_trip::<ProjectProofRunSummaryDto>(run_summary.clone())?;
    assert_round_trip::<ProjectJournalSummaryDto>(journal_summary.clone())?;
    assert_round_trip::<ProjectClaimSummaryDto>(claim_summary.clone())?;
    assert_round_trip::<ProjectProofSnapshotDto>(json!({
        "proofRoot":".enforce/proofs", "currentGit":git, "journal":journal_summary,
        "runs":[run_summary], "claim":claim_summary
    }))?;
    Ok(())
}
