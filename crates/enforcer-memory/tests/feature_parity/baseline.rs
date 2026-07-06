//! The [`BaselineAdapter`] seam: the live comparison against an
//! installed `codebase-memory-mcp` baseline
//! (`MEMORY_RETRIEVAL_PARITY_HARNESS.md` §0/§1 -- "baseline: installed
//! codebase-memory-mcp MCP/CLI"). This pass extends the X06.9 skeleton's
//! [`BaselineState::NotInstalled`]-only seam with a real CLI-mode driver
//! ([`CliDriver`]): it spawns the installed binary as `<binary> cli
//! <tool> '<json>'` (the documented CLI equivalent surface --
//! `docs/plans/enforcer-selfhost-plan/refs/x06-baseline-tool-schemas.md`
//! §16: default output is the unwrapped inner JSON on stdout, exit
//! 0/1) and avoids JSON-RPC session management entirely.
//!
//! # Why a driver now
//!
//! `MEMORY_RETRIEVAL_PARITY_HARNESS.md` §6 (non-acceptance cases) is
//! explicit that a stub/placeholder tool result is a harness FAILURE --
//! so this module never fabricates a baseline response. Every method on
//! [`BaselineAdapter`] either returns real data from a live probe/call or
//! surfaces [`BaselineState::NotInstalled`] / a capability-state value;
//! there is no code path that invents a plausible-looking fake result.
//! [`CliDriver::call`] propagates real stdout/stderr/exit-status; it
//! never synthesizes a response when the process fails to run.

use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, Instant};

/// Whether the baseline binary is available to drive at all, and if
/// not, why. Mirrors the capability-state doctrine already used by
/// `enforcer_memory::embed::LoadState` (D-03) -- never silently
/// upgraded to "installed" if the probe did not actually succeed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BaselineState {
    /// The `codebase-memory-mcp` binary was not found on `PATH` (or the
    /// configured override path). Honest fact this harness must record,
    /// never treated as an error.
    NotInstalled,
    /// The binary was found at `path` but has not been version-probed
    /// yet.
    FoundUnprobed { path: PathBuf },
}

impl BaselineState {
    pub fn is_installed(&self) -> bool {
        !matches!(self, BaselineState::NotInstalled)
    }
}

/// The seam a live parity runner drives. Every method takes `&self`
/// (never `&mut self`) because probing/querying a baseline process is
/// inherently a read-only comparison operation from this harness's
/// point of view.
pub trait BaselineAdapter {
    /// Human-readable adapter name, e.g. `"codebase-memory-mcp"`, for
    /// proof-artifact labeling.
    fn name(&self) -> &str;

    /// Probe whether the baseline is available to drive right now.
    /// Must be side-effect-free beyond the probe itself (no indexing,
    /// no writes) -- `MEMORY_RETRIEVAL_PARITY_HARNESS.md` §0 requires
    /// "same repo fixture, same git commit" for both systems, so a
    /// probe must never mutate shared state a later real run depends
    /// on.
    fn probe(&self) -> BaselineState;
}

/// The concrete adapter for `codebase-memory-mcp`
/// (`MEMORY_RETRIEVAL_PARITY_HARNESS.md`'s named baseline). Probes
/// `PATH` for a binary named `binary_name` (configurable so tests can
/// probe a name guaranteed absent without depending on the real
/// binary's actual name matching exactly).
pub struct CodebaseMemoryMcpAdapter {
    binary_name: String,
}

impl CodebaseMemoryMcpAdapter {
    /// The baseline's real binary name per
    /// `MEMORY_RETRIEVAL_PARITY_HARNESS.md`/`MEMORY_RETRIEVAL_OWNER_INTENT.md`
    /// (`codebase-memory-mcp`, from <https://github.com/DeusData/codebase-memory-mcp>).
    pub const DEFAULT_BINARY_NAME: &'static str = "codebase-memory-mcp";

    pub fn new() -> Self {
        Self {
            binary_name: Self::DEFAULT_BINARY_NAME.to_string(),
        }
    }

    /// Construct an adapter probing for a specific binary name --
    /// exposed so tests can assert the [`BaselineState::NotInstalled`]
    /// path deterministically against a name guaranteed not to exist,
    /// independent of whether the real baseline happens to be
    /// installed on the machine running the test.
    pub fn with_binary_name(binary_name: impl Into<String>) -> Self {
        Self {
            binary_name: binary_name.into(),
        }
    }

    /// Locate `self.binary_name` on `PATH` using the platform's own
    /// lookup rule (`where` on Windows, `which`-equivalent elsewhere),
    /// so this respects `PATHEXT`/executable-bit rules this harness
    /// would otherwise have to reimplement.
    fn locate_on_path(&self) -> Option<PathBuf> {
        let (probe_cmd, probe_arg): (&str, &str) = if cfg!(windows) {
            ("where", "")
        } else {
            ("which", "")
        };
        let mut command = Command::new(probe_cmd);
        if !probe_arg.is_empty() {
            command.arg(probe_arg);
        }
        command.arg(&self.binary_name);
        let output = command.output().ok()?;
        if !output.status.success() {
            return None;
        }
        let stdout = String::from_utf8_lossy(&output.stdout);
        let first_line = stdout.lines().next()?.trim();
        if first_line.is_empty() {
            None
        } else {
            Some(PathBuf::from(first_line))
        }
    }
}

impl Default for CodebaseMemoryMcpAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl BaselineAdapter for CodebaseMemoryMcpAdapter {
    fn name(&self) -> &str {
        "codebase-memory-mcp"
    }

    fn probe(&self) -> BaselineState {
        match self.locate_on_path() {
            Some(path) => BaselineState::FoundUnprobed { path },
            None => BaselineState::NotInstalled,
        }
    }
}

/// One real invocation of `<binary> cli <tool> <json>`: the raw
/// stdout/stderr, exit status, and measured wall-clock latency. Never
/// constructed from anything but an actual spawned child process --
/// there is no "synthetic" constructor, matching this module's
/// anti-fabrication doctrine.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct CliCallResult {
    pub tool: String,
    pub request_json: String,
    pub stdout: String,
    pub stderr: String,
    pub exit_success: bool,
    pub latency_ms: f64,
}

impl CliCallResult {
    /// Parse `stdout` as JSON. CLI mode's default (non-`--json`) output
    /// is the unwrapped inner tool JSON on stdout (per
    /// `x06-baseline-tool-schemas.md` §16) -- but stdout also carries
    /// `level=info ...` log lines ahead of the JSON body on some builds
    /// (observed on the installed 0.8.1 binary), so this takes the
    /// LAST line that parses as JSON rather than assuming stdout is
    /// pure JSON. Returns `None` (never a fabricated empty object) when
    /// no line parses.
    pub fn parsed_json(&self) -> Option<serde_json::Value> {
        for line in self.stdout.lines().rev() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(trimmed) {
                return Some(value);
            }
        }
        // Some responses are pretty-printed multi-line JSON with no
        // single-line log prefix mixed in -- fall back to parsing the
        // whole non-log-prefixed stdout as one document.
        let non_log: String = self
            .stdout
            .lines()
            .filter(|line| !line.trim_start().starts_with("level="))
            .collect::<Vec<_>>()
            .join("\n");
        serde_json::from_str(non_log.trim()).ok()
    }
}

/// Errors specific to spawning/running the CLI driver itself (distinct
/// from a tool call that ran but returned `isError`/a non-zero exit --
/// that is a normal [`CliCallResult`], not a [`CliDriverError`]).
#[derive(Debug, thiserror::Error)]
pub enum CliDriverError {
    #[error("baseline binary not installed (BaselineState::NotInstalled)")]
    NotInstalled,
    #[error("failed to spawn baseline process: {0}")]
    Spawn(#[source] std::io::Error),
}

/// Drives the installed `codebase-memory-mcp` binary's `cli <tool>
/// <json>` subcommand form -- the CLI mode explicitly documented as
/// avoiding JSON-RPC session management
/// (`x06-baseline-tool-schemas.md` §16).
#[derive(Debug)]
pub struct CliDriver {
    binary_path: PathBuf,
}

impl CliDriver {
    /// Construct a driver for an already-probed, installed baseline.
    /// Returns [`CliDriverError::NotInstalled`] rather than panicking
    /// when `state` reports [`BaselineState::NotInstalled`] -- callers
    /// (the parity runner) must handle that as an honest "unrunnable"
    /// case, never treat it as a bug to unwrap through.
    pub fn from_state(state: &BaselineState) -> Result<Self, CliDriverError> {
        match state {
            BaselineState::FoundUnprobed { path } => Ok(Self {
                binary_path: path.clone(),
            }),
            BaselineState::NotInstalled => Err(CliDriverError::NotInstalled),
        }
    }

    /// Run `<binary> cli <tool> <request_json>` and capture the result.
    /// `request_json` must be a compact single-line JSON object (forward
    /// slashes in any path values -- the installed 0.8.1 binary's CLI
    /// JSON parser does not accept unescaped backslashes in string
    /// values, confirmed empirically: a Windows path with `\`
    /// separators makes the parser report `repo_path is required` even
    /// though the key is present, because the odd escape sequence
    /// breaks the value string. Every caller in this harness normalizes
    /// paths to forward slashes before building `request_json` for
    /// exactly this reason).
    pub fn call(&self, tool: &str, request_json: &str) -> Result<CliCallResult, CliDriverError> {
        let start = Instant::now();
        let output = Command::new(&self.binary_path)
            .arg("cli")
            .arg(tool)
            .arg(request_json)
            .output()
            .map_err(CliDriverError::Spawn)?;
        let latency_ms = duration_to_ms(start.elapsed());

        Ok(CliCallResult {
            tool: tool.to_string(),
            request_json: request_json.to_string(),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            exit_success: output.status.success(),
            latency_ms,
        })
    }
}

fn duration_to_ms(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1000.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probe_reports_not_installed_for_a_binary_name_guaranteed_absent() {
        // A GUID-shaped name cannot collide with any real PATH entry,
        // so this deterministically exercises the NotInstalled path
        // regardless of whether the real codebase-memory-mcp baseline
        // happens to be installed on the machine running this test.
        let adapter = CodebaseMemoryMcpAdapter::with_binary_name(
            "enforcer-x06-9-baseline-probe-guard-9f3c9e5e-does-not-exist",
        );
        let state = adapter.probe();
        assert_eq!(state, BaselineState::NotInstalled);
        assert!(!state.is_installed());
    }

    #[test]
    fn default_adapter_name_matches_the_documented_baseline() {
        let adapter = CodebaseMemoryMcpAdapter::new();
        assert_eq!(adapter.name(), "codebase-memory-mcp");
        assert_eq!(
            CodebaseMemoryMcpAdapter::DEFAULT_BINARY_NAME,
            "codebase-memory-mcp"
        );
    }

    #[test]
    fn probe_is_side_effect_free_and_repeatable() {
        // Calling probe() twice must be idempotent -- no state mutation
        // between calls (PARITY_HARNESS §0: same repo/commit for every
        // comparison run).
        let adapter = CodebaseMemoryMcpAdapter::with_binary_name(
            "enforcer-x06-9-baseline-probe-guard-9f3c9e5e-does-not-exist",
        );
        assert_eq!(adapter.probe(), adapter.probe());
    }

    #[test]
    fn baseline_state_is_installed_is_false_only_for_not_installed() {
        let found = BaselineState::FoundUnprobed {
            path: PathBuf::from("/usr/bin/codebase-memory-mcp"),
        };
        assert!(found.is_installed());
        assert!(!BaselineState::NotInstalled.is_installed());
    }

    #[test]
    fn cli_driver_from_state_rejects_not_installed_without_spawning(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let outcome = CliDriver::from_state(&BaselineState::NotInstalled);
        let error = match outcome {
            Err(error) => error,
            Ok(_) => return Err("NotInstalled must not construct a driver".into()),
        };
        assert!(matches!(error, CliDriverError::NotInstalled));
        Ok(())
    }

    #[test]
    fn parsed_json_takes_the_last_json_line_skipping_log_prefixes(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let result = CliCallResult {
            tool: "list_projects".to_string(),
            request_json: "{}".to_string(),
            stdout: "level=info msg=mem.init budget_mb=16106\n{\"projects\":[]}".to_string(),
            stderr: String::new(),
            exit_success: true,
            latency_ms: 1.0,
        };
        let parsed = result.parsed_json().ok_or("must parse the JSON line")?;
        assert_eq!(parsed, serde_json::json!({"projects": []}));
        Ok(())
    }

    #[test]
    fn parsed_json_is_none_for_pure_log_output_never_fabricates_a_value() {
        let result = CliCallResult {
            tool: "index_repository".to_string(),
            request_json: "{}".to_string(),
            stdout: "level=info msg=mem.init budget_mb=16106\nrepo_path is required".to_string(),
            stderr: String::new(),
            exit_success: false,
            latency_ms: 1.0,
        };
        assert_eq!(result.parsed_json(), None);
    }
}
