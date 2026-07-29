//! JSON-RPC request transport DTOs.
//!
//! BOUNDARY-INVARIANT: raw request JSON is accepted only here and remains a
//! DTO until the sink dispatches it.
//! boundaryOwnerNote: enforcer-mcp owns MCP request decoding.

/// Raw MCP JSON-RPC request at the stdio boundary.
// ROUNDTRIP-TEST: crates/enforcer-mcp/tests/wire_roundtrip.rs::rpc_message_dto_round_trip_preserves_optional_params
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
/// Raw MCP JSON-RPC request at the stdio boundary.
pub struct RpcMessageDto {
    // DEFAULT-JUSTIFICATION: JSON-RPC notifications legitimately omit the request id.
    #[serde(default)]
    pub id: Option<serde_json::Value>,
    pub method: String,
    // DEFAULT-JUSTIFICATION: JSON-RPC methods may have no parameter object.
    #[serde(default)]
    pub params: Option<serde_json::Value>,
}

/// Parsed request parts consumed by the stdio boundary dispatcher.
pub struct RpcCallParts {
    pub id: Option<serde_json::Value>,
    pub method: String,
    pub params: Option<serde_json::Value>,
}

impl From<RpcMessageDto> for RpcCallParts {
    fn from(value: RpcMessageDto) -> Self {
        Self {
            id: value.id,
            method: value.method,
            params: value.params,
        }
    }
}

impl RpcMessageDto {
    /// Explicit boundary mapper for the sink: raw JSON is not carried past
    /// the transport adapter as a request object.
    pub fn into_call_parts(self) -> (Option<serde_json::Value>, String, Option<serde_json::Value>) {
        let parts = RpcCallParts::from(self);
        (parts.id, parts.method, parts.params)
    }

    /// Returns whether this request is a fire-and-forget notification.
    pub fn is_notification(&self) -> bool {
        self.id.is_none() && self.method.starts_with("notifications/")
    }
}

#[cfg(test)]
mod tests {
    use super::RpcMessageDto;

    #[test]
    fn invalid_request_input_is_rejected() {
        let invalid = r#"{\"id\":1}"#;
        assert!(matches!(
            serde_json::from_str::<RpcMessageDto>(invalid),
            Err(error) if error.is_syntax()
        ));
    }

    #[test]
    fn request_dto_round_trips_at_the_wire_boundary() -> Result<(), serde_json::Error> {
        let original: RpcMessageDto =
            serde_json::from_str(r#"{"id":7,"method":"tools/call","params":{"name":"example"}}"#)?;
        let encoded = serde_json::to_string(&original)?;
        let decoded: RpcMessageDto = serde_json::from_str(&encoded)?;
        assert_eq!(decoded.method, "tools/call");
        assert_eq!(decoded.id, Some(serde_json::json!(7)));
        Ok(())
    }
}
