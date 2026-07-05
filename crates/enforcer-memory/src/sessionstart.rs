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

/// One line of the recall pack's active-lesson digest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveLessonSummary {
    pub lesson_id: String,
    pub incident_count: usize,
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
    pub total_active_lessons: usize,
}

impl RecallPack {
    /// Render as plain text suitable for a hook's `additionalContext`
    /// field: one line per active lesson plus an overflow note. Kept
    /// here (not in `enforcer-install`) because it is a pure function of
    /// the pack's own data -- the hook-wiring crate only needs to embed
    /// this string, not reimplement its formatting.
    pub fn render(&self) -> String {
        if self.active_lessons.is_empty() {
            return "enforcer-memory: no active (landed) lessons recorded yet.".to_string();
        }
        let mut lines = vec!["enforcer-memory active lessons:".to_string()];
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
            lines.push(format!(
                "- (+{} more active lesson{})",
                self.total_active_lessons - shown,
                if self.total_active_lessons - shown == 1 {
                    ""
                } else {
                    "s"
                }
            ));
        }
        lines.join("\n")
    }
}

/// Compute the recall pack for `graph`, capping the digest at `limit`
/// active lessons (most-recently-landed first). `limit = 0` returns an
/// empty digest with an accurate `total_active_lessons` count -- never
/// an error, since "how many lessons are active" is still meaningful
/// even with a zero-size digest.
pub fn recall_pack(graph: &MemoryGraph, limit: usize) -> RecallPack {
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
            lesson_id: id.to_string(),
            incident_count: graph.incidents_for_lesson(id).len(),
        })
        .collect();
    RecallPack {
        active_lessons,
        total_active_lessons,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ingest::{ingest_observation, Observation};
    use crate::record::{MemoryRecord, Provenance, RecordDomain, RecordKind};

    fn landed_record(id: &str) -> MemoryRecord {
        MemoryRecord {
            schema_version: 1,
            id: id.to_string(),
            ts: "2026-07-04T00:00:00Z".to_string(),
            kind: RecordKind::Lesson,
            domain: RecordDomain::Harness,
            statement: format!("statement for {id}"),
            why: None,
            how_to_apply: None,
            applies_to: vec![],
            evidence: None,
            routes: vec![],
            landed_at: vec!["commit abc".to_string()],
            supersedes: None,
            provenance: Provenance {
                writer: "primary".to_string(),
                ..Default::default()
            },
        }
    }

    #[test]
    fn recall_pack_is_empty_and_honest_on_a_fresh_graph() {
        let graph = MemoryGraph::new();
        let pack = recall_pack(&graph, 5);
        assert!(pack.active_lessons.is_empty());
        assert_eq!(pack.total_active_lessons, 0);
        assert!(pack.render().contains("no active"));
    }

    #[test]
    fn recall_pack_lists_active_lessons_with_incident_counts() {
        let mut graph = MemoryGraph::new();
        graph.ingest_record(landed_record("mem-a-0001"));
        ingest_observation(
            &mut graph,
            Observation {
                lesson_id: "mem-a-0001".to_string(),
                rule_id: None,
                fault_class: None,
                repo_context: "crates/foo".to_string(),
                clean: false,
                source_surface: "scan".to_string(),
                ts: "2026-07-04T01:00:00Z".to_string(),
            },
        );
        let pack = recall_pack(&graph, 5);
        assert_eq!(pack.active_lessons.len(), 1);
        assert_eq!(pack.active_lessons[0].lesson_id, "mem-a-0001");
        assert_eq!(pack.active_lessons[0].incident_count, 1);
        assert!(pack.render().contains("mem-a-0001"));
    }

    #[test]
    fn recall_pack_respects_limit_and_reports_overflow() {
        let mut graph = MemoryGraph::new();
        graph.ingest_record(landed_record("mem-a-0001"));
        graph.ingest_record(landed_record("mem-a-0002"));
        graph.ingest_record(landed_record("mem-a-0003"));
        let pack = recall_pack(&graph, 2);
        assert_eq!(pack.active_lessons.len(), 2);
        assert_eq!(pack.total_active_lessons, 3);
        assert!(pack.render().contains("+1 more"));
    }

    #[test]
    fn recall_pack_excludes_unlanded_lessons() {
        let mut graph = MemoryGraph::new();
        let mut unlanded = landed_record("mem-a-0001");
        unlanded.landed_at.clear();
        graph.ingest_record(unlanded);
        let pack = recall_pack(&graph, 5);
        assert!(pack.active_lessons.is_empty());
        assert_eq!(pack.total_active_lessons, 0);
    }
}
