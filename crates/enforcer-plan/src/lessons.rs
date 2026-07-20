//! x05 — the lesson-capture self-heal loop.
//!
//! # Charter
//!
//! Owner requirement (2026-07-04): "whatever we learn somehow needs to go
//! into the harness — lesson learnt is captured, turned into a skill or
//! rule — memory self-healing." Live orchestration of this very plan keeps
//! producing lessons (see the seed corpus
//! `docs/plans/enforcer-selfhost-plan/refs/orchestration-lessons.md`,
//! L1..L26) but capture today is MANUAL. This module mechanizes it:
//!
//! - [`LessonRecord`] — a structured, serde, branded-id record with a
//!   [`LessonDomain`] (`Harness` | `Code` — the DUAL-DOMAIN learning thesis)
//!   and one or more [`LessonRoute`]s declaring which harness surface the
//!   lesson ships through.
//! - [`LessonLedger`] — an append-only NDJSON ledger at `.enforce/lessons.ndjson`,
//!   built directly on `enforcer_core::hash_chain` + `enforcer_core::ndjson_writer`
//!   (the same primitives `enforcer-proof`'s journal rides): verify-on-open,
//!   never rewrite a prior row, a pending `landed_at` fill-in is a NEW
//!   supersede-append record, never an edit.
//! - [`add`], [`list`], [`route`] — the CLI seam for arc-22 / MCP tool seam
//!   for arc-21 (`enforcer lesson add|list|route`), plain functions over an
//!   injected ledger path so callers (and this module's tests) never depend
//!   on a global filesystem location.
//! - Route emitters ([`emit_doctrine_block`], [`emit_skill`],
//!   [`emit_rule_candidate`], [`emit_forest_node`]) rendering from
//!   `templates/lesson-*.tpl`, pure over an injected [`EmitFs`] (temp-dir
//!   testable), honoring `dry_run` (zero writes) and preserving unrelated
//!   file content (managed-block replace-in-place, never a full overwrite).
//! - [`run_doctor`] — the fail-closed check (feeds the c07 shared doctor):
//!   every non-`PlanDoc` lesson must have >=1 landed artifact whose content
//!   contains the lesson id, or it is `Severity::Error`, not a warning.
//!   `PlanDoc`-only routes are transitional and flagged `Severity::Warning`.
//! - [`import_seed_corpus`] — a one-shot, idempotent importer reading the
//!   refs ledger table (`refs/orchestration-lessons.md` plus any
//!   `refs/lessons/*.md` domain shards, per the ledger's own L18 split
//!   policy) and any `memory/streams/*.ndjson` worker-memory streams,
//!   mapping each row's `ships-via` column to routes.
//!
//! A `Code`-domain lesson routed [`LessonRoute::RuleCandidate`] REQUIRES
//! fail/pass fixtures at landing (the d01 parity oracle applies) — enforced
//! by [`run_doctor`], not merely documented.
//!
//! This module owns only `src/lessons.rs` + `templates/lesson-*.tpl` +
//! `tests/fixtures/lessons/**` (x05) — disjoint from every b01-b06 module
//! in this crate. No `pub use` barrel (workspace doctrine): consumers path
//! through `enforcer_plan::lessons::*` directly.

use std::collections::{HashMap, HashSet};

use enforcer_core::hash_chain::{link_digest, verify_chain};
use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::core_types::ChainBreak;
use enforcer_domain::findings::Finding;
use enforcer_domain::hashes::Sha256;
use enforcer_domain::ids::RuleId;
use enforcer_domain::paths::RelPath;
use enforcer_domain::plan_types::{
    ArtifactRef, CapturedDate, LedgerLineIndex, LessonDomain, LessonId, LessonRoute,
    LessonSequence, LessonText, ObservedEvidence, PlanArtifactPath, PlanCondition,
    PlanDocumentText, PlanEmissionMode, PlanFileContent, PlanImportCount, PlanWriteOutcome,
    RuleCandidateFixtures,
};
use enforcer_domain::severity::Severity;

use crate::boundary::finding::build_lesson_finding as lesson_finding;
use crate::boundary::lessons::{
    artifact_error, decode_memory_stream_record, parse_seed_rows, render_lesson_template,
    replace_or_append_block, LedgerLine, MemoryStreamRecord, SeedRow,
};
use crate::boundary::values::{artifact_path, diagnostic_detail, document_text, file_content};
use crate::error::PlanError;

fn seed_row_to_record(row: SeedRow) -> Result<LessonRecord, PlanError> {
    let SeedRow {
        id: raw_id,
        date,
        observed,
        lesson,
        landed_at: raw_landed_at,
        ships_via,
    } = row;
    let landed_at = if raw_landed_at.trim().is_empty() {
        Vec::new()
    } else {
        vec![ArtifactRef::try_from(format!(
            "{}#{}",
            "docs/plans/enforcer-selfhost-plan/refs/orchestration-lessons.md", raw_id
        ))
        .map_err(seed_decode_error)?]
    };
    let observed = observed.parse().map_err(seed_decode_error)?;
    let ships_via = file_content(ships_via);
    let landed_at_text = file_content(raw_landed_at);
    let domain = sniff_domain(&observed);
    let routes = sniff_routes(&ships_via, &landed_at_text);
    Ok(LessonRecord {
        id: LessonId::try_from(raw_id).map_err(seed_decode_error)?,
        date: date.parse().map_err(seed_decode_error)?,
        domain,
        observed,
        lesson: lesson.parse().map_err(seed_decode_error)?,
        routes,
        landed_at,
        supersedes_seq: None,
    })
}

fn seed_decode_error(error: DecodeError) -> PlanError {
    PlanError::SeedDecode(error)
}

fn sniff_routes(ships_via: &PlanFileContent, landed_at: &PlanFileContent) -> Vec<LessonRoute> {
    let haystack = format!("{} {}", ships_via.as_str(), landed_at.as_str()).to_lowercase();
    let mut routes = Vec::new();
    if haystack.contains("doctrine payload") || haystack.contains("c01") {
        routes.push(LessonRoute::DoctrineBlock);
    }
    if haystack.contains("skill") {
        routes.push(LessonRoute::Skill);
    }
    if haystack.contains("rule") || haystack.contains("d01") || haystack.contains("d-track") {
        routes.push(LessonRoute::RuleCandidate);
    }
    if haystack.contains("forest") || haystack.contains("b06") {
        routes.push(LessonRoute::ForestNode);
    }
    if routes.is_empty() {
        routes.push(LessonRoute::PlanDoc);
    }
    routes
}

fn sniff_domain(observed: &ObservedEvidence) -> LessonDomain {
    if observed
        .as_str()
        .to_lowercase()
        .trim_start()
        .starts_with("[code]")
    {
        LessonDomain::Code
    } else {
        LessonDomain::Harness
    }
}

// ---------------------------------------------------------------------
// Canonical lesson records
// ---------------------------------------------------------------------

// Historical lesson brands now live in `enforcer_domain::plan_types`; this
// module consumes those canonical types and owns only their persistence flow.
// ---------------------------------------------------------------------
// Record shape
// ---------------------------------------------------------------------

/// One captured lesson. serde camelCase on the wire (workspace convention).
/// SERIALIZATION-DOC: the append-only ledger serializes this exact public
/// record; boundary decoding validates its branded values before persistence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LessonRecord {
    /// Branded lesson id (`L1`, `L26`, `L11-FILL`, ...). Globally unique;
    /// ids never reset (L18 doctrine, applies equally to the mechanized
    /// ledger).
    pub id: LessonId,
    /// ISO-8601 date the lesson was captured (free-form string, matching
    /// the seed corpus's own `YYYY-MM-DD` cells — not parsed into a real
    /// date type since the corpus itself is prose-sourced).
    pub date: CapturedDate,
    /// Harness or code lesson (the dual-domain learning thesis).
    pub domain: LessonDomain,
    /// Live evidence that triggered the lesson (the `observed` cell).
    pub observed: ObservedEvidence,
    /// The lesson itself — what to do differently.
    pub lesson: LessonText,
    /// Every harness surface this lesson ships through. Never empty for a
    /// lesson that has been routed (an empty vec means "captured, not yet
    /// routed" — [`run_doctor`] treats that the same as zero landed
    /// artifacts: `Severity::Error` unless every route present is
    /// `PlanDoc`).
    pub routes: Vec<LessonRoute>,
    /// Landed artifact references, one per route that has actually been
    /// emitted (may be shorter than `routes` for a partially-landed
    /// lesson — that partial state is exactly what [`run_doctor`] fails
    /// closed on).
    // DEFAULT-JUSTIFICATION: historical append-only rows predate this
    // collection; absent data is the same as no landed artifact references.
    pub landed_at: Vec<ArtifactRef>,
    /// Set on a supersede-append record: the id of the earlier record this
    /// one supersedes (same `id`, later journal position). `None` on a
    /// lesson's first capture.
    // DEFAULT-JUSTIFICATION: old rows have no supersession and must retain
    // that explicit `None` state when replayed.
    pub supersedes_seq: Option<LessonSequence>,
}

impl LessonRecord {
    /// True when every declared route has landed (or the lesson declares no
    /// routes needing a landing — i.e. every route is `PlanDoc`).
    pub fn landing_condition(&self) -> PlanCondition {
        // A captured lesson with no route has no landing plan yet. It must
        // remain pending so the CLI's pending view agrees with `run_doctor`'s
        // fail-closed treatment of an unrouted ledger row.
        if self.routes.is_empty() {
            return PlanCondition::Unsatisfied;
        }
        let required_routes = self
            .routes
            .iter()
            .copied()
            .filter(|route| *route != LessonRoute::PlanDoc)
            .collect::<HashSet<_>>();
        let landed_artifacts = self.landed_at.iter().collect::<HashSet<_>>();
        if required_routes.len() <= landed_artifacts.len() {
            PlanCondition::Satisfied
        } else {
            PlanCondition::Unsatisfied
        }
    }

    /// True when this lesson's only declared route(s) are `PlanDoc`
    /// (transitional-only capture, never `Severity::Error`).
    pub fn plan_doc_condition(&self) -> PlanCondition {
        if !self.routes.is_empty() && self.routes.iter().all(|r| *r == LessonRoute::PlanDoc) {
            PlanCondition::Satisfied
        } else {
            PlanCondition::Unsatisfied
        }
    }
}

// ---------------------------------------------------------------------
// Ledger: append-only NDJSON, hash-chained, verify-on-open
// ---------------------------------------------------------------------

/// One on-disk ledger line: the record plus the hash-chain digest folding
/// in the previous line's digest. Mirrors `enforcer-proof`'s
/// `JournalLine` shape exactly (same tamper-evidence contract), kept
/// crate-local since `enforcer-plan` does not depend on `enforcer-proof`.
/// An append-only, hash-chained NDJSON lesson ledger at `path` (by
/// convention `.enforce/lessons.ndjson`, but callers inject the path so
/// tests never touch a real repo-relative location).
#[derive(Debug)]
pub struct LessonLedger {
    path: PlanArtifactPath,
    last_digest: Option<Sha256>,
}

/// Ledger tamper detected on open or replay: a prior row's recorded digest
/// no longer matches its recomputed digest (payload edited), or the chain
/// order was disturbed (rows swapped).
///
/// BRAND-INVARIANT: these fields remain private because a tamper report is
/// constructed only from hash-chain verification; callers receive its typed
/// `PlanError` diagnostic rather than manufacturing a partial report.
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
#[error(
    "lesson ledger tamper detected at line {line_index} (recorded {recorded}, expected {expected})"
)]
pub struct LedgerTamper {
    /// Zero-based line index of the first broken link.
    line_index: LedgerLineIndex,
    /// The digest recorded on the broken line.
    recorded: Sha256,
    /// The digest recomputed from payload + previous digest.
    expected: Sha256,
}

impl LessonLedger {
    /// Open (or create) the ledger at `path`, verifying the existing chain
    /// (if any) before returning. Fails closed on any break — the same
    /// verify-on-open discipline `enforcer-proof`'s journal uses.
    pub fn open(path: PlanArtifactPath) -> Result<Self, PlanError> {
        let last_digest = if path.as_path().exists() {
            let lines = read_lines(&path)?;
            verify_lines(&lines)?;
            // CLONE-JUSTIFICATION: the ledger retains the terminal digest
            // after the parsed line vector is dropped, so the next append
            // can extend the verified chain without retaining all rows.
            lines.last().map(|line| line.digest.clone())
        } else {
            None
        };
        Ok(Self { path, last_digest })
    }

    /// Append one NEW lesson capture. Fails if a record with the same id
    /// already exists and is not itself the special `-FILL` convention (use
    /// [`LessonLedger::supersede`] to fill in a pending `landed_at` instead
    /// of calling `append` again for the same id).
    pub fn append(&mut self, record: LessonRecord) -> Result<(), PlanError> {
        if record.supersedes_seq.is_some() {
            return Err(artifact_error(
                &self.path,
                format!(
                    "lesson `{}` declares a supersession; use supersede to create linked ledger rows",
                    record.id
                ),
            ));
        }
        let existing = self.list()?;
        if existing.iter().any(|r| r.id == record.id) {
            // ALLOC-JUSTIFICATION: `PlanError` owns a stable filesystem
            // location and diagnostic after this failed append returns.
            return Err(artifact_error(
                &self.path,
                format!(
                    "lesson `{}` already captured; use supersede to fill in landed_at",
                    record.id
                ),
            ));
        }
        self.write_line(record)
    }

    /// Append a supersede record for `id`: never rewrites the prior row,
    /// appends a NEW row carrying the same id, the (merged) landed
    /// artifacts, and `supersedes_seq` set to the prior row's position.
    /// This is the mechanized form of the seed corpus's own rule: "Never
    /// edit existing rows except to fill a previously-pending `landed-at`."
    pub fn supersede(
        &mut self,
        id: &LessonId,
        additional_landed_at: Vec<ArtifactRef>,
    ) -> Result<(), PlanError> {
        let existing = self.list()?;
        let (seq, prior) = existing
            .iter()
            .enumerate()
            .rev()
            .find(|(_, r)| r.id == *id)
            // ALLOC-JUSTIFICATION: the error crosses this lookup boundary
            // and must retain the owned ledger path and lesson identity.
            .ok_or_else(|| {
                artifact_error(
                    &self.path,
                    format!("cannot supersede unknown lesson `{id}`"),
                )
            })?;
        // CLONE-JUSTIFICATION: supersession writes a new immutable ledger
        // row; the verified prior row must remain unchanged in the history.
        let mut merged = prior.clone();
        for artifact in additional_landed_at {
            if !merged.landed_at.contains(&artifact) {
                merged.landed_at.push(artifact);
            }
        }
        merged.supersedes_seq = Some(PlanImportCount::from(seq).into());
        self.write_line(merged)
    }

    fn write_line(&mut self, record: LessonRecord) -> Result<(), PlanError> {
        // ALLOC-JUSTIFICATION: canonical journal bytes and digest text outlive
        // serialization frames and are persisted as one immutable chain row.
        let canonical =
            serde_json::to_vec(&record).map_err(|error| artifact_error(&self.path, error))?;
        let digest = link_digest(self.last_digest.as_ref(), &canonical);
        // CLONE-JUSTIFICATION: the writer borrows a line while `last_digest`
        // must retain the same value for the subsequent append.
        let line = LedgerLine {
            record,
            digest: digest.clone(),
        };
        let mut writer: enforcer_core::ndjson_writer::NdjsonWriter<LedgerLine> =
            enforcer_core::ndjson_writer::NdjsonWriter::open(self.path.as_path())
                .map_err(|error| artifact_error(&self.path, error))?;
        writer
            .append(&line)
            .map_err(|error| artifact_error(&self.path, error))?;
        self.last_digest = Some(digest);
        Ok(())
    }

    /// Re-read the ledger from disk and verify-on-replay (independent of
    /// in-memory state, so a caller can re-validate a ledger another
    /// process may have appended to since it was opened).
    pub fn verify_on_replay(&self) -> Result<PlanImportCount, PlanError> {
        let lines = read_lines(&self.path)?;
        verify_lines(&lines)?;
        Ok(PlanImportCount::from(lines.len()))
    }

    /// Every record currently on disk, in append order (INCLUDING
    /// supersede rows — callers wanting "latest state per id" should use
    /// [`LessonLedger::latest`]).
    pub fn list(&self) -> Result<Vec<LessonRecord>, PlanError> {
        Ok(read_lines(&self.path)?
            .into_iter()
            .map(|line| line.record)
            .collect())
    }

    /// The latest (most-recently-appended) record per lesson id — the
    /// effective current state after folding in every supersede-append.
    pub fn latest(&self) -> Result<Vec<LessonRecord>, PlanError> {
        let mut by_id: HashMap<LessonId, LessonRecord> = HashMap::new();
        for record in self.list()? {
            // CLONE-JUSTIFICATION: the map owns a key while the full record
            // remains its value for the latest-state projection.
            by_id.insert(record.id.clone(), record);
        }
        let mut records: Vec<LessonRecord> = by_id.into_values().collect();
        records.sort_by(|a, b| a.id.as_str().cmp(b.id.as_str()));
        Ok(records)
    }
}

fn read_lines(path: &PlanArtifactPath) -> Result<Vec<LedgerLine>, PlanError> {
    if !path.as_path().exists() {
        return Ok(Vec::new());
    }
    // ALLOC-JUSTIFICATION: IO diagnostics outlive the source error and must
    // own the requested ledger path and operating-system message.
    enforcer_core::ndjson_writer::read_all(path.as_path())
        .map_err(|error| crate::boundary::lessons::artifact_error(path, error))
}

fn verify_lines(lines: &[LedgerLine]) -> Result<(), PlanError> {
    // ALLOC-JUSTIFICATION: hash-chain verification consumes canonical owned
    // bytes for all records before it evaluates links against their digests.
    let canonical: Vec<Vec<u8>> = lines
        .iter()
        .map(|line| {
            serde_json::to_vec(&line.record).map_err(|error| PlanError::Io {
                path: artifact_path("lesson ledger".into()),
                reason: diagnostic_detail(format!("{error}")),
            })
        })
        .collect::<Result<_, _>>()?;
    let links = canonical
        .iter()
        .map(Vec::as_slice)
        .zip(lines.iter().map(|line| &line.digest));
    verify_chain(links).map_err(|chain_break| match chain_break {
        ChainBreak::DigestMismatch {
            index,
            recorded,
            expected,
        } => {
            let index_value: usize = index.into();
            tamper_to_plan_error(&LedgerTamper {
                line_index: PlanImportCount::from(index_value).into(),
                recorded,
                expected,
            })
        }
        ChainBreak::LengthMismatch {
            recorded_digests,
            data_lines,
            ..
        } => PlanError::Io {
            path: artifact_path("lesson ledger".into()),
            reason: {
                let recorded_count: usize = recorded_digests.into();
                let data_count: usize = data_lines.into();
                diagnostic_detail(format!(
                    "hash-chain length mismatch: {} recorded digest(s), {} data line(s)",
                    PlanImportCount::from(recorded_count),
                    PlanImportCount::from(data_count)
                ))
            },
        },
    })?;
    verify_supersession_state(lines)?;
    Ok(())
}

/// Verify that a supersede-append record extends the latest prior state of
/// the same lesson. Hash-chain integrity alone proves row order and bytes,
/// but not that a hash-valid row obeys the ledger's state-transition
/// contract: supersession may only add landed artifacts, never replace the
/// lesson identity or discard previous landing evidence.
fn verify_supersession_state(lines: &[LedgerLine]) -> Result<(), PlanError> {
    for (index, line) in lines.iter().enumerate() {
        let invalid = |reason: &str| PlanError::Io {
            path: artifact_path("lesson ledger".into()),
            reason: diagnostic_detail(format!(
                "invalid lesson supersession at line {index}: {reason}"
            )),
        };
        let Some(prior_index) = line.record.supersedes_seq else {
            continue;
        };
        let prior_position = usize::from(PlanImportCount::from(prior_index));
        let Some(prior) = lines.get(prior_position) else {
            return Err(invalid("references a missing prior row"));
        };
        if prior_position >= index {
            return Err(invalid("must reference an earlier row"));
        }
        if prior.record.id != line.record.id {
            return Err(invalid("references a different lesson id"));
        }
        let latest_prior_index = lines
            .iter()
            .enumerate()
            .take(index)
            .filter_map(|(candidate_index, candidate)| {
                (candidate.record.id == line.record.id).then_some(candidate_index)
            })
            .next_back();
        if latest_prior_index != Some(prior_position) {
            return Err(invalid(
                "does not extend the latest prior state for its lesson",
            ));
        }
        let unchanged_identity = prior.record.date == line.record.date
            && prior.record.domain == line.record.domain
            && prior.record.observed == line.record.observed
            && prior.record.lesson == line.record.lesson
            && prior.record.routes == line.record.routes;
        if !unchanged_identity {
            return Err(invalid("changes immutable lesson identity fields"));
        }
        if !prior
            .record
            .landed_at
            .iter()
            .all(|artifact| line.record.landed_at.contains(artifact))
        {
            return Err(invalid("removes a previously landed artifact"));
        }
    }
    Ok(())
}

fn tamper_to_plan_error(tamper: &LedgerTamper) -> PlanError {
    // ALLOC-JUSTIFICATION: typed diagnostics own the tamper description once
    // the borrowed verification report is no longer available.
    PlanError::Io {
        path: artifact_path("lesson ledger".into()),
        reason: diagnostic_detail(tamper.to_string()),
    }
}

// ---------------------------------------------------------------------
// CLI/MCP seam: add / list / route
// ---------------------------------------------------------------------

/// `enforcer lesson add` — capture a new lesson. CLI seam for arc-22, MCP
/// tool seam for arc-21.
pub fn add(ledger_path: PlanArtifactPath, record: LessonRecord) -> Result<LessonRecord, PlanError> {
    let mut ledger = LessonLedger::open(ledger_path)?;
    // CLONE-JUSTIFICATION: append consumes the persisted row while the CLI
    // contract returns the caller's original validated record.
    ledger.append(record.clone())?;
    Ok(record)
}

/// `enforcer lesson list` — list captured lessons, optionally filtered by
/// route or pending-only (a lesson with at least one un-landed route).
pub fn list(
    ledger_path: PlanArtifactPath,
    route_filter: Option<LessonRoute>,
    pending_only: PlanCondition,
) -> Result<Vec<LessonRecord>, PlanError> {
    let ledger = LessonLedger::open(ledger_path)?;
    let mut records = ledger.latest()?;
    if let Some(route) = route_filter {
        records.retain(|r| r.routes.contains(&route));
    }
    if matches!(pending_only, PlanCondition::Satisfied) {
        records.retain(|record| matches!(record.landing_condition(), PlanCondition::Unsatisfied));
    }
    Ok(records)
}

/// Filesystem seam the emitters write through. A plain trait (not a
/// generic parameter) so callers can inject an in-memory implementation in
/// consumer tests without touching a real temp dir, per the workpack's
/// "pure over injected fs (temp-dir testable)" requirement.
pub trait EmitFs {
    /// Read a file's content. A missing file is `Ok(None)`; an unreadable
    /// existing path is a typed error and must never be mistaken for absence.
    fn read(&self, path: &PlanArtifactPath) -> Result<Option<PlanFileContent>, PlanError>;
    /// Write a file's full content (creating parent dirs as needed).
    fn write(
        &mut self,
        path: &PlanArtifactPath,
        content: &PlanFileContent,
    ) -> Result<(), PlanError>;
}

/// A real-filesystem [`EmitFs`] implementation.
#[derive(Debug, Default)]
pub struct RealFs;

impl EmitFs for RealFs {
    fn read(&self, path: &PlanArtifactPath) -> Result<Option<PlanFileContent>, PlanError> {
        match std::fs::read_to_string(path.as_path()) {
            Ok(content) => Ok(Some(file_content(content))),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            // ALLOC-JUSTIFICATION: PlanError owns the failed boundary path
            // and diagnostic after the operating-system error is released.
            Err(error) => Err(artifact_error(path, error)),
        }
    }

    fn write(
        &mut self,
        path: &PlanArtifactPath,
        content: &PlanFileContent,
    ) -> Result<(), PlanError> {
        if let Some(parent) = path.as_path().parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent).map_err(|error| artifact_error(path, error))?;
            }
        }
        std::fs::write(path.as_path(), content.as_str())
            .map_err(|error| artifact_error(path, error))
    }
}

/// Deterministic `{{name}}` token substitution — a byte-for-byte copy
/// of the same minimal contract `crate::templates` (b03) and
/// `crate::agents_forest` (b06) each independently established (both are
/// private/local to their own templates): missing token -> typed
/// error, never a panic.
/// One emitter's outcome: the rendered artifact text, the target path it
/// was (or would be) written to, and whether a write actually happened
/// (always `false` when `dry_run` was set).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmitOutcome<'a> {
    /// Target path the artifact was (or would be) written to.
    pub path: &'a PlanArtifactPath,
    /// Rendered artifact text.
    pub rendered: PlanDocumentText,
    /// `true` iff a write actually happened.
    pub wrote: PlanWriteOutcome,
}

/// Render one route's template for `record` and, unless `dry_run`, append
/// (or replace-in-place) the rendered managed block into `target_path`
/// through `fs`. Preserves any unrelated existing content in
/// `target_path`: if the file already exists and already carries a managed
/// block for this exact lesson id, that block is replaced in place;
/// otherwise the rendered block is appended.
/// Replace an existing managed block naming `lesson_id` in `existing` with
/// `new_block`, or append `new_block` if no such block is present.
/// Detection is anchored on the lesson id appearing inside an HTML-comment
/// marker line (`<!-- ...lesson_id... -->`) so unrelated content (other
/// lessons' blocks, hand-authored prose) is never touched.
/// Doctrine-block emitter template, embedded at compile time.
const DOCTRINE_BLOCK_TEMPLATE: &str = include_str!("../templates/lesson-doctrine-block.tpl");
/// Skill emitter template, embedded at compile time.
const SKILL_TEMPLATE: &str = include_str!("../templates/lesson-skill.tpl");
/// Rule-candidate emitter template, embedded at compile time.
const RULE_CANDIDATE_TEMPLATE: &str = include_str!("../templates/lesson-rule-candidate.tpl");
/// Forest-node emitter template, embedded at compile time.
const FOREST_NODE_TEMPLATE: &str = include_str!("../templates/lesson-forest-node.tpl");

fn emit_route<'a>(
    fs: &mut dyn EmitFs,
    template: &PlanFileContent,
    record: &LessonRecord,
    target_path: &'a PlanArtifactPath,
    mode: PlanEmissionMode,
) -> Result<EmitOutcome<'a>, PlanError> {
    let rendered = render_lesson_template(template.as_str(), record)?;
    if matches!(mode, PlanEmissionMode::DryRun) {
        return Ok(EmitOutcome {
            path: target_path,
            rendered: document_text(rendered),
            wrote: PlanWriteOutcome::DryRun,
        });
    }
    let existing = fs.read(target_path)?;
    let merged = replace_or_append_block(
        existing.as_ref().map_or("", PlanFileContent::as_str),
        &rendered,
        record.id.as_str(),
    );
    fs.write(target_path, &file_content(merged))?;
    Ok(EmitOutcome {
        path: target_path,
        rendered: document_text(rendered),
        wrote: PlanWriteOutcome::Written,
    })
}

/// Emit the doctrine-block route (c01 shared install payload) for `record`.
pub fn emit_doctrine_block<'a>(
    fs: &mut dyn EmitFs,
    record: &LessonRecord,
    target_path: &'a PlanArtifactPath,
    mode: PlanEmissionMode,
) -> Result<EmitOutcome<'a>, PlanError> {
    emit_route(
        fs,
        // ALLOC-JUSTIFICATION: the embedded template is validated into an owned canonical
        // content value for the emitter call; the allocation is bounded by the static asset.
        &file_content(DOCTRINE_BLOCK_TEMPLATE.to_owned()),
        record,
        target_path,
        mode,
    )
}

/// Emit the skill route (a keyed section in the enforcer skill) for
/// `record`.
pub fn emit_skill<'a>(
    fs: &mut dyn EmitFs,
    record: &LessonRecord,
    target_path: &'a PlanArtifactPath,
    mode: PlanEmissionMode,
) -> Result<EmitOutcome<'a>, PlanError> {
    emit_route(
        fs,
        // ALLOC-JUSTIFICATION: the embedded template is validated into an owned canonical
        // content value for the emitter call; the allocation is bounded by the static asset.
        &file_content(SKILL_TEMPLATE.to_owned()),
        record,
        target_path,
        mode,
    )
}

/// Emit the rule-candidate route (a d01 scaffolder input record) for
/// `record`. Callers MUST NOT treat this emission alone as "landed" for a
/// `Code`-domain lesson — [`run_doctor`] additionally requires fail/pass
/// fixtures to exist before a `Code`+`RuleCandidate` lesson counts as
/// landed (see [`RuleCandidateFixtures`]).
pub fn emit_rule_candidate<'a>(
    fs: &mut dyn EmitFs,
    record: &LessonRecord,
    target_path: &'a PlanArtifactPath,
    mode: PlanEmissionMode,
) -> Result<EmitOutcome<'a>, PlanError> {
    emit_route(
        fs,
        // ALLOC-JUSTIFICATION: the embedded template is validated into an owned canonical
        // content value for the emitter call; the allocation is bounded by the static asset.
        &file_content(RULE_CANDIDATE_TEMPLATE.to_owned()),
        record,
        target_path,
        mode,
    )
}

/// Emit the forest-node route (a b06 decision-forest node fragment) for
/// `record`. The fragment is schema-compatible with b06's own managed-block
/// conventions (`<!-- forest-node:... -->` / `LEAF ->` pointer) but this
/// module does not call into `crate::agents_forest` directly — coordination
/// is by fragment schema, not shared files (workpack "Parallel Ownership
/// Notes").
pub fn emit_forest_node<'a>(
    fs: &mut dyn EmitFs,
    record: &LessonRecord,
    target_path: &'a PlanArtifactPath,
    mode: PlanEmissionMode,
) -> Result<EmitOutcome<'a>, PlanError> {
    emit_route(
        fs,
        // ALLOC-JUSTIFICATION: the embedded template is validated into an owned canonical
        // content value for the emitter call; the allocation is bounded by the static asset.
        &file_content(FOREST_NODE_TEMPLATE.to_owned()),
        record,
        target_path,
        mode,
    )
}

/// `enforcer lesson route <id>` — run every emitter implied by `record`'s
/// declared routes against the given target paths, honoring `dry_run`.
/// `targets` maps each non-`PlanDoc` route present in `record.routes` to
/// the artifact path it should land at; a route with no entry in `targets`
/// is skipped (not an error — callers may route a subset at a time).
pub fn route<'a>(
    fs: &mut dyn EmitFs,
    record: &LessonRecord,
    targets: &'a HashMap<LessonRoute, PlanArtifactPath>,
    mode: PlanEmissionMode,
) -> Result<Vec<EmitOutcome<'a>>, PlanError> {
    let mut outcomes = Vec::new();
    let mut emitted_routes = HashSet::new();
    for declared_route in &record.routes {
        if !emitted_routes.insert(*declared_route) {
            continue;
        }
        let Some(target) = targets.get(declared_route) else {
            continue;
        };
        let outcome = match declared_route {
            LessonRoute::DoctrineBlock => emit_doctrine_block(fs, record, target, mode)?,
            LessonRoute::Skill => emit_skill(fs, record, target, mode)?,
            LessonRoute::RuleCandidate => emit_rule_candidate(fs, record, target, mode)?,
            LessonRoute::ForestNode => emit_forest_node(fs, record, target, mode)?,
            LessonRoute::PlanDoc => continue,
        };
        outcomes.push(outcome);
    }
    Ok(outcomes)
}

// ---------------------------------------------------------------------
// Fail-closed doctor
// ---------------------------------------------------------------------

fn synthetic_doctor_path() -> Result<RelPath, PlanError> {
    // ALLOC-JUSTIFICATION: RelPath and the typed PlanError both own their
    // values; this fixed synthetic finding path crosses that owned boundary.
    RelPath::try_from("lessons.ndjson".to_owned()).map_err(|error| PlanError::Io {
        path: artifact_path("lesson doctor".into()),
        reason: diagnostic_detail(error.to_string()),
    })
}

/// Contributed to the c07 shared doctor: every non-`PlanDoc` lesson must
/// have >=1 landed artifact whose content contains the lesson id, or it is
/// `Severity::Error` (not a warning, not a skip). `PlanDoc`-only lessons are
/// TRANSITIONAL and flagged `Severity::Warning` (prose is not a landing).
/// A `Code`-domain lesson routed `RuleCandidate` additionally requires
/// `rule_candidate_fixtures` to report [`RuleCandidateFixtures::satisfied`].
pub fn run_doctor(
    rule_id: &RuleId,
    records: &[LessonRecord],
    landed_artifact_contents: &HashMap<ArtifactRef, PlanFileContent>,
    rule_candidate_fixtures: &HashMap<LessonId, RuleCandidateFixtures>,
) -> Result<Vec<Finding>, PlanError> {
    let mut findings = Vec::new();
    let file = synthetic_doctor_path()?;

    for record in records {
        if matches!(record.plan_doc_condition(), PlanCondition::Satisfied) {
            findings.push(lesson_finding(
                rule_id,
                Severity::Warning,
                format!(
                    "lesson `{}` routes only to PlanDoc (transitional prose is not a landing)",
                    record.id
                ),
                &file,
            ));
            continue;
        }

        if record.routes.is_empty() {
            findings.push(lesson_finding(
                rule_id,
                Severity::Error,
                format!("lesson `{}` has no declared route", record.id),
                &file,
            ));
            continue;
        }

        let landed_ids_present: HashSet<&ArtifactRef> = record
            .landed_at
            .iter()
            .filter(|artifact_ref| {
                landed_artifact_contents
                    .get(*artifact_ref)
                    .is_some_and(|content| content.as_str().contains(record.id.as_str()))
            })
            .collect();

        let non_plan_doc_routes = record
            .routes
            .iter()
            .copied()
            .filter(|route| *route != LessonRoute::PlanDoc)
            .collect::<HashSet<_>>()
            .len();

        if landed_ids_present.is_empty() && non_plan_doc_routes > 0 {
            findings.push(lesson_finding(
                rule_id,
                Severity::Error,
                format!(
                    "lesson `{}` has zero landed artifacts containing its own id (or its \
                     declared landed_at entries are missing/do not contain the id)",
                    record.id
                ),
                &file,
            ));
            continue;
        }

        if landed_ids_present.len() < non_plan_doc_routes {
            findings.push(lesson_finding(
                rule_id,
                Severity::Error,
                format!(
                    "lesson `{}` declares {non_plan_doc_routes} non-PlanDoc route(s) but only \
                     {} landed artifact(s) verified to contain its id",
                    record.id,
                    landed_ids_present.len()
                ),
                &file,
            ));
            continue;
        }

        if record.domain == LessonDomain::Code
            && record.routes.contains(&LessonRoute::RuleCandidate)
        {
            let fixtures = rule_candidate_fixtures.get(&record.id);
            if !matches!(fixtures, Some(RuleCandidateFixtures::Complete)) {
                findings.push(lesson_finding(
                    rule_id,
                    Severity::Error,
                    format!(
                        "lesson `{}` is a Code-domain lesson routed RuleCandidate but has no \
                         fail+pass fixtures at landing",
                        record.id
                    ),
                    &file,
                ));
            }
        }
    }

    Ok(findings)
}

// ---------------------------------------------------------------------
// Seed-corpus import
// ---------------------------------------------------------------------

/// Parse every `| id | date | observed | lesson | landed-at | ships-via |`
/// table row out of one seed-ledger markdown document (the preamble
/// `refs/orchestration-lessons.md` or a `refs/lessons/<domain>-NN.md`
/// shard). Skips the header row and the `|---|---|...` separator row.
/// Tolerant of blank lines between rows (the seed corpus inserts blank
/// lines between some rows for readability).
/// Map a seed row's `ships-via` free text to zero or more `LessonRoute` values
/// by keyword sniffing (the seed corpus's `ships-via` column is prose, not
/// a closed vocabulary — e.g. "c01 doctrine payload (worker-protocol
/// snippet)", "b06 decision forest", "fixed MCP tool behavior (arc-16)").
/// A row matching no known keyword lands as `PlanDoc` (transitional-only —
/// safer than silently dropping it) so [`run_doctor`] still reports it
/// (as a `Warning`), never silently skips it.
/// Sniff a seed row's domain from its `observed` cell's explicit `[code]`
/// / `[harness]` tag (rows L13+ per the ledger's own doctrine); rows
/// without a tag default to `Harness` (the ledger's stated default for
/// "the rest" of the untagged seed rows).
fn memory_record_to_lesson(raw: MemoryStreamRecord) -> Option<Result<LessonRecord, PlanError>> {
    // Only fold in memory-stream records that look like a lesson capture
    // (carry both an id starting with `L`/`mem-` shape AND a lesson body) —
    // ordinary provenance/status records in the same stream are silently
    // skipped, not erred on, since this importer's job is lesson rows, not
    // full memory-stream validation.
    let MemoryStreamRecord {
        id,
        date,
        domain,
        observed,
        lesson,
        ships_via,
        landed_at,
    } = raw;
    if !id.starts_with('L') {
        return None;
    }
    let raw_lesson = lesson?;
    Some((|| -> Result<LessonRecord, PlanError> {
        let lesson: LessonText = raw_lesson.parse().map_err(PlanError::SeedDecode)?;
        let observed: ObservedEvidence = observed
            .unwrap_or_default()
            .parse()
        .map_err(PlanError::SeedDecode)?;
        let id: LessonId = id.parse().map_err(PlanError::SeedDecode)?;
        let ships_via = file_content(ships_via.unwrap_or_default());
        let landed_at_cell = file_content(landed_at.unwrap_or_default());
        let landed_at = if landed_at_cell.as_str().trim().is_empty() {
            Vec::new()
        } else {
            vec![landed_at_cell
                .as_str()
                .parse()
                .map_err(PlanError::SeedDecode)?]
        };
        let domain = match domain.as_deref() {
            Some("code") => LessonDomain::Code,
            _ => sniff_domain(&observed),
        };
        Ok(LessonRecord {
            id,
            date: date
                .unwrap_or_default()
                .parse()
            .map_err(PlanError::SeedDecode)?,
            domain,
            observed,
            lesson,
            routes: sniff_routes(&ships_via, &landed_at_cell),
            landed_at,
            supersedes_seq: None,
        })
    })())
}

/// Outcome of one [`import_seed_corpus`] call: how many lesson rows were
/// discovered across every source, and how many were newly appended
/// (idempotence proof: a second run over unchanged sources reports
/// `newly_appended == 0`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ImportOutcome {
    /// Total lesson rows discovered across every source this run.
    pub discovered: PlanImportCount,
    /// Rows newly appended to the ledger this run (0 on a repeat import
    /// over unchanged sources).
    pub newly_appended: PlanImportCount,
}

/// A seed import candidate before its persisted identity is assigned. The
/// source kind is deliberately kept outside [`LessonRecord`]: it is importer
/// provenance used to make a repeated displayed `L<number>` label unique,
/// not a claim that the historical source row itself had a new identifier.
struct SeedImportCandidate {
    record: LessonRecord,
    source_kind: SeedImportSourceKind,
}

#[derive(Clone, Copy)]
enum SeedImportSourceKind {
    Markdown,
    Memory,
}

/// Assign stable persisted ids to every candidate. Historical seed ledgers
/// are append-only prose and may legitimately reuse a displayed `L<number>`
/// label. A duplicate label therefore cannot be the ledger's identity.
///
/// Unique labels retain their familiar `L<number>` id. Every repeated label
/// becomes `L<number>-SRC-<sha256>`; the digest is derived from stable source
/// kind and canonical record payload, never a mutable row position. That
/// keeps the original, user-visible label while making each imported row
/// independently addressable. Only byte-identical source records receive an
/// ordinal suffix, because there is otherwise no semantic distinction between
/// their source identities; reordering them preserves the same persisted ids.
fn assign_seed_import_ids(
    mut candidates: Vec<SeedImportCandidate>,
) -> Result<Vec<SeedImportCandidate>, PlanError> {
    let mut label_counts: HashMap<String, usize> = HashMap::new();
    for candidate in candidates.iter() {
        // ALLOC-JUSTIFICATION: the count map owns labels while records are
        // inspected later, after this short-lived candidate borrow ends.
        *label_counts
            .entry(candidate.record.id.as_str().to_owned())
            .or_default() += 1;
    }

    let mut identical_record_occurrences = HashMap::new();
    for candidate in candidates.iter_mut() {
        // ALLOC-JUSTIFICATION: the persisted id must outlive this mutable
        // record borrow while it is used as a map key and later ledger id.
        let displayed_label = candidate.record.id.as_str().to_owned();
        let is_repeated_label =
            matches!(label_counts.get(&displayed_label), Some(count) if *count > 1);
        if !is_repeated_label {
            continue;
        }

        let payload = serde_json::to_vec(&candidate.record).map_err(|error| PlanError::Io {
            // ALLOC-JUSTIFICATION: `PlanError` owns diagnostics after the
            // fallible serialization frame has returned.
            path: artifact_path("seed corpus".into()),
            reason: diagnostic_detail(error.to_string()),
        })?;
        // ALLOC-JUSTIFICATION: SHA-256 consumes a stable, owned byte stream
        // combining immutable source kind with the canonical record payload.
        let mut identity_material = b"lesson-seed-import-v1\0".to_vec();
        identity_material.extend_from_slice(match candidate.source_kind {
            SeedImportSourceKind::Markdown => b"markdown\0",
            SeedImportSourceKind::Memory => b"memory\0",
        });
        identity_material.extend_from_slice(&payload);
        let digest = link_digest(None, &identity_material);
        let id = match identical_record_occurrences.entry((displayed_label, digest)) {
            std::collections::hash_map::Entry::Occupied(mut entry) => {
                let occurrence = {
                    let count = entry.get_mut();
                    *count += 1;
                    *count
                };
                let (label, digest) = entry.key();
                format!("{label}-SRC-{}-{occurrence}", digest.hex())
            }
            std::collections::hash_map::Entry::Vacant(entry) => {
                let (label, digest) = entry.key();
                let id = format!("{label}-SRC-{}", digest.hex());
                entry.insert(1);
                id
            }
        };
        candidate.record.id = id.parse().map_err(|error: DecodeError| PlanError::Io {
            // ALLOC-JUSTIFICATION: conversion diagnostics cross the
            // importer boundary as owned `PlanError` values.
            path: artifact_path("seed corpus".into()),
            reason: diagnostic_detail(error.to_string()),
        })?;
    }
    Ok(candidates)
}

/// One-shot, idempotent importer: reads the seed ledger's markdown table
/// (the preamble doc plus every `refs/lessons/*.md` domain shard, per the
/// ledger's own L18 split policy) and every `memory/streams/*.ndjson`
/// worker-memory stream, mapping each row's `ships-via` column to routes.
/// Repeated displayed source labels are assigned deterministic persisted
/// [`LessonId`]s, then each previously unseen record is appended. Re-running
/// over unchanged sources adds nothing (idempotent).
pub fn import_seed_corpus(
    ledger: &mut LessonLedger,
    seed_markdown_sources: &[PlanFileContent],
    memory_stream_sources: &[PlanFileContent],
) -> Result<ImportOutcome, PlanError> {
    let existing_ids: std::collections::HashSet<LessonId> =
        ledger.latest()?.into_iter().map(|r| r.id).collect();

    let mut candidates = Vec::new();

    for markdown in seed_markdown_sources {
        for row in parse_seed_rows(markdown.as_str()) {
            candidates.push(SeedImportCandidate {
                record: seed_row_to_record(row)?,
                source_kind: SeedImportSourceKind::Markdown,
            });
        }
    }

    for stream in memory_stream_sources {
        for line in stream.as_str().lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let Ok(raw) = decode_memory_stream_record(trimmed) else {
                continue;
            };
            let Some(result) = memory_record_to_lesson(raw) else {
                continue;
            };
            candidates.push(SeedImportCandidate {
                record: result?,
                source_kind: SeedImportSourceKind::Memory,
            });
        }
    }

    let candidates = assign_seed_import_ids(candidates)?;
    let discovered = PlanImportCount::from(candidates.len());
    let mut newly_appended = PlanImportCount::default();
    for candidate in candidates {
        if !existing_ids.contains(&candidate.record.id) {
            ledger.append(candidate.record)?;
            newly_appended.increment();
        }
    }

    Ok(ImportOutcome {
        discovered,
        newly_appended,
    })
}
