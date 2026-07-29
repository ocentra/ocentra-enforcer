use enforcer_domain::mcp_types::McpToolName;
use enforcer_mcp::aliases::{alias_name, normalize_tool_name};
use proptest::{prop_assert_eq, proptest, test_runner::TestCaseError};

fn tool_name(raw: &str) -> Result<McpToolName, TestCaseError> {
    McpToolName::try_new(raw).map_err(|error| TestCaseError::fail(error.to_string()))
}

proptest! {
    #[test]
    fn normalize_tool_name_preserves_arbitrary_non_alias_names(
        suffix in "[a-z][a-z0-9_]{0,48}"
    ) {
        let raw = format!("custom_{suffix}");
        let name = tool_name(&raw)?;
        let normalized = normalize_tool_name(&name)
            .map_err(|error| TestCaseError::fail(error.to_string()))?;
        prop_assert_eq!(normalized, name);
    }

    #[test]
    fn normalize_tool_name_reverses_generated_canonical_aliases(
        // "ocentra_enforcer_" is 17 bytes; keep generated canonical names
        // within the 64-byte McpToolName boundary.
        suffix in "[a-z][a-z0-9_]{0,46}"
    ) {
        let raw = format!("ocentra_enforcer_{suffix}");
        let canonical = tool_name(&raw)?;
        let alias = alias_name(&canonical)
            .map_err(|error| TestCaseError::fail(error.to_string()))?;
        let normalized = normalize_tool_name(&alias)
            .map_err(|error| TestCaseError::fail(error.to_string()))?;
        prop_assert_eq!(normalized, canonical);
    }
}
