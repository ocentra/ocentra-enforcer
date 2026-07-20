//! Process-input boundary for the canonical run context.

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::core_types::{RunContext, RUN_CONTEXT_ENV_VAR};

/// Resolve explicit flag, then supplied environment value, then the silent
/// default. Non-default inputs use the canonical domain parser.
pub fn resolve(flag: Option<&str>, env_value: Option<&str>) -> Result<RunContext, DecodeError> {
    if let Some(raw) = flag {
        return raw.parse();
    }
    if let Some(raw) = env_value {
        return raw.parse();
    }
    Ok(RunContext::default())
}

/// Resolve from an explicit flag and the real process environment.
pub fn resolve_from_process_env(flag: Option<&str>) -> Result<RunContext, DecodeError> {
    let env_value = std::env::var(RUN_CONTEXT_ENV_VAR).ok();
    resolve(flag, env_value.as_deref())
}

#[cfg(test)]
mod tests {
    use super::{resolve, resolve_from_process_env};
    use crate::error::Result;
    use enforcer_domain::boundary::decode_error::DecodeError;
    use enforcer_domain::core_types::{
        RunContext, AGENT_INLINE_TOKEN, HUMAN_REVIEW_TOKEN, RUN_CONTEXT_ENV_VAR,
    };

    const ALL_CONTEXTS: &[RunContext] = &[RunContext::AgentInline, RunContext::HumanReview];
    const REJECTED_TOKENS: &[&str] = &[
        "silent",
        "",
        "agent-inline-agent-inline-agent-inline-agent-inline-agent-inline-agent-inline",
        "AGENT-INLINE",
        "human_review",
        "human-review ",
        " agent-inline",
        "human-review-and-then-some",
    ];

    #[test]
    fn every_variant_round_trips_token_serde_and_from_str() -> Result<()> {
        for context in ALL_CONTEXTS.iter().copied() {
            let token = context.as_token();
            assert_eq!(token.parse::<RunContext>()?, context);
            let wire = serde_json::to_string(&context)?;
            assert_eq!(wire, format!("\"{token}\""));
            assert_eq!(serde_json::from_str::<RunContext>(&wire)?, context);
        }
        Ok(())
    }

    #[test]
    fn tokens_are_the_locked_kebab_case_wire_form() {
        assert_eq!(RunContext::AgentInline.as_token(), "agent-inline");
        assert_eq!(RunContext::HumanReview.as_token(), "human-review");
        assert_eq!(RUN_CONTEXT_ENV_VAR, "ENFORCER_RUN_CONTEXT");
    }

    #[test]
    fn malformed_tokens_are_rejected_with_the_typed_decode_error() -> Result<()> {
        for bad in REJECTED_TOKENS {
            let Err(error) = bad.parse::<RunContext>() else {
                return Err(DecodeError::new(
                    "runContext",
                    format!("malformed token `{bad}` must be rejected"),
                )
                .into());
            };
            assert_eq!(error.path, "runContext");
            assert_eq!(error.input_hint.as_deref(), Some(*bad));
        }
        Ok(())
    }

    #[test]
    fn resolution_precedence_is_flag_then_env_then_default() -> Result<()> {
        assert_eq!(resolve(None, None)?, RunContext::AgentInline);
        assert_eq!(
            resolve(None, Some(HUMAN_REVIEW_TOKEN))?,
            RunContext::HumanReview
        );
        assert_eq!(
            resolve(Some(AGENT_INLINE_TOKEN), Some(HUMAN_REVIEW_TOKEN))?,
            RunContext::AgentInline
        );
        Ok(())
    }

    #[test]
    fn resolve_rejects_unknown_values() {
        let flag_error =
            resolve(Some("silent"), None).expect_err("invalid flags should not be accepted");
        assert_eq!(flag_error.path, "runContext");
        assert_eq!(flag_error.input_hint.as_deref(), Some("silent"));

        let env_error = resolve(None, Some("humanreview"))
            .expect_err("invalid environment values should not be accepted");
        assert_eq!(env_error.path, "runContext");
        assert_eq!(env_error.input_hint.as_deref(), Some("humanreview"));
    }

    #[test]
    fn explicit_flag_short_circuits_process_environment() -> Result<()> {
        assert_eq!(
            resolve_from_process_env(Some(AGENT_INLINE_TOKEN))?,
            RunContext::AgentInline
        );
        assert_eq!(
            resolve_from_process_env(Some(HUMAN_REVIEW_TOKEN))?,
            RunContext::HumanReview
        );
        Ok(())
    }
}
