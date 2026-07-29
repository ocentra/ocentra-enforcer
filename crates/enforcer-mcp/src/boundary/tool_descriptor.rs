//! MCP `tools/list` wire descriptor.
//!
//! BOUNDARY-INVARIANT: this serde shape is emitted only at the MCP protocol
//! boundary; registry selection data does not own a serialization contract.

// ROUNDTRIP-TEST: crates/enforcer-mcp/tests/wire_roundtrip.rs::
// tool_descriptor_dto_round_trip_preserves_input_schema
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
/// Serialized MCP `tools/list` descriptor.
pub struct ToolDescriptorDto {
    pub name: String,
    pub description: String,
    #[serde(rename = "inputSchema")]
    pub input_schema: serde_json::Value,
}

/// Registry-owned descriptor values after explicit wire DTO conversion.
pub struct ToolDescriptorParts {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

impl From<ToolDescriptorDto> for ToolDescriptorParts {
    // NEGATIVE-TEST: crates/enforcer-mcp/tests/wire_roundtrip.rs::
    // malformed_tool_descriptor_is_rejected_before_domain_conversion
    fn from(value: ToolDescriptorDto) -> Self {
        Self {
            name: value.name,
            description: value.description,
            input_schema: value.input_schema,
        }
    }
}
