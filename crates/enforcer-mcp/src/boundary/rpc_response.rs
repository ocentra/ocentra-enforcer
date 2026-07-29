//! JSON-RPC response transport DTOs.
//!
//! BOUNDARY-INVARIANT: raw JSON-RPC replies are constructed only here from
//! already-resolved router outcomes.
//! boundaryOwnerNote: enforcer-mcp owns MCP response encoding.

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::mcp_types::{RpcErrorBody, RpcErrorCode, RpcErrorMessage};

/// Raw MCP JSON-RPC success reply at the stdio boundary.
// ROUNDTRIP-TEST: crates/enforcer-mcp/tests/wire_roundtrip.rs::
// rpc_response_dtos_round_trip_preserves_success_and_error_shapes
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
/// Raw MCP JSON-RPC success reply at the stdio boundary.
pub struct RpcResultDto {
    pub jsonrpc: String,
    pub id: serde_json::Value,
    pub result: serde_json::Value,
}

/// Serialized JSON-RPC success parts after explicit DTO conversion.
pub struct RpcResultParts {
    pub jsonrpc: String,
    pub id: serde_json::Value,
    pub result: serde_json::Value,
}

impl From<RpcResultDto> for RpcResultParts {
    fn from(value: RpcResultDto) -> Self {
        Self {
            jsonrpc: value.jsonrpc,
            id: value.id,
            result: value.result,
        }
    }
}

impl RpcResultDto {
    /// Builds a JSON-RPC 2.0 success reply.
    pub fn new(id: serde_json::Value, result: serde_json::Value) -> Self {
        Self {
            jsonrpc: "2.0".to_owned(),
            id,
            result,
        }
    }

    /// Explicit egress mapper: callers hand this DTO straight to the wire
    /// encoder rather than exposing a serde-bearing core response type.
    pub fn into_wire_parts(self) -> (String, serde_json::Value, serde_json::Value) {
        let parts = RpcResultParts::from(self);
        (parts.jsonrpc, parts.id, parts.result)
    }
}

/// Raw MCP JSON-RPC error reply at the stdio boundary.
// ROUNDTRIP-TEST: crates/enforcer-mcp/tests/wire_roundtrip.rs::
// rpc_response_dtos_round_trip_preserves_success_and_error_shapes
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
/// Raw MCP JSON-RPC error reply at the stdio boundary.
pub struct RpcErrorDto {
    pub jsonrpc: String,
    pub id: serde_json::Value,
    pub error: RpcErrorBodyDto,
}

/// Raw MCP JSON-RPC error body at the stdio boundary.
// ROUNDTRIP-TEST: crates/enforcer-mcp/tests/wire_roundtrip.rs::
// rpc_response_dtos_round_trip_preserves_success_and_error_shapes
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
/// Raw MCP JSON-RPC error body at the stdio boundary.
pub struct RpcErrorBodyDto {
    pub code: i64,
    pub message: String,
}

/// Converted JSON-RPC error parts after the raw DTO has crossed ingress.
pub struct RpcErrorResponse {
    pub jsonrpc: String,
    pub id: serde_json::Value,
    pub error: RpcErrorBody,
}

impl From<RpcErrorBody> for RpcErrorBodyDto {
    fn from(value: RpcErrorBody) -> Self {
        Self {
            code: value.code().into(),
            message: value.message().to_string(),
        }
    }
}

fn decode_error_code(value: i64) -> Result<RpcErrorCode, DecodeError> {
    match value {
        -32700 => Ok(RpcErrorCode::ParseError),
        -32600 => Ok(RpcErrorCode::InvalidRequest),
        -32601 => Ok(RpcErrorCode::MethodNotFound),
        -32602 => Ok(RpcErrorCode::InvalidParams),
        -32603 => Ok(RpcErrorCode::InternalError),
        _ => Err(DecodeError::new(
            "rpcErrorCode",
            "must be a supported JSON-RPC server error code",
        )),
    }
}

impl TryFrom<RpcErrorBodyDto> for RpcErrorBody {
    type Error = DecodeError;

    fn try_from(value: RpcErrorBodyDto) -> Result<Self, Self::Error> {
        Ok(Self::new(
            decode_error_code(value.code)?,
            RpcErrorMessage::try_new(&value.message)?,
        ))
    }
}

impl TryFrom<RpcErrorDto> for RpcErrorResponse {
    type Error = DecodeError;

    fn try_from(value: RpcErrorDto) -> Result<Self, Self::Error> {
        Ok(Self {
            jsonrpc: value.jsonrpc,
            id: value.id,
            error: RpcErrorBody::try_from(value.error)?,
        })
    }
}

impl RpcErrorDto {
    /// Builds a JSON-RPC 2.0 error reply.
    pub fn new(id: serde_json::Value, error: RpcErrorBody) -> Self {
        Self {
            jsonrpc: "2.0".to_owned(),
            id,
            error: error.into(),
        }
    }

    /// Explicit egress mapper for the transport serializer.
    pub fn into_wire_parts(self) -> Result<RpcErrorResponse, DecodeError> {
        self.try_into()
    }
}

#[cfg(test)]
mod tests {
    use super::{RpcErrorDto, RpcResultDto};
    use enforcer_domain::mcp_types::{RpcErrorBody, RpcErrorCode, RpcErrorMessage};

    #[test]
    fn invalid_response_input_is_rejected() {
        let invalid = r#"{\"jsonrpc\":\"2.0\",\"id\":1}"#;
        assert!(matches!(
            serde_json::from_str::<RpcErrorDto>(invalid),
            Err(error) if error.is_syntax()
        ));
    }

    #[test]
    fn response_dtos_round_trip_at_the_wire_boundary() -> Result<(), Box<dyn std::error::Error>> {
        let result = RpcResultDto::new(serde_json::json!(3), serde_json::json!({"ok": true}));
        let result_json = serde_json::to_string(&result)?;
        let decoded_result: RpcResultDto = serde_json::from_str(&result_json)?;
        assert_eq!(decoded_result.id, serde_json::json!(3));

        let error = RpcErrorDto::new(
            serde_json::json!(3),
            RpcErrorBody::new(
                RpcErrorCode::MethodNotFound,
                RpcErrorMessage::try_new("missing")?,
            ),
        );
        let error_json = serde_json::to_string(&error)?;
        let decoded_error: RpcErrorDto = serde_json::from_str(&error_json)?;
        assert_eq!(
            decoded_error.error.code,
            i64::from(RpcErrorCode::MethodNotFound)
        );
        Ok(())
    }
}
