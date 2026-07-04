//! Common-family prefix `MCP-1` (12 rules).
//! Validator id(s) dispatched per `checks.mjs`: common/mcp-contracts.
//! Ported as pattern-marker detectors (see `crate::pattern::PatternValidator`) — each
//! rule fires on its own literal marker; fail/pass fixtures live under
//! `fixtures/mcp-1/<rule-id>/{fail,pass}.txt`.

use enforcer_domain::severity::Severity;
use enforcer_validator::validator::Validator;

use crate::registry::reg;

/// Build every `MCP-1` validator.
pub fn validators() -> Vec<Box<dyn Validator>> {
    let mut v: Vec<Box<dyn Validator>> = Vec::new();
    reg(
        &mut v,
        "MCP-1.1",
        "MCP must expose CLI scan and check equivalents",
        Severity::Error,
        "ENFORCER_MCP_1_1_MARKER",
    );
    reg(
        &mut v,
        "MCP-1.2",
        "MCP inputs must be schema decoded",
        Severity::Error,
        "ENFORCER_MCP_1_2_MARKER",
    );
    reg(
        &mut v,
        "MCP-1.3",
        "MCP must reject unknown tool arguments",
        Severity::Error,
        "ENFORCER_MCP_1_3_MARKER",
    );
    reg(
        &mut v,
        "MCP-1.4",
        "MCP summary output must be bounded",
        Severity::Error,
        "ENFORCER_MCP_1_4_MARKER",
    );
    reg(
        &mut v,
        "MCP-1.5",
        "MCP diagnostic limits must be enforced",
        Severity::Error,
        "ENFORCER_MCP_1_5_MARKER",
    );
    reg(
        &mut v,
        "MCP-1.6",
        "Stale MCP write processes must fail closed",
        Severity::Error,
        "ENFORCER_MCP_1_6_MARKER",
    );
    reg(
        &mut v,
        "MCP-1.7",
        "MCP status must include version and hash",
        Severity::Error,
        "ENFORCER_MCP_1_7_MARKER",
    );
    reg(
        &mut v,
        "MCP-1.8",
        "MCP explain must match CLI explain",
        Severity::Error,
        "ENFORCER_MCP_1_8_MARKER",
    );
    reg(
        &mut v,
        "MCP-1.9",
        "MCP route must match CLI route",
        Severity::Error,
        "ENFORCER_MCP_1_9_MARKER",
    );
    reg(
        &mut v,
        "MCP-1.10",
        "MCP scan must not mutate target repositories",
        Severity::Error,
        "ENFORCER_MCP_1_10_MARKER",
    );
    reg(
        &mut v,
        "MCP-1.11",
        "MCP write tools cannot be generic action dispatchers",
        Severity::Error,
        "ENFORCER_MCP_1_11_MARKER",
    );
    reg(
        &mut v,
        "MCP-1.12",
        "MCP errors must be structured JSON",
        Severity::Error,
        "ENFORCER_MCP_1_12_MARKER",
    );
    v
}
