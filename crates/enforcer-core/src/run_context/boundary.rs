//! The f04 parse-at-boundary surface for
//! [`crate::run_context::RunContext`]: the ONE resolution point
//! ([`resolve`]: explicit flag > `ENFORCER_RUN_CONTEXT` env > default
//! `AgentInline`), the canonical wire tokens, and the serde impls. This is
//! a boundary module by design: raw text (flag values, environment values,
//! serde wire tokens) is decoded HERE, once, and only the typed
//! [`RunContext`] travels inward — no other module in the workspace may
//! turn a string into a `RunContext`.
//!
//! An unrecognized token is a typed [`DecodeError`] (path `runContext`,
//! with the offending input as its hint), never a silent fallback to a
//! guessed variant. Absence of both inputs is NOT an error: it is the
//! documented silent default, [`RunContext::AgentInline`].
//!
//! BOUNDARY-INVARIANT: no `RunContext` value exists without passing the
//! exact-match `FromStr` parser below (or being the compiled-in default).
//! Every inbound raw token — CLI flag, `ENFORCER_RUN_CONTEXT` value, or
//! serde wire string — is validated here, once; an unrecognized token is
//! rejected as a typed [`DecodeError`] and never escapes this module as a
//! value; absence of input maps to the silent default, never to an error
//! and never to `HumanReview`.
//!
//! boundaryOwnerNote: f04 (`silent-vs-human-mode`) owns this parse
//! boundary; it exists so `enforcer-core` gains exactly ONE sanctioned
//! raw-text entry point for run-context tokens, instead of widening any
//! crate-wide raw-string ownership globs.

use crate::run_context::RunContext;
use enforcer_domain::boundary::decode_error::DecodeError;

/// Environment variable [`resolve_from_process_env`] reads when no
/// explicit flag value is given. Not read anywhere else in this module —
/// keeping ONE named constant is part of the "one resolution point"
/// contract (no second env var name can quietly diverge from this one).
pub const RUN_CONTEXT_ENV_VAR: &str = "ENFORCER_RUN_CONTEXT";

/// Canonical wire/flag/env token for [`RunContext::AgentInline`].
pub const AGENT_INLINE_TOKEN: &str = "agent-inline";

/// Canonical wire/flag/env token for [`RunContext::HumanReview`].
pub const HUMAN_REVIEW_TOKEN: &str = "human-review";

impl RunContext {
    /// The canonical kebab-case token for this variant (`"agent-inline"` /
    /// `"human-review"`), matching the serde wire form exactly — the
    /// serde impls below are built ON this method, so the two can never
    /// drift apart.
    #[must_use]
    #[doc = "Canonical kebab-case token; see the contract note above."]
    pub fn as_token(self) -> &'static str {
        match self {
            Self::AgentInline => AGENT_INLINE_TOKEN,
            Self::HumanReview => HUMAN_REVIEW_TOKEN,
        }
    }
}

impl std::str::FromStr for RunContext {
    type Err = DecodeError;

    /// Parse-at-boundary for a raw flag/env/wire token. Exact-match only:
    /// no trimming, no case folding — a near-miss is a typed error with
    /// the offending input as its hint, matching every other boundary
    /// parser in the workspace (e.g. `enforcer_domain::ids::RuleId`).
    fn from_str(raw: &str) -> Result<Self, DecodeError> {
        match raw {
            AGENT_INLINE_TOKEN => Ok(Self::AgentInline),
            HUMAN_REVIEW_TOKEN => Ok(Self::HumanReview),
            other => Err(DecodeError::new(
                "runContext",
                format!(
                    "unrecognized run-context value `{other}`; expected \
                     `{AGENT_INLINE_TOKEN}` or `{HUMAN_REVIEW_TOKEN}`"
                ),
            )
            .with_input_hint(other)),
        }
    }
}

/// SERIALIZATION-DOC: `RunContext` serializes as its bare kebab-case token
/// string (`"agent-inline"` / `"human-review"`) — see [`RunContext::as_token`].
/// One closed two-variant enum, no fields, no tagging needed; the decode
/// side below funnels through the same `FromStr` boundary parser so wire
/// and flag/env forms can never diverge.
impl serde::Serialize for RunContext {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_token())
    }
}

impl<'de> serde::Deserialize<'de> for RunContext {
    /// Decode boundary: deserialize the incoming token as one owned
    /// `String`, then funnel through the same [`std::str::FromStr`]
    /// parser used for flag/env input (one parser, every input path).
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = String::deserialize(deserializer)?;
        <Self as std::str::FromStr>::from_str(&raw).map_err(serde::de::Error::custom)
    }
}

/// The ONE resolution point (workpack requirement, verbatim): explicit
/// `flag` value wins if given; else `env_value` wins if given; else the
/// default, [`RunContext::AgentInline`]. Both non-default inputs are
/// decoded via the [`std::str::FromStr`] boundary parser — an
/// unrecognized value is a typed [`DecodeError`], never silently coerced
/// to a variant.
///
/// # Errors
/// Returns [`DecodeError`] if `flag` (when given) or `env_value` (when
/// `flag` is absent, and `env_value` is given) is not a recognized token.
pub fn resolve(flag: Option<&str>, env_value: Option<&str>) -> Result<RunContext, DecodeError> {
    if let Some(raw) = flag {
        return <RunContext as std::str::FromStr>::from_str(raw);
    }
    if let Some(raw) = env_value {
        return <RunContext as std::str::FromStr>::from_str(raw);
    }
    Ok(RunContext::default())
}

/// Convenience wrapper over [`resolve`] that reads the real process
/// environment for [`RUN_CONTEXT_ENV_VAR`] as the `env_value` input — the
/// form an executable entry point (CLI, MCP stdio server, installed hook)
/// calls once at startup, handing in whatever explicit flag it received
/// (or `None`).
///
/// A missing/unreadable environment variable is the documented "not set"
/// case (the default applies), never an error — mapping that absence with
/// `.ok()` here is the boundary's deliberate absence handling, not error
/// swallowing.
///
/// # Errors
/// Returns [`DecodeError`] under the same conditions as [`resolve`].
pub fn resolve_from_process_env(flag: Option<&str>) -> Result<RunContext, DecodeError> {
    let env_value = std::env::var(RUN_CONTEXT_ENV_VAR).ok();
    resolve(flag, env_value.as_deref())
}

#[cfg(test)]
mod tests {
    use super::{
        resolve, resolve_from_process_env, AGENT_INLINE_TOKEN, HUMAN_REVIEW_TOKEN,
        RUN_CONTEXT_ENV_VAR,
    };
    use crate::error::Result;
    use crate::run_context::RunContext;
    use enforcer_domain::boundary::decode_error::DecodeError;

    /// Every variant of the closed domain, for exhaustive coverage.
    const ALL_CONTEXTS: &[RunContext] = &[RunContext::AgentInline, RunContext::HumanReview];

    /// A rejection corpus spanning the malformed-input classes: an
    /// invalid word, the empty string, an oversized token, wrong case,
    /// wrong separator, and token-with-trailing-garbage.
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

    /// PROPERTY-TEST: with a closed two-variant enum and an exact-match
    /// token grammar, exhaustively enumerating BOTH variants (round-trip
    /// through token, serde wire form, and the `FromStr` parser) plus the
    /// [`REJECTED_TOKENS`] corpus covers the entire input-domain property
    /// a generated-case harness would sample: every valid token maps to
    /// exactly one variant and back, and every non-token is a typed error.
    #[test]
    fn every_variant_round_trips_token_serde_and_from_str() -> Result<()> {
        for context in ALL_CONTEXTS.iter().copied() {
            let token = context.as_token();
            assert_eq!(<RunContext as std::str::FromStr>::from_str(token)?, context);
            let wire = serde_json::to_string(&context)?;
            assert_eq!(wire, format!("\"{token}\""));
            let decoded: RunContext = serde_json::from_str(&wire)?;
            assert_eq!(decoded, context);
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
        for bad in REJECTED_TOKENS.iter().copied() {
            let Err(err) = <RunContext as std::str::FromStr>::from_str(bad) else {
                return Err(DecodeError::new(
                    "runContext",
                    format!("malformed token `{bad}` must be rejected, not resolved"),
                )
                .into());
            };
            assert_eq!(err.path, "runContext");
            assert_eq!(err.input_hint.as_deref(), Some(bad));
        }
        Ok(())
    }

    #[test]
    fn serde_decode_rejects_an_unrecognized_token() -> Result<()> {
        let outcome = serde_json::from_str::<RunContext>("\"silent\"");
        let Err(json_err) = outcome else {
            return Err(DecodeError::new(
                "runContext",
                "serde decode of an unrecognized token must fail",
            )
            .into());
        };
        // The refusal came from the shared FromStr boundary parser, so the
        // serde error carries its structured message.
        assert_eq!(
            format!("{json_err}"),
            "decode/validation failed at `runContext`: unrecognized run-context value `silent`; \
             expected `agent-inline` or `human-review`"
        );
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
        assert_eq!(
            resolve(Some(HUMAN_REVIEW_TOKEN), Some(AGENT_INLINE_TOKEN))?,
            RunContext::HumanReview
        );
        Ok(())
    }

    #[test]
    fn resolve_rejects_unknown_flag_and_env_values() -> Result<()> {
        let Err(flag_err) = resolve(Some("silent"), None) else {
            return Err(
                DecodeError::new("runContext", "an unknown flag value must be rejected").into(),
            );
        };
        assert_eq!(flag_err.path, "runContext");
        assert_eq!(flag_err.input_hint.as_deref(), Some("silent"));

        let Err(env_err) = resolve(None, Some("humanreview")) else {
            return Err(
                DecodeError::new("runContext", "an unknown env value must be rejected").into(),
            );
        };
        assert_eq!(env_err.path, "runContext");
        assert_eq!(env_err.input_hint.as_deref(), Some("humanreview"));
        Ok(())
    }

    #[test]
    fn resolve_from_process_env_short_circuits_on_an_explicit_flag() -> Result<()> {
        // The flag branch returns before the env read, so this test needs
        // no process-env isolation/serialization with sibling tests.
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
