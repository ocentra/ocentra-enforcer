//! b05 — the `/plan` command EMITTER.
//!
//! # Charter (workpack b05 — BINDING)
//!
//! `skills/plan/SKILL.md` documents the plan workflow in prose; this
//! module is the mechanical bridge into a harness's own first-class
//! command surface: a `/plan` command that, once installed, invokes the
//! REAL `enforcer` binary's `plan new`/`plan check` subcommands (b01's
//! scaffolder, b02's validator, via `enforcer-cli`'s `Command::Plan`
//! grammar) — never a hand-written per-harness hook that reimplements
//! scaffolding or validation logic in the harness's own scripting
//! surface, and never a dispatch that short-circuits to a fixed/fake
//! result.
//!
//! # Harness-neutral by construction
//!
//! [`render_command_body`] produces the SAME dispatch text (the literal
//! `enforcer plan new`/`enforcer plan check` invocation lines) regardless
//! of target harness; only the WRAPPER differs:
//! - [`render_claude_command`] wraps the body in Claude's `.claude/commands/
//!   plan.md` frontmatter shape (`description` + body), matching the
//!   existing `.claude/agents/<name>.md` frontmatter convention this crate
//!   already emits (see [`crate::adapters::claude::ClaudeAdapter::render_agent_descriptor`]).
//! - [`render_generic_command`] wraps the same body in a tool-neutral
//!   Markdown doc with no harness-specific frontmatter, for any harness
//!   with no first-class slash-command surface — a caller drops it
//!   wherever that harness reads freeform command/skill prose.
//!
//! # `enforcer plan` dispatch status (documented, not silent)
//!
//! `enforcer-cli`'s `Command::Plan` variant is a RESERVED clap subcommand
//! (`crates/enforcer-cli/src/main.rs`) that today returns
//! `ExitCode::InternalError` with an explicit "not wired in this
//! skeleton" message — wiring that dispatch to `enforcer-plan`'s live
//! scaffolder/validator is `enforcer-cli`'s own scope, not this
//! workpack's (`owns: skills/plan/**, crates/enforcer-install/src/
//! commands/plan.rs, crates/enforcer-plan/tests/self_validate.rs` —
//! `enforcer-cli` is not in that set). This emitter still targets the
//! REAL binary invocation syntax the eventual wiring will answer to —
//! it is not a workaround that calls into `enforcer-plan` directly or
//! fabricates a success response.

/// Every dispatch line this command's body renders, keyed by the literal
/// `enforcer` subcommand invocation. Shared by both harness wrappers so the
/// two can never drift into reimplementing scaffolding/validation logic
/// differently per harness.
const DISPATCH_LINES: &[&str] = &[
    "enforcer plan new <name>",
    "enforcer plan new <name> --force",
    "enforcer plan check",
];

/// This command's one-line description, shared by both harness wrappers.
pub const DESCRIPTION: &str = "Scaffold, author, and self-validate an Ocentra-methodology \
    plan directory via the real enforcer binary (b01 scaffolder + b02 PLAN-* validator).";

/// Render the harness-neutral command BODY: the workflow steps plus the
/// literal `enforcer` binary dispatch lines from [`DISPATCH_LINES`]. Both
/// [`render_claude_command`] and [`render_generic_command`] embed this
/// verbatim — there is exactly one place the dispatch text lives.
#[must_use]
pub fn render_command_body() -> String {
    let mut body = String::new();
    let Some((new_plan, remaining)) = DISPATCH_LINES.split_first() else {
        return body;
    };
    let Some((forced_new_plan, remaining)) = remaining.split_first() else {
        return body;
    };
    let Some((check_plan, _)) = remaining.split_first() else {
        return body;
    };
    body.push_str(
        "Dispatch every step through the real `enforcer` binary — never reimplement the \
         scaffolder or the PLAN-* validator inline in this command.\n\n\
         1. Scaffold a new plan directory (b01):\n\n",
    );
    body.push_str("```bash\n");
    body.push_str(new_plan);
    body.push('\n');
    body.push_str(forced_new_plan);
    body.push('\n');
    body.push_str("```\n\n");
    body.push_str("2. Author each workpack's capsule/frontmatter/sections by hand.\n\n");
    body.push_str(
        "3. Self-validate against the PLAN-* structure validator (b02) before assigning:\n\n",
    );
    body.push_str("```bash\n");
    body.push_str(check_plan);
    body.push('\n');
    body.push_str("```\n\n");
    body.push_str(
        "4. Treat a non-zero exit as a hard block — fix the workpack, never bypass the \
         check. See `skills/plan/SKILL.md` for the full workflow and the `ruleId`-to-\
         `Validator` map.\n",
    );
    body
}

/// Render `.claude/commands/plan.md` — the Claude-native `/plan` slash
/// command, matching the frontmatter shape
/// [`crate::adapters::claude::ClaudeAdapter::render_agent_descriptor`]
/// already establishes for `.claude/agents/<name>.md` (a `---` fenced
/// frontmatter block naming `description`, then the body).
#[must_use]
pub fn render_claude_command() -> String {
    format!(
        "---\ndescription: {DESCRIPTION}\n---\n\n# /plan\n\n{}",
        render_command_body()
    )
}

/// Render the harness-neutral fallback: a plain Markdown doc with no
/// harness-specific frontmatter, for any harness with no first-class
/// slash-command surface. Callers drop this wherever that harness reads
/// freeform command/skill prose.
#[must_use]
pub fn render_generic_command() -> String {
    format!("# /plan\n\n{DESCRIPTION}\n\n{}", render_command_body())
}

/// True when `rendered` dispatches through the real `enforcer` binary
/// invocation syntax (every [`DISPATCH_LINES`] entry present verbatim) —
/// the mechanical "not a stub" check both the Claude and generic renderers
/// must satisfy. A dispatch that hardcodes a fixed/fake result, or that
/// calls into `enforcer-plan` directly instead of the binary, fails this
/// check.
#[must_use]
pub fn dispatches_via_real_binary(rendered: &str) -> bool {
    DISPATCH_LINES.iter().all(|line| rendered.contains(line))
}

#[cfg(test)]
mod tests {
    use super::{
        dispatches_via_real_binary, render_claude_command, render_command_body,
        render_generic_command, DESCRIPTION, DISPATCH_LINES,
    };

    #[test]
    fn claude_command_carries_frontmatter_and_description() {
        let rendered = render_claude_command();
        assert!(rendered.starts_with("---\n"));
        assert!(rendered.contains(&format!("description: {DESCRIPTION}")));
        assert!(rendered.contains("# /plan"));
    }

    #[test]
    fn claude_command_dispatches_via_the_real_binary_not_a_stub() {
        let rendered = render_claude_command();
        assert!(dispatches_via_real_binary(&rendered));
    }

    #[test]
    fn generic_command_has_no_frontmatter_fence() {
        let rendered = render_generic_command();
        assert!(!rendered.starts_with("---\n"));
        assert!(rendered.starts_with("# /plan"));
        assert!(rendered.contains(DESCRIPTION));
    }

    #[test]
    fn generic_command_dispatches_via_the_real_binary_not_a_stub() {
        let rendered = render_generic_command();
        assert!(dispatches_via_real_binary(&rendered));
    }

    #[test]
    fn claude_and_generic_share_the_identical_dispatch_body() {
        // Both wrappers must embed byte-identical dispatch text -- the
        // workpack's "harness-neutral, never a per-harness hand-written
        // hook" contract made concrete: only the frontmatter wrapper may
        // differ, never the underlying invocation.
        let body = render_command_body();
        assert!(render_claude_command().contains(&body));
        assert!(render_generic_command().contains(&body));
    }

    #[test]
    fn dispatch_lines_never_short_circuit_to_a_fixed_result() {
        // Every dispatch line names a real `enforcer plan <verb>`
        // subcommand token -- never a literal that could be satisfied by a
        // hardcoded "success" string with no binary invocation at all.
        for line in DISPATCH_LINES {
            assert!(line.starts_with("enforcer plan "));
        }
    }

    #[test]
    fn dispatches_via_real_binary_is_false_on_a_stub_that_fakes_success() {
        let fake_stub = "This command always reports success.";
        assert!(!dispatches_via_real_binary(fake_stub));
    }

    #[test]
    fn command_body_documents_both_the_scaffold_and_validate_steps() {
        let body = render_command_body();
        assert!(body.contains("plan new"));
        assert!(body.contains("plan check"));
        assert!(body.contains("SKILL.md"));
    }
}
