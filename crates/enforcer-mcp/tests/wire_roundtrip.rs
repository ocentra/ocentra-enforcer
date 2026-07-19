use enforcer_domain::boundary::core::measured_surface;
use enforcer_domain::mcp_types::{
    ArtifactEntry, ArtifactPath, ArtifactState, ChangedArtifact, McpFingerprint, McpFreshness,
    McpToolName, RpcErrorBody, RpcErrorCode, RpcErrorMessage, StalenessReport,
};
use enforcer_mcp::boundary::fingerprint::{ArtifactSlotDto, ChangedArtifactDto, McpFingerprintDto};
use enforcer_mcp::boundary::fingerprint_artifact::{ArtifactEntryDto, ArtifactStateDto};
use enforcer_mcp::boundary::rpc_request::RpcMessageDto;
use enforcer_mcp::boundary::rpc_response::{
    RpcErrorBodyDto, RpcErrorDto, RpcErrorResponse, RpcResultDto,
};
use enforcer_mcp::boundary::staleness_report::{StalenessDto, StalenessReportDto};
use enforcer_mcp::boundary::surface_measurement::SurfaceMeasurementDto;
use enforcer_mcp::boundary::tool_descriptor::ToolDescriptorDto;
use enforcer_mcp::gate::stale_fallback;
use serde::{de::DeserializeOwned, Serialize};
use std::path::Path;

fn round_trip<T>(value: &T) -> Result<T, serde_json::Error>
where
    T: Serialize + DeserializeOwned,
{
    serde_json::from_str(&serde_json::to_string(value)?)
}

#[test]
fn artifact_entry_dto_round_trip_preserves_present_state() -> Result<(), serde_json::Error> {
    let value = ArtifactEntryDto {
        path: "target/enforcer".to_owned(),
        state: ArtifactStateDto::Present {
            sha256: "0".repeat(64),
            byte_length: 19,
        },
    };
    assert_eq!(round_trip(&value)?, value);
    Ok(())
}

#[test]
fn changed_artifact_dto_round_trip_preserves_named_slot() -> Result<(), serde_json::Error> {
    let value = ChangedArtifactDto {
        slot: ArtifactSlotDto::Ruleset,
        startup: None,
        current: Some(ArtifactEntryDto {
            path: "rules/index.json".to_owned(),
            state: ArtifactStateDto::Missing,
        }),
    };
    assert_eq!(round_trip(&value)?, value);
    Ok(())
}

#[test]
fn mcp_fingerprint_dto_round_trip_preserves_artifacts() -> Result<(), serde_json::Error> {
    let value = McpFingerprintDto {
        digest: "1".repeat(64),
        package_version: "0.1.0".to_owned(),
        binary: ArtifactEntryDto {
            path: "target/enforcer".to_owned(),
            state: ArtifactStateDto::Missing,
        },
        ruleset: None,
    };
    assert_eq!(round_trip(&value)?, value);
    Ok(())
}

#[test]
fn staleness_report_dto_round_trip_preserves_change_list() -> Result<(), serde_json::Error> {
    let value = StalenessReportDto {
        verdict: StalenessDto::Stale,
        startup_digest: "2".repeat(64),
        current_digest: "3".repeat(64),
        changed: Vec::new(),
    };
    assert_eq!(round_trip(&value)?, value);
    Ok(())
}

#[test]
fn rpc_message_dto_round_trip_preserves_optional_params() -> Result<(), serde_json::Error> {
    let value: RpcMessageDto = serde_json::from_str(
        r#"{"id":7,"method":"tools/call","params":{"name":"ocentra_enforcer_mcp_status"}}"#,
    )?;
    let decoded = round_trip(&value)?;
    assert_eq!(decoded.method, value.method);
    assert_eq!(decoded.params, value.params);
    Ok(())
}

#[test]
fn surface_measurement_dto_round_trip_preserves_scores() -> Result<(), serde_json::Error> {
    let value = SurfaceMeasurementDto {
        surface: measured_surface(4, 256),
        ratchet_passed: Some(true),
        efficiency_score: 0.75,
        efficiency_confidence: 0.9,
    };
    assert_eq!(round_trip(&value)?, value);
    Ok(())
}

#[test]
fn stale_fallback_dto_round_trip_preserves_refusal_contract(
) -> Result<(), Box<dyn std::error::Error>> {
    let tool = McpToolName::try_new("ocentra_enforcer_coordination_claim")?;
    let value = stale_fallback(
        &tool,
        McpFreshness::Stale,
        &ArtifactPath::from_path(Path::new("C:/enforcer/bin/enforcer.exe")),
    );
    assert_eq!(round_trip(&value)?, value);
    Ok(())
}

#[test]
fn malformed_fallback_command_is_rejected_before_domain_conversion() {
    let malformed = r#"{"recommendedTool":"ocentra_enforcer_run","command":"not-an-array"}"#;
    assert!(matches!(
        serde_json::from_str::<enforcer_mcp::gate::FallbackCommandDto>(malformed),
        Err(error) if error.is_data()
    ));
}

#[test]
fn rpc_response_dtos_round_trip_preserves_success_and_error_shapes(
) -> Result<(), Box<dyn std::error::Error>> {
    let success = RpcResultDto::new(serde_json::json!(7), serde_json::json!({"ok": true}));
    assert_eq!(round_trip(&success)?, success);
    let error = RpcErrorDto::new(
        serde_json::json!(7),
        RpcErrorBody::new(
            RpcErrorCode::MethodNotFound,
            RpcErrorMessage::try_new("missing")?,
        ),
    );
    assert_eq!(round_trip(&error)?, error);
    Ok(())
}

#[test]
fn tool_descriptor_dto_round_trip_preserves_input_schema() -> Result<(), serde_json::Error> {
    let value = ToolDescriptorDto {
        name: "ocentra_enforcer_check".to_owned(),
        description: "Check a bounded surface.".to_owned(),
        input_schema: serde_json::json!({"type": "object"}),
    };
    assert_eq!(round_trip(&value)?, value);
    Ok(())
}

#[test]
fn malformed_tool_descriptor_is_rejected_before_domain_conversion() {
    let malformed = r#"{"name":"ocentra_enforcer_check","inputSchema":{"type":"object"}}"#;
    assert!(matches!(
        serde_json::from_str::<ToolDescriptorDto>(malformed),
        Err(error) if error.is_data()
    ));
}

#[test]
fn malformed_surface_measurement_is_rejected_before_domain_conversion() {
    let malformed = r#"{"surface":"invalid","ratchetPassed":true,"efficiencyScore":0.5,"efficiencyConfidence":0.8}"#;
    assert!(matches!(
        serde_json::from_str::<SurfaceMeasurementDto>(malformed),
        Err(error) if error.is_data()
    ));
}

#[test]
fn fingerprint_boundary_conversions_reject_invalid_canonical_values() {
    let invalid_state = ArtifactState::try_from(ArtifactStateDto::Present {
        sha256: "not-a-digest".to_owned(),
        byte_length: 19,
    });
    assert!(matches!(
        invalid_state,
        Err(error) if error.path == "artifact.state.sha256"
    ));

    let invalid_entry = ArtifactEntry::try_from(ArtifactEntryDto {
        path: " ".to_owned(),
        state: ArtifactStateDto::Missing,
    });
    assert!(matches!(
        invalid_entry,
        Err(error) if error.path == "artifact.path"
    ));

    let invalid_fingerprint = McpFingerprint::try_from(McpFingerprintDto {
        digest: "not-a-digest".to_owned(),
        package_version: "1.0.0".to_owned(),
        binary: ArtifactEntryDto {
            path: "target/enforcer".to_owned(),
            state: ArtifactStateDto::Missing,
        },
        ruleset: None,
    });
    assert!(matches!(
        invalid_fingerprint,
        Err(error) if error.path == "fingerprint.digest"
    ));

    let invalid_change = ChangedArtifact::try_from(ChangedArtifactDto {
        slot: ArtifactSlotDto::Binary,
        startup: Some(ArtifactEntryDto {
            path: " ".to_owned(),
            state: ArtifactStateDto::Missing,
        }),
        current: None,
    });
    assert!(matches!(
        invalid_change,
        Err(error) if error.path == "artifact.path"
    ));
}

#[test]
fn response_and_staleness_boundary_conversions_reject_invalid_canonical_values() {
    let invalid_error_body = RpcErrorBody::try_from(RpcErrorBodyDto {
        code: 0,
        message: "invalid code".to_owned(),
    });
    assert!(matches!(
        invalid_error_body,
        Err(error) if error.path == "rpcErrorCode"
    ));

    let invalid_error_response = RpcErrorResponse::try_from(RpcErrorDto {
        jsonrpc: "2.0".to_owned(),
        id: serde_json::json!(7),
        error: RpcErrorBodyDto {
            code: 0,
            message: "invalid code".to_owned(),
        },
    });
    assert!(matches!(
        invalid_error_response,
        Err(error) if error.path == "rpcErrorCode"
    ));

    let invalid_staleness = StalenessReport::try_from(StalenessReportDto {
        verdict: StalenessDto::Stale,
        startup_digest: "not-a-digest".to_owned(),
        current_digest: "0".repeat(64),
        changed: Vec::new(),
    });
    assert!(matches!(
        invalid_staleness,
        Err(error) if error.path == "staleness.startupDigest"
    ));
}
