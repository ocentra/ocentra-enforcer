//! The [`BaselineAdapter`] seam: the future live comparison against an
//! installed `codebase-memory-mcp` baseline
//! (`MEMORY_RETRIEVAL_PARITY_HARNESS.md` §0/§1 -- "baseline: installed
//! codebase-memory-mcp MCP/CLI"). This lane ships the trait plus a
//! capability probe and the [`NotInstalled`](BaselineState::NotInstalled)
//! honest state; it does NOT ship a live MCP/CLI driver. Building that
//! driver (spawning the baseline process, speaking its MCP JSON-RPC
//! wire protocol, normalizing its responses) is out of scope for this
//! SKELETON per the mission brief -- wiring it is future work for
//! whichever run actually executes the parity harness end to end.
//!
//! # Why a trait now, a driver later
//!
//! `MEMORY_RETRIEVAL_PARITY_HARNESS.md` §6 (non-acceptance cases) is
//! explicit that a stub/placeholder tool result is a harness FAILURE --
//! so this module never fabricates a baseline response. Every method on
//! [`BaselineAdapter`] either returns real data from a live probe or
//! surfaces [`BaselineState::NotInstalled`] / a capability-state value;
//! there is no code path that invents a plausible-looking fake result.

use std::path::PathBuf;
use std::process::Command;

/// Whether the baseline binary is available to drive at all, and if
/// not, why. Mirrors the capability-state doctrine already used by
/// `enforcer_memory::embed::LoadState` (D-03) -- never silently
/// upgraded to "installed" if the probe did not actually succeed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BaselineState {
    /// The `codebase-memory-mcp` binary was not found on `PATH` (or the
    /// configured override path). This is the expected state in this
    /// SKELETON pass and in most CI/dev environments -- it is not an
    /// error, it is an honest fact the proof artifacts must record
    /// (per the mission brief: "expect mostly red/pending today; that
    /// is CORRECT").
    NotInstalled,
    /// The binary was found at `path` but has not been version-probed
    /// yet (the live JSON-RPC driver that would do that is future
    /// work).
    FoundUnprobed { path: PathBuf },
}

impl BaselineState {
    pub fn is_installed(&self) -> bool {
        !matches!(self, BaselineState::NotInstalled)
    }
}

/// The seam a future live parity runner drives. Every method takes
/// `&self` (never `&mut self`) because probing/querying a baseline
/// process is inherently a read-only comparison operation from this
/// harness's point of view.
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
}
