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
use std::path::{Path, PathBuf};

use enforcer_core::error::DecodeError;
use enforcer_core::hash_chain::{link_digest, verify_chain};
use enforcer_domain::findings::Finding;
use enforcer_domain::ids::RuleId;
use enforcer_domain::paths::RelPath;
use enforcer_domain::severity::Severity;

use crate::error::PlanError;

// ---------------------------------------------------------------------
// Branded ids
// ---------------------------------------------------------------------

/// Declare a branded string newtype with a validation function, serde
/// parse-at-boundary wiring, and accessors. A crate-local copy of the same
/// minimal contract `enforcer_domain::ids` establishes (that macro is
/// private to its own crate) — this module's ids are `enforcer-plan`-local,
/// not workspace-shared domain ids, so they live here rather than being
/// smuggled into `enforcer-domain` for one feature pack.
macro_rules! branded_string {
    ($(#[$doc:meta])* $name:ident, $field_path:literal, $validate:path) => {
        $(#[$doc])*
        #[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
        #[serde(try_from = "String", into = "String")]
        pub struct $name(String);

        impl $name {
            /// View the validated inner value.
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl TryFrom<String> for $name {
            type Error = DecodeError;

            fn try_from(raw: String) -> Result<Self, DecodeError> {
                $validate(&raw)?;
                Ok(Self(raw))
            }
        }

        impl std::str::FromStr for $name {
            type Err = DecodeError;

            fn from_str(raw: &str) -> Result<Self, DecodeError> {
                Self::try_from(raw.to_owned())
            }
        }

        impl From<$name> for String {
            fn from(value: $name) -> String {
                value.0
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(&self.0)
            }
        }
    };
}

fn validate_lesson_id(raw: &str) -> Result<(), DecodeError> {
    // e.g. `L1`, `L26`, `L11-FILL` (the seed corpus's own supersede-fill
    // convention for a row whose `landed-at` was pending).
    let ok = raw
        .strip_prefix('L')
        .and_then(|rest| {
            let (number, suffix) = rest
                .split_once('-')
                .map_or((rest, None), |(number, suffix)| (number, Some(suffix)));
            let number_is_valid = !number.is_empty() && number.chars().all(|c| c.is_ascii_digit());
            let suffix_is_valid = suffix.is_none_or(|suffix| {
                !suffix.is_empty()
                    && suffix.split('-').all(|segment| {
                        !segment.is_empty() && segment.chars().all(|c| c.is_ascii_alphanumeric())
                    })
            });
            (number_is_valid && suffix_is_valid).then_some(())
        })
        .is_some();
    if ok {
        Ok(())
    } else {
        Err(DecodeError::new(
            "lessonId",
            "expected `L<number>[-SUFFIX]` (e.g. `L1`, `L26`, `L11-FILL`)",
        ))
    }
}

fn validate_artifact_ref(raw: &str) -> Result<(), DecodeError> {
    let ok = !raw.is_empty() && raw.len() <= 512;
    if ok {
        Ok(())
    } else {
        Err(DecodeError::new(
            "artifactRef",
            "expected a non-empty landed-artifact reference (path#anchor or path)",
        ))
    }
}

branded_string!(
    /// Branded lesson identifier (e.g. `L1`, `L26`, `L11-FILL`).
    LessonId,
    "lessonId",
    validate_lesson_id
);

branded_string!(
    /// Branded reference to a landed artifact (a file path, optionally with
    /// a `#anchor` naming the exact managed block/section that contains the
    /// lesson id).
    ArtifactRef,
    "artifactRef",
    validate_artifact_ref
);

/// Invalid date shapes are rejected at the persistence boundary; the empty
/// value remains the explicit representation of a legacy stream omission.
fn validate_captured_date(raw: &str) -> Result<(), DecodeError> {
    let is_iso_date = raw.len() == 10
        && raw.as_bytes().get(4) == Some(&b'-')
        && raw.as_bytes().get(7) == Some(&b'-')
        && raw
            .chars()
            .enumerate()
            .all(|(index, character)| matches!(index, 4 | 7) || character.is_ascii_digit());
    if raw.is_empty() || is_iso_date {
        Ok(())
    } else {
        Err(DecodeError::new(
            "capturedDate",
            "expected an ISO-8601 YYYY-MM-DD date or an absent date from a legacy stream",
        ))
    }
}

fn validate_lesson_text(raw: &str) -> Result<(), DecodeError> {
    if raw.trim().is_empty() {
        Err(DecodeError::new(
            "lessonText",
            "expected non-empty lesson text",
        ))
    } else {
        Ok(())
    }
}

fn validate_observed_evidence(_: &str) -> Result<(), DecodeError> {
    // Worker-memory imports may not carry an observation; preserve that
    // historical boundary input as an explicit empty evidence value.
    Ok(())
}

branded_string!(
    /// Captured calendar date, or the explicit empty legacy-stream value.
    CapturedDate,
    "capturedDate",
    validate_captured_date
);

branded_string!(
    /// The durable lesson statement; empty lessons cannot enter the ledger.
    LessonText,
    "lessonText",
    validate_lesson_text
);

branded_string!(
    /// Observed evidence attached to a lesson capture.
    ObservedEvidence,
    "observedEvidence",
    validate_observed_evidence
);

// ---------------------------------------------------------------------
// Record shape
// ---------------------------------------------------------------------

/// The learning thesis is DUAL-DOMAIN (`RUST_ARCHITECTURE` "The learning
/// thesis"): orchestration/protocol lessons and coding-fault/fix-pattern
/// lessons flow through the same loop.
/// SERIALIZATION-DOC: this is the stable persisted vocabulary for an
/// append-only lesson ledger. Its existing scalar representation is retained
/// so a reader can replay historic rows without a lossy format migration.
/// SERDE-TAG-JUSTIFICATION: this unit-only vocabulary is deliberately stored
/// as a scalar domain name; an adjacent tag would change canonical historic
/// ledger rows without adding information.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LessonDomain {
    /// An orchestration/protocol/harness lesson (mail lifecycle, claim
    /// discipline, worktree hygiene, ...).
    Harness,
    /// A coding-fault/fix-pattern lesson. Routed `RuleCandidate` REQUIRES
    /// fail/pass fixtures at landing (see [`LessonRoute::RuleCandidate`]).
    Code,
}

/// One harness surface a lesson can ship through.
/// SERIALIZATION-DOC: route names are durable ledger values. Keeping their
/// scalar representation makes previous append-only records replayable.
/// SERDE-TAG-JUSTIFICATION: this unit-only vocabulary is deliberately stored
/// as a single stable route name; adding an adjacent tag would change the
/// canonical bytes used by the existing hash chain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LessonRoute {
    /// The c01 shared install payload (`AGENTS.md`/`CLAUDE.md` managed
    /// blocks).
    DoctrineBlock,
    /// A keyed section appended to the enforcer skill.
    Skill,
    /// A d01 scaffolder rule-candidate input stub. A `Code`-domain lesson
    /// routed here REQUIRES fail/pass fixtures at landing — the d01 parity
    /// oracle applies; a coding lesson without fixtures cannot land.
    RuleCandidate,
    /// A b06 decision-forest node fragment.
    ForestNode,
    /// An `EXECUTION_MODEL` §-ref. TRANSITIONAL ONLY: prose is not a
    /// landing, so a lesson whose ONLY route is `PlanDoc` is flagged
    /// `Severity::Warning` by [`run_doctor`], never `Severity::Error`.
    PlanDoc,
}

impl LessonRoute {
    /// The `templates/lesson-<kind>.tpl` file this route renders from, or
    /// `None` for [`LessonRoute::PlanDoc`] (a plan-doc route has no
    /// dedicated emitter template — it names an existing plan section by
    /// convention, it does not render a new artifact).
    pub fn template_name(self) -> Option<&'static str> {
        match self {
            LessonRoute::DoctrineBlock => Some("lesson-doctrine-block.tpl"),
            LessonRoute::Skill => Some("lesson-skill.tpl"),
            LessonRoute::RuleCandidate => Some("lesson-rule-candidate.tpl"),
            LessonRoute::ForestNode => Some("lesson-forest-node.tpl"),
            LessonRoute::PlanDoc => None,
        }
    }
}

/// One captured lesson. serde camelCase on the wire (workspace convention).
/// SERIALIZATION-DOC: the append-only ledger serializes this exact public
/// record; boundary decoding validates its branded values before persistence.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
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
    #[serde(default)]
    pub landed_at: Vec<ArtifactRef>,
    /// Set on a supersede-append record: the id of the earlier record this
    /// one supersedes (same `id`, later journal position). `None` on a
    /// lesson's first capture.
    // DEFAULT-JUSTIFICATION: old rows have no supersession and must retain
    // that explicit `None` state when replayed.
    #[serde(default)]
    pub supersedes_seq: Option<usize>,
}

impl LessonRecord {
    /// True when every declared route has landed (or the lesson declares no
    /// routes needing a landing — i.e. every route is `PlanDoc`).
    pub fn is_fully_landed(&self) -> bool {
        // A captured lesson with no route has no landing plan yet. It must
        // remain pending so the CLI's pending view agrees with `run_doctor`'s
        // fail-closed treatment of an unrouted ledger row.
        if self.routes.is_empty() {
            return false;
        }
        let required_routes = self
            .routes
            .iter()
            .copied()
            .filter(|route| *route != LessonRoute::PlanDoc)
            .collect::<HashSet<_>>();
        let landed_artifacts = self.landed_at.iter().collect::<HashSet<_>>();
        required_routes.len() <= landed_artifacts.len()
    }

    /// True when this lesson's only declared route(s) are `PlanDoc`
    /// (transitional-only capture, never `Severity::Error`).
    pub fn is_plan_doc_only(&self) -> bool {
        !self.routes.is_empty() && self.routes.iter().all(|r| *r == LessonRoute::PlanDoc)
    }
}

// ---------------------------------------------------------------------
// Ledger: append-only NDJSON, hash-chained, verify-on-open
// ---------------------------------------------------------------------

/// One on-disk ledger line: the record plus the hash-chain digest folding
/// in the previous line's digest. Mirrors `enforcer-proof`'s
/// `JournalLine` shape exactly (same tamper-evidence contract), kept
/// crate-local since `enforcer-plan` does not depend on `enforcer-proof`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
struct LedgerLine {
    record: LessonRecord,
    digest: String,
}

/// An append-only, hash-chained NDJSON lesson ledger at `path` (by
/// convention `.enforce/lessons.ndjson`, but callers inject the path so
/// tests never touch a real repo-relative location).
#[derive(Debug)]
pub struct LessonLedger {
    path: PathBuf,
    last_digest: Option<String>,
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
    line_index: usize,
    /// The digest recorded on the broken line.
    recorded: String,
    /// The digest recomputed from payload + previous digest.
    expected: String,
}

impl LessonLedger {
    /// Open (or create) the ledger at `path`, verifying the existing chain
    /// (if any) before returning. Fails closed on any break — the same
    /// verify-on-open discipline `enforcer-proof`'s journal uses.
    pub fn open(path: &Path) -> Result<Self, PlanError> {
        let last_digest = if path.exists() {
            let lines = read_lines(path)?;
            verify_lines(&lines)?;
            // CLONE-JUSTIFICATION: the ledger retains the terminal digest
            // after the parsed line vector is dropped, so the next append
            // can extend the verified chain without retaining all rows.
            lines.last().map(|line| line.digest.clone())
        } else {
            None
        };
        Ok(Self {
            path: path.to_path_buf(),
            last_digest,
        })
    }

    /// Append one NEW lesson capture. Fails if a record with the same id
    /// already exists and is not itself the special `-FILL` convention (use
    /// [`LessonLedger::supersede`] to fill in a pending `landed_at` instead
    /// of calling `append` again for the same id).
    pub fn append(&mut self, record: LessonRecord) -> Result<(), PlanError> {
        if record.supersedes_seq.is_some() {
            return Err(PlanError::Io {
                path: self.path.display().to_string(),
                reason: format!(
                    "lesson `{}` declares a supersession; use supersede to create linked ledger rows",
                    record.id
                ),
            });
        }
        let existing = self.list()?;
        if existing.iter().any(|r| r.id == record.id) {
            // ALLOC-JUSTIFICATION: `PlanError` owns a stable filesystem
            // location and diagnostic after this failed append returns.
            return Err(PlanError::Io {
                path: self.path.display().to_string(),
                reason: format!(
                    "lesson `{}` already captured; use supersede to fill in landed_at",
                    record.id
                ),
            });
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
            .ok_or_else(|| PlanError::Io {
                path: self.path.display().to_string(),
                reason: format!("cannot supersede unknown lesson `{id}`"),
            })?;
        // CLONE-JUSTIFICATION: supersession writes a new immutable ledger
        // row; the verified prior row must remain unchanged in the history.
        let mut merged = prior.clone();
        for artifact in additional_landed_at {
            if !merged.landed_at.contains(&artifact) {
                merged.landed_at.push(artifact);
            }
        }
        merged.supersedes_seq = Some(seq);
        self.write_line(merged)
    }

    fn write_line(&mut self, record: LessonRecord) -> Result<(), PlanError> {
        // ALLOC-JUSTIFICATION: canonical journal bytes and digest text outlive
        // serialization frames and are persisted as one immutable chain row.
        let canonical = serde_json::to_vec(&record).map_err(|e| PlanError::Io {
            path: self.path.display().to_string(),
            reason: e.to_string(),
        })?;
        let digest = link_digest(self.last_digest.as_deref(), &canonical);
        // CLONE-JUSTIFICATION: the writer borrows a line while `last_digest`
        // must retain the same value for the subsequent append.
        let line = LedgerLine {
            record,
            digest: digest.clone(),
        };
        let mut writer: enforcer_core::ndjson_writer::NdjsonWriter<LedgerLine> =
            enforcer_core::ndjson_writer::NdjsonWriter::open(&self.path).map_err(|e| {
                PlanError::Io {
                    path: self.path.display().to_string(),
                    reason: e.to_string(),
                }
            })?;
        writer.append(&line).map_err(|e| PlanError::Io {
            path: self.path.display().to_string(),
            reason: e.to_string(),
        })?;
        self.last_digest = Some(digest);
        Ok(())
    }

    /// Re-read the ledger from disk and verify-on-replay (independent of
    /// in-memory state, so a caller can re-validate a ledger another
    /// process may have appended to since it was opened).
    pub fn verify_on_replay(&self) -> Result<usize, PlanError> {
        let lines = read_lines(&self.path)?;
        verify_lines(&lines)
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

fn read_lines(path: &Path) -> Result<Vec<LedgerLine>, PlanError> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    // ALLOC-JUSTIFICATION: IO diagnostics outlive the source error and must
    // own the requested ledger path and operating-system message.
    enforcer_core::ndjson_writer::read_all(path).map_err(|e| PlanError::Io {
        path: path.display().to_string(),
        reason: e.to_string(),
    })
}

fn verify_lines(lines: &[LedgerLine]) -> Result<usize, PlanError> {
    // ALLOC-JUSTIFICATION: hash-chain verification consumes canonical owned
    // bytes for all records before it evaluates links against their digests.
    let canonical: Vec<Vec<u8>> = lines
        .iter()
        .map(|line| {
            serde_json::to_vec(&line.record).map_err(|error| PlanError::Io {
                path: "lesson ledger".to_owned(),
                reason: error.to_string(),
            })
        })
        .collect::<Result<_, _>>()?;
    let links = canonical
        .iter()
        .map(Vec::as_slice)
        .zip(lines.iter().map(|line| line.digest.as_str()));
    verify_chain(links)
        .map_err(|break_| LedgerTamper {
            line_index: break_.index,
            recorded: break_.recorded,
            expected: break_.expected,
        })
        .map_err(|tamper| tamper_to_plan_error(&tamper))?;
    verify_supersession_state(lines)?;
    Ok(lines.len())
}

/// Verify that a supersede-append record extends the latest prior state of
/// the same lesson. Hash-chain integrity alone proves row order and bytes,
/// but not that a hash-valid row obeys the ledger's state-transition
/// contract: supersession may only add landed artifacts, never replace the
/// lesson identity or discard previous landing evidence.
fn verify_supersession_state(lines: &[LedgerLine]) -> Result<(), PlanError> {
    for (index, line) in lines.iter().enumerate() {
        let Some(prior_index) = line.record.supersedes_seq else {
            continue;
        };
        let Some(prior) = lines.get(prior_index) else {
            return Err(invalid_supersession(
                index,
                "references a missing prior row",
            ));
        };
        if prior_index >= index {
            return Err(invalid_supersession(index, "must reference an earlier row"));
        }
        if prior.record.id != line.record.id {
            return Err(invalid_supersession(
                index,
                "references a different lesson id",
            ));
        }
        let latest_prior_index = lines[..index]
            .iter()
            .rposition(|candidate| candidate.record.id == line.record.id);
        if latest_prior_index != Some(prior_index) {
            return Err(invalid_supersession(
                index,
                "does not extend the latest prior state for its lesson",
            ));
        }
        let unchanged_identity = prior.record.date == line.record.date
            && prior.record.domain == line.record.domain
            && prior.record.observed == line.record.observed
            && prior.record.lesson == line.record.lesson
            && prior.record.routes == line.record.routes;
        if !unchanged_identity {
            return Err(invalid_supersession(
                index,
                "changes immutable lesson identity fields",
            ));
        }
        if !prior
            .record
            .landed_at
            .iter()
            .all(|artifact| line.record.landed_at.contains(artifact))
        {
            return Err(invalid_supersession(
                index,
                "removes a previously landed artifact",
            ));
        }
    }
    Ok(())
}

fn invalid_supersession(line_index: usize, reason: &str) -> PlanError {
    PlanError::Io {
        path: "lesson ledger".to_owned(),
        reason: format!("invalid lesson supersession at line {line_index}: {reason}"),
    }
}

fn tamper_to_plan_error(tamper: &LedgerTamper) -> PlanError {
    // ALLOC-JUSTIFICATION: typed diagnostics own the tamper description once
    // the borrowed verification report is no longer available.
    PlanError::Io {
        path: "lesson ledger".to_owned(),
        reason: tamper.to_string(),
    }
}

// ---------------------------------------------------------------------
// CLI/MCP seam: add / list / route
// ---------------------------------------------------------------------

/// `enforcer lesson add` — capture a new lesson. CLI seam for arc-22, MCP
/// tool seam for arc-21.
pub fn add(ledger_path: &Path, record: LessonRecord) -> Result<LessonRecord, PlanError> {
    let mut ledger = LessonLedger::open(ledger_path)?;
    // CLONE-JUSTIFICATION: append consumes the persisted row while the CLI
    // contract returns the caller's original validated record.
    ledger.append(record.clone())?;
    Ok(record)
}

/// `enforcer lesson list` — list captured lessons, optionally filtered by
/// route or pending-only (a lesson with at least one un-landed route).
pub fn list(
    ledger_path: &Path,
    route_filter: Option<LessonRoute>,
    pending_only: bool,
) -> Result<Vec<LessonRecord>, PlanError> {
    let ledger = LessonLedger::open(ledger_path)?;
    let mut records = ledger.latest()?;
    if let Some(route) = route_filter {
        records.retain(|r| r.routes.contains(&route));
    }
    if pending_only {
        records.retain(|r| !r.is_fully_landed());
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
    fn read(&self, path: &Path) -> Result<Option<String>, PlanError>;
    /// Write a file's full content (creating parent dirs as needed).
    fn write(&mut self, path: &Path, content: &str) -> Result<(), PlanError>;
}

/// A real-filesystem [`EmitFs`] implementation.
#[derive(Debug, Default)]
pub struct RealFs;

impl EmitFs for RealFs {
    fn read(&self, path: &Path) -> Result<Option<String>, PlanError> {
        match std::fs::read_to_string(path) {
            Ok(content) => Ok(Some(content)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            // ALLOC-JUSTIFICATION: PlanError owns the failed boundary path
            // and diagnostic after the operating-system error is released.
            Err(error) => Err(PlanError::Io {
                path: path.display().to_string(),
                reason: error.to_string(),
            }),
        }
    }

    fn write(&mut self, path: &Path, content: &str) -> Result<(), PlanError> {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent).map_err(|e| PlanError::Io {
                    path: path.display().to_string(),
                    reason: e.to_string(),
                })?;
            }
        }
        std::fs::write(path, content).map_err(|e| PlanError::Io {
            path: path.display().to_string(),
            reason: e.to_string(),
        })
    }
}

/// Deterministic `{{name}}` placeholder substitution — a byte-for-byte copy
/// of the same minimal contract `crate::templates` (b03) and
/// `crate::agents_forest` (b06) each independently established (both are
/// private/local to their own templates): missing placeholder -> typed
/// error, never a panic.
fn render(template: &str, bindings: &HashMap<String, String>) -> Result<String, PlanError> {
    // ALLOC-JUSTIFICATION: rendering performs successive substitutions and
    // returns an owned artifact that outlives both borrowed inputs.
    let mut result = template.to_owned();
    for (name, value) in bindings {
        let placeholder = format!("{{{{{name}}}}}");
        if result.contains(&placeholder) {
            result = result.replace(&placeholder, value);
        }
    }
    if let Some(pos) = result.find("{{") {
        if let Some(unresolved) = result.get(pos..) {
            if let Some(end) = unresolved.find("}}") {
                if let Some(placeholder_length) = end.checked_add(2) {
                    if let Some(placeholder) = unresolved.get(..placeholder_length) {
                        // ALLOC-JUSTIFICATION: the missing token is retained in the
                        // returned diagnostic after the mutable render buffer is gone.
                        return Err(PlanError::Io {
                            path: "lesson template".to_owned(),
                            reason: format!("missing placeholder: {placeholder}"),
                        });
                    }
                }
            }
        }
    }
    Ok(result)
}

fn domain_marker(domain: LessonDomain) -> &'static str {
    match domain {
        LessonDomain::Harness => "harness",
        LessonDomain::Code => "code",
    }
}

fn render_bindings(record: &LessonRecord) -> HashMap<String, String> {
    // ALLOC-JUSTIFICATION: a rendered template needs owned substitutions
    // after the borrowed lesson record has left this helper.
    let mut bindings = HashMap::new();
    bindings.insert("lesson_id".to_owned(), record.id.as_str().to_owned());
    bindings.insert("date".to_owned(), record.date.as_str().to_owned());
    bindings.insert("domain".to_owned(), domain_marker(record.domain).to_owned());
    bindings.insert("observed".to_owned(), record.observed.as_str().to_owned());
    bindings.insert("lesson".to_owned(), record.lesson.as_str().to_owned());
    bindings
}

/// One emitter's outcome: the rendered artifact text, the target path it
/// was (or would be) written to, and whether a write actually happened
/// (always `false` when `dry_run` was set).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmitOutcome {
    /// Target path the artifact was (or would be) written to.
    pub path: PathBuf,
    /// Rendered artifact text.
    pub rendered: String,
    /// `true` iff a write actually happened.
    pub wrote: bool,
}

/// Render one route's template for `record` and, unless `dry_run`, append
/// (or replace-in-place) the rendered managed block into `target_path`
/// through `fs`. Preserves any unrelated existing content in
/// `target_path`: if the file already exists and already carries a managed
/// block for this exact lesson id, that block is replaced in place;
/// otherwise the rendered block is appended.
fn emit_route(
    fs: &mut dyn EmitFs,
    template: &str,
    record: &LessonRecord,
    target_path: &Path,
    dry_run: bool,
) -> Result<EmitOutcome, PlanError> {
    let rendered = render(template, &render_bindings(record))?;
    if dry_run {
        return Ok(EmitOutcome {
            path: target_path.to_path_buf(),
            rendered,
            wrote: false,
        });
    }
    let existing = match fs.read(target_path)? {
        Some(content) => content,
        // ALLOC-JUSTIFICATION: the empty owned buffer is required as the
        // replacement/append accumulator when the target is genuinely absent.
        None => String::new(),
    };
    let merged = replace_or_append_block(&existing, &rendered, record.id.as_str());
    fs.write(target_path, &merged)?;
    Ok(EmitOutcome {
        path: target_path.to_path_buf(),
        rendered,
        wrote: true,
    })
}

/// Replace an existing managed block naming `lesson_id` in `existing` with
/// `new_block`, or append `new_block` if no such block is present.
/// Detection is anchored on the lesson id appearing inside an HTML-comment
/// marker line (`<!-- ...lesson_id... -->`) so unrelated content (other
/// lessons' blocks, hand-authored prose) is never touched.
fn replace_or_append_block(existing: &str, new_block: &str, lesson_id: &str) -> String {
    // ALLOC-JUSTIFICATION: managed-block matching needs an owned needle
    // while iterating borrowed lines from the existing artifact.
    let marker_needle = lesson_id.to_owned();
    let open_marker_line = existing
        .lines()
        .find(|line| line.trim_start().starts_with("<!--") && line.contains(&marker_needle));
    let Some(open_line) = open_marker_line else {
        if existing.is_empty() {
            return new_block.to_owned();
        }
        return format!("{}\n\n{}", existing.trim_end(), new_block);
    };
    // Find the matching close marker (same lesson id, contains "/").
    let lines: Vec<&str> = existing.lines().collect();
    let Some(open_idx) = lines.iter().position(|line| *line == open_line) else {
        return format!("{}\n\n{}", existing.trim_end(), new_block);
    };
    let Some(lines_from_open) = lines.get(open_idx..) else {
        return format!("{}\n\n{}", existing.trim_end(), new_block);
    };
    let close_offset = lines_from_open
        .iter()
        .position(|line| line.trim_start().starts_with("<!-- /") && line.contains(&marker_needle))
        .and_then(|offset| open_idx.checked_add(offset));
    let Some(close_idx) = close_offset else {
        return format!("{}\n\n{}", existing.trim_end(), new_block);
    };
    let Some(after_close_start) = close_idx.checked_add(1) else {
        return format!("{}\n\n{}", existing.trim_end(), new_block);
    };
    let Some(before_open) = lines.get(..open_idx) else {
        return format!("{}\n\n{}", existing.trim_end(), new_block);
    };
    let Some(after_close) = lines.get(after_close_start..) else {
        return format!("{}\n\n{}", existing.trim_end(), new_block);
    };
    let mut out_lines: Vec<&str> = Vec::new();
    out_lines.extend_from_slice(before_open);
    for new_line in new_block.lines() {
        out_lines.push(new_line);
    }
    out_lines.extend_from_slice(after_close);
    let mut out = out_lines.join("\n");
    out.push('\n');
    out
}

/// Doctrine-block emitter template, embedded at compile time.
const DOCTRINE_BLOCK_TEMPLATE: &str = include_str!("../templates/lesson-doctrine-block.tpl");
/// Skill emitter template, embedded at compile time.
const SKILL_TEMPLATE: &str = include_str!("../templates/lesson-skill.tpl");
/// Rule-candidate emitter template, embedded at compile time.
const RULE_CANDIDATE_TEMPLATE: &str = include_str!("../templates/lesson-rule-candidate.tpl");
/// Forest-node emitter template, embedded at compile time.
const FOREST_NODE_TEMPLATE: &str = include_str!("../templates/lesson-forest-node.tpl");

/// Emit the doctrine-block route (c01 shared install payload) for `record`.
pub fn emit_doctrine_block(
    fs: &mut dyn EmitFs,
    record: &LessonRecord,
    target_path: &Path,
    dry_run: bool,
) -> Result<EmitOutcome, PlanError> {
    emit_route(fs, DOCTRINE_BLOCK_TEMPLATE, record, target_path, dry_run)
}

/// Emit the skill route (a keyed section in the enforcer skill) for
/// `record`.
pub fn emit_skill(
    fs: &mut dyn EmitFs,
    record: &LessonRecord,
    target_path: &Path,
    dry_run: bool,
) -> Result<EmitOutcome, PlanError> {
    emit_route(fs, SKILL_TEMPLATE, record, target_path, dry_run)
}

/// Emit the rule-candidate route (a d01 scaffolder input stub) for
/// `record`. Callers MUST NOT treat this emission alone as "landed" for a
/// `Code`-domain lesson — [`run_doctor`] additionally requires fail/pass
/// fixtures to exist before a `Code`+`RuleCandidate` lesson counts as
/// landed (see [`RuleCandidateFixtures`]).
pub fn emit_rule_candidate(
    fs: &mut dyn EmitFs,
    record: &LessonRecord,
    target_path: &Path,
    dry_run: bool,
) -> Result<EmitOutcome, PlanError> {
    emit_route(fs, RULE_CANDIDATE_TEMPLATE, record, target_path, dry_run)
}

/// Emit the forest-node route (a b06 decision-forest node fragment) for
/// `record`. The fragment is schema-compatible with b06's own managed-block
/// conventions (`<!-- forest-node:... -->` / `LEAF ->` pointer) but this
/// module does not call into `crate::agents_forest` directly — coordination
/// is by fragment schema, not shared files (workpack "Parallel Ownership
/// Notes").
pub fn emit_forest_node(
    fs: &mut dyn EmitFs,
    record: &LessonRecord,
    target_path: &Path,
    dry_run: bool,
) -> Result<EmitOutcome, PlanError> {
    emit_route(fs, FOREST_NODE_TEMPLATE, record, target_path, dry_run)
}

/// `enforcer lesson route <id>` — run every emitter implied by `record`'s
/// declared routes against the given target paths, honoring `dry_run`.
/// `targets` maps each non-`PlanDoc` route present in `record.routes` to
/// the artifact path it should land at; a route with no entry in `targets`
/// is skipped (not an error — callers may route a subset at a time).
pub fn route(
    fs: &mut dyn EmitFs,
    record: &LessonRecord,
    targets: &HashMap<LessonRoute, PathBuf>,
    dry_run: bool,
) -> Result<Vec<EmitOutcome>, PlanError> {
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
            LessonRoute::DoctrineBlock => emit_doctrine_block(fs, record, target, dry_run)?,
            LessonRoute::Skill => emit_skill(fs, record, target, dry_run)?,
            LessonRoute::RuleCandidate => emit_rule_candidate(fs, record, target, dry_run)?,
            LessonRoute::ForestNode => emit_forest_node(fs, record, target, dry_run)?,
            LessonRoute::PlanDoc => continue,
        };
        outcomes.push(outcome);
    }
    Ok(outcomes)
}

// ---------------------------------------------------------------------
// Fail-closed doctor
// ---------------------------------------------------------------------

/// Whether a `Code`-domain lesson routed `RuleCandidate` has its required
/// fail/pass fixtures. Callers supply this (the doctor does not walk the
/// filesystem itself for fixture discovery — that is the d01 scaffolder's
/// concern) so the doctor stays a pure function over supplied facts,
/// consistent with every other `Validator`-family check in this crate.
/// The parity state of the fail/pass fixture pair required for a
/// code-domain rule candidate. This is a closed state model rather than two
/// independently mutable flags, so callers cannot represent an unnamed or
/// ambiguous fixture condition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleCandidateFixtures {
    /// Both required fixture classes are present.
    Complete,
    /// Neither required fixture class is present.
    MissingBoth,
    /// A failing fixture is present, but its passing counterpart is absent.
    MissingPass,
    /// A passing fixture is present, but its failing counterpart is absent.
    MissingFail,
}

fn lesson_finding(rule_id: &RuleId, severity: Severity, detail: String, file: &RelPath) -> Finding {
    // CLONE-JUSTIFICATION: findings are durable diagnostic values and must
    // own the rule and source path after the doctor input has been released.
    Finding {
        rule_id: rule_id.clone(),
        severity,
        title: "lesson-capture doctor".to_owned(),
        detail,
        file: file.clone(),
        line: 1,
        snippet: None,
    }
}

fn synthetic_doctor_path() -> Result<RelPath, PlanError> {
    // ALLOC-JUSTIFICATION: RelPath and the typed PlanError both own their
    // values; this fixed synthetic finding path crosses that owned boundary.
    RelPath::try_from("lessons.ndjson".to_owned()).map_err(|error| PlanError::Io {
        path: "lesson doctor".to_owned(),
        reason: error.to_string(),
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
    landed_artifact_contents: &HashMap<ArtifactRef, String>,
    rule_candidate_fixtures: &HashMap<LessonId, RuleCandidateFixtures>,
) -> Result<Vec<Finding>, PlanError> {
    let mut findings = Vec::new();
    let file = synthetic_doctor_path()?;

    for record in records {
        if record.is_plan_doc_only() {
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
                    .is_some_and(|content| content.contains(record.id.as_str()))
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

/// One row parsed from the seed ledger's markdown table.
#[derive(Debug, Clone, PartialEq, Eq)]
struct SeedRow {
    id: String,
    date: String,
    observed: String,
    lesson: String,
    landed_at: String,
    ships_via: String,
}

/// Parse every `| id | date | observed | lesson | landed-at | ships-via |`
/// table row out of one seed-ledger markdown document (the preamble
/// `refs/orchestration-lessons.md` or a `refs/lessons/<domain>-NN.md`
/// shard). Skips the header row and the `|---|---|...` separator row.
/// Tolerant of blank lines between rows (the seed corpus inserts blank
/// lines between some rows for readability).
fn parse_seed_row(line: &str) -> Option<SeedRow> {
    let trimmed = line.trim();
    if !trimmed.starts_with('|') {
        return None;
    }

    let mut cells = trimmed
        .trim_start_matches('|')
        .trim_end_matches('|')
        .split('|')
        .map(str::trim);
    let id = cells.next()?;
    if id == "id"
        || id
            .chars()
            .all(|character| character == '-' || character == ':')
    {
        return None;
    }
    if !id.starts_with('L') {
        return None;
    }

    let date = cells.next()?;
    let observed = cells.next()?;
    let lesson = cells.next()?;
    let landed_at = cells.next()?;
    let ships_via = cells.next()?;
    // ALLOC-JUSTIFICATION: the parsed row owns all cells so conversion can
    // validate them after this borrowed markdown line has been released.
    Some(SeedRow {
        id: id.to_owned(),
        date: date.to_owned(),
        observed: observed.to_owned(),
        lesson: lesson.to_owned(),
        landed_at: landed_at.to_owned(),
        ships_via: ships_via.to_owned(),
    })
}

fn parse_seed_rows(markdown: &str) -> Vec<SeedRow> {
    markdown.lines().filter_map(parse_seed_row).collect()
}

/// Map a seed row's `ships-via` free text to zero or more `LessonRoute` values
/// by keyword sniffing (the seed corpus's `ships-via` column is prose, not
/// a closed vocabulary — e.g. "c01 doctrine payload (worker-protocol
/// snippet)", "b06 decision forest", "fixed MCP tool behavior (arc-16)").
/// A row matching no known keyword lands as `PlanDoc` (transitional-only —
/// safer than silently dropping it) so [`run_doctor`] still reports it
/// (as a `Warning`), never silently skips it.
fn sniff_routes(ships_via: &str, landed_at: &str) -> Vec<LessonRoute> {
    // ALLOC-JUSTIFICATION: route classification normalizes two borrowed
    // boundary cells into one short-lived, case-insensitive search buffer.
    let haystack = format!("{ships_via} {landed_at}").to_lowercase();
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

/// Sniff a seed row's domain from its `observed` cell's explicit `[code]`
/// / `[harness]` tag (rows L13+ per the ledger's own doctrine); rows
/// without a tag default to `Harness` (the ledger's stated default for
/// "the rest" of the untagged seed rows).
fn sniff_domain(observed: &str) -> LessonDomain {
    // ALLOC-JUSTIFICATION: case normalization is required before matching
    // historical free-text evidence independent of the source's casing.
    let lower = observed.to_lowercase();
    if lower.trim_start().starts_with("[code]") {
        LessonDomain::Code
    } else {
        LessonDomain::Harness
    }
}

fn seed_row_to_record(row: &SeedRow) -> Result<LessonRecord, PlanError> {
    // ALLOC-JUSTIFICATION: seed parsing converts borrowed table cells into
    // independently owned, validated ledger records.
    // CLONE-JUSTIFICATION: the imported record owns its id while the parsed
    // seed row remains available for the remaining route and artifact fields.
    let id = LessonId::try_from(row.id.clone()).map_err(|e: DecodeError| PlanError::Io {
        path: "seed corpus".to_owned(),
        reason: e.to_string(),
    })?;
    let landed_at = if row.landed_at.trim().is_empty() {
        Vec::new()
    } else {
        vec![ArtifactRef::try_from(format!(
            "{}#{}",
            "docs/plans/enforcer-selfhost-plan/refs/orchestration-lessons.md", row.id
        ))
        .map_err(|e: DecodeError| PlanError::Io {
            path: "seed corpus".to_owned(),
            reason: e.to_string(),
        })?]
    };
    Ok(LessonRecord {
        id,
        // CLONE-JUSTIFICATION: the durable record owns each validated cell;
        // the parsed seed row remains borrowed for its other conversions.
        // ALLOC-JUSTIFICATION: decode errors retain owned diagnostics after
        // the borrowed markdown row has been released.
        date: row
            .date
            .clone()
            .parse()
            .map_err(|error: DecodeError| PlanError::Io {
                path: "seed corpus".to_owned(),
                reason: error.to_string(),
            })?,
        domain: sniff_domain(&row.observed),
        // CLONE-JUSTIFICATION: evidence becomes an independently owned
        // branded field in the persisted append-only record.
        // ALLOC-JUSTIFICATION: a validation failure must keep its owned
        // diagnostic beyond this borrowed seed-row conversion.
        observed: row
            .observed
            .clone()
            .parse()
            .map_err(|error: DecodeError| PlanError::Io {
                path: "seed corpus".to_owned(),
                reason: error.to_string(),
            })?,
        // CLONE-JUSTIFICATION: lesson text is persisted independently of
        // the short-lived parsed table row.
        // ALLOC-JUSTIFICATION: conversion errors own their message across
        // the importer boundary.
        lesson: row
            .lesson
            .clone()
            .parse()
            .map_err(|error: DecodeError| PlanError::Io {
                path: "seed corpus".to_owned(),
                reason: error.to_string(),
            })?,
        routes: sniff_routes(&row.ships_via, &row.landed_at),
        landed_at,
        supersedes_seq: None,
    })
}

/// One NDJSON memory-stream record this importer also folds in
/// (`memory/streams/*.ndjson`, per the x05 workpack's additional import
/// sources note). Deliberately minimal/tolerant: only the fields this
/// importer needs, everything else in a real memory-stream record is
/// ignored rather than causing a decode failure.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct MemoryStreamRecord {
    id: String,
    // DEFAULT-JUSTIFICATION: streams are evolving boundary payloads; each
    // optional field must decode as absent so the importer can fail closed
    // only after it identifies an actual lesson record.
    #[serde(default)]
    date: Option<String>,
    #[serde(default)]
    domain: Option<String>,
    #[serde(default)]
    observed: Option<String>,
    #[serde(default)]
    lesson: Option<String>,
    #[serde(default)]
    ships_via: Option<String>,
    #[serde(default)]
    landed_at: Option<String>,
}

fn memory_record_to_lesson(raw: &MemoryStreamRecord) -> Option<Result<LessonRecord, PlanError>> {
    // Only fold in memory-stream records that look like a lesson capture
    // (carry both an id starting with `L`/`mem-` shape AND a lesson body) —
    // ordinary provenance/status records in the same stream are silently
    // skipped, not erred on, since this importer's job is lesson rows, not
    // full memory-stream validation.
    if !raw.id.starts_with('L') {
        return None;
    }
    // CLONE-JUSTIFICATION: conversion consumes a lesson body while the raw
    // transport record remains borrowed for the remaining validated fields.
    let raw_lesson = raw.lesson.clone()?;
    Some((|| -> Result<LessonRecord, PlanError> {
        // ALLOC-JUSTIFICATION: each decode failure owns a stable boundary
        // name and message after the transport record is no longer borrowed.
        let lesson: LessonText =
            raw_lesson
                .parse()
                .map_err(|error: DecodeError| PlanError::Io {
                    path: "memory stream".to_owned(),
                    reason: error.to_string(),
                })?;
        // CLONE-JUSTIFICATION: optional transport evidence becomes an owned
        // validated value while the raw DTO remains borrowed.
        let observed: ObservedEvidence =
            raw.observed
                .clone()
                .unwrap_or_default()
                .parse()
                .map_err(|error: DecodeError| PlanError::Io {
                    path: "memory stream".to_owned(),
                    reason: error.to_string(),
                })?;
        let id: LessonId = raw.id.parse().map_err(|e: DecodeError| PlanError::Io {
            path: "memory stream".to_owned(),
            reason: e.to_string(),
        })?;
        // CLONE-JUSTIFICATION: route classification needs an owned value
        // independent of the raw transport input.
        let ships_via = raw.ships_via.clone().unwrap_or_default();
        let landed_at_cell = raw.landed_at.clone().unwrap_or_default();
        let landed_at = if landed_at_cell.trim().is_empty() {
            Vec::new()
        } else {
            vec![landed_at_cell
                .parse()
                .map_err(|e: DecodeError| PlanError::Io {
                    path: "memory stream".to_owned(),
                    reason: e.to_string(),
                })?]
        };
        let domain = match raw.domain.as_deref() {
            Some("code") => LessonDomain::Code,
            _ => sniff_domain(observed.as_str()),
        };
        Ok(LessonRecord {
            id,
            date: raw
                .date
                .clone()
                .unwrap_or_default()
                .parse()
                .map_err(|error: DecodeError| PlanError::Io {
                    path: "memory stream".to_owned(),
                    reason: error.to_string(),
                })?,
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
    pub discovered: usize,
    /// Rows newly appended to the ledger this run (0 on a repeat import
    /// over unchanged sources).
    pub newly_appended: usize,
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
fn assign_seed_import_ids(candidates: &mut Vec<SeedImportCandidate>) -> Result<(), PlanError> {
    let mut label_counts: HashMap<String, usize> = HashMap::new();
    for candidate in candidates.iter() {
        // ALLOC-JUSTIFICATION: the count map owns labels while records are
        // inspected later, after this short-lived candidate borrow ends.
        *label_counts
            .entry(candidate.record.id.as_str().to_owned())
            .or_default() += 1;
    }

    let mut identical_record_occurrences: HashMap<(String, String), usize> = HashMap::new();
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
            path: "seed corpus".to_owned(),
            reason: error.to_string(),
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
        let fingerprint = digest
            .strip_prefix("sha256:")
            .ok_or_else(|| PlanError::Io {
                // ALLOC-JUSTIFICATION: `PlanError` owns the structural
                // digest diagnostic after this import call returns.
                path: "seed corpus".to_owned(),
                reason: "seed identity digest did not contain a SHA-256 prefix".to_owned(),
            })?;
        // CLONE-JUSTIFICATION: the occurrence table owns the complete stable
        // identity while `fingerprint` remains borrowed from `digest`.
        let occurrence = identical_record_occurrences
            .entry((displayed_label.clone(), fingerprint.to_owned()))
            .or_default();
        *occurrence += 1;
        let id = if *occurrence == 1 {
            format!("{displayed_label}-SRC-{fingerprint}")
        } else {
            format!("{displayed_label}-SRC-{fingerprint}-{occurrence}")
        };
        candidate.record.id = id.parse().map_err(|error: DecodeError| PlanError::Io {
            // ALLOC-JUSTIFICATION: conversion diagnostics cross the
            // importer boundary as owned `PlanError` values.
            path: "seed corpus".to_owned(),
            reason: error.to_string(),
        })?;
    }
    Ok(())
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
    seed_markdown_sources: &[String],
    memory_stream_sources: &[String],
) -> Result<ImportOutcome, PlanError> {
    let existing_ids: std::collections::HashSet<LessonId> =
        ledger.latest()?.into_iter().map(|r| r.id).collect();

    let mut candidates = Vec::new();

    for markdown in seed_markdown_sources {
        for row in parse_seed_rows(markdown) {
            candidates.push(SeedImportCandidate {
                record: seed_row_to_record(&row)?,
                source_kind: SeedImportSourceKind::Markdown,
            });
        }
    }

    for stream in memory_stream_sources {
        for line in stream.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let Ok(raw) = serde_json::from_str::<MemoryStreamRecord>(trimmed) else {
                continue;
            };
            let Some(result) = memory_record_to_lesson(&raw) else {
                continue;
            };
            candidates.push(SeedImportCandidate {
                record: result?,
                source_kind: SeedImportSourceKind::Memory,
            });
        }
    }

    assign_seed_import_ids(&mut candidates)?;
    let discovered = candidates.len();
    let mut newly_appended = 0usize;
    for candidate in candidates {
        if !existing_ids.contains(&candidate.record.id) {
            ledger.append(candidate.record)?;
            newly_appended += 1;
        }
    }

    Ok(ImportOutcome {
        discovered,
        newly_appended,
    })
}
