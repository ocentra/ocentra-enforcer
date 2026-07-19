use enforcer_domain::boundary::mcp::{execution_mode, write_intent};
use enforcer_domain::mcp_types::{McpExecutionMode, McpWriteIntent};

#[test]
fn optional_wire_booleans_preserve_absence_and_both_values() {
    assert_eq!(write_intent(None), McpWriteIntent::Unspecified);
    assert_eq!(write_intent(Some(true)), McpWriteIntent::Write);
    assert_eq!(write_intent(Some(false)), McpWriteIntent::ReadOnly);
    assert_eq!(execution_mode(None), McpExecutionMode::Unspecified);
    assert_eq!(execution_mode(Some(true)), McpExecutionMode::DryRun);
    assert_eq!(execution_mode(Some(false)), McpExecutionMode::Apply);
}
