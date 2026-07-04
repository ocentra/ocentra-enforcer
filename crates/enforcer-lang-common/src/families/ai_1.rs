//! Common-family prefix `AI-1` (1 rule).
//! Validator id(s) dispatched per `checks.mjs`: common/ai-rule-index.
//! Ported as pattern-marker detectors (see `crate::pattern::PatternValidator`) — each
//! rule fires on its own literal marker; fail/pass fixtures live under
//! `fixtures/ai-1/<rule-id>/{fail,pass}.txt`.

use enforcer_domain::severity::Severity;
use enforcer_validator::validator::Validator;

use crate::registry::reg;

/// Build every `AI-1` validator.
pub fn validators() -> Vec<Box<dyn Validator>> {
    let mut v: Vec<Box<dyn Validator>> = Vec::new();
    reg(
        &mut v,
        "AI-1.1",
        "Agent rule docs must be indexed",
        Severity::Error,
        "ENFORCER_AI_1_1_MARKER",
    );
    v
}
