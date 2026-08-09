//! CP06 consumer conformance: CyberSkills declares security-engine demand
//! over the accepted UL07 typed harness boundary without adding a runner,
//! registry, result schema, or live external engine.
// sourceOwner: docs/plans/cyberskills-parity-plan/workpacks/cp06-external-engine-module.md
// generator: hand-authored CP06 consumer conformance source
// schemaHash: sha256:6544d45bb1e35f0ad3f6266d8f539a6cfd1595f7b47fae1c06eee8961f2e32b7

use std::path::{Path, PathBuf};

use enforcer_core::error::Error as CoreError;
use enforcer_domain::harness_types::{
    HarnessCommandArgument, HarnessExecutionLimits, HarnessRunId, HarnessStepVersion,
    HarnessToolAvailability, HarnessToolDecision, HarnessToolName, HarnessToolRequirement,
    HarnessToolSpec,
};
use enforcer_domain::paths::RepoRoot;
use enforcer_harness::adapters::cyberskills::recorded::parse_recorded;
use enforcer_harness::adapters::cyberskills::seam::AdapterOutcome;
use enforcer_harness::execution::{validate_allowlisted_request, ExecuteRequest};
use serde::{Deserialize, Serialize};

const CONTRACT_FIXTURE: &str =
    include_str!("fixtures/cp06_security_consumer/security-engine-contract.json");

// BOUNDARY-INVARIANT: this CP06 demand schema describes a consumer contract;
// it is not a second generic runner, registry, or normalized result schema.
// ROUNDTRIP-TEST: crates/enforcer-harness/tests/cp06_security_consumer_contract.rs
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SecurityEngineConsumerContract {
    schema_version: String,
    engine_id: String,
    target_kinds: Vec<String>,
    output_protocol: OutputProtocol,
    policy: SecurityEnginePolicy,
    required_evidence: Vec<String>,
    outcomes: Vec<String>,
    source_owner: String,
    generator: String,
    schema_hash: String,
    reproducibility_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OutputProtocol {
    name: String,
    version: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SecurityEnginePolicy {
    executable: String,
    argument_template: Vec<String>,
    working_directory: String,
    environment: String,
    network: String,
    credentials: String,
    timeout_ms: u64,
    max_output_bytes: u64,
    max_artifacts: u32,
}

fn contract() -> Result<SecurityEngineConsumerContract, serde_json::Error> {
    serde_json::from_str(CONTRACT_FIXTURE)
}

fn repo_root() -> Result<RepoRoot, Box<dyn std::error::Error>> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = manifest_dir.join("../..").canonicalize()?;
    Ok(RepoRoot::try_from(root.as_path())?)
}

fn reviewed_spec() -> Result<HarnessToolSpec, Box<dyn std::error::Error>> {
    let executable = HarnessCommandArgument::try_new("security-engine-reference".to_owned())?;
    let limits = HarnessExecutionLimits::try_new(1000, 8192, 4)?;
    let version = HarnessStepVersion::from_manifest("1")
        .ok_or("contract version must be a non-empty non-control value")?;
    Ok(HarnessToolSpec::try_new(
        HarnessToolName::from_adapter("security-engine-reference"),
        vec![executable],
        HarnessToolRequirement::Required,
        limits,
        Some(version),
    )?)
}

fn request_with_command(
    command: Vec<HarnessCommandArgument>,
) -> Result<ExecuteRequest, Box<dyn std::error::Error>> {
    Ok(ExecuteRequest {
        repo_root: repo_root()?,
        cwd: Some("crates/enforcer-harness".to_owned()),
        run_id: HarnessRunId::from_adapter("cp06-consumer-contract"),
        tool: HarnessToolName::from_adapter("security-engine-reference"),
        language: None,
        command,
        crate_name: None,
        package_name: None,
        domain: None,
        tags: Vec::new(),
    })
}

#[test]
fn contract_fixture_is_versioned_and_complete() -> Result<(), Box<dyn std::error::Error>> {
    let contract = contract()?;
    assert_eq!(
        contract.schema_version,
        "cyberskills.security-engine-consumer.v1"
    );
    assert_eq!(contract.engine_id, "security-engine-reference");
    assert_eq!(contract.target_kinds.len(), 3);
    assert_eq!(
        contract.output_protocol.name,
        "cyberskills-recorded-adapter"
    );
    assert_eq!(contract.output_protocol.version, "1");
    assert_eq!(contract.policy.network, "denied");
    assert_eq!(contract.policy.credentials, "denied");
    assert_eq!(contract.policy.environment, "allowlist-only");
    assert_eq!(contract.required_evidence.len(), 8);
    assert_eq!(contract.outcomes.len(), 6);
    assert_eq!(
        contract.source_owner,
        "docs/plans/cyberskills-parity-plan/workpacks/cp06-external-engine-module.md"
    );
    assert_eq!(
        contract.generator,
        "hand-authored CP06 consumer conformance source"
    );
    assert_eq!(
        contract.schema_hash,
        "sha256:6544d45bb1e35f0ad3f6266d8f539a6cfd1595f7b47fae1c06eee8961f2e32b7"
    );
    assert_eq!(contract.reproducibility_hash, contract.schema_hash);
    Ok(())
}

#[test]
fn contract_fixture_roundtrips_without_schema_drift() -> Result<(), Box<dyn std::error::Error>> {
    let original = contract()?;
    let wire = serde_json::to_vec(&original)?;
    let decoded: SecurityEngineConsumerContract = serde_json::from_slice(&wire)?;
    assert_eq!(decoded, original);
    Ok(())
}

#[test]
fn required_consumer_blocks_every_non_running_availability() {
    let unavailable = [
        HarnessToolAvailability::Missing,
        HarnessToolAvailability::VersionMismatch,
        HarnessToolAvailability::Misconfigured,
        HarnessToolAvailability::TimedOut,
        HarnessToolAvailability::Failed,
        HarnessToolAvailability::MalformedOutput,
    ];
    for availability in unavailable {
        assert_eq!(
            availability.decision(HarnessToolRequirement::Required),
            HarnessToolDecision::Block,
            "{availability:?} must not satisfy a required security-engine component"
        );
    }
    assert_eq!(
        HarnessToolAvailability::Available.decision(HarnessToolRequirement::Required),
        HarnessToolDecision::Run
    );
    assert_eq!(
        HarnessToolAvailability::Missing.decision(HarnessToolRequirement::Optional),
        HarnessToolDecision::Warn
    );
    assert_eq!(
        HarnessToolAvailability::Missing.decision(HarnessToolRequirement::Advisory),
        HarnessToolDecision::NotApplicable
    );
}

#[test]
fn policy_rejection_stays_outside_engine_success() -> Result<(), Box<dyn std::error::Error>> {
    let spec = reviewed_spec()?;
    let wrong_command = vec![HarnessCommandArgument::try_new(
        "unreviewed-command".to_owned(),
    )?];
    let request = request_with_command(wrong_command)?;
    assert!(matches!(
        validate_allowlisted_request(&request, &spec),
        Err(CoreError::InvalidConfig(message))
            if message == "allowlisted command does not match the reviewed template"
    ));
    Ok(())
}

#[test]
fn recorded_run_is_the_only_consumer_success_shape() -> Result<(), Box<dyn std::error::Error>> {
    let ran = parse_recorded(r#"{"toolPresent":true,"outcome":"ran","ran":1,"findings":[]}"#)?;
    assert!(matches!(ran, AdapterOutcome::Ran { ran: 1, .. }));

    let unavailable =
        parse_recorded(r#"{"toolPresent":false,"outcome":"skipped","ran":0,"findings":[]}"#)?;
    assert!(matches!(unavailable, AdapterOutcome::Skipped { ran: 0 }));

    let malformed =
        parse_recorded(r#"{"toolPresent":true,"outcome":"ran","ran":1,"findings":"not-an-array"}"#);
    assert!(matches!(
        malformed,
        Err(CoreError::Decode(error)) if error.path == "cyberskillsAdapter"
    ));

    let dishonest_absence =
        parse_recorded(r#"{"toolPresent":false,"outcome":"pass","ran":0,"findings":[]}"#);
    assert!(matches!(
        dishonest_absence,
        Err(CoreError::Decode(error)) if error.path == "cyberskillsAdapter.outcome"
    ));
    Ok(())
}

#[test]
fn contract_does_not_execute_external_engine() -> Result<(), Box<dyn std::error::Error>> {
    let contract = contract()?;
    assert_eq!(contract.policy.network, "denied");
    assert_eq!(contract.policy.credentials, "denied");
    assert_eq!(contract.policy.working_directory, "repo-relative");
    assert_eq!(contract.policy.argument_template, ["--input", "{target}"]);
    let _ = Path::new("crates/enforcer-harness");
    Ok(())
}
