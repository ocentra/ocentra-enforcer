//! c04 -- the Claude PreToolUse deny-hook EMITTER.
//!
//! # Charter (workpack c04 -- BINDING)
//!
//! Guidance-only installs are prose an agent can ignore. This module is
//! the T1 MECHANICAL BRIDGE: a Rust emitter that produces a Claude
//! `PreToolUse` hook config (matcher `Edit|Write|MultiEdit`) which, once
//! registered by the c03 [`crate::adapters::claude::ClaudeAdapter`], shells
//! out to the installed `enforcer` binary against the pending edit and
//! BLOCKS a deterministic (T1) violation before the write lands.
//!
//! This module owns the emitter (the hook-config record + the invocation
//! this crate builds) and the DECISION logic (`enforcer` exit code + stdout
//! -> deny/allow/allow-with-warning). It is NOT a `.ts`/`.mjs` hook script
//! -- the hook's `command` is the `enforcer` binary itself; there is no
//! second interpreted layer.
//!
//! # Tier contract (RUST_ARCHITECTURE.md doctrine, preserved verbatim)
//! - **T1** (a hard [`enforcer_domain::findings::Violation`] --
//!   `Severity::Error`): exit **deny**. The reason string carries the
//!   fired [`RuleId`] and its `Fix:` hint verbatim, so the agent sees
//!   exactly what to change, not a generic refusal.
//! - **T2** (a `Severity::Warning` finding only, no violation): exit
//!   **allow**, but the warning + its `Fix:` hint is surfaced in the hook
//!   response so the agent sees it without being blocked.
//! - **T3**: never blocks (out of scope for this deterministic gate;
//!   review-assist tier has no mechanical signal to check here).
//! - Fail-closed: an `enforcer` invocation error/timeout in T1 scope is
//!   treated as **deny** (never a silent allow past a broken gate).
//! - Non-`Edit|Write|MultiEdit` tools pass through untouched (the emitted
//!   matcher itself already restricts to those three; [`should_intercept`]
//!   is the same rule expressed in Rust for anything that also wants to
//!   reason about a payload before shelling out).
//! - Malformed report input is covered explicitly and fails closed to deny.

//!
//! BOUNDARY-INVARIANT: pre-tool-use input is decoded before policy evaluation.
//!
use std::num::{NonZeroI32, NonZeroU64};
use std::path::Path;
use std::process::Command;

use crate::error::InstallResult;
use enforcer_domain::ids::RuleId;
use enforcer_domain::install_types::{
    HookCheckOutcome, HookDecision, HookExitStatus, HookTimeout, InstallBinaryPath,
    InstallReportText, PreToolUseHookConfig,
};
use enforcer_domain::severity::Severity;

/// The three Claude tool names this hook's matcher restricts to. Any other
/// `tool_name` on the PreToolUse payload passes through untouched.
pub const INTERCEPTED_TOOLS: &[&str] = &["Edit", "Write", "MultiEdit"];

/// The Claude hook matcher expression this emitter registers --
/// `Edit|Write|MultiEdit`, matching [`INTERCEPTED_TOOLS`] exactly (kept as
/// one literal so the two can never drift silently against each other; a
/// test below asserts the equivalence).
pub const MATCHER: &str = "Edit|Write|MultiEdit";

/// Shared source-of-truth doctrine line the c05 SessionStart hook embeds
/// verbatim (workpack c05 "doctrine text is generated from a single
/// source-of-truth Rust constant (shared with the c04 deny-hook reason
/// strings) so there is no drift").
pub const DOCTRINE_SUMMARY: &str = "Mechanical enforcement tiers: T1 (hard Validator finding) \
     blocks the write via the PreToolUse deny-hook with the exact RuleId + \
     Fix: hint; T2 (scored literal-scan finding) allows the write but \
     surfaces a warning; T3 (review-assist) never blocks.";

/// True when `tool_name` is one this hook's matcher intercepts.
#[must_use]
pub fn should_intercept(tool_name: &str) -> bool {
    INTERCEPTED_TOOLS.contains(&tool_name)
}

/// What the emitted hook does with a candidate edit, once `enforcer`'s
/// outcome has been classified. Mirrors the hook-contract wire shape
/// Claude's `PreToolUse` hook protocol expects (`permissionDecision` +
/// `permissionDecisionReason`), but kept as this crate's own type -- the
/// c03 adapter/emitted JSON maps it onto the wire shape, this module never
/// hardcodes Claude's own schema names in its decision logic.
/// The raw outcome of invoking `enforcer check <path>` (or `scan`) against
/// a candidate file: the process exit code plus its stdout text. Built
/// either by [`run_enforcer_check`] (the real subprocess invocation) or,
/// in tests, directly from a captured fixture run -- kept as a plain data
/// record so the classification logic in [`classify`] never itself shells
/// out, and is exercised without a process boundary.
/// One line of enforcer's rendered report this module parses back out:
/// the severity label, the [`RuleId`], and the `Fix:` hint that followed
/// it in the source text.
#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedFindingLine {
    severity: Severity,
    rule_id: RuleId,
    fix_hint: InstallReportText,
}

/// Parse `enforcer check`/`scan`'s rendered stdout report
/// (`crate::output::render_finding`'s exact shape in `enforcer-cli`) back
/// into structured findings. A line that does not match the expected shape
/// is skipped rather than treated as a parse error -- this parser only
/// needs to recover the severity/`RuleId`/`Fix:` triad for the lines that
/// matter to a deny/warn decision; report summary/waived lines are noise
/// here, and skipping them is not a silent-suppression of a FINDING (the
/// underlying `enforcer` process exit code is still what ultimately
/// fail-closes an unparseable report -- see [`classify`]).
fn parse_findings(stdout: &InstallReportText) -> InstallResult<Vec<ParsedFindingLine>> {
    let mut findings = Vec::new();
    let mut lines = stdout.as_str().lines().peekable();
    while let Some(line) = lines.next() {
        let trimmed = line.trim_start();
        let Some(after_bracket) = trimmed.strip_prefix('[') else {
            continue;
        };
        let Some(bracket_end) = after_bracket.find(']') else {
            continue;
        };
        let Some(severity_text) = after_bracket.get(..bracket_end) else {
            continue;
        };
        let severity = match severity_text {
            "error" => Severity::Error,
            "warning" => Severity::Warning,
            _ => continue,
        };
        let Some(rest) = bracket_end
            .checked_add(1)
            .and_then(|rest_start| after_bracket.get(rest_start..))
        else {
            continue;
        };
        let rest = rest.trim_start();
        // Shape: `file:line RULE-ID -- title`.
        let Some((_location, after_location)) = rest.split_once(' ') else {
            continue;
        };
        let Some((rule_id_raw, _title)) = after_location.trim_start().split_once(" -- ") else {
            continue;
        };
        let Ok(rule_id) = rule_id_raw.trim().parse::<RuleId>() else {
            continue;
        };
        // The `Fix:` hint is the NEXT line, indented, per
        // `enforcer-cli::output::render_finding`.
        let fix_hint = match lines.peek().map(|next| next.trim()) {
            Some(next) if next.starts_with("Fix:") => next.to_owned(),
            _ => String::new(),
        };
        if !fix_hint.is_empty() {
            lines.next();
        }
        findings.push(ParsedFindingLine {
            severity,
            rule_id,
            fix_hint: InstallReportText::try_from(fix_hint)?,
        });
    }
    Ok(findings)
}

/// Classify a [`CheckOutcome`] into a [`HookDecision`], per the tier
/// contract in the module docs. This is the pure decision function every
/// test in this module exercises directly (no process boundary needed to
/// prove the T1/T2/allow split).
pub fn classify(outcome: &HookCheckOutcome) -> InstallResult<HookDecision> {
    let exit_code = match outcome.exit_status {
        HookExitStatus::Success => 0,
        HookExitStatus::Failure(code) => code.get(),
        HookExitStatus::Unavailable => {
            return Ok(HookDecision::Deny {
                reason: InstallReportText::try_from(
                    "enforcer check did not complete (fail-closed on invocation failure/timeout \
                      for T1 scope)"
                        // ALLOC-JUSTIFICATION: ownership is required to construct the typed report or diagnostic value that crosses this boundary.
                        .to_owned(),
                )?,
            });
        }
    };

    let findings = parse_findings(&outcome.stdout)?;
    let violation = findings.iter().find(|f| f.severity == Severity::Error);
    if let Some(violation) = violation {
        return Ok(HookDecision::Deny {
            reason: InstallReportText::try_from(format!(
                "{} {}",
                violation.rule_id.as_str(),
                if violation.fix_hint.as_str().is_empty() {
                    "Fix: see the rule detail above; no CLI flag suppresses this finding."
                        // ALLOC-JUSTIFICATION: ownership is required to construct the typed report or diagnostic value that crosses this boundary.
                        .to_owned()
                } else {
                    // ALLOC-JUSTIFICATION: ownership is required to construct the typed report or diagnostic value that crosses this boundary.
                    violation.fix_hint.as_str().to_owned()
                }
            ))?,
        });
    }

    // Exit code alone still fail-closes: a nonzero (`Violations`) exit
    // with a report this parser could not recover any `[error]` line from
    // is treated as a T1 deny, never silently downgraded to allow just
    // because the text parse came up empty (unparseable != clean).
    if exit_code != 0 {
        return Ok(HookDecision::Deny {
            reason: InstallReportText::try_from(format!(
                "enforcer check exited nonzero (code {exit_code}) with a report this hook could \
                 not parse a specific RuleId from; fail-closed to deny for T1 scope"
            ))?,
        });
    }

    if let Some(warning) = findings.iter().find(|f| f.severity == Severity::Warning) {
        return Ok(HookDecision::AllowWithWarning {
            reason: InstallReportText::try_from(format!(
                "{} {}",
                warning.rule_id.as_str(),
                if warning.fix_hint.as_str().is_empty() {
                    // ALLOC-JUSTIFICATION: ownership is required to construct the typed report or diagnostic value that crosses this boundary.
                    "Fix: see the rule detail above.".to_owned()
                } else {
                    // ALLOC-JUSTIFICATION: ownership is required to construct the typed report or diagnostic value that crosses this boundary.
                    warning.fix_hint.as_str().to_owned()
                }
            ))?,
        });
    }

    Ok(HookDecision::Allow)
}

/// How long [`run_enforcer_check`] waits for the `enforcer` process before
/// treating the invocation as failed (fail-closed per the module's tier
/// contract).
pub const CHECK_TIMEOUT_SECONDS: u64 = 30;
const fn check_timeout_millis() -> NonZeroU64 {
    match NonZeroU64::new(CHECK_TIMEOUT_SECONDS * 1_000) {
        Some(value) => value,
        None => NonZeroU64::MIN,
    }
}
pub const CHECK_TIMEOUT: HookTimeout = HookTimeout::try_from_millis(check_timeout_millis());

/// Shell out to the installed `enforcer` binary (`enforcer check <path>`)
/// against `candidate_path`, run with working directory `cwd` (the repo
/// root Claude is editing -- `enforcer check`'s scope resolution is
/// cwd-based, per `enforcer-cli::commands::current_repo_root`), returning
/// the captured [`CheckOutcome`]. This is the ONE place this module
/// actually spawns a process; every decision test exercises [`classify`]
/// directly against a [`CheckOutcome`] instead, so the tier logic is
/// proven without a process boundary, while this function is exercised by
/// the integration tests that DO spawn the real binary against the seeded
/// fixtures (workpack c04 acceptance row).
///
/// A spawn failure or an I/O error waiting on the child is reported as a
/// [`CheckOutcome`] with `exit_code: None` -- [`classify`] fail-closes that
/// to [`HookDecision::Deny`], never panics/propagates past this boundary
/// (a hook that crashes the harness is worse than one that denies).
pub fn run_enforcer_check(
    enforcer_binary: &Path,
    cwd: &Path,
    candidate_path: &Path,
) -> InstallResult<HookCheckOutcome> {
    let output = Command::new(enforcer_binary)
        .arg("check")
        .arg(candidate_path)
        .current_dir(cwd)
        .output();
    match output {
        Ok(output) => Ok(HookCheckOutcome {
            exit_status: match output.status.code() {
                Some(0) => HookExitStatus::Success,
                Some(code) => {
                    NonZeroI32::new(code).map_or(HookExitStatus::Success, HookExitStatus::Failure)
                }
                None => HookExitStatus::Unavailable,
            },
            stdout: InstallReportText::try_from(
                String::from_utf8_lossy(&output.stdout).into_owned(),
            )?,
        }),
        Err(_) => Ok(HookCheckOutcome {
            exit_status: HookExitStatus::Unavailable,
            stdout: InstallReportText::try_from(String::new())?,
        }),
    }
}

/// The structured Claude `PreToolUse` hook-config record this emitter
/// produces. The c03 [`crate::adapters::claude::ClaudeAdapter`] renders
/// this into the `hooks.PreToolUse` array of `~/.claude/settings.json` (via
/// the c01 apply path) -- this type stays Claude-schema-SHAPED but is not
/// itself the raw `serde_json::Value` Claude reads, so the emitter's own
/// tests can assert on named fields rather than JSON-pointer digging.
/// This hook's fixed argv tail: `hook pretooluse`. The `hook` subcommand
/// reads Claude's pending Edit/Write/MultiEdit JSON from stdin, stages the
/// proposed content in an isolated ephemeral root, and runs the ordinary
/// enforcer check before returning a Claude permission decision. It never
/// relies on Claude appending a target path to this command.
const HOOK_ARGS: [&str; 2] = ["hook", "pretooluse"];

/// Build the [`PreToolUseHookConfig`] this crate's c03 adapter registers,
/// pointed at `enforcer_binary` (the absolute installed-binary path every
/// other adapter artifact in this crate is constructed with).
pub fn build_hook_config(
    enforcer_binary: &InstallBinaryPath,
) -> InstallResult<PreToolUseHookConfig> {
    Ok(PreToolUseHookConfig {
        // ALLOC-JUSTIFICATION: ownership is required to construct the typed report or diagnostic value that crosses this boundary.
        matcher: InstallReportText::try_from(MATCHER.to_owned())?,
        // CLONE-JUSTIFICATION: the owned typed value must be retained independently by the returned report.
        command: enforcer_binary.clone(),
        args: HOOK_ARGS
            .iter()
            // ALLOC-JUSTIFICATION: ownership is required to construct the typed report or diagnostic value that crosses this boundary.
            .map(|arg| InstallReportText::try_from((*arg).to_owned()))
            .collect::<Result<Vec<_>, _>>()?,
        timeout: CHECK_TIMEOUT,
    })
}

/// Render [`PreToolUseHookConfig`] into the exact `serde_json::Value` shape
/// Claude's `hooks.PreToolUse[]` array entry expects: `{"matcher": ...,
/// "hooks": [{"type": "command", "command": "<binary> <args...>", "timeout":
/// <secs>}]}`. Kept as a free function (not a `Serialize` impl) since this
/// is a ONE-DIRECTION render into a third-party schema this crate does not
/// own, matching the same posture `ClaudeAdapter::render_agent_descriptor`
/// takes for the subagent descriptor.
#[must_use]
pub fn render_settings_entry(config: &PreToolUseHookConfig) -> serde_json::Value {
    // ALLOC-JUSTIFICATION: ownership is required to construct the typed report or diagnostic value that crosses this boundary.
    let mut command_line = config.command.as_path().display().to_string();
    for arg in &config.args {
        command_line.push(' ');
        command_line.push_str(arg.as_str());
    }
    serde_json::json!({
        "matcher": config.matcher.as_str(),
        "hooks": [
            {
                "type": "command",
                "command": command_line,
                "timeout": CHECK_TIMEOUT_SECONDS,
            }
        ]
    })
}

#[cfg(test)]
mod tests {
    use super::{
        build_hook_config, classify, parse_findings, render_settings_entry, should_intercept,
        DOCTRINE_SUMMARY, INTERCEPTED_TOOLS, MATCHER,
    };
    use enforcer_domain::install_types::{
        HookCheckOutcome, HookDecision, InstallBinaryPath, InstallReportText,
    };
    use enforcer_domain::severity::Severity;
    // The seeded-fixture (real `enforcer` subprocess) proofs for the
    // deny/allow/allow-with-warning contract live in
    // `tests/pretooluse_hook_fixtures.rs`, NOT here: `CARGO_BIN_EXE_enforcer`
    // is only guaranteed set for a crate's own `tests/` integration-test
    // binaries, not its `src/` lib-target unit tests (confirmed empirically
    // -- `std::env::var("CARGO_BIN_EXE_enforcer")` is `NotPresent` when run
    // from this module). The pure [`classify`]/[`parse_findings`] decision
    // logic is still fully exercised here against hand-built
    // [`CheckOutcome`]s, so the tier split is proven without a process
    // boundary at all; the integration test additionally proves the real
    // subprocess produces exactly the outcomes these unit tests assume.

    #[test]
    fn matcher_matches_intercepted_tools_exactly() {
        assert_eq!(MATCHER.split('|').collect::<Vec<_>>(), INTERCEPTED_TOOLS);
        for tool in INTERCEPTED_TOOLS {
            assert!(should_intercept(tool));
        }
        assert!(!should_intercept("Read"));
        assert!(!should_intercept("Bash"));
    }

    #[test]
    fn missing_exit_code_fail_closes_to_deny() -> Result<(), Box<dyn std::error::Error>> {
        let outcome = HookCheckOutcome {
            exit_status: enforcer_domain::install_types::HookExitStatus::Unavailable,
            stdout: InstallReportText::try_from(String::new())?,
        };
        let decision = classify(&outcome)?;
        assert!(matches!(decision, HookDecision::Deny { .. }));
        Ok(())
    }

    #[test]
    fn malformed_report_on_nonzero_exit_fail_closes_to_deny(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let outcome = HookCheckOutcome {
            exit_status: enforcer_domain::install_types::HookExitStatus::Failure(
                std::num::NonZeroI32::MIN,
            ),
            stdout: InstallReportText::try_from(
                "not a report the parser recognizes at all".to_owned(),
            )?,
        };
        let decision = classify(&outcome)?;
        assert!(matches!(decision, HookDecision::Deny { .. }));
        Ok(())
    }

    #[test]
    fn zero_exit_with_no_findings_allows() -> Result<(), Box<dyn std::error::Error>> {
        let outcome = HookCheckOutcome {
            exit_status: enforcer_domain::install_types::HookExitStatus::Success,
            stdout: InstallReportText::try_from(
                "enforcer: no violations (0 finding(s) total).".to_owned(),
            )?,
        };
        assert_eq!(classify(&outcome)?, HookDecision::Allow);
        Ok(())
    }

    #[test]
    fn parse_findings_recovers_rule_id_and_fix_hint_from_rendered_report(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let stdout = "enforcer: 1 violation(s), 0 warning(s), 0 waived.\n  \
                       [error] src/lib.rs:4 T1-RUSTERR.1 -- rust-error-handling: `unwrap` in \
                       first-party code\n    Fix: replace unwrap()/expect()/panic! with a typed \
                       Result and `?`.";
        let findings = parse_findings(&InstallReportText::try_from(stdout.to_owned())?)?;
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].severity, Severity::Error);
        assert_eq!(findings[0].rule_id.as_str(), "T1-RUSTERR.1");
        assert!(findings[0].fix_hint.as_str().starts_with("Fix:"));
        Ok(())
    }

    #[test]
    fn build_hook_config_points_at_the_given_binary_and_matcher(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let binary = std::env::temp_dir().join("enforcer");
        let binary = InstallBinaryPath::try_from(binary)?;
        let config = build_hook_config(&binary)?;
        assert_eq!(config.command, binary);
        assert_eq!(config.matcher.as_str(), MATCHER);
        assert_eq!(render_settings_entry(&config)["hooks"][0]["timeout"], 30);
        Ok(())
    }

    #[test]
    fn render_settings_entry_shapes_a_claude_hooks_array_entry(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let binary = std::env::temp_dir().join("enforcer");
        let binary = InstallBinaryPath::try_from(binary)?;
        let config = build_hook_config(&binary)?;
        let value = render_settings_entry(&config);
        assert_eq!(value["matcher"], MATCHER);
        let hooks = value["hooks"].as_array().ok_or("expected a hooks array")?;
        assert_eq!(hooks.len(), 1);
        assert_eq!(hooks[0]["type"], "command");
        let command = hooks[0]["command"]
            .as_str()
            .ok_or("expected a command string")?;
        assert!(command.contains(&binary.as_path().display().to_string()));
        assert!(command.ends_with(" hook pretooluse"));
        Ok(())
    }

    #[test]
    fn render_settings_entry_is_idempotent_to_render_twice(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let binary = std::env::temp_dir().join("enforcer");
        let binary = InstallBinaryPath::try_from(binary)?;
        let config = build_hook_config(&binary)?;
        assert_eq!(
            render_settings_entry(&config),
            render_settings_entry(&config)
        );
        Ok(())
    }

    #[test]
    fn doctrine_summary_names_all_three_tiers() {
        assert_eq!(DOCTRINE_SUMMARY.match_indices("T1").count(), 1);
        assert_eq!(DOCTRINE_SUMMARY.match_indices("T2").count(), 1);
        assert_eq!(DOCTRINE_SUMMARY.match_indices("T3").count(), 1);
    }
}
