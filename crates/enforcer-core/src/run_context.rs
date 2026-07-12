//! `RunContext` (f04): the ONE formalized silent-vs-human signal every
//! surface in the workspace resolves through, per
//! `docs/plans/enforcer-selfhost-plan/workpacks/f04-silent-vs-human-mode.md`.
//!
//! # The type
//! [`RunContext::AgentInline`] — silent: an agent is running checks inline
//! while coding, or a mechanical hook (the c04 `PreToolUse` deny-hook) is
//! firing. STRUCTURED output only. No UI render, no server start, no popup.
//! [`RunContext::HumanReview`] — a human explicitly asked to review: may
//! open the Tauri desktop UI or the served self-contained HTML fallback
//! (presentation only, per `RUST_ARCHITECTURE.md`).
//!
//! # Split: domain type here, parse-at-boundary in [`boundary`]
//! This file owns only the closed domain enum and the UI/server gate — no
//! raw text enters or leaves any signature here. All decoding of outside
//! input (flag values, environment values, serde wire tokens) lives in the
//! [`boundary`] child module, which also owns the ONE resolution point
//! ([`boundary::resolve`]: flag > env > default `AgentInline`) and the
//! serde impls. An unrecognized token there is a typed
//! [`crate::error::DecodeError`], never a silent fallback to a guessed
//! variant.
//!
//! # The UI/server gate
//! [`RunContext::guard_ui_or_server`] is the ONE gate a UI/server entry
//! point calls before doing anything observable (binding a socket,
//! spawning a window): it returns [`SilentModeRefusal`] for
//! `AgentInline`, `Ok(())` only for `HumanReview`. This is enforced at the
//! type/gate level (a caller must explicitly call and honor it — there is
//! no ambient global this module mutates), not advisory prose.
//!
//! # Threading landed vs. deferred (read before assuming a surface is wired)
//! This workpack owns exactly this module plus its tests (parallel-ownership
//! note in the `.md`). Concretely, as of this pass:
//! - **Landed here**: the type, the one resolution point + typed
//!   parse-at-boundary error ([`boundary`]), and the reusable gate
//!   ([`RunContext::guard_ui_or_server`]).
//! - **Landed with zero code change, by construction**: the c04
//!   `PreToolUse` deny-hook (`enforcer-install::hooks::pretooluse`) never
//!   passes a `--run-context`/`ENFORCER_RUN_CONTEXT` value, so it already
//!   resolves to the default, `AgentInline` — the acceptance criterion
//!   "resolve with no mode set (deny-hook path) -> AgentInline" holds
//!   without touching that crate.
//! - **Deferred (follow-up, not this workpack's owned files)**: actually
//!   calling [`RunContext::guard_ui_or_server`] from
//!   `enforcer_ui::serve::run` / the MCP `ocentra_enforcer_ui` tool
//!   handler (`enforcer-mcp::router`) / `enforcer-cli`'s `Serve`/`Ui`
//!   dispatch. Those files are the integration points named in this
//!   workpack's dispatch instructions as contested (owned by
//!   concurrently-running Track G / f01 workpacks); this pass implements
//!   the seam here rather than colliding with that in-flight work.
//!   `crates/enforcer-ui/src/serve.rs`'s own module docs already flag
//!   exactly this ("`enforcer-core`'s run-context gate (f04) has not
//!   landed as of this workpack") — the follow-up is for that owner to
//!   call `enforcer_core::run_context::boundary::resolve(..)` and then
//!   `guard_ui_or_server()` immediately before `TcpListener::bind` in
//!   `enforcer_ui::serve::run`, and for the MCP/CLI call sites to do the
//!   same before delegating to it.

pub mod boundary;

/// The formalized silent-vs-human run signal. A closed two-variant domain
/// enum: construction from outside input happens ONLY through the
/// [`boundary`] module (wire form `"agent-inline"` / `"human-review"`,
/// resolution precedence flag > env > default), so every in-crate holder
/// of a `RunContext` value is holding an already-validated signal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[doc = "Closed silent-vs-human signal; see the contract note above."]
pub enum RunContext {
    /// Silent: an agent running checks inline while coding, or a
    /// mechanical hook (the c04 deny-hook). STRUCTURED output only — no
    /// UI render, no server start, no popup.
    AgentInline,
    /// A human explicitly asked for a review. May open the Tauri desktop
    /// UI or the served self-contained HTML fallback (presentation only).
    HumanReview,
}

impl Default for RunContext {
    /// Silent by default, per the workpack's precedence rule
    /// (`flag > env > default AgentInline`) — a caller that never wires
    /// resolution in at all still gets the safe (no-UI) behavior rather
    /// than an accidental `HumanReview`.
    fn default() -> Self {
        Self::AgentInline
    }
}

impl RunContext {
    /// The UI/server gate: `Ok(())` only under [`RunContext::HumanReview`],
    /// else [`SilentModeRefusal`]. Every UI/server entry point (Tauri
    /// shell, served-HTML fallback listener, MCP `ui` tool launch path)
    /// must call this BEFORE doing anything observable (binding a socket,
    /// spawning a window) — enforced by each such call site actually
    /// invoking it (see the module doc's "deferred" list for which call
    /// sites still need to be wired), not by this module reaching out and
    /// blocking them itself.
    ///
    /// # Errors
    /// Returns [`SilentModeRefusal`] when `self` is
    /// [`RunContext::AgentInline`].
    pub fn guard_ui_or_server(self) -> Result<(), SilentModeRefusal> {
        match self {
            Self::HumanReview => Ok(()),
            Self::AgentInline => Err(SilentModeRefusal),
        }
    }
}

/// Raised by [`RunContext::guard_ui_or_server`] when a UI/server surface
/// is attempted under [`RunContext::AgentInline`]. Kept as its own small
/// error type (not folded into [`crate::error::Error`]) — this is a
/// caller-facing refusal a UI/server entry point matches on directly, the
/// same posture `enforcer_ui::serve::BindError` already takes for its own
/// fail-closed gate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error(
    "UI/server surfaces require RunContext::HumanReview; refusing to start under \
     RunContext::AgentInline (silent mode) -- no socket is bound, no window is opened"
)]
#[doc = "Typed refusal from the UI/server gate; see the note above."]
pub struct SilentModeRefusal;

#[cfg(test)]
mod tests {
    use super::{RunContext, SilentModeRefusal};

    #[test]
    fn default_is_agent_inline_silent() {
        assert_eq!(RunContext::default(), RunContext::AgentInline);
    }

    #[test]
    fn guard_refuses_agent_inline_with_the_typed_refusal() {
        assert_eq!(
            RunContext::AgentInline.guard_ui_or_server(),
            Err(SilentModeRefusal)
        );
    }

    #[test]
    fn guard_permits_human_review() {
        assert_eq!(RunContext::HumanReview.guard_ui_or_server(), Ok(()));
    }
}
