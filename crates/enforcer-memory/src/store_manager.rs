//! X06 core parity: operational-robustness analogs of the baseline's
//! store lifecycle -- idle-store eviction and an index-run supervision
//! seam.
//!
//! Baseline binding: the workpack names two concrete baseline behaviors
//! to mirror honestly rather than invent from scratch:
//!
//! - **idle store eviction**: the baseline keeps a per-project open-store
//!   cache and evicts an entry once it has gone untouched for
//!   `CBM_STORE_IDLE_TIMEOUT_S` seconds (default 60s per the mission
//!   brief). [`StoreCache`] below is the same shape: a keyed cache of
//!   `T` (whatever a caller's "open store" handle is -- this module is
//!   generic over `T` rather than hardwired to
//!   [`crate::code_graph::CodeGraph`] or [`crate::store::Store`], since
//!   this lane does not own either of those types' files) with a
//!   last-touch timestamp per entry and an explicit [`StoreCache::evict_idle`]
//!   sweep.
//! - **index-run isolation**: the baseline can run `index_repository` in
//!   a supervised child process (`CBM_INDEX_SUPERVISOR` env-gated) so a
//!   crashing/hanging index run cannot take the whole server down with
//!   it. A real subprocess-spawn implementation needs a binary to spawn
//!   (this crate is a library, not the binary that would exec itself) --
//!   [`IndexSupervisor`] is the trait *seam* for that: [`InProcessSupervisor`]
//!   is the only implementation this crate ships (runs the closure
//!   in-process, no isolation at all -- today's actual behavior, stated
//!   plainly rather than implied otherwise), and this module's docs on
//!   [`IndexSupervisor`] itself describe exactly what a subprocess
//!   implementation would need to do and where it would plug in. No
//!   subprocess implementation ships in this crate.
//!
//! # Deterministic testing -- injected clock, no sleeps
//!
//! [`StoreCache`] never calls a wall-clock function itself: every
//! mutating/inspecting method takes `now: Instant` as an explicit
//! argument, supplied by the caller. Production callers pass
//! `Instant::now()`; tests pass a synthetic, monotonically-advanced
//! `Instant` (see the `tests` module below) so idle-eviction behavior is
//! exercised without a real sleep anywhere in this crate's test suite.

use crate::owned_boundary::Retained;
use enforcer_domain::memory_types::{StoreCacheContains, StoreCacheIdleTimeout, StoreCacheInstant};
use std::collections::HashMap;
use std::hash::Hash;
use std::time::Duration;

/// Matches the baseline's `CBM_STORE_IDLE_TIMEOUT_S` default exactly
/// (mission brief: "default 60s to match baseline STORE_IDLE_TIMEOUT_S").
pub const DEFAULT_IDLE_TIMEOUT: Duration = Duration::from_secs(60);

/// One cached entry: the value plus the [`Instant`] it was last touched
/// (opened or accessed) at.
struct CacheEntry<T> {
    value: T,
    last_touched: StoreCacheInstant,
}

/// A per-key cache of open-store handles (or any `T`) with idle-timeout
/// eviction, mirroring the baseline's per-project open-store cache (see
/// module docs). `K` is typically a project id/name; `T` is whatever a
/// caller's "open store" type is -- this module does not know or care
/// (it owns none of [`crate::store::Store`]'s or
/// [`crate::code_graph::CodeGraph`]'s files, per this lane's scope).
///
/// # Eviction never drops a store mid-use
///
/// [`Self::get_or_insert_with`] always refreshes `last_touched` to `now`
/// on every access (a store "in use" right now is, by definition, not
/// idle) *before* returning a reference -- so a caller holding the
/// returned reference across a subsequent [`Self::evict_idle`] call on
/// a **different** key can never have *this* key's entry evicted out
/// from under it within the same access; the entry's clock only starts
/// counting idle time again after the access that touched it completes.
/// This is a single-threaded, synchronous cache (no `Arc`/lock inside
/// `T`) -- concurrent-access safety across threads is explicitly out of
/// scope for this seam, matching this lane's library-layer (not
/// server-runtime) charter.
pub struct StoreCache<K, T> {
    entries: HashMap<K, CacheEntry<T>>,
    idle_timeout: StoreCacheIdleTimeout,
}

impl<K, T> std::fmt::Debug for StoreCache<K, T> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("StoreCache")
            .field("entry_count", &self.entries.len())
            .field("idle_timeout", &self.idle_timeout)
            .finish()
    }
}

impl<K, T> StoreCache<K, T>
where
    K: Eq + Hash + Clone,
{
    /// A cache with the baseline-matching [`DEFAULT_IDLE_TIMEOUT`] (60s).
    pub fn new() -> Self {
        Self::with_idle_timeout(DEFAULT_IDLE_TIMEOUT)
    }

    /// A cache with a caller-supplied idle timeout -- the baseline's
    /// `CBM_STORE_IDLE_TIMEOUT_S` env var is this crate's config-layer
    /// concern to read and pass in here, not this module's.
    pub fn with_idle_timeout(idle_timeout: impl Into<StoreCacheIdleTimeout>) -> Self {
        Self {
            entries: HashMap::new(),
            idle_timeout: idle_timeout.into(),
        }
    }

    /// How many entries are currently cached (idle or not) -- for tests
    /// and diagnostics.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Whether `key` is currently cached, regardless of idle state --
    /// for tests and diagnostics.
    pub fn contains(&self, key: &K) -> StoreCacheContains {
        self.entries.contains_key(key).into()
    }

    /// Fetch `key`'s cached value, opening it via `open` on a miss.
    /// Every call (hit or miss) refreshes the entry's `last_touched` to
    /// `now` -- see the struct docs' "never drops a store mid-use" note.
    pub fn get_or_insert_with(
        &mut self,
        key: K,
        now: StoreCacheInstant,
        open: impl FnOnce() -> T,
    ) -> &mut T {
        let entry = self.entries.entry(key).or_insert_with(|| CacheEntry {
            value: open(),
            last_touched: now,
        });
        entry.last_touched = now;
        &mut entry.value
    }

    /// Explicitly remove `key` from the cache (e.g. the caller closed
    /// the underlying store itself and wants it out of the cache too),
    /// returning the evicted value if it was present.
    pub fn remove(&mut self, key: &K) -> Option<T> {
        self.entries.remove(key).map(|entry| entry.value)
    }

    /// Evict every entry whose `last_touched` is at least
    /// `self.idle_timeout` behind `now`, returning the evicted keys (in
    /// arbitrary order -- callers needing a stable order should sort the
    /// returned `Vec` themselves; this module does not impose one since
    /// `K` is not required to be `Ord`).
    ///
    /// An entry touched at exactly `now - idle_timeout` IS evicted
    /// (`elapsed >= idle_timeout`, not `>`) -- matching the ordinary
    /// "timeout means timeout" reading of an idle-timeout config value.
    pub fn evict_idle(&mut self, now: StoreCacheInstant) -> Vec<K> {
        let mut evicted = Vec::new();
        self.entries.retain(|key, entry| {
            let elapsed = now.saturating_duration_since(entry.last_touched);
            let keep = elapsed < self.idle_timeout.get();
            if !keep {
                evicted.push(key.retained());
            }
            keep
        });
        evicted
    }
}

impl<K, T> Default for StoreCache<K, T>
where
    K: Eq + Hash + Clone,
{
    fn default() -> Self {
        Self::new()
    }
}

/// The seam for index-run isolation (module docs' "index-run isolation"
/// section). `run` executes `work` and returns its result; an
/// [`InProcessSupervisor`] (the only implementation this crate ships)
/// simply calls `work()` directly -- no isolation, no crash containment,
/// stated plainly rather than implied otherwise.
///
/// # What a subprocess implementation would need to do
///
/// A real subprocess-isolated implementation (not shipped here) would,
/// per the baseline's `CBM_INDEX_SUPERVISOR`-gated behavior:
///
/// 1. Serialize `work`'s inputs (the `repo_path`/`mode`/etc. arguments a
///    caller like [`crate::mcp`]'s `index_repository` handler already
///    has) to a form a child process can receive (argv/env/a temp JSON
///    file) -- `work: impl FnOnce() -> R` as written here cannot cross a
///    process boundary itself (closures are not serializable), so the
///    trait's real subprocess arm would need a differently-shaped
///    method (e.g. `run_index(&self, args: IndexArgs) -> Result<IndexOutcome, ...>`)
///    rather than a generic closure -- this module's [`IndexSupervisor::run`]
///    signature is deliberately the *in-process-friendly* shape; a
///    subprocess variant is a distinct, wider interface this module
///    does not attempt to pre-guess.
/// 2. Spawn `std::env::current_exe()` (or a documented sibling binary)
///    with a subcommand that re-enters the indexing pipeline headlessly,
///    piping structured progress/result back over stdout or a temp file.
/// 3. Enforce a wall-clock timeout on the child and kill+report on
///    expiry, so a hung index run cannot hang the supervisor either.
/// 4. Map a nonzero child exit code / a timeout / a malformed result
///    payload to a typed failure the caller can distinguish from "index
///    ran and found a real error" (same "never silent, never synthetic data"
///    posture as the rest of this crate).
///
/// None of steps 1-4 are implemented in this crate -- this doc comment
/// is the documented implementation point the mission brief asks for,
/// not a promise of hidden functionality.
pub trait IndexSupervisor {
    /// Run `work`, returning whatever it returns. [`InProcessSupervisor`]
    /// runs it in-process, synchronously, with no isolation.
    fn run<R>(&self, work: impl FnOnce() -> R) -> R;
}

/// The only [`IndexSupervisor`] this crate ships: runs `work` directly,
/// in-process, synchronously. This is today's actual behavior for every
/// existing `index_repository` call site in this crate (`src/mcp.rs`'s
/// `handle_index_repository`, `src/cli.rs`'s mirror) -- wiring
/// `IndexSupervisor` into those call sites so the choice is runtime- or
/// env-var-selectable (matching the baseline's `CBM_INDEX_SUPERVISOR`
/// gate) is a follow-up integration step in the caller, not something
/// this module does on its own (this module only defines the seam and
/// its in-process default).
#[derive(Debug, Clone, Copy, Default)]
pub struct InProcessSupervisor;

impl IndexSupervisor for InProcessSupervisor {
    fn run<R>(&self, work: impl FnOnce() -> R) -> R {
        work()
    }
}

/// The env var name a caller wiring [`IndexSupervisor`] selection into
/// `index_repository` should read, mirroring the baseline's
/// `CBM_INDEX_SUPERVISOR` gate name exactly (mission brief: "env-var
/// gated like the baseline's CBM_INDEX_SUPERVISOR"). This module does
/// not read the env var itself (no ambient I/O in a library seam type) --
/// it only names the contract so every caller reads the same variable.
pub const INDEX_SUPERVISOR_ENV_VAR: &str = "CBM_INDEX_SUPERVISOR";
