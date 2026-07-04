//! Claude Code in-session hook emitters (c04 `PreToolUse` deny-hook, c05
//! `SessionStart` reminder). Both are Claude-specific in-session
//! mechanisms registered by the c03 [`crate::adapters::claude::ClaudeAdapter`]
//! — distinct from [`crate::emitters`]'s consumer-repo git-hook/CI
//! artifacts (see that module's doc comment).
//!
//! # Ownership (mount-point deviation)
//!
//! This barrel file is not in any single workpack's `owns:` line the same
//! way [`crate::emitters`] is not: c04 owns `pretooluse.rs`, c05 owns
//! `sessionstart.rs`, and both packs add ONLY their own `pub mod` line here
//! — never touching the other's line — so the two files land concurrently
//! without conflict. [`DOCTRINE_TEXT`] (and the `T1`/`T2`/`T3` tier tokens
//! it carries) is the single source of truth both hooks read from, so the
//! PreToolUse deny reason and the SessionStart reminder can never drift
//! against each other.
pub mod pretooluse;
pub mod sessionstart;

/// Tier token embedded verbatim in [`DOCTRINE_TEXT`] — a hard/deterministic
/// validator, fail-closed. Shared literal so both c04 and c05 assert
/// against the exact same token rather than each hardcoding `"T1"`. The
/// full bullet-line prefix (not the bare `"T1"` substring) is deliberate:
/// the doctrine's header line `"(T1/T2/T3):"` also contains the bare
/// substring, so a token check against JUST `"T1"` would still pass even
/// if a whole tier's bullet line were deleted — this constant is the
/// exact bullet-line prefix so removing a tier's line is what actually
/// trips the seeded-violation test.
pub const TIER_T1_TOKEN: &str = "- T1 —";
/// Tier token embedded verbatim in [`DOCTRINE_TEXT`] — a scored/advisory
/// but still mechanical check (regex/AST/heuristic), never blocking. See
/// [`TIER_T1_TOKEN`] for why this is the bullet-line prefix, not the bare
/// `"T2"` substring.
pub const TIER_T2_TOKEN: &str = "- T2 —";
/// Tier token embedded verbatim in [`DOCTRINE_TEXT`] — justified prose,
/// only when mechanization is impossible, and only ever proven by a
/// labeling gate. See [`TIER_T1_TOKEN`] for why this is the bullet-line
/// prefix, not the bare `"T3"` substring.
pub const TIER_T3_TOKEN: &str = "- T3 —";

/// The single source-of-truth mechanical-enforcement doctrine text: the
/// enforcer-first reminder plus the T1/T2/T3 tier summary. Both the c05
/// `SessionStart` reminder body and the c04 `PreToolUse` deny-hook reason
/// strings are generated FROM this constant — never a paraphrase
/// maintained separately in either hook — so tier wording can never drift
/// between "what the agent is told at session start" and "what the deny
/// hook says when it blocks a write" (TEST_PROOF_EXPECTATIONS.md
/// `claude-sessionstart-injects` binds this file's byte-identical output
/// to a pinned snapshot; any drift in this constant is a deliberate,
/// snapshot-updating change, never an accident).
pub const DOCTRINE_TEXT: &str = concat!(
    "Enforcer-first: before any edit, run `enforcer scan`/`enforcer check` ",
    "(or the `mcp__enforcer__ocentra_enforcer_scan`/`_check` MCP tools) and ",
    "the coordination guard (`mcp__enforcer__ocentra_enforcer_coordination_guard`) ",
    "before writing. A self-review is never a substitute for the mechanical gate.\n",
    "\n",
    "Mechanical-enforcement doctrine (T1/T2/T3):\n",
    "- T1 — hard/deterministic validator, fail-closed. A PreToolUse deny-hook ",
    "blocks the write and names the exact `ruleId` + `Fix:` hint.\n",
    "- T2 — scored/advisory but still mechanical (regex/AST/heuristic emitting ",
    "score+confidence); surfaced as a warning, never blocks.\n",
    "- T3 — justified prose, only when mechanization is impossible; always ",
    "labeled `advisory, no mechanization possible + <reason>`.\n",
);

#[cfg(test)]
mod tests {
    use super::{DOCTRINE_TEXT, TIER_T1_TOKEN, TIER_T2_TOKEN, TIER_T3_TOKEN};

    #[test]
    fn doctrine_text_carries_every_tier_token() {
        assert!(DOCTRINE_TEXT.contains(TIER_T1_TOKEN));
        assert!(DOCTRINE_TEXT.contains(TIER_T2_TOKEN));
        assert!(DOCTRINE_TEXT.contains(TIER_T3_TOKEN));
    }

    #[test]
    fn doctrine_text_names_the_enforcer_first_marker() {
        assert!(DOCTRINE_TEXT.starts_with("Enforcer-first"));
    }
}
