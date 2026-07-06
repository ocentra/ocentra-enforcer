//! Analytics read model.
//!
//! # The DuckDB decision (recorded honestly, per owner intent: never fake
//! green)
//!
//! The subpack's hard requirement is "DuckDB analytics read model", with
//! an explicit escape hatch: *"if the `duckdb` crate's bundled build is
//! too heavy/unreliable on Windows, implement the analytics read model
//! behind a trait with a deterministic in-process fallback and gate real
//! DuckDB behind an optional feature"*.
//!
//! This crate takes that escape hatch. Reasoning:
//!
//! - The `duckdb` crate's `bundled` feature compiles DuckDB's C++
//!   amalgamation from source. On this workspace's Windows toolchain
//!   that is a multi-hundred-MB, multi-minute cold build that this
//!   lane's build/test gate (a `cargo build -p enforcer-memory` /
//!   `cargo test -p enforcer-memory` loop run repeatedly during
//!   development, then again in CI) cannot safely assume will complete
//!   inside a normal gate window, and does not currently have a cached
//!   toolchain/artifact story in this repo's CI. Making it a default,
//!   always-on dependency of `enforcer-memory` would make the crate's
//!   OWN build flaky on the exact platform this workpack targets first
//!   (Windows) — the opposite of "local-first, mechanically testable".
//! - The requirement this subpack actually needs met is "an analytics
//!   read model exists, is queryable, and is exercised by tests" — not
//!   "the literal `duckdb` crate is linked". [`AnalyticsReadModel`] is
//!   the seam: any backend that can aggregate observation-log entries
//!   satisfies it.
//! - [`InProcessAnalytics`] is the DEFAULT backend: a small,
//!   dependency-free, deterministic aggregator over the same
//!   observation-log entries the SQLite operational model replays. It
//!   ships in every build, needs no feature flag, and its output is
//!   exactly reproducible (no query planner nondeterminism to reason
//!   about in tests).
//! - Real DuckDB rides the SAME trait behind the `duckdb-analytics`
//!   Cargo feature (see `Cargo.toml`). It is NOT enabled by default and
//!   NOT exercised by this lane's gate — wiring an actual
//!   `DuckDbAnalytics: AnalyticsReadModel` impl is an explicit deferred
//!   follow-up (tracked in the final report), so this module does not
//!   claim a capability it has not proven. This is the fake-green
//!   distinction the owner-intent doc draws: a `#[cfg(feature =
//!   "duckdb-analytics")]` stub that is never compiled by the gate would
//!   be worse than not mentioning DuckDB at all, so none is added here.

use crate::error::Result;
use crate::schema::ObservationLogEntry;

/// One aggregate analytics answer: counts of clean vs. non-clean
/// observations, grouped by `repo_context`. Intentionally the simplest
/// aggregate that still proves the read model is real (group-by +
/// count), not a placeholder that returns a constant.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RepoContextCounts {
    pub repo_context: String,
    pub clean: u64,
    pub findings: u64,
}

/// The analytics read model seam. Any backend — the in-process fallback
/// here, or a real DuckDB-backed implementation behind the
/// `duckdb-analytics` feature — satisfies this trait.
pub trait AnalyticsReadModel {
    /// Load (replacing any prior state) the given observation entries
    /// into the analytics backend.
    fn load(&mut self, entries: &[ObservationLogEntry]) -> Result<()>;

    /// Aggregate clean/finding counts grouped by `repo_context`, sorted
    /// by `repo_context` for deterministic output.
    fn counts_by_repo_context(&self) -> Result<Vec<RepoContextCounts>>;
}

/// The default, dependency-free analytics backend: a deterministic
/// in-process aggregator. See the module-level DuckDB decision record
/// above for why this — not `duckdb` — is the default.
#[derive(Debug, Clone, Default)]
pub struct InProcessAnalytics {
    entries: Vec<ObservationLogEntry>,
}

impl AnalyticsReadModel for InProcessAnalytics {
    fn load(&mut self, entries: &[ObservationLogEntry]) -> Result<()> {
        self.entries = entries.to_vec();
        Ok(())
    }

    fn counts_by_repo_context(&self) -> Result<Vec<RepoContextCounts>> {
        let mut by_context: std::collections::BTreeMap<String, RepoContextCounts> =
            std::collections::BTreeMap::new();
        for entry in &self.entries {
            let bucket = by_context
                .entry(entry.repo_context.clone())
                .or_insert_with(|| RepoContextCounts {
                    repo_context: entry.repo_context.clone(),
                    clean: 0,
                    findings: 0,
                });
            if entry.clean {
                bucket.clean += 1;
            } else {
                bucket.findings += 1;
            }
        }
        Ok(by_context.into_values().collect())
    }
}
