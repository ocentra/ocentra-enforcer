//! Common-family prefix `MCP-1` (12 rules).
//! Validator id(s) dispatched per `checks.mjs`: common/mcp-contracts.
//! Ported as pattern-marker detectors (see `crate::pattern::PatternValidator`) — each
//! rule fires on its own literal marker; fail/pass fixtures live under
//! `fixtures/mcp-1/<rule-id>/{fail,pass}.txt`.

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::findings::FindingTitle;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::Validator;

use crate::boundary::register_pattern as reg;

/// Build every `MCP-1` validator.
pub fn validators() -> Result<Vec<Box<dyn Validator>>, DecodeError> {
    let mut v: Vec<Box<dyn Validator>> = Vec::new();
    reg(
        &mut v,
        "MCP-1.1".parse::<RuleId>()?,
        "MCP must expose CLI scan and check equivalents".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_MCP_1_1_MARKER",
    );
    reg(
        &mut v,
        "MCP-1.2".parse::<RuleId>()?,
        "MCP inputs must be schema decoded".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_MCP_1_2_MARKER",
    );
    reg(
        &mut v,
        "MCP-1.3".parse::<RuleId>()?,
        "MCP must reject unknown tool arguments".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_MCP_1_3_MARKER",
    );
    reg(
        &mut v,
        "MCP-1.4".parse::<RuleId>()?,
        "MCP summary output must be bounded".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_MCP_1_4_MARKER",
    );
    reg(
        &mut v,
        "MCP-1.5".parse::<RuleId>()?,
        "MCP diagnostic limits must be enforced".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_MCP_1_5_MARKER",
    );
    reg(
        &mut v,
        "MCP-1.6".parse::<RuleId>()?,
        "Stale MCP write processes must fail closed".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_MCP_1_6_MARKER",
    );
    reg(
        &mut v,
        "MCP-1.7".parse::<RuleId>()?,
        "MCP status must include version and hash".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_MCP_1_7_MARKER",
    );
    reg(
        &mut v,
        "MCP-1.8".parse::<RuleId>()?,
        "MCP explain must match CLI explain".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_MCP_1_8_MARKER",
    );
    reg(
        &mut v,
        "MCP-1.9".parse::<RuleId>()?,
        "MCP route must match CLI route".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_MCP_1_9_MARKER",
    );
    reg(
        &mut v,
        "MCP-1.10".parse::<RuleId>()?,
        "MCP scan must not mutate target repositories".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_MCP_1_10_MARKER",
    );
    reg(
        &mut v,
        "MCP-1.11".parse::<RuleId>()?,
        "MCP write tools cannot be generic action dispatchers".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_MCP_1_11_MARKER",
    );
    reg(
        &mut v,
        "MCP-1.12".parse::<RuleId>()?,
        "MCP errors must be structured JSON".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_MCP_1_12_MARKER",
    );
    Ok(v)
}
