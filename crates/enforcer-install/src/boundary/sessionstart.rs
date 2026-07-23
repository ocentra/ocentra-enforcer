//! c05 — the Claude Code `SessionStart` hook emitter.
//!
//! # Charter (workpack c05, TEST_PROOF_EXPECTATIONS.md
//! `claude-sessionstart-injects` — BINDING)
//!
//! Claude Code has no equivalent of Codex's global `AGENTS.md` doctrine
//! block: Claude does not read `AGENTS.md`, and nothing today injects the
//! enforcer-first / mechanical-enforcement (T1/T2/T3) reminder into a
//! fresh session's context. This module is the Rust EMITTER — never a
//! `.ts`/`.mjs` hook script — that produces:
//! - [`sessionstart_hook_config`]: the structured `SessionStart` hook
//!   config record the c03 [`crate::adapters::claude::ClaudeAdapter`]
//!   registers into `~/.claude.json` (`command` is a thin shim invoking
//!   the installed `enforcer` binary; never inline hook logic embedded in
//!   the harness config itself).
//! - [`reminder_body`]: the exact `additionalContext` text the hook
//!   injects, generated from the single source-of-truth
//!   [`crate::hooks::DOCTRINE_TEXT`] constant (shared with the c04
//!   `PreToolUse` deny-hook reason strings, so the two can never drift).
//!
//! # Determinism (byte-identical output, snapshot-pinned)
//!
//! [`reminder_body`] takes no runtime input beyond the caller-supplied
//! `enforcer` binary path and is a pure function of that path plus the
//! fixed [`crate::hooks::DOCTRINE_TEXT`] constant: the SAME `binary_path`
//! ALWAYS produces byte-identical output, so the fixture snapshot at
//! `tests/fixtures/sessionstart_hook/reminder.txt` catches any drift.
//!
//! # Registration is c03's job, not this module's
//!
//! This module only COMPUTES the hook config record; it never writes
//! `~/.claude.json` itself (no `std::fs` write in this file). The c03
//! adapter's `plan`/`apply` decide WHERE/WHEN this config is merged into
//! the harness's settings, same separation as c04's `pretooluse` module.
//!
//! BOUNDARY-INVARIANT: third-party SessionStart JSON is decoded into
//! [`SessionStartHookConfig`] and validated by serde before callers can
//! convert or render it; malformed event names and missing required fields
//! are rejected instead of reaching the install domain as untyped JSON.

use std::path::Path;

use enforcer_domain::{
    boundary::decode_error::DecodeError,
    install_types::{
        HookEvent, SessionStartHookCommand, SessionStartHookConfig, SessionStartHookMatcher,
        SessionStartHookReminderBody,
    },
};
use serde::{Deserialize, Serialize};

use crate::hooks::DOCTRINE_TEXT;

/// The enforcer-first marker string every reminder body starts with —
/// asserted verbatim by the proof test so "the enforcer-first marker
/// string" (TEST_PROOF_EXPECTATIONS.md) names one exact, checkable value
/// rather than a vague paraphrase.
pub const ENFORCER_FIRST_MARKER: &str = "Enforcer-first";

/// Which Claude Code lifecycle event this hook config binds to. `Claude
/// Code` only fires the reminder-injection shape studied here on
/// `SessionStart`; kept as an explicit enum (rather than a bare `&str`)
/// so a future c05-adjacent pack cannot silently mis-target `PreToolUse`
/// (c04's event) by typo.
/// The structured `SessionStart` hook config record this module computes
/// and the c03 adapter registers verbatim into `~/.claude.json`'s
/// `hooks.SessionStart` array. Mirrors Claude Code's own hook-config
/// shape (`matcher` + one-or-more `hooks[].command`) closely enough that
/// c03 can serialize this directly, while staying a plain Rust type here
/// (no direct JSON literal authored in the adapter).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionStartHookConfigDto {
    /// Always [`HookEvent::SessionStart`] for this emitter.
    #[serde(with = "crate::boundary::install_type_wire::hook_event")]
    pub event: HookEvent,
    /// Claude Code hook `matcher` — empty string matches every session
    /// source (`startup`/`resume`/`clear`/`compact`), never scoped to one.
    pub matcher: String,
    /// The absolute path of the installed `enforcer` binary this hook
    /// shells out to. The hook `command` is a THIN shim over this binary
    /// (`<binary_path> hooks sessionstart`) — no hook logic is duplicated
    /// inline in the harness config.
    pub command: String,
    /// The exact `additionalContext` text this hook injects when the
    /// shim runs offline/degraded (e.g. before the real binary subcommand
    /// exists) — byte-identical to [`reminder_body`] so a caller never
    /// needs to re-derive it, and the c03 adapter can register it as a
    /// deterministic fallback payload alongside the shim command.
    pub reminder_body: String,
}

impl TryFrom<SessionStartHookConfigDto> for SessionStartHookConfig {
    type Error = DecodeError;

    fn try_from(value: SessionStartHookConfigDto) -> Result<Self, Self::Error> {
        Ok(Self {
            event: value.event,
            matcher: SessionStartHookMatcher::try_new(value.matcher)?,
            command: SessionStartHookCommand::try_new(value.command)?,
            reminder_body: SessionStartHookReminderBody::try_new(value.reminder_body)?,
        })
    }
}

/// Render the exact reminder body this hook injects as `additionalContext`
/// at session start: the [`ENFORCER_FIRST_MARKER`] line naming the
/// concrete MCP tools + the PreToolUse deny gate, followed by the
/// mechanical-enforcement doctrine (verbatim [`DOCTRINE_TEXT`]) generated
/// from the single source-of-truth constant shared with c04's deny-hook
/// reason strings.
///
/// Deterministic: the same `enforcer_binary_path` always yields
/// byte-identical output — no timestamps, no environment reads, no
/// randomness — so the snapshot fixture
/// (`tests/fixtures/sessionstart_hook/reminder.txt`) can pin it exactly.
#[must_use]
pub fn reminder_body(enforcer_binary_path: &Path) -> String {
    format!(
        "{ENFORCER_FIRST_MARKER}: this repository has the `enforcer` mechanical-\
         enforcement gate installed at `{}`. Prefer `enforcer scan`/`enforcer check` \
         (or the `mcp__enforcer__ocentra_enforcer_scan` / \
         `mcp__enforcer__ocentra_enforcer_check` MCP tools) and the coordination \
         guard (`mcp__enforcer__ocentra_enforcer_coordination_guard`) BEFORE editing \
         files — a PreToolUse deny gate also blocks T1 violations on write, so \
         running the check first avoids a blocked edit.\n\
         \n\
         {DOCTRINE_TEXT}",
        enforcer_binary_path.display()
    )
}

/// Compute the [`SessionStartHookConfig`] wire record the c03 adapter registers.
/// Pure: never touches disk, never inspects `enforcer_binary_path` beyond
/// rendering it into the command/reminder strings (no existence check —
/// that is c03's `plan`/`verify` job against the real filesystem).
#[must_use]
// ROUNDTRIP-TEST: crates/enforcer-install/src/boundary/sessionstart.rs::session_start_hook_config_dto_round_trip_through_json
pub fn sessionstart_hook_config(enforcer_binary_path: &Path) -> SessionStartHookConfigDto {
    SessionStartHookConfigDto {
        event: HookEvent::SessionStart,
        matcher: String::new(),
        command: format!("{} hooks sessionstart", enforcer_binary_path.display()),
        reminder_body: reminder_body(enforcer_binary_path),
    }
}

/// Render [`SessionStartHookConfig`] into the exact `serde_json::Value`
/// Claude's `hooks.SessionStart[]` array entry expects: `{"matcher":
/// ..., "hooks": [{"type": "command", "command": "...",
/// "additionalContext": "..."}]}`.
#[must_use]
pub fn render_settings_entry(config: &SessionStartHookConfigDto) -> serde_json::Value {
    serde_json::json!({
        "matcher": config.matcher,
        "hooks": [
            {
                "type": "command",
                "command": config.command,
                "additionalContext": config.reminder_body,
            }
        ]
    })
}

#[cfg(test)]
mod tests {
    // negative: malformed SessionStartHookConfigDto values must fail conversion into SessionStartHookConfig.
    use super::{
        reminder_body, render_settings_entry, sessionstart_hook_config, HookEvent,
        SessionStartHookConfig, SessionStartHookConfigDto, ENFORCER_FIRST_MARKER,
    };
    use crate::hooks::{TIER_T1_TOKEN, TIER_T2_TOKEN, TIER_T3_TOKEN};
    use std::path::{Path, PathBuf};

    fn fixture_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/sessionstart_hook")
            .join(name)
    }

    #[test]
    fn session_start_hook_config_dto_rejects_an_invalid_empty_command(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let malformed = serde_json::json!({
            "event": "sessionStart",
            "matcher": "",
            "command": "",
            "reminderBody": "reminder"
        })
        .to_string();
        let dto: SessionStartHookConfigDto = serde_json::from_str(&malformed)?;
        let error = match enforcer_domain::install_types::SessionStartHookConfig::try_from(dto) {
            Ok(_) => return Err("empty hook commands were unexpectedly accepted".into()),
            Err(error) => error,
        };
        assert_eq!(error.path, "command");
        Ok(())
    }

    #[test]
    fn session_start_hook_config_dto_rejects_an_invalid_empty_reminder_body(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let malformed = serde_json::json!({
            "event": "sessionStart",
            "matcher": "",
            "command": "/opt/enforcer/bin/enforcer hooks sessionstart",
            "reminderBody": ""
        })
        .to_string();
        let dto: SessionStartHookConfigDto = serde_json::from_str(&malformed)?;
        let error = match enforcer_domain::install_types::SessionStartHookConfig::try_from(dto) {
            Ok(_) => return Err("empty hook reminder body was unexpectedly accepted".into()),
            Err(error) => error,
        };
        assert_eq!(error.path, "reminderBody");
        Ok(())
    }

    fn sample_binary_path() -> PathBuf {
        // A fixed, platform-neutral path (no real filesystem dependency)
        // so the rendered reminder body is stable across CI/dev machines
        // and matches the pinned golden fixture byte-for-byte.
        PathBuf::from("/opt/enforcer/bin/enforcer")
    }

    #[test]
    fn reminder_body_contains_enforcer_first_marker() {
        let body = reminder_body(&sample_binary_path());
        assert!(body.as_str().starts_with(ENFORCER_FIRST_MARKER));
        assert!(body.as_str().contains(ENFORCER_FIRST_MARKER));
    }

    #[test]
    fn reminder_body_names_the_concrete_mcp_tools_and_deny_gate() {
        let body = reminder_body(&sample_binary_path());
        assert!(body.as_str().contains("enforcer scan"));
        assert!(body.as_str().contains("enforcer check"));
        assert!(body
            .as_str()
            .contains("mcp__enforcer__ocentra_enforcer_scan"));
        assert!(body
            .as_str()
            .contains("mcp__enforcer__ocentra_enforcer_check"));
        assert!(body
            .as_str()
            .contains("mcp__enforcer__ocentra_enforcer_coordination_guard"));
        assert!(body.as_str().contains("PreToolUse deny gate"));
    }

    #[test]
    fn reminder_body_carries_every_doctrine_tier_token() {
        let body = reminder_body(&sample_binary_path());
        assert!(body.as_str().contains(TIER_T1_TOKEN));
        assert!(body.as_str().contains(TIER_T2_TOKEN));
        assert!(body.as_str().contains(TIER_T3_TOKEN));
    }

    #[test]
    fn reminder_body_is_deterministic_for_the_same_input() {
        let a = reminder_body(&sample_binary_path());
        let b = reminder_body(&sample_binary_path());
        assert_eq!(a, b, "same input must yield byte-identical output");
    }

    #[test]
    fn session_start_hook_config_dto_rejects_invalid_values() {
        assert!(SessionStartHookConfig::try_from(SessionStartHookConfigDto {
            event: HookEvent::SessionStart,
            matcher: String::new(),
            command: String::new(),
            reminder_body: reminder_body(&sample_binary_path()),
        })
        .is_err());

        assert!(SessionStartHookConfig::try_from(SessionStartHookConfigDto {
            event: HookEvent::SessionStart,
            matcher: String::new(),
            command: "/opt/enforcer/bin/enforcer hooks sessionstart".to_owned(),
            reminder_body: String::new(),
        })
        .is_err());
    }

    #[test]
    fn reminder_body_varies_only_by_the_embedded_binary_path() {
        let a = reminder_body(&sample_binary_path());
        let b = reminder_body(Path::new("/other/enforcer"));
        assert_ne!(a, b);
        // Stripping the two distinct embedded paths must leave the
        // doctrine text identical — the only permitted variation.
        assert!(a.replace("/opt/enforcer/bin/enforcer", "X") == b.replace("/other/enforcer", "X"));
    }

    #[test]
    fn reminder_body_matches_the_pinned_snapshot_byte_for_byte(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let actual = reminder_body(&sample_binary_path());
        let golden = std::fs::read_to_string(fixture_path("reminder.txt"))?;
        assert_eq!(
            actual, golden,
            "reminder body drifted from the pinned snapshot at \
             tests/fixtures/sessionstart_hook/reminder.txt — update the fixture \
             deliberately if this change is intended"
        );
        Ok(())
    }

    #[test]
    fn hook_config_targets_sessionstart_with_an_empty_matcher() {
        let config = sessionstart_hook_config(&sample_binary_path());
        assert_eq!(config.event, HookEvent::SessionStart);
        assert_eq!(config.matcher, "");
    }

    #[test]
    fn hook_config_command_is_a_thin_shim_over_the_binary_path() {
        let config = sessionstart_hook_config(&sample_binary_path());
        assert_eq!(
            config.command,
            "/opt/enforcer/bin/enforcer hooks sessionstart"
        );
    }

    #[test]
    fn hook_config_reminder_body_matches_the_standalone_function() {
        let path = sample_binary_path();
        let config = sessionstart_hook_config(&path);
        assert_eq!(config.reminder_body, reminder_body(&path));
    }

    #[test]
    /// Round-trip proof for the third-party SessionStart hook schema.
    fn session_start_hook_config_dto_round_trip_through_json(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let config = sessionstart_hook_config(&sample_binary_path());
        let wire = serde_json::to_string(&config)?;
        assert!(wire.as_str().contains("\"sessionStart\""));
        assert!(wire.as_str().contains("\"reminderBody\""));
        let back: SessionStartHookConfigDto = serde_json::from_str(&wire)?;
        assert_eq!(back, config);
        Ok(())
    }

    #[test]
    fn malformed_hook_event_is_rejected_at_the_json_boundary() {
        let malformed = serde_json::json!({
            "event": "not-a-hook-event",
            "matcher": "",
            "command": "/opt/enforcer/bin/enforcer hooks sessionstart",
            "reminderBody": "Enforcer-first"
        });
        let decoded = serde_json::from_value::<super::SessionStartHookConfigDto>(malformed);
        assert!(decoded.is_err_and(|error| error.to_string().contains("unknown variant")));
    }

    #[test]
    fn render_settings_entry_shapes_a_claude_hooks_array_entry(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let config = sessionstart_hook_config(&sample_binary_path());
        let value = render_settings_entry(&config);
        assert_eq!(value["matcher"], "");
        let hooks = value["hooks"].as_array().ok_or("expected a hooks array")?;
        assert_eq!(hooks.len(), 1);
        assert_eq!(hooks[0]["type"], "command");
        assert_eq!(hooks[0]["command"], config.command);
        assert_eq!(hooks[0]["additionalContext"], config.reminder_body);
        Ok(())
    }

    #[test]
    fn render_settings_entry_is_idempotent_to_render_twice() {
        let config = sessionstart_hook_config(&sample_binary_path());
        assert_eq!(
            render_settings_entry(&config),
            render_settings_entry(&config)
        );
    }
}
