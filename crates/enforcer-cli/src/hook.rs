//! Harness hook entry points.
//!
//! Claude Code invokes a `PreToolUse` command with the proposed tool call as
//! JSON on stdin.  This module turns that payload into an isolated candidate
//! file, runs the normal `check` command against that candidate, and emits a
//! Claude-native permission decision.  The on-disk target is never modified
//! while the hook is deciding whether the proposed write is safe.

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use enforcer_domain::core_types::ExitCode;
use enforcer_domain::install_types::{
    HookCheckOutcome, HookDecision, HookExitStatus, InstallReportText,
};
use serde::Deserialize;
use serde_json::Value;

use crate::output;

const INTERCEPTED_TOOLS: [&str; 3] = ["Edit", "Write", "MultiEdit"];

#[derive(Debug, Deserialize)]
struct PreToolUsePayload {
    cwd: Option<String>,
    tool_name: Option<String>,
    tool_input: Option<Value>,
}

struct Candidate {
    root: PathBuf,
    relative_path: PathBuf,
}

impl Candidate {
    fn stage(payload: &PreToolUsePayload) -> Result<Self, String> {
        let tool_name = payload
            .tool_name
            .as_deref()
            .ok_or_else(|| "PreToolUse payload is missing tool_name".to_owned())?;
        if !INTERCEPTED_TOOLS.contains(&tool_name) {
            return Err(format!("unsupported PreToolUse tool `{tool_name}`"));
        }
        let input = payload
            .tool_input
            .as_ref()
            .ok_or_else(|| "PreToolUse payload is missing tool_input".to_owned())?;
        let cwd = payload
            .cwd
            .as_deref()
            .map(PathBuf::from)
            .unwrap_or(std::env::current_dir().map_err(|error| error.to_string())?);
        let cwd = fs::canonicalize(&cwd)
            .map_err(|error| format!("cannot resolve hook cwd `{}`: {error}", cwd.display()))?;
        if !cwd.is_dir() {
            return Err(format!("hook cwd `{}` is not a directory", cwd.display()));
        }
        let raw_path = input
            .get("file_path")
            .or_else(|| input.get("path"))
            .and_then(Value::as_str)
            .ok_or_else(|| "PreToolUse tool_input is missing file_path".to_owned())?;
        let target = resolve_target(&cwd, Path::new(raw_path))?;
        let relative_path = target
            .strip_prefix(&cwd)
            .map_err(|error| {
                format!(
                    "candidate path `{}` escaped the hook cwd `{}`: {error}",
                    target.display(),
                    cwd.display()
                )
            })?
            .to_path_buf();
        let content = candidate_content(tool_name, &target, input)?;

        let root = unique_temp_root()?;
        let destination = root.join(&relative_path);
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("cannot create candidate directory: {error}"))?;
        }
        fs::write(&destination, content).map_err(|error| {
            format!(
                "cannot stage candidate `{}`: {error}",
                destination.display()
            )
        })?;
        let config = cwd.join("ocentra-enforcer.config.json");
        if config.is_file() {
            fs::copy(&config, root.join("ocentra-enforcer.config.json"))
                .map_err(|error| format!("cannot stage enforcer config: {error}"))?;
        }
        Ok(Self {
            root,
            relative_path,
        })
    }
}

impl Drop for Candidate {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn unique_temp_root() -> Result<PathBuf, String> {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| format!("system clock is before epoch: {error}"))?
        .as_nanos();
    let base = std::env::temp_dir();
    let root = base.join(format!(
        "enforcer-pretooluse-{}-{stamp}",
        std::process::id()
    ));
    fs::create_dir_all(&root)
        .map_err(|error| format!("cannot create hook staging root: {error}"))?;
    Ok(root)
}

fn resolve_target(cwd: &Path, raw: &Path) -> Result<PathBuf, String> {
    let joined = if raw.is_absolute() {
        raw.to_path_buf()
    } else {
        cwd.join(raw)
    };
    let file_name = joined
        .file_name()
        .ok_or_else(|| "PreToolUse file_path must name a file".to_owned())?;
    let parent = joined
        .parent()
        .ok_or_else(|| "PreToolUse file_path has no parent".to_owned())?;
    let parent = fs::canonicalize(parent).map_err(|error| {
        format!(
            "cannot resolve candidate parent `{}`: {error}",
            parent.display()
        )
    })?;
    let target = parent.join(file_name);
    if !target.starts_with(cwd) {
        return Err("PreToolUse file_path escapes the hook cwd".to_owned());
    }
    match fs::symlink_metadata(&target) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            return Err(
                "PreToolUse file_path targets a symlink; refusing an ambiguous write".to_owned(),
            );
        }
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => {
            return Err(format!(
                "cannot inspect candidate target `{}`: {error}",
                target.display()
            ));
        }
    }
    Ok(target)
}

fn candidate_content(tool_name: &str, target: &Path, input: &Value) -> Result<String, String> {
    match tool_name {
        "Write" => input
            .get("content")
            .and_then(Value::as_str)
            .map(str::to_owned)
            .ok_or_else(|| "Write tool_input is missing string content".to_owned()),
        "Edit" => {
            let current = fs::read_to_string(target).map_err(|error| {
                format!("cannot read Edit target `{}`: {error}", target.display())
            })?;
            apply_edit(&current, input)
        }
        "MultiEdit" => {
            let mut current = fs::read_to_string(target).map_err(|error| {
                format!(
                    "cannot read MultiEdit target `{}`: {error}",
                    target.display()
                )
            })?;
            let edits = input
                .get("edits")
                .and_then(Value::as_array)
                .ok_or_else(|| "MultiEdit tool_input is missing edits".to_owned())?;
            for edit in edits {
                current = apply_edit(&current, edit)?;
            }
            Ok(current)
        }
        _ => Err(format!("unsupported PreToolUse tool `{tool_name}`")),
    }
}

fn apply_edit(current: &str, input: &Value) -> Result<String, String> {
    let old = input
        .get("old_string")
        .and_then(Value::as_str)
        .ok_or_else(|| "edit payload is missing old_string".to_owned())?;
    let new = input
        .get("new_string")
        .and_then(Value::as_str)
        .ok_or_else(|| "edit payload is missing new_string".to_owned())?;
    if old.is_empty() {
        return Err("edit payload old_string must not be empty".to_owned());
    }
    let occurrences = current.match_indices(old).count();
    if occurrences == 0 {
        return Err("edit payload old_string was not found in the target".to_owned());
    }
    let replace_all = input
        .get("replace_all")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if !replace_all && occurrences != 1 {
        return Err(
            "edit payload old_string matched more than once without replace_all".to_owned(),
        );
    }
    if replace_all {
        Ok(current.replace(old, new))
    } else {
        let index = current
            .find(old)
            .ok_or_else(|| "edit payload old_string was not found in the target".to_owned())?;
        let before = current
            .get(..index)
            .ok_or_else(|| "edit payload match boundary was invalid".to_owned())?;
        let after_start = index
            .checked_add(old.len())
            .ok_or_else(|| "edit payload match boundary overflowed".to_owned())?;
        let after = current
            .get(after_start..)
            .ok_or_else(|| "edit payload match boundary was invalid".to_owned())?;
        let mut result = String::with_capacity(current.len() - old.len() + new.len());
        result.push_str(before);
        result.push_str(new);
        result.push_str(after);
        Ok(result)
    }
}

fn execute_check(candidate: &Candidate) -> Result<HookCheckOutcome, String> {
    let binary = std::env::current_exe().map_err(|error| error.to_string())?;
    let output_path = candidate.root.join("pretooluse-check.stdout");
    let stdout_file = fs::File::create(&output_path)
        .map_err(|error| format!("cannot create hook check output file: {error}"))?;
    let mut child = Command::new(binary)
        .args(["check", candidate.relative_path.to_string_lossy().as_ref()])
        .current_dir(&candidate.root)
        .stdin(Stdio::null())
        .stdout(Stdio::from(stdout_file))
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| format!("cannot run enforcer check for hook candidate: {error}"))?;
    let timeout = Duration::from_secs(enforcer_install::hooks::pretooluse::CHECK_TIMEOUT_SECONDS);
    let deadline = Instant::now()
        .checked_add(timeout)
        .ok_or_else(|| "hook check timeout deadline overflowed".to_owned())?;
    let status = loop {
        match child
            .try_wait()
            .map_err(|error| format!("cannot poll enforcer check for hook candidate: {error}"))?
        {
            Some(status) => break status,
            None if Instant::now() >= deadline => {
                let kill_result = child.kill();
                let wait_result = child.wait();
                if let Err(error) = kill_result {
                    return Err(format!(
                        "enforcer check timed out and could not be terminated: {error}"
                    ));
                }
                if let Err(error) = wait_result {
                    return Err(format!(
                        "enforcer check timed out and could not be reaped: {error}"
                    ));
                }
                return Err(format!(
                    "enforcer check exceeded {} seconds; fail-closed to deny",
                    enforcer_install::hooks::pretooluse::CHECK_TIMEOUT_SECONDS
                ));
            }
            None => thread::sleep(Duration::from_millis(25)),
        }
    };
    let stdout = fs::read_to_string(&output_path)
        .map_err(|error| format!("cannot read enforcer check output: {error}"))?;
    fs::remove_file(&output_path)
        .map_err(|error| format!("cannot remove enforcer check output: {error}"))?;
    let exit_status = match status.code() {
        Some(0) => HookExitStatus::Success,
        Some(code) => std::num::NonZeroI32::new(code)
            .map(HookExitStatus::Failure)
            .unwrap_or(HookExitStatus::Success),
        None => HookExitStatus::Unavailable,
    };
    Ok(HookCheckOutcome {
        exit_status,
        stdout: InstallReportText::try_from(stdout).map_err(|error| error.to_string())?,
    })
}

fn deny_pretooluse(reason: String) -> ExitCode {
    match InstallReportText::try_from(reason) {
        Ok(reason) => output::print_pretooluse_decision(&HookDecision::Deny { reason }),
        Err(error) => output::print_internal_error(&format!(
            "cannot encode PreToolUse denial reason; fail-closed to deny: {error}"
        )),
    }
    ExitCode::Violations
}

/// Execute the Claude `PreToolUse` stdin bridge.
pub fn run_pretooluse() -> ExitCode {
    let mut raw = String::new();
    if let Err(error) = std::io::stdin().read_to_string(&mut raw) {
        return deny_pretooluse(format!(
            "cannot read PreToolUse payload; fail-closed to deny: {error}"
        ));
    }
    let payload: PreToolUsePayload = match serde_json::from_str(&raw) {
        Ok(payload) => payload,
        Err(error) => {
            return deny_pretooluse(format!(
                "malformed PreToolUse payload; fail-closed to deny: {error}"
            ));
        }
    };
    if payload
        .tool_name
        .as_deref()
        .is_some_and(|tool| !INTERCEPTED_TOOLS.contains(&tool))
    {
        output::print_pretooluse_decision(&HookDecision::Allow);
        return ExitCode::Success;
    }
    let candidate = match Candidate::stage(&payload) {
        Ok(candidate) => candidate,
        Err(error) => {
            return deny_pretooluse(format!(
                "cannot stage proposed PreToolUse edit; fail-closed to deny: {error}"
            ));
        }
    };
    let outcome = match execute_check(&candidate) {
        Ok(outcome) => outcome,
        Err(error) => {
            return deny_pretooluse(format!(
                "cannot execute PreToolUse enforcement; fail-closed to deny: {error}"
            ));
        }
    };
    let decision = match enforcer_install::hooks::pretooluse::classify(&outcome) {
        Ok(decision) => decision,
        Err(error) => {
            return deny_pretooluse(format!(
                "cannot classify PreToolUse enforcement result; fail-closed to deny: {error}"
            ))
        }
    };
    let exit = if matches!(decision, HookDecision::Deny { .. }) {
        ExitCode::Violations
    } else {
        ExitCode::Success
    };
    output::print_pretooluse_decision(&decision);
    exit
}
