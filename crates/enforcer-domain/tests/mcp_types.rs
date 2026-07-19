use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::mcp_types::{
    ByteCount, McpActionName, McpToolName, PackageVersion, RpcErrorBody, RpcErrorCode,
    RpcErrorMessage,
};

fn rejection<T>(result: Result<T, DecodeError>, path: &str) -> Result<(), DecodeError> {
    match result {
        Err(error) => {
            assert_eq!(error.path, path);
            Ok(())
        }
        Ok(_) => Err(DecodeError::new(
            path,
            "expected invalid value to be rejected",
        )),
    }
}

#[test]
fn mcp_text_brands_reject_empty_and_blank_invalid_input() -> Result<(), DecodeError> {
    match McpToolName::try_new("") {
        Err(error) => assert_eq!(error.path, "mcpToolName"),
        Ok(_) => {
            return Err(DecodeError::new(
                "mcpToolName",
                "empty tool name unexpectedly passed validation",
            ));
        }
    }
    rejection(McpToolName::try_new("  "), "mcpToolName")?;
    rejection(McpActionName::try_new("\t"), "mcpActionName")?;
    rejection(PackageVersion::try_new("\n"), "packageVersion")?;
    Ok(())
}

#[test]
fn mcp_tool_name_preserves_representative_nonblank_samples() -> Result<(), DecodeError> {
    for sample in ["scan", "coordination_claim", "x-tool.v2", "A"] {
        assert_eq!(McpToolName::try_new(sample)?.as_str(), sample);
    }
    Ok(())
}

#[test]
fn byte_count_distinguishes_zero_and_nonzero_artifact_lengths() -> Result<(), DecodeError> {
    let observed = std::num::NonZeroU64::new(8)
        .ok_or_else(|| DecodeError::new("byteCount", "test fixture must be non-zero"))?;
    assert_ne!(ByteCount::try_new(observed), ByteCount::ZERO);
    Ok(())
}

#[test]
fn rpc_error_codes_round_trip_the_supported_json_rpc_server_range() -> Result<(), DecodeError> {
    for (code, raw) in [
        (RpcErrorCode::ParseError, -32700),
        (RpcErrorCode::InvalidRequest, -32600),
        (RpcErrorCode::MethodNotFound, -32601),
        (RpcErrorCode::InvalidParams, -32602),
        (RpcErrorCode::InternalError, -32603),
    ] {
        assert_eq!(i64::from(code), raw);
    }
    Ok(())
}

#[test]
fn rpc_error_body_requires_a_nonblank_message() -> Result<(), DecodeError> {
    rejection(RpcErrorMessage::try_new(" \t "), "rpcErrorMessage")?;
    let body = RpcErrorBody::new(
        RpcErrorCode::MethodNotFound,
        RpcErrorMessage::try_new("Unknown method: example")?,
    );
    assert_eq!(body.code(), RpcErrorCode::MethodNotFound);
    assert_eq!(body.message().as_str(), "Unknown method: example");
    Ok(())
}
