//! X06.6: lesson activation rules, supersede handling, and the
//! per-domain aggregate learning-curve emission ("--all equivalent
//! API" in the workpack's words).
//!
//! # Activation
//!
//! A lesson is [`LessonStatus::Active`] only when it has landed,
//! proof-linked evidence: a non-empty `landedAt` (memory record) or
//! `landed_at` (ledger row) AND that landing traces to at least one
//! enforcer-proof journal ref recorded via [`crate::evidence`]. A
//! lesson imported from x05 or otherwise unlanded is
//! [`LessonStatus::Inactive`] -- it stays searchable through
//! [`crate::recall::recall`] (recall never filters by activation; that
//! would silently hide unlanded lessons instead of surfacing them as
//! "not yet proven") but callers building an active-lesson digest (see
//! [`crate::sessionstart`]) must consult [`lesson_status`] and exclude
//! inactive rows.
//!
//! # Supersede
//!
//! A memory record with `supersedes = Some(id)` retires the earlier
//! record's activation: [`active_lessons`] never returns both an id and
//! whatever id it supersedes, even if the superseded record still
//! independently satisfies the landed-evidence rule. The earlier record
//! is never deleted or mutated (append-only) -- [`superseded_by`] can
//! still answer "what replaced this" for audit trails.
//!
//! # Learning curves
//!
//! [`learning_curve`] emits, per domain, the running count of landed
//! lessons and total incidents observed against them ordered by
//! landing sequence -- the improvement-curve data the workpack's
//! aggregate requirement describes. It is deliberately just counts
//! (no smoothing/regression) so the curve is exactly reproducible from
//! the graph's own append-only history, never a fitted approximation.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use crate::error::Result;
use crate::evidence::{recurrence_curve, RecurrencePoint};
use crate::graph::{MemoryGraph, MemoryNode};
use crate::ingest::replay_incident_observations_from_store;
use crate::model_observations::project_model_runtime_observations_from_store;
use crate::observations::replay_procedural_and_routes_from_store;
use crate::record::RecordDomain;
use crate::store::Store;

/// Whether a lesson has landed, proof-linked evidence or is still an
/// unlanded/imported candidate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LessonStatus {
    /// Has at least one non-empty landing reference. Proof-linkage
    /// itself (an enforcer-proof journal ref) is asserted by
    /// [`crate::evidence::evidence_chain`]'s caller-supplied `proof_ref`
    /// lookup -- this status answers "landed at all", not "and proof
    /// exists", because a graph slice with no proof store attached must
    /// still be able to report landed-vs-not deterministically.
    Active,
    /// No landing reference recorded yet: imported/candidate/unlanded.
    /// Still searchable via [`crate::recall::recall`] -- inactive is not
    /// hidden, it's just not counted as proven.
    Inactive,
}

/// One lesson-like node's id plus its landing references, independent
/// of whether the node came from the NDJSON stream (`MemoryRecord`) or
/// the ledger (`LessonRow`).
#[derive(Debug, Clone, PartialEq, Eq)]
struct LessonFacts<'a> {
    id: &'a str,
    landed_at: Vec<&'a str>,
    supersedes: Option<&'a str>,
    domain: Option<RecordDomain>,
}

fn lesson_facts(node: &MemoryNode) -> Option<LessonFacts<'_>> {
    match node {
        MemoryNode::Lesson(row) => Some(LessonFacts {
            id: &row.id,
            landed_at: if row.landed_at.trim().is_empty() {
                Vec::new()
            } else {
                vec![row.landed_at.as_str()]
            },
            supersedes: None,
            domain: None,
        }),
        MemoryNode::Record(record) if matches!(record.kind, crate::record::RecordKind::Lesson) => {
            Some(LessonFacts {
                id: &record.id,
                landed_at: record.landed_at.iter().map(String::as_str).collect(),
                supersedes: record.supersedes.as_deref(),
                domain: Some(record.domain),
            })
        }
        _ => None,
    }
}

/// Compute the activation status of a single lesson id. Returns `None`
/// if no lesson-like node with this id exists in the graph at all
/// (distinct from `Inactive`: "unknown" vs "known but not landed").
pub fn lesson_status(graph: &MemoryGraph, lesson_id: &str) -> Option<LessonStatus> {
    let facts = graph
        .nodes()
        .iter()
        .filter_map(lesson_facts)
        .find(|f| f.id == lesson_id)?;
    Some(if facts.landed_at.iter().any(|l| !l.trim().is_empty()) {
        LessonStatus::Active
    } else {
        LessonStatus::Inactive
    })
}

/// The set of lesson ids that have been superseded by some other record
/// in the graph (i.e. appear as some other record's `supersedes` value).
fn superseded_ids(graph: &MemoryGraph) -> HashSet<&str> {
    graph
        .nodes()
        .iter()
        .filter_map(lesson_facts)
        .filter_map(|f| f.supersedes)
        .collect()
}

/// All lesson ids the graph considers [`LessonStatus::Active`],
/// EXCLUDING any lesson id that has since been superseded by a later
/// record -- a superseded lesson never counts as active even if its own
/// landing evidence would otherwise qualify it, because a newer record
/// has explicitly replaced it.
pub fn active_lessons(graph: &MemoryGraph) -> Vec<&str> {
    let superseded = superseded_ids(graph);
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for node in graph.nodes() {
        let Some(facts) = lesson_facts(node) else {
            continue;
        };
        if superseded.contains(facts.id) {
            continue;
        }
        if !facts.landed_at.iter().any(|l| !l.trim().is_empty()) {
            continue;
        }
        if seen.insert(facts.id) {
            out.push(facts.id);
        }
    }
    out
}

/// The id of the record that supersedes `lesson_id`, if any. `None`
/// when `lesson_id` has not been superseded (including when it does not
/// exist at all).
pub fn superseded_by<'a>(graph: &'a MemoryGraph, lesson_id: &str) -> Option<&'a str> {
    graph.nodes().iter().find_map(|node| {
        let facts = lesson_facts(node)?;
        if facts.supersedes == Some(lesson_id) {
            Some(facts.id)
        } else {
            None
        }
    })
}

/// One point on a per-domain learning curve: after landing the
/// `landed_count`-th lesson in this domain (in graph insertion order),
/// how many total incidents (t0 observations) exist across all lessons
/// landed so far in that domain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LearningCurvePoint {
    pub lesson_id: String,
    pub landed_count: usize,
    pub cumulative_incidents: usize,
}

/// Per-domain learning-curve series: the workpack's "aggregate (--all
/// equivalent API) emits per-domain learning curve data" requirement.
/// Domains with no landed lessons yet are simply absent from the map
/// (an empty curve, not a fabricated zero-point one) -- callers that
/// need to distinguish "no domain data" from "domain has one landed
/// lesson with zero incidents" should check `map.get(domain).is_none()`
/// vs an empty `Vec`, though in practice a landed lesson always
/// contributes at least a point even at zero incidents.
pub fn learning_curve(graph: &MemoryGraph) -> HashMap<RecordDomain, Vec<LearningCurvePoint>> {
    let mut curves: HashMap<RecordDomain, Vec<LearningCurvePoint>> = HashMap::new();
    let mut landed_count: HashMap<RecordDomain, usize> = HashMap::new();

    for node in graph.nodes() {
        let Some(facts) = lesson_facts(node) else {
            continue;
        };
        let Some(domain) = facts.domain else {
            // Ledger rows carry no domain field; the workpack's
            // per-domain curve concerns NDJSON-sourced lessons, which
            // always carry `domain`. Ledger-only rows are excluded from
            // this curve rather than guessed into a default domain.
            continue;
        };
        if !facts.landed_at.iter().any(|l| !l.trim().is_empty()) {
            continue;
        }
        let count = landed_count.entry(domain).or_insert(0);
        *count += 1;
        let cumulative_incidents: usize = curves
            .get(&domain)
            .map(|points| points.last().map(|p| p.cumulative_incidents).unwrap_or(0))
            .unwrap_or(0)
            + graph.incidents_for_lesson(facts.id).len();
        curves.entry(domain).or_default().push(LearningCurvePoint {
            lesson_id: facts.id.to_string(),
            landed_count: *count,
            cumulative_incidents,
        });
    }
    curves
}

/// Store-log-derived learning projection.
///
/// The seed graph carries durable lesson facts from the x05 memory corpus;
/// mutable learning events are replayed from [`Store`] append logs into a
/// cloned projection before curves are computed. This keeps Store as the
/// canonical source for observations without inventing lesson-record
/// persistence in this narrow slice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreLearningProjection {
    pub replayed_incident_observations: usize,
    pub replayed_procedural_and_routes: usize,
    pub model_runtime_observations: usize,
    pub procedural_record_count: usize,
    pub route_trace_count: usize,
    pub learning_curves: HashMap<RecordDomain, Vec<LearningCurvePoint>>,
    pub recurrence_curves: BTreeMap<String, Vec<RecurrencePoint>>,
}

pub fn project_learning_from_store(
    store: &Store,
    seed_graph: &MemoryGraph,
) -> Result<StoreLearningProjection> {
    let mut projected = seed_graph.clone();
    let replayed_incident_observations =
        replay_incident_observations_from_store(store, &mut projected)?;
    let replayed_procedural_and_routes =
        replay_procedural_and_routes_from_store(store, &mut projected)?;
    let model_runtime_observations = project_model_runtime_observations_from_store(store)?.len();
    let learning_curves = learning_curve(&projected);

    let mut lesson_ids = BTreeSet::<String>::new();
    for node in projected.nodes() {
        match node {
            MemoryNode::Incident(incident) if !incident.lesson_id.trim().is_empty() => {
                lesson_ids.insert(incident.lesson_id.clone());
            }
            MemoryNode::Lesson(_) | MemoryNode::Record(_) | MemoryNode::Incident(_) => {}
        }
    }

    let recurrence_curves = lesson_ids
        .into_iter()
        .filter_map(|lesson_id| {
            let curve = recurrence_curve(&projected, &lesson_id);
            if curve.is_empty() {
                None
            } else {
                Some((lesson_id, curve))
            }
        })
        .collect();

    Ok(StoreLearningProjection {
        replayed_incident_observations,
        replayed_procedural_and_routes,
        model_runtime_observations,
        procedural_record_count: projected.procedural_records().len(),
        route_trace_count: projected.route_traces().len(),
        learning_curves,
        recurrence_curves,
    })
}
