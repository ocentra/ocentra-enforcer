//! JSON decoding owned by the install crate's transport boundary.

//! BOUNDARY-INVARIANT: JSON is decoded into canonical domain values at this seam.
//!
/// Decode an external JSON document into its untyped transport tree.
///
/// # Errors
/// Returns the original serde decoding error when the document is invalid.
pub(crate) fn decode_value(raw: &str) -> Result<serde_json::Value, serde_json::Error> {
    serde_json::from_str(raw)
}

/// Set the generic MCP fixture command while the document remains at the
/// untyped JSON boundary.
#[cfg(test)]
pub(crate) fn with_mcp_command(
    mut value: serde_json::Value,
    command: &std::path::Path,
) -> Result<serde_json::Value, crate::error::InstallError> {
    let Some(command_slot) = value.pointer_mut("/mcpServers/enforcer/command") else {
        return Err(crate::error::InstallError::MalformedConfig {
            // ALLOC-JUSTIFICATION: the typed diagnostic owns its logical JSON location.
            path: "/mcpServers/enforcer/command".to_owned(),
            // ALLOC-JUSTIFICATION: the typed diagnostic owns its static explanation.
            reason: "fixture lacks the command field".to_owned(),
        });
    };
    *command_slot = serde_json::Value::String(command.display().to_string());
    Ok(value)
}
