//! X06.6: the SessionStart recall-pack seam.
//!
//! c05 (`crates/enforcer-install/src/hooks/sessionstart.rs`, out of this
//! lane's file claim) computes a Claude SessionStart hook config that
//! injects an enforcer-first reminder + mechanical-enforcement doctrine
//! into every new session's context. That pack's own text flags a
//! follow-up: the doctrine text is static, but the workpack's continuous-
//! learning contract wants sessions to start with LIVE memory context
//! too (active lessons, recent incidents) -- "the system constantly
//! asks... did retrieval improve" only works if each new session already
//! knows what was learned before it started.
//!
//! [`recall_pack`] is that memory-side payload: a deterministic,
//! bounded summary of the graph's current learning state, computed
//! purely from an in-memory [`MemoryGraph`] (no I/O, no network, no
//! model call) so it is cheap enough to run on every session start. It
//! does NOT itself register a hook or write `additionalContext` JSON --
//! wiring this into an actual SessionStart hook config is c05/c03's own
//! file (`crates/enforcer-install/**`, out of scope here); this module
//! is the seam that wiring would call.

use crate::graph::MemoryGraph;
use crate::learning::active_lessons;
use crate::owned_boundary::RetainedDisplay;
use enforcer_domain::memory_types::{
    MemorySessionActiveLessonCount, MemorySessionIncidentCount, MemorySessionLessonId,
    MemorySessionRecallLimit, MemorySessionRecallText,
};

/// One line of the recall pack's active-lesson digest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveLessonSummary {
    pub lesson_id: MemorySessionLessonId,
    pub incident_count: MemorySessionIncidentCount,
}

/// The bounded, deterministic memory payload a SessionStart hook would
/// inject at the start of a new session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecallPack {
    /// Active (landed, non-superseded) lessons, most-recently-inserted
    /// first, capped at `limit`.
    pub active_lessons: Vec<ActiveLessonSummary>,
    /// How many active lessons exist in total (before the `limit` cap),
    /// so a caller rendering only the top N can still say "+N more".
    pub total_active_lessons: MemorySessionActiveLessonCount,
}

impl RecallPack {
    /// Render as plain text suitable for a hook's `additionalContext`
    /// field: one line per active lesson plus an overflow note. Kept
    /// here (not in `enforcer-install`) because it is a pure function of
    /// the pack's own data -- the hook-wiring crate only needs to embed
    /// this string, not reimplement its formatting.
    pub fn render(&self) -> MemorySessionRecallText {
        if self.active_lessons.is_empty() {
            return "enforcer-memory: no active (landed) lessons recorded yet.".into();
        }
        let mut lines = vec!["enforcer-memory active lessons:".retained_display()];
        for summary in &self.active_lessons {
            lines.push(format!(
                "- {} ({} incident{} observed)",
                summary.lesson_id,
                summary.incident_count,
                if summary.incident_count == 1 { "" } else { "s" }
            ));
        }
        let shown = self.active_lessons.len();
        if self.total_active_lessons > shown {
            let hidden = self.total_active_lessons.get() - shown;
            lines.push(format!(
                "- (+{} more active lesson{})",
                hidden,
                if hidden == 1 { "" } else { "s" }
            ));
        }
        lines.join("\n").into()
    }
}

/// Compute the recall pack for `graph`, capping the digest at `limit`
/// active lessons (most-recently-landed first). `limit = 0` returns an
/// empty digest with an accurate `total_active_lessons` count -- never
/// an error, since "how many lessons are active" is still meaningful
/// even with a zero-size digest.
pub fn recall_pack(graph: &MemoryGraph, limit: impl Into<MemorySessionRecallLimit>) -> RecallPack {
    let limit = limit.into().get();
    let mut ids = active_lessons(graph);
    // Most-recently-landed first: `active_lessons` returns graph
    // insertion order (oldest first), so reverse for a "what's newest"
    // session-start digest.
    ids.reverse();
    let total_active_lessons = ids.len();
    let active_lessons = ids
        .into_iter()
        .take(limit)
        .map(|id| ActiveLessonSummary {
            incident_count: graph.incidents_for_lesson(&id).len().into(),
            lesson_id: id.as_str().into(),
        })
        .collect();
    RecallPack {
        active_lessons,
        total_active_lessons: total_active_lessons.into(),
    }
}
