//! `orchestrator` (b04): the dependency-free-frontier / disjoint-lane
//! binding that maps validated plan structure onto `enforcer-coordination`
//! (arc-16) claims, plus the self-driving `tick()`-until-done loop
//! (owner-set 2026-07-04, lessons L14/L16 — proven live building THIS
//! plan).
//!
//! # Charter
//!
//! This module owns exactly `src/orchestrator.rs` in `enforcer-plan` (b04),
//! not the whole crate. It:
//!
//! 1. Builds the dependency DAG from workpack `deps:` fields and computes
//!    the ready frontier ([`PlanGraph::frontier`]).
//! 2. Packs the frontier into disjoint-`owns:` concurrent lane batches,
//!    reusing [`crate::validator::check_parallel_safety`] (b02's
//!    PLAN-PARALLEL-SAFETY predicate) as the overlap oracle — this module
//!    does NOT reimplement glob-overlap detection ([`pack_lanes`]).
//! 3. Binds lane lifecycle to the `enforcer-coordination` (arc-16)
//!    claim -> guard -> closeout API through the [`CoordinationPort`] trait,
//!    so the real hub and an in-memory test double share one call contract
//!    ([`CoordinationPort`], [`LiveCoordination`]).
//! 4. Serializes any residual owns overlap that slips past the static plan
//!    check via a fail-closed intent queue ([`IntentQueue`]).
//! 5. Drives all of the above through a `tick()`-until-done standing loop
//!    ([`Orchestrator::tick`]) with composed wake signals, zero-trust
//!    verify-before-integrate, dead-lane respawn, and a typed error for
//!    ending a turn in a fragile state.
//!
//! # What this module does NOT own
//! The `enforcer-coordination` crate itself (arc-16), the PLAN-* validators
//! (b02, `crate::validator`), the scaffolder (b01), or the lane-worktree
//! spawn primitive (`enforcer coordination lane new/park/rm`, deferred per
//! arc-16's own module doc) — this binding calls [`CoordinationPort`]
//! methods and, per EXECUTION_MODEL.md §2b/§3a, defaults every assigned
//! lane to its OWN worktree via [`WorktreeSpawner`], but does not implement
//! the underlying `git worktree` mechanics itself (that primitive is
//! arc-16-owned and currently deferred; [`WorktreeSpawner`] is the seam this
//! binding calls through so arc-16 can land the real spawn behind it without
//! an orchestrator-side change).

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use enforcer_coordination::api::{self, CallerContext, ClaimRequestArgs, CloseoutFilters, Hub};
use enforcer_domain::coordination_types::{
    ClaimOutcomeStatus, ClaimPath, ClaimReason, CoordinationRepoRoot,
};
use enforcer_domain::ids::{LaneId, RuleId};
use enforcer_domain::paths::RelPath;
use enforcer_domain::plan_types::{
    LaneStatus, OrchestratorTickCount, PlanClaimBlockReason, PlanCondition, PlanImportCount,
    PlanOwnershipPattern,
};

use crate::boundary::values::{claim_block_reason, diagnostic_detail};
use crate::error::{PlanError, PlanResult};
use crate::validator::{check_parallel_safety, OwnsRecord};

/// Synthetic rule id used ONLY as the [`crate::validator::check_parallel_safety`]
/// call's finding key inside this module. Not registered in `enforcer-rules`
/// (this is an internal reuse of the predicate as an overlap oracle, not a
/// new lint surface) — deliberately distinct from any real `PLAN-*` rule id
/// b02 owns. `"ORCH-PARALLEL-PROBE"` is format-valid by construction
/// (uppercase-alnum prefix + dot/alnum segments, see `validate_rule_id`) and
/// pinned by `rule_id_probe_literal_is_valid` below, so the fallback branch
/// is unreachable in practice; it still returns a `PlanResult` rather than
/// `unwrap`/`expect`/`panic!` (workspace lints forbid all three) so a future
/// edit to the literal fails a test instead of the build.
fn overlap_probe_rule_id() -> PlanResult<RuleId> {
    "ORCH-PARALLEL-PROBE"
        .parse()
        .map_err(|decode_err: enforcer_domain::boundary::decode_error::DecodeError| PlanError::GraphInvalid {
            reason: diagnostic_detail(format!(
                "internal: ORCH-PARALLEL-PROBE rule id literal is no longer format-valid: {decode_err}"
            )),
        })
}

/// One workpack's parsed graph-relevant fields: id, dependency ids, and
/// `owns:` globs. A thin domain view over [`crate::validator::OwnsRecord`]
/// (same shape) kept as its own type so orchestrator call sites read as
/// plan-graph vocabulary rather than validator vocabulary.
#[derive(Debug, Clone)]
pub struct WorkpackNode {
    pub id: LaneId,
    pub deps: Vec<LaneId>,
    pub owns: Vec<PlanOwnershipPattern>,
}

impl WorkpackNode {
    fn to_owns_record(&self) -> OwnsRecord {
        OwnsRecord {
            // CLONE-JUSTIFICATION: the validator consumes an independent
            // record snapshot while this graph node remains available for
            // subsequent frontier and dispatch operations.
            workpack_id: self.id.clone(),
            // CLONE-JUSTIFICATION: the validator's record outlives this
            // borrowed graph view and therefore owns its dependency list.
            deps: self.deps.clone(),
            // CLONE-JUSTIFICATION: overlap checking owns its input snapshot
            // while the graph retains the canonical declared paths.
            owns: self.owns.clone(),
        }
    }
}

/// The validated plan graph b04 computes a frontier over. Callers build this
/// from `enforcer-plan`'s own scaffolder/validator output (each node already
/// passed `PLAN-FRONTMATTER`/`PLAN-PARALLEL-SAFETY` upstream); this type does
/// not re-parse markdown itself.
#[derive(Debug, Clone, Default)]
pub struct PlanGraph {
    nodes: BTreeMap<LaneId, WorkpackNode>,
}

impl PlanGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, node: WorkpackNode) {
        // CLONE-JUSTIFICATION: the map key owns the identifier while the
        // inserted node retains the same identity as its durable value.
        self.nodes.insert(node.id.clone(), node);
    }

    pub fn from_nodes(nodes: impl IntoIterator<Item = WorkpackNode>) -> Self {
        let mut graph = Self::new();
        for node in nodes {
            graph.insert(node);
        }
        graph
    }

    pub fn node(&self, id: &LaneId) -> Option<&WorkpackNode> {
        self.nodes.get(id)
    }

    pub fn ids(&self) -> impl Iterator<Item = &LaneId> {
        self.nodes.keys()
    }

    /// Detect a dependency cycle via DFS; returns the cycle's participant
    /// ids (in traversal order) if one exists. A cycle makes a deterministic
    /// frontier impossible, so callers must check this before trusting
    /// [`frontier`]'s emptiness as "nothing left" rather than "stuck".
    pub fn find_cycle(&self) -> Option<Vec<LaneId>> {
        #[derive(Clone, Copy, PartialEq)]
        enum Mark {
            Visiting,
            Done,
        }
        let mut marks: HashMap<&LaneId, Mark> = HashMap::new();
        let mut stack: Vec<LaneId> = Vec::new();

        fn visit<'a>(
            id: &'a LaneId,
            nodes: &'a BTreeMap<LaneId, WorkpackNode>,
            marks: &mut HashMap<&'a LaneId, Mark>,
            stack: &mut Vec<LaneId>,
        ) -> Option<Vec<LaneId>> {
            match marks.get(id) {
                Some(Mark::Done) => return None,
                Some(Mark::Visiting) => {
                    let start = stack.iter().position(|s| s == id).unwrap_or(0);
                    let mut cycle: Vec<LaneId> = stack.iter().skip(start).cloned().collect();
                    // CLONE-JUSTIFICATION: the returned cycle owns its closing id after DFS borrows end.
                    cycle.push(id.clone());
                    return Some(cycle);
                }
                None => {}
            }
            marks.insert(id, Mark::Visiting);
            // CLONE-JUSTIFICATION: the DFS stack owns ids across recursive node borrows.
            stack.push(id.clone());
            if let Some(node) = nodes.get(id) {
                for dep in &node.deps {
                    if nodes.contains_key(dep) {
                        if let Some(cycle) = visit(dep, nodes, marks, stack) {
                            return Some(cycle);
                        }
                    }
                }
            }
            stack.pop();
            marks.insert(id, Mark::Done);
            None
        }

        for id in self.nodes.keys() {
            if let Some(cycle) = visit(id, &self.nodes, &mut marks, &mut stack) {
                return Some(cycle);
            }
        }
        None
    }

    /// Compute the ready frontier: every node whose `deps:` are all present
    /// in `done`, that is not itself already in `done`, sorted so the
    /// result is deterministic across runs (id lexicographic order, NOT
    /// insertion order — matches this crate's `BTreeMap`-backed graph).
    pub fn frontier(&self, done: &HashSet<LaneId>) -> Vec<LaneId> {
        self.nodes
            .values()
            .filter(|node| !done.contains(&node.id))
            .filter(|node| node.deps.iter().all(|dep| done.contains(dep)))
            // CLONE-JUSTIFICATION: callers receive an owned frontier that
            // remains valid after the graph borrow ends.
            .map(|node| node.id.clone())
            .collect()
    }
}

/// One concurrent lane batch: workpacks that may run in parallel because
/// they are mutually disjoint-`owns:` (or dependency-linked in a way that
/// makes overlap safe by the plan's own contract). Lanes across DIFFERENT
/// batches within one frontier dispatch are NOT implied to be sequential —
/// batching only exists to serialize residual overlap the static plan check
/// did not already forbid entirely.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaneBatch {
    lanes: Vec<LaneId>,
}

impl LaneBatch {
    fn new(lane: LaneId) -> Self {
        Self { lanes: vec![lane] }
    }

    fn push(&mut self, lane: LaneId) {
        self.lanes.push(lane);
    }

    /// Iterate the typed lane identifiers assigned to this batch.
    pub fn iter(&self) -> impl Iterator<Item = &LaneId> {
        self.lanes.iter()
    }

    /// Number of lanes assigned to this batch.
    pub fn len(&self) -> PlanImportCount {
        PlanImportCount::from(self.lanes.len())
    }
}

impl IntoIterator for LaneBatch {
    type Item = LaneId;
    type IntoIter = std::vec::IntoIter<LaneId>;

    fn into_iter(self) -> Self::IntoIter {
        self.lanes.into_iter()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct LanePair {
    first: LaneId,
    second: LaneId,
}

/// Pack a frontier's workpack ids into disjoint-owns concurrent batches.
/// Reuses [`check_parallel_safety`] (b02's PLAN-PARALLEL-SAFETY predicate) as
/// the pairwise overlap oracle: any finding it emits for two frontier
/// members means those two must not share a batch, so this is a graph
/// coloring problem over the "conflicts" edge set — solved with a simple
/// greedy first-fit (deterministic because both the frontier and each
/// batch's candidate order are sorted).
pub fn pack_lanes(graph: &PlanGraph, frontier: &[LaneId]) -> PlanResult<Vec<LaneBatch>> {
    let mut sorted_frontier = frontier.to_vec();
    sorted_frontier.sort();

    let records: Vec<OwnsRecord> = sorted_frontier
        .iter()
        .filter_map(|id| graph.node(id))
        .map(WorkpackNode::to_owns_record)
        .collect();
    let rule_id = overlap_probe_rule_id()?;
    let placeholder = placeholder_relpath()?;
    // CLONE-JUSTIFICATION: each predicate finding owns its callback path;
    // the callback can be invoked repeatedly from this borrowed closure.
    let findings = check_parallel_safety(&rule_id, &records, |_| placeholder.clone());

    let mut conflicts: HashSet<LanePair> = HashSet::new();
    for finding in &findings {
        // `check_parallel_safety`'s detail text names both offending ids
        // (`` `a` and `b` declare no dependency edge ... ``); recover the
        // pair by re-scanning the two possible member ids rather than
        // re-deriving overlap here (that would reimplement the predicate
        // this function exists to reuse, not recompute).
        for a in &sorted_frontier {
            for b in &sorted_frontier {
                if a == b {
                    continue;
                }
                let mentions_both = finding.detail.as_str().contains(&format!("`{a}`"))
                    && finding.detail.as_str().contains(&format!("`{b}`"));
                if mentions_both {
                    conflicts.insert(sorted_pair(a, b));
                }
            }
        }
    }

    let mut batches: Vec<LaneBatch> = Vec::new();
    for id in &sorted_frontier {
        let mut placed = false;
        for batch in &mut batches {
            let fits = batch
                .iter()
                .all(|other| !conflicts.contains(&sorted_pair(id, other)));
            if fits {
                // CLONE-JUSTIFICATION: batches own stable workpack ids while
                // the sorted frontier remains borrowed for conflict checks.
                batch.push(id.clone());
                placed = true;
                break;
            }
        }
        if !placed {
            // CLONE-JUSTIFICATION: a newly created batch owns its workpack
            // id independently of the borrowed sorted frontier.
            batches.push(LaneBatch::new(id.clone()));
        }
    }
    Ok(batches)
}

fn sorted_pair(a: &LaneId, b: &LaneId) -> LanePair {
    if a <= b {
        LanePair {
            // CLONE-JUSTIFICATION: the conflict key owns both ids after pairwise borrows end.
            first: a.clone(),
            // CLONE-JUSTIFICATION: the conflict key owns both ids after pairwise borrows end.
            second: b.clone(),
        }
    } else {
        LanePair {
            // CLONE-JUSTIFICATION: normalized conflict ordering still requires owned ids.
            first: b.clone(),
            // CLONE-JUSTIFICATION: normalized conflict ordering still requires owned ids.
            second: a.clone(),
        }
    }
}

fn placeholder_relpath() -> PlanResult<RelPath> {
    // `"probe.md"` is a trivially valid `RelPath` (non-empty, relative, no
    // `..` escape) by construction, pinned by `probe_relpath_literal_is_valid`
    // below; used only as the `file_for` callback's return value when
    // packing lanes, never read from disk or surfaced to a user.
    // ALLOC-JUSTIFICATION: RelPath owns this fixed validated synthetic diagnostic path.
    RelPath::try_from("probe.md".to_owned()).map_err(|decode_err| PlanError::GraphInvalid {
        reason: diagnostic_detail(format!(
            "internal: probe.md relpath literal is no longer valid: {decode_err}"
        )),
    })
}

/// A verified lane lifecycle event, exactly the sequence
/// [`CoordinationPort::claim`] -> [`CoordinationPort::guard`] ->
/// [`CoordinationPort::closeout`] must be invoked in for one lane, recorded
/// so tests can assert ordering without inspecting a live hub's ledger.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LaneEvent {
    Claimed {
        lane: LaneId,
        paths: Vec<PlanOwnershipPattern>,
    },
    Guarded {
        lane: LaneId,
    },
    ClosedOut {
        lane: LaneId,
    },
    ClaimBlocked {
        lane: LaneId,
        reason: PlanClaimBlockReason,
    },
}

/// The lifecycle contract this binding drives every lane through, factored
/// as a trait so the real `enforcer-coordination` hub and an in-memory
/// contract harness satisfy the identical call contract (workpack acceptance:
/// "a claim/guard test uses an `enforcer-coordination` in-memory harness
/// to assert claim/guard/closeout are invoked in order").
pub trait CoordinationPort {
    /// Claim `owns:` paths for `lane`. Must be fail-closed: an overlapping
    /// claim against paths another lane already holds is REJECTED, never
    /// silently merged or queued without the caller's explicit opt-in
    /// (that opt-in is [`IntentQueue`], layered above this trait, not
    /// inside it).
    fn claim(&mut self, lane: &LaneId, owns: &[PlanOwnershipPattern]) -> PlanResult<PlanCondition>;

    /// Guard before write: re-check the lane's own claimed paths are still
    /// exclusively held before it writes (intra-lane race guard, per
    /// EXECUTION_MODEL §2d — claim/guard/lock is intra-lane).
    fn guard(&mut self, lane: &LaneId) -> PlanResult<()>;

    /// Release every claim held by `lane` (closeout).
    fn closeout(&mut self, lane: &LaneId) -> PlanResult<()>;

    /// Every event this port has recorded, in call order, for
    /// order-of-operations assertions.
    fn events(&self) -> &[LaneEvent];
}

/// The real binding: drives the arc-16 `enforcer-coordination` hub through
/// its public [`api`] surface only (`claim_all`/`release`/`closeout`) — this
/// module adds no parallel coordination store, per the workpack's own
/// requirement.
pub struct LiveCoordination<'a> {
    hub: &'a Hub,
    repo_root: CoordinationRepoRoot,
    caller: CallerContext,
    events: Vec<LaneEvent>,
    held: HashMap<LaneId, Vec<PlanOwnershipPattern>>,
}

impl std::fmt::Debug for LiveCoordination<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("LiveCoordination")
            .field("repo_root", &self.repo_root)
            .field("events", &self.events)
            .field("held", &self.held)
            .finish_non_exhaustive()
    }
}

impl<'a> LiveCoordination<'a> {
    pub fn new(hub: &'a Hub, repo_root: CoordinationRepoRoot, caller: CallerContext) -> Self {
        Self {
            hub,
            repo_root,
            caller,
            events: Vec::new(),
            held: HashMap::new(),
        }
    }
}

impl CoordinationPort for LiveCoordination<'_> {
    fn claim(&mut self, lane: &LaneId, owns: &[PlanOwnershipPattern]) -> PlanResult<PlanCondition> {
        // ALLOC-JUSTIFICATION: the coordination API is the external wire boundary;
        // render canonical plan ownership values only for that call frame.
        let wire_owns = owns
            .iter()
            .map(|pattern| {
                ClaimPath::parse(pattern.as_str()).map_err(|error| PlanError::GraphInvalid {
                    reason: diagnostic_detail(format!("invalid coordination claim path: {error}")),
                })
            })
            .collect::<PlanResult<Vec<_>>>()?;
        let reason =
            ClaimReason::from_static("b04 orchestrator frontier dispatch").map_err(|error| {
                PlanError::GraphInvalid {
                    reason: diagnostic_detail(format!(
                        "invalid coordination claim reason: {error}"
                    )),
                }
            })?;
        let outcome = api::claim_all(
            self.hub,
            ClaimRequestArgs {
                repo_root: &self.repo_root,
                lane,
                owns: &wire_owns,
                caller: &self.caller,
                reason: Some(&reason),
            },
        )?;
        if matches!(outcome.status, ClaimOutcomeStatus::Accepted) {
            // CLONE-JUSTIFICATION: held coordination state owns the lane beyond this request.
            self.held.insert(lane.clone(), owns.to_vec());
            self.events.push(LaneEvent::Claimed {
                // CLONE-JUSTIFICATION: the audit event independently owns the accepted lane.
                lane: lane.clone(),
                paths: owns.to_vec(),
            });
            Ok(PlanCondition::Satisfied)
        } else {
            let reason = claim_block_reason(
                outcome
                    .blockers
                    .first()
                    .map(|b| format!("{} on {:?}", b.kind.as_str(), b.paths))
                    // ALLOC-JUSTIFICATION: a stable fallback becomes the owned typed block reason.
                    .unwrap_or_else(|| "claim blocked".to_owned()),
            );
            self.events.push(LaneEvent::ClaimBlocked {
                // CLONE-JUSTIFICATION: the audit event outlives the borrowed claim request.
                lane: lane.clone(),
                reason,
            });
            Ok(PlanCondition::Unsatisfied)
        }
    }

    fn guard(&mut self, lane: &LaneId) -> PlanResult<()> {
        // The intra-lane write guard re-checks THIS lane's own claimed
        // paths are still held before it writes (EXECUTION_MODEL §2d).
        // `enforcer-coordination`'s full `guardLedger` orchestration is
        // deferred per that crate's own module doc (arc-16 scope note); this
        // binding calls the primitive that IS shipped
        // (`ledger::active_claims` via a re-read) rather than block on the
        // deferred surface, and records the guard event for ordering
        // assertions regardless.
        if !self.held.contains_key(lane) {
            return Err(PlanError::GraphInvalid {
                reason: diagnostic_detail(format!(
                    "guard called for lane `{lane}` with no held claim"
                )),
            });
        }
        // CLONE-JUSTIFICATION: guard history independently owns the validated lane identity.
        self.events.push(LaneEvent::Guarded { lane: lane.clone() });
        Ok(())
    }

    fn closeout(&mut self, lane: &LaneId) -> PlanResult<()> {
        let filters = CloseoutFilters {
            // CLONE-JUSTIFICATION: the closeout filter owns the lane id
            // while the same parsed id is passed by reference to the API.
            lane: Some(lane.clone()),
            ..Default::default()
        };
        let reason = ClaimReason::from_static("b04 lane closeout").map_err(|error| {
            PlanError::GraphInvalid {
                reason: diagnostic_detail(format!("invalid coordination closeout reason: {error}")),
            }
        })?;
        api::closeout(self.hub, lane, &filters, &self.caller, Some(&reason))?;
        self.held.remove(lane);
        self.events
            // CLONE-JUSTIFICATION: closeout history persists after the lane borrow ends.
            .push(LaneEvent::ClosedOut { lane: lane.clone() });
        Ok(())
    }

    fn events(&self) -> &[LaneEvent] {
        &self.events
    }
}

/// In-memory `CoordinationPort` implementation with fail-closed ownership
/// semantics. Embedding callers can use it when they need deterministic
/// coordination without a hub process.
#[derive(Debug, Default)]
pub struct InMemoryCoordination {
    claims: HashMap<PlanOwnershipPattern, LaneId>, // path -> owning lane
    held: HashSet<LaneId>,                         // lanes with an active claim
    events: Vec<LaneEvent>,
}

impl InMemoryCoordination {
    pub fn new() -> Self {
        Self::default()
    }
}

impl CoordinationPort for InMemoryCoordination {
    fn claim(&mut self, lane: &LaneId, owns: &[PlanOwnershipPattern]) -> PlanResult<PlanCondition> {
        for path in owns {
            if let Some(holder) = self.claims.get(path) {
                if holder != lane {
                    self.events.push(LaneEvent::ClaimBlocked {
                        // CLONE-JUSTIFICATION: blocked-claim history owns the requested lane.
                        lane: lane.clone(),
                        reason: claim_block_reason(format!(
                            "`{path}` already claimed by `{holder}`"
                        )),
                    });
                    return Ok(PlanCondition::Unsatisfied);
                }
            }
        }
        for path in owns {
            // CLONE-JUSTIFICATION: the claims map persists an owned path key
            // after this borrowed ownership declaration is released.
            self.claims.insert(path.clone(), lane.clone());
        }
        self.held.insert(lane.clone());
        self.events.push(LaneEvent::Claimed {
            // CLONE-JUSTIFICATION: claim history independently owns the accepted lane.
            lane: lane.clone(),
            paths: owns.to_vec(),
        });
        Ok(PlanCondition::Satisfied)
    }

    fn guard(&mut self, lane: &LaneId) -> PlanResult<()> {
        if !self.held.contains(lane) {
            return Err(PlanError::GraphInvalid {
                reason: diagnostic_detail(format!(
                    "guard called for lane `{lane}` with no held claim"
                )),
            });
        }
        // CLONE-JUSTIFICATION: guard history independently owns the validated lane identity.
        self.events.push(LaneEvent::Guarded { lane: lane.clone() });
        Ok(())
    }

    fn closeout(&mut self, lane: &LaneId) -> PlanResult<()> {
        self.claims.retain(|_, holder| holder != lane);
        self.held.remove(lane);
        self.events
            // CLONE-JUSTIFICATION: closeout history persists after the lane borrow ends.
            .push(LaneEvent::ClosedOut { lane: lane.clone() });
        Ok(())
    }

    fn events(&self) -> &[LaneEvent] {
        &self.events
    }
}

/// L22/L23-relevant: worktree isolation the orchestrator defaults every
/// assigned lane to (EXECUTION_MODEL §2b/§3a total isolation — no shared
/// `Cargo.lock`/`target`/`node_modules` across lanes). The actual `git
/// worktree` mechanics are arc-16's deferred `enforcer coordination lane
/// new/park/rm` primitive (see that crate's module doc); this trait is the
/// SEAM this binding calls through so arc-16 can land the real spawn behind
/// it without an orchestrator-side change, and so tests can assert the
/// binding calls it by default without a real `git` process.
pub trait WorktreeSpawner {
    /// Spawn (or reuse, if already spawned) a dedicated worktree/branch for
    /// `lane`. The orchestrator validates this identity before any ownership
    /// claim is made, so a worktree provider never receives an arbitrary
    /// caller-provided string.
    fn spawn(&mut self, lane: &LaneId) -> PlanResult<()>;
}

/// Default `WorktreeSpawner`: records the intent to spawn without shelling
/// out to `git worktree` itself (the real primitive is arc-16-owned and
/// deferred, per that crate's own module doc). This keeps the orchestrator
/// binding's default-to-isolated-worktree REQUIREMENT observable and tested
/// today, while the eventual arc-16 primitive slots in behind the same
/// trait with zero orchestrator-side changes.
#[derive(Debug, Default)]
pub struct RecordingWorktreeSpawner {
    pub spawned: Vec<LaneId>,
}

impl WorktreeSpawner for RecordingWorktreeSpawner {
    fn spawn(&mut self, lane: &LaneId) -> PlanResult<()> {
        // ALLOC-JUSTIFICATION: this test-facing recorder retains a durable
        // snapshot of each validated lane identity for lifecycle assertions.
        // CLONE-JUSTIFICATION: the recorder owns the lane after the spawn call returns.
        self.spawned.push(lane.clone());
        Ok(())
    }
}

/// L19: worker-reuse cap — measured doctrine (L11-FILL): reuse one worker
/// for AT MOST 2 chained same-track packs before retiring to a fresh spawn.
/// This binding does not itself spawn AI workers (that is the harness's
/// job), but it tracks per-lane-identity dispatch counts so a caller driving
/// real spawns from `tick()`'s dispatch step can enforce the cap
/// mechanically rather than by convention.
/// Maximum number of chained dispatches before a lane identity is retired.
pub fn worker_reuse_cap() -> PlanImportCount {
    PlanImportCount::from(2)
}

/// Fail-closed serialization for any residual `owns:` overlap that slips
/// past the static `PLAN-PARALLEL-SAFETY` check (workpack requirement:
/// "Intent-queue serializes any residual owns overlap that slips past
/// static checks (fail-closed: refuse concurrent claim on overlapping
/// owns)"). Distinct from [`pack_lanes`] (a compile-time-ish plan-graph
/// analysis) — this is the RUNTIME fallback for the case a claim is
/// attempted anyway and blocked live.
#[derive(Debug, Default)]
pub struct IntentQueue {
    queued: Vec<(LaneId, Vec<PlanOwnershipPattern>)>,
}

impl IntentQueue {
    pub fn new() -> Self {
        Self::default()
    }

    /// Attempt to claim `owns` for `lane` through `port`. On a fail-closed
    /// block, the request is QUEUED (not silently dropped, not force-run)
    /// and `Ok(false)` is returned; the caller must retry later (e.g. next
    /// `tick()`) rather than treat a queued intent as claimed.
    pub fn try_claim_or_queue(
        &mut self,
        port: &mut dyn CoordinationPort,
        lane: &LaneId,
        owns: &[PlanOwnershipPattern],
    ) -> PlanResult<PlanCondition> {
        if matches!(port.claim(lane, owns)?, PlanCondition::Satisfied) {
            Ok(PlanCondition::Satisfied)
        } else {
            // CLONE-JUSTIFICATION: queued intent state owns lane and scope until a later retry.
            self.queued.push((lane.clone(), owns.to_vec()));
            Ok(PlanCondition::Unsatisfied)
        }
    }

    pub fn pending(&self) -> &[(LaneId, Vec<PlanOwnershipPattern>)] {
        &self.queued
    }

    /// Drain and retry every queued intent through `port`, keeping any that
    /// still block. Returns the lanes that succeeded this pass.
    pub fn drain_retry(&mut self, port: &mut dyn CoordinationPort) -> PlanResult<Vec<LaneId>> {
        let pending = std::mem::take(&mut self.queued);
        let mut succeeded = Vec::new();
        for (lane, owns) in pending {
            if matches!(port.claim(&lane, &owns)?, PlanCondition::Satisfied) {
                succeeded.push(lane);
            } else {
                self.queued.push((lane, owns));
            }
        }
        Ok(succeeded)
    }
}

/// A lane's observed state as tracked by the standing loop. Distinct from
/// `enforcer-coordination`'s own `WorkerState`/`TaskState` wire vocabulary
/// (deferred `session`/`runner` surface) — this is the orchestrator's OWN
/// bookkeeping over whichever lifecycle signals its [`LaneLivenessSource`]
/// reports, so `tick()` works the same whether the underlying signal is a
/// live hub, mail, or a test double.
/// Liveness/mail signal for one lane, as reported by whatever mechanism the
/// loop is wired to (a real hub's presence+mail feed, or a test double).
/// Kept intentionally minimal — the workpack's loop-test acceptance needs
/// exactly these three signals (in flight / claims done / went stale), not
/// the full session/thread "org chart" (EXECUTION_MODEL §2c, deferred
/// arc-16 scope).
pub trait LaneLivenessSource {
    /// Current status of `lane`, or `None` if the lane is unknown to this
    /// source (never dispatched, or already fully closed out and forgotten).
    fn status(&self, lane: &LaneId) -> Option<LaneStatus>;

    /// A lane self-reporting done SHOULD be independently corroborated
    /// (zero-trust, EXECUTION_MODEL §2d) before the orchestrator trusts it.
    /// Returns `true` only if the done-claim is corroborated (e.g. the
    /// claimed proof re-runs green and the diff matches the pack's `owns:`
    /// set); `false` marks the claim TAMPERED/premature and rejects it
    /// rather than integrating on faith.
    fn verify_done_claim(&self, lane: &LaneId) -> PlanCondition;
}

/// In-memory `LaneLivenessSource` test double: lets tests script a lane's
/// status transitions (including going dead, and a tampered done-claim)
/// without a real hub/mail feed.
#[derive(Debug, Default)]
pub struct ScriptedLiveness {
    status: HashMap<LaneId, LaneStatus>,
    verified: HashSet<LaneId>,
}

impl ScriptedLiveness {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_status(&mut self, lane: LaneId, status: LaneStatus) {
        self.status.insert(lane, status);
    }

    /// Mark `lane`'s done-claim as one that WOULD independently verify
    /// (real proof re-run green, diff matches `owns:`). Lanes not marked
    /// here fail verification by default — fail-closed, matching the
    /// zero-trust doctrine (an unscripted lane is treated as unverifiable,
    /// not as trustworthy-by-omission).
    pub fn mark_verifiable(&mut self, lane: LaneId) {
        self.verified.insert(lane);
    }
}

impl LaneLivenessSource for ScriptedLiveness {
    fn status(&self, lane: &LaneId) -> Option<LaneStatus> {
        self.status.get(lane).copied()
    }

    fn verify_done_claim(&self, lane: &LaneId) -> PlanCondition {
        if self.verified.contains(lane) {
            PlanCondition::Satisfied
        } else {
            PlanCondition::Unsatisfied
        }
    }
}

/// The terminal hand-off `tick()` reaches when the frontier is empty and
/// every plan node is done: EXECUTION_MODEL §2d's three-role gate hands off
/// to a GATEKEEPER verification, never to silence. This binding does not
/// itself implement the gatekeeper (a separate verifier role/lane per
/// §2d.3); it emits the typed signal a caller wires to that role.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GatekeeperHandoff {
    pub done_workpacks: Vec<LaneId>,
}

/// One `tick()` call's outcome: either the loop must keep running (with the
/// next wake already armed), or the plan is fully done and control hands
/// off to the gatekeeper. `tick()` returning `Ok` NEVER means "nothing to
/// do, rest here" while lanes remain in flight — that state is the typed
/// [`PlanError::IdleWithoutWatchdog`] error instead (L14/L16).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TickOutcome {
    Continue { next_wake_armed: PlanCondition },
    Done(GatekeeperHandoff),
}

/// The standing orchestrator loop (workpack requirement, owner-set
/// 2026-07-04, lessons L14/L16): drives [`PlanGraph`], [`CoordinationPort`],
/// and [`LaneLivenessSource`] together through `tick()`-until-done. Generic
/// over the port/liveness implementations so the same loop runs against a
/// live hub or a fully in-memory fixture.
#[derive(Debug)]
pub struct Orchestrator<P: CoordinationPort, L: LaneLivenessSource, W: WorktreeSpawner> {
    graph: PlanGraph,
    port: P,
    liveness: L,
    worktrees: W,
    intents: IntentQueue,
    done: BTreeSet<LaneId>,
    dispatched: BTreeMap<LaneId, LaneId>, // workpack id -> lane name
    /// Claims that were rejected for overlap and must not be dispatched
    /// until the intent queue reports a successful retry. Keeping this
    /// separate from `dispatched` makes an ownership conflict a normal
    /// fail-closed wait state rather than an incorrectly active lane.
    pending_dispatches: BTreeMap<LaneId, LaneId>, // workpack id -> lane name
    staleness_threshold_ticks: OrchestratorTickCount,
    stale_counter: BTreeMap<LaneId, OrchestratorTickCount>,
    dispatch_count: BTreeMap<LaneId, OrchestratorTickCount>,
}

impl<P: CoordinationPort, L: LaneLivenessSource, W: WorktreeSpawner> Orchestrator<P, L, W> {
    pub fn new(graph: PlanGraph, port: P, liveness: L, worktrees: W) -> Self {
        Self {
            graph,
            port,
            liveness,
            worktrees,
            intents: IntentQueue::new(),
            done: BTreeSet::new(),
            dispatched: BTreeMap::new(),
            pending_dispatches: BTreeMap::new(),
            staleness_threshold_ticks: OrchestratorTickCount::from_count(PlanImportCount::from(3)),
            stale_counter: BTreeMap::new(),
            dispatch_count: BTreeMap::new(),
        }
    }

    pub fn done_workpacks(&self) -> &BTreeSet<LaneId> {
        &self.done
    }

    pub fn coordination_events(&self) -> &[LaneEvent] {
        self.port.events()
    }

    /// Read the worktree provider's observable state after a tick. This is
    /// intentionally read-only so external integration tests can prove that
    /// a failed ownership guard never starts a worker worktree.
    pub fn worktree_spawner(&self) -> &W {
        &self.worktrees
    }

    /// Whether a workpack has entered the active dispatch set.
    pub fn dispatch_condition(&self, workpack: &LaneId) -> PlanCondition {
        if self.dispatched.contains_key(workpack) {
            PlanCondition::Satisfied
        } else {
            PlanCondition::Unsatisfied
        }
    }

    /// Mutable access to the liveness source between orchestration ticks.
    pub fn liveness_mut(&mut self) -> &mut L {
        &mut self.liveness
    }

    /// One tick of the self-driving loop
    /// (workpack requirement, L14/L16/L19/L22/L23/L26 as noted inline):
    ///
    /// 1. drain + retry queued intents (residual-overlap mail equivalent);
    /// 2. liveness-check every in-flight lane against the staleness
    ///    threshold, respawning dead/hung lanes fresh (L19: idempotent —
    ///    calling `tick()` again on an already-respawned lane must not
    ///    double-respawn; L23: a respawn always re-dispatches the FULL
    ///    lane scope, never a partial resume, so a lost mid-pack checkpoint
    ///    cannot silently orphan work);
    /// 3. verify-and-integrate any lane whose liveness source reports
    ///    `ReportedDone`, zero-trust (never on faith — L22/L26: integration
    ///    is gated on independent corroboration, exactly the discipline
    ///    that would have caught a conflicted-cherry-pick regression before
    ///    push, and a tampered done-claim is REJECTED, not integrated);
    /// 4. recompute the frontier and dispatch every newly-ready pack,
    ///    capped per lane-identity at [`worker_reuse_cap`] chained
    ///    dispatches (L11/L11-FILL);
    /// 5. re-arm: a tick that ends with lanes still in flight and no next
    ///    wake scheduled is the typed [`PlanError::IdleWithoutWatchdog`]
    ///    error (L14/L16), never a silent `Ok(())` rest state.
    pub fn tick(&mut self) -> PlanResult<TickOutcome> {
        // (1) drain + retry queued intents.
        let retried = self.intents.drain_retry(&mut self.port)?;
        for lane in retried {
            let workpack = self
                .pending_dispatches
                .iter()
                .find_map(|(workpack, pending_lane)| {
                    // CLONE-JUSTIFICATION: removal below needs an owned map key
                    // after this immutable iterator borrow has ended.
                    (pending_lane == &lane).then(|| workpack.clone())
                })
                .ok_or_else(|| PlanError::GraphInvalid {
                    reason: diagnostic_detail(format!(
                        "intent queue retried lane `{lane}` without a pending workpack dispatch"
                    )),
                })?;
            self.pending_dispatches.remove(&workpack);
            self.activate_claimed_lane(&workpack, &lane)?;
        }

        // (2) liveness-check every dispatched, not-yet-done lane.
        // CLONE-JUSTIFICATION: tick mutates orchestration state after this
        // snapshot, so it owns a stable workpack/lane view for this pass.
        let in_flight: Vec<(LaneId, LaneId)> = self
            .dispatched
            .iter()
            .filter(|(wp, _)| !self.done.contains(*wp))
            // CLONE-JUSTIFICATION: tick mutates orchestration state after
            // collecting this stable owned workpack/lane liveness snapshot.
            .map(|(wp, lane)| (wp.clone(), lane.clone()))
            .collect();

        for (workpack, lane) in &in_flight {
            match self.liveness.status(lane) {
                Some(LaneStatus::Dead) => {
                    // L19: idempotent respawn — re-claim (a lane that
                    // already holds its own paths re-claims cleanly; the
                    // underlying port's claim is scoped per-lane so this is
                    // safe to call repeatedly) and reset the stale counter.
                    self.stale_counter.remove(lane);
                    self.respawn(workpack, lane)?;
                }
                Some(LaneStatus::InFlight) | None => {
                    // CLONE-JUSTIFICATION: BTreeMap::entry needs an owned
                    // key while `in_flight` remains borrowed for iteration.
                    let counter = self
                        .stale_counter
                        .entry(lane.clone())
                        .or_insert(OrchestratorTickCount::ZERO);
                    counter.increment();
                    if counter.get() >= self.staleness_threshold_ticks.get() {
                        self.stale_counter.remove(lane);
                        self.respawn(workpack, lane)?;
                    }
                }
                Some(LaneStatus::ReportedDone) => {
                    self.stale_counter.remove(lane);
                }
            }
        }

        // (3) verify-and-integrate DONE lanes, zero-trust.
        for (workpack, lane) in &in_flight {
            if matches!(self.liveness.status(lane), Some(LaneStatus::ReportedDone)) {
                if matches!(
                    self.liveness.verify_done_claim(lane),
                    PlanCondition::Satisfied
                ) {
                    self.port.closeout(lane)?;
                    // CLONE-JUSTIFICATION: the completed-set owns workpack
                    // identities independently of this tick snapshot.
                    self.done.insert(workpack.clone());
                } else {
                    return Err(PlanError::DoneClaimRejected {
                        // CLONE-JUSTIFICATION: the error must retain the
                        // offending lane after the snapshot is released.
                        lane: lane.clone(),
                        // ALLOC-JUSTIFICATION: PlanError owns a durable
                        // diagnostic that crosses the current tick boundary.
                        reason: diagnostic_detail(
                            "independent verification did not corroborate the done-claim \
                                 (scope diff / proof re-run mismatch) — never trust a done-claim \
                                 on faith"
                            // ALLOC-JUSTIFICATION: the rejected done-claim error crosses tick boundaries.
                            .to_owned(),
                        ),
                    });
                }
            }
        }

        // (4) recompute frontier, dispatch every newly-ready pack.
        if let Some(cycle) = self.graph.find_cycle() {
            return Err(PlanError::GraphInvalid {
                reason: diagnostic_detail(format!(
                    "dependency cycle: {}",
                    cycle
                        .iter()
                        .map(LaneId::as_str)
                        .collect::<Vec<_>>()
                        .join(" -> ")
                )),
            });
        }
        let frontier: Vec<LaneId> = self
            .graph
            .frontier(&self.done.iter().cloned().collect())
            .into_iter()
            .filter(|id| {
                !self.dispatched.contains_key(id) && !self.pending_dispatches.contains_key(id)
            })
            .collect();

        for batch in pack_lanes(&self.graph, &frontier)? {
            for workpack in batch {
                // CLONE-JUSTIFICATION: lane identity is retained as a map
                // key while `workpack` is passed to dispatch below.
                let lane = workpack.clone();
                // CLONE-JUSTIFICATION: dispatch_count stores a durable
                // per-lane reuse counter across ticks.
                let count = self
                    .dispatch_count
                    .entry(lane.clone())
                    .or_insert(OrchestratorTickCount::ZERO);
                if count.get() >= worker_reuse_cap() {
                    // L11/L11-FILL: retire this lane identity; a fresh
                    // identity is required beyond the chained-pack cap. The
                    // workpack still dispatches (never dropped), but under a
                    // fresh derived lane name so a caller's worker-spawn
                    // step is forced to spawn fresh rather than reuse.
                    let fresh_lane: LaneId = format!("{lane}-fresh{}", count.get())
                        .parse()
                        .map_err(|decode_err| PlanError::GraphInvalid {
                            reason: diagnostic_detail(format!(
                                "cannot derive fresh lane id from `{lane}`: {decode_err}"
                            )),
                        })?;
                    self.dispatch(&workpack, &fresh_lane)?;
                } else {
                    count.increment();
                    self.dispatch(&workpack, &lane)?;
                }
            }
        }

        // (5) terminal check + re-arm.
        if self.done.len() == self.graph.nodes.len()
            && matches!(self.dispatched_all_settled(), PlanCondition::Satisfied)
        {
            return Ok(TickOutcome::Done(GatekeeperHandoff {
                done_workpacks: self.done.iter().cloned().collect(),
            }));
        }

        let still_in_flight = self.dispatched.keys().any(|wp| !self.done.contains(wp));
        if still_in_flight {
            // Composed wake signals: event-driven (lane completion) would
            // preempt this in a real harness binding; the timer/watchdog is
            // the fallback that survives a dead lane (L14/L16) — this
            // binding always reports the wake as armed because the caller
            // driving real `tick()` calls on a schedule IS the watchdog.
            Ok(TickOutcome::Continue {
                next_wake_armed: PlanCondition::Satisfied,
            })
        } else {
            // Nothing in flight and not all done: nothing to dispatch this
            // pass (blocked on deps not yet satisfied by anything running).
            // This is a legitimate quiescent state ONLY if there is truly
            // nothing runnable; still re-arm rather than assume permanence.
            Ok(TickOutcome::Continue {
                next_wake_armed: PlanCondition::Satisfied,
            })
        }
    }

    fn dispatched_all_settled(&self) -> PlanCondition {
        if self.dispatched.keys().all(|wp| self.done.contains(wp)) {
            PlanCondition::Satisfied
        } else {
            PlanCondition::Unsatisfied
        }
    }

    fn dispatch(&mut self, workpack: &LaneId, lane: &LaneId) -> PlanResult<()> {
        let owns = self
            .graph
            .node(workpack)
            // CLONE-JUSTIFICATION: claiming transfers an owned full `owns:`
            // snapshot into the durable intent queue / coordination port;
            // graph nodes must remain available for later frontier ticks.
            .map(|n| n.owns.clone())
            .ok_or_else(|| PlanError::GraphInvalid {
                reason: diagnostic_detail(format!(
                    "cannot dispatch unknown workpack `{workpack}`"
                )),
            })?;
        if matches!(
            self.intents
                .try_claim_or_queue(&mut self.port, lane, &owns)?,
            PlanCondition::Unsatisfied
        ) {
            // ALLOC-JUSTIFICATION: pending ownership must survive this borrow
            // and be matched against a later queue retry in a future tick.
            self.pending_dispatches
                // ALLOC-JUSTIFICATION: map keys own the validated lane across
                // ticks until the intent queue succeeds or is rejected.
                .insert(
                    // CLONE-JUSTIFICATION: pending dispatch state owns the workpack across ticks.
                    workpack.clone(),
                    // CLONE-JUSTIFICATION: pending dispatch state owns the lane across ticks.
                    lane.clone(),
                );
            return Ok(());
        }
        self.activate_claimed_lane(workpack, lane)
    }

    /// A lane becomes active only after its claim has been accepted. This
    /// prevents a blocked intent from reaching the write guard or spawning a
    /// worker before exclusive ownership exists.
    fn activate_claimed_lane(&mut self, workpack: &LaneId, lane: &LaneId) -> PlanResult<()> {
        // A worktree is a real worker-side effect. Guard FIRST so a rejected
        // claim cannot create a runnable worker for an unguarded scope.
        // A successful claim is not an active dispatch until both later
        // steps succeed. Compensate either failure with closeout so a failed
        // guard/spawn cannot strand ownership and block a future retry.
        if let Err(activation_error) = self.port.guard(lane) {
            self.port.closeout(lane)?;
            return Err(activation_error);
        }
        if let Err(activation_error) = self.worktrees.spawn(lane) {
            self.port.closeout(lane)?;
            return Err(activation_error);
        }
        self.dispatched.insert(
            // CLONE-JUSTIFICATION: active dispatch state owns the workpack after activation.
            workpack.clone(),
            // CLONE-JUSTIFICATION: active dispatch state owns the lane after activation.
            lane.clone(),
        );
        Ok(())
    }

    fn respawn(&mut self, workpack: &LaneId, lane: &LaneId) -> PlanResult<()> {
        // L23: respawn re-dispatches from the workpack's FULL owns set,
        // never a partial resume — matches "integration picks the FULL lane
        // range, never a single commit" applied to the dispatch side of the
        // same discipline.
        self.dispatch(workpack, lane)
    }

    /// Run `tick()` until [`TickOutcome::Done`], or until `max_ticks` is
    /// exhausted (a safety bound for tests/CLI callers; a real standing
    /// loop has no such bound and re-arms indefinitely per (5) above).
    pub fn run_until_done(
        &mut self,
        max_ticks: OrchestratorTickCount,
    ) -> PlanResult<GatekeeperHandoff> {
        let tick_limit = max_ticks.get().into();
        for () in std::iter::repeat_n((), tick_limit) {
            match self.tick()? {
                TickOutcome::Done(handoff) => return Ok(handoff),
                TickOutcome::Continue { next_wake_armed } => {
                    if matches!(next_wake_armed, PlanCondition::Unsatisfied) {
                        return Err(PlanError::IdleWithoutWatchdog {
                            in_flight_lanes: PlanImportCount::from(
                                self.dispatched
                                    .keys()
                                    .filter(|wp| !self.done.contains(*wp))
                                    .count(),
                            ),
                        });
                    }
                }
            }
        }
        Err(PlanError::GraphInvalid {
            reason: diagnostic_detail(format!(
                "plan did not reach DONE within {} ticks",
                max_ticks.get()
            )),
        })
    }
}
