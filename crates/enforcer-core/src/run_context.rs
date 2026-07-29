//! Behavioral gate for the canonical silent-vs-human run context.
//!
//! The [`enforcer_domain::core_types::RunContext`] value and its wire parser
//! live in the dependency-leaf domain crate. This module owns only the
//! observable UI/server gate and process-environment resolution boundary.

pub mod boundary;

use enforcer_domain::core_types::RunContext;

/// Raised when a UI/server surface is attempted in silent agent mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error(
    "UI/server surfaces require RunContext::HumanReview; refusing to start under \
     RunContext::AgentInline (silent mode) -- no socket is bound, no window is opened"
)]
pub struct SilentModeRefusal;

/// Refuse observable UI/server behavior unless the canonical context
/// explicitly represents human review.
pub fn guard_ui_or_server(context: RunContext) -> Result<(), SilentModeRefusal> {
    match context {
        RunContext::HumanReview => Ok(()),
        RunContext::AgentInline => Err(SilentModeRefusal),
    }
}

#[cfg(test)]
mod tests {
    use super::{guard_ui_or_server, SilentModeRefusal};
    use enforcer_domain::core_types::RunContext;

    #[test]
    fn default_is_agent_inline_silent() {
        assert_eq!(RunContext::default(), RunContext::AgentInline);
    }

    #[test]
    fn guard_refuses_agent_inline_with_the_typed_refusal() {
        assert_eq!(
            guard_ui_or_server(RunContext::AgentInline),
            Err(SilentModeRefusal)
        );
    }

    #[test]
    fn guard_permits_human_review() {
        assert_eq!(guard_ui_or_server(RunContext::HumanReview), Ok(()));
    }
}
