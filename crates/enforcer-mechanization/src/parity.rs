//! The whole-registry, fail-closed 5-way parity oracle: `ruleId <->
//! validator <-> doc-anchor <-> {fail,pass fixtures} <-> registry-record`.
//!
//! [`crate::oracle::accept_rule`] proves ONE candidate record against ONE
//! supplied validator. This module is the registry-wide sweep the d01
//! workpack's `parity` module owns: it walks every [`RuleRecord`] in a
//! loaded [`RuleRegistry`], resolves each record's validator from a
//! caller-supplied lookup (this crate does not — and must not — dynamically
//! load `enforcer-lang-*` validator implementations; that wiring belongs to
//! whatever binary composes the lang crates), and emits a structured
//! [`Finding`] for every parity gap instead of a `println!`/`process::exit`
//! (workspace lint policy: no `unwrap`/`expect`/`panic`/`print_*`).
//!
//! Doctrine (RUST_ARCHITECTURE / TEST_PROOF_EXPECTATIONS): every T1/T2 rule
//! requires the full 5-way chain; every T3 rule requires only the
//! MECHANICALLY-CHECKED presence of the verbatim label
//! `advisory, no mechanization possible + <reason>` in its `tags` — the
//! *presence* of that label is itself a T1 check, enforced here.
//!
//! This module also owns the REVERSE-direction check: a caller-supplied
//! inventory of validator/doc/fixture artifacts that claim to back a rule
//! must all trace back to a registry record — an artifact with no matching
//! `RuleId` in the registry is an orphan and fails closed exactly like a
//! missing link in the forward direction.

use std::collections::BTreeSet;

use enforcer_domain::findings::Finding;
use enforcer_domain::ids::RuleId;
use enforcer_domain::paths::{RelPath, RepoRoot};
use enforcer_domain::severity::Tier;
use enforcer_rules::registry::{RuleRecord, RuleRegistry};
use enforcer_validator::harness::run_fixture_parity;
use enforcer_validator::validator::{ValidationInput, Validator};

/// Verbatim T3 label prefix. A T3 record is only accepted if at least one
/// of its `tags` starts with this exact string (doctrine: the label is
/// `advisory, no mechanization possible + <reason>`, so the reason varies
/// but the prefix must match byte-for-byte).
const T3_LABEL_PREFIX: &str = "advisory, no mechanization possible";

macro_rules! parity_gap_finding {
    ($record:expr, $detail:expr $(,)?) => {{
        let title_text = format!("rule-scaffold-parity gap: {}", $record.title);
        let detail_text = $detail;
        let detail_ref: &str = detail_text.as_ref();
        mechanization_finding!(
            Clone::clone(&$record.rule_id),
            title_text.as_str(),
            detail_ref,
            Clone::clone(&$record.fixtures.fail),
        )
    }};
}

/// A resolver for one [`RuleRecord`]'s [`Validator`] implementation.
/// Returns `None` when no validator is wired for that rule id — the sweep
/// treats that identically to "validator missing" for T1/T2 records.
///
/// This crate cannot itself construct `enforcer-lang-*` validator instances
/// (that would invert the dependency graph); callers (a future
/// `enforcer-cli`, or a test) own the mapping from `RuleId` to a concrete
/// `&dyn Validator`.
pub trait ValidatorLookup {
    /// Resolve the validator for `rule_id`, if one is wired.
    fn resolve(&self, rule_id: &RuleId) -> Option<&dyn Validator>;
}

impl<T: ValidatorLookup + ?Sized> ValidatorLookup for &T {
    fn resolve(&self, rule_id: &RuleId) -> Option<&dyn Validator> {
        (**self).resolve(rule_id)
    }
}

/// One registry-wide parity sweep: the live [`RuleRegistry`], a validator
/// lookup, the repo root fixture paths resolve against, and the reverse
/// orphan inventory (artifact rule ids claimed by validator/doc/fixture
/// content OUTSIDE the registry — normally empty; a non-empty entry not
/// present in the registry is an orphan).
#[derive(Debug, Clone)]
pub struct ParityOracle<'a> {
    registry: &'a RuleRegistry,
    repo_root: RepoRoot,
    orphan_candidates: BTreeSet<RuleId>,
}

impl<'a> ParityOracle<'a> {
    /// Build a sweep over `registry`, resolving fixtures relative to
    /// `repo_root`. `orphan_candidates` is the reverse-direction inventory:
    /// every rule id any validator/doc/fixture artifact CLAIMS to
    /// implement, whether or not it has a registry record — pass an empty
    /// set when the caller has no such inventory to check.
    pub fn new(
        registry: &'a RuleRegistry,
        repo_root: RepoRoot,
        orphan_candidates: BTreeSet<RuleId>,
    ) -> Self {
        Self {
            registry,
            repo_root,
            orphan_candidates,
        }
    }

    /// Run the full sweep against every registry record plus the reverse
    /// orphan inventory, using `lookup` to resolve validators. Returns the
    /// structured findings — an EMPTY vec means full parity, never a
    /// boolean or an exit code.
    pub fn sweep(&self, lookup: &dyn ValidatorLookup) -> Vec<Finding> {
        let mut findings = Vec::new();
        for record in self.registry.iter() {
            findings.extend(self.check_record(record, lookup));
        }
        findings.extend(self.check_orphans());
        findings
    }

    fn check_record(&self, record: &RuleRecord, lookup: &dyn ValidatorLookup) -> Vec<Finding> {
        match record.tier {
            Tier::T1 | Tier::T2 => self.check_mechanized_record(record, lookup),
            Tier::T3 => self.check_labeled_record(record),
        }
    }

    /// T1/T2: full 5-way chain. Doc anchor must resolve, a validator must
    /// be wired and its `rule_id()` must match, and it must fire on the
    /// fail fixture while staying silent on the pass fixture.
    fn check_mechanized_record(
        &self,
        record: &RuleRecord,
        lookup: &dyn ValidatorLookup,
    ) -> Vec<Finding> {
        let mut findings = Vec::new();

        if let Some(finding) = self.check_doc_anchor(record) {
            findings.push(finding);
        }

        let Some(validator) = lookup.resolve(&record.rule_id) else {
            findings.extend(parity_gap_finding!(
                record,
                "no validator wired for this rule id — parity requires a firing validator",
            ));
            return findings;
        };

        if validator.rule_id() != &record.rule_id {
            findings.extend(parity_gap_finding!(
                record,
                format!(
                    "resolved validator implements `{}`, not `{}`",
                    validator.rule_id(),
                    record.rule_id
                ),
            ));
            return findings;
        }

        if let Err(source) = run_fixture_parity(
            validator,
            &self.repo_root,
            &record.fixtures.fail,
            &record.fixtures.pass,
        ) {
            findings.extend(parity_gap_finding!(record, format!("{source}")));
        }

        findings
    }

    /// T3: no behavioral fixture required, but the verbatim label MUST be
    /// present in `tags` — the label's presence is itself T1-enforced.
    fn check_labeled_record(&self, record: &RuleRecord) -> Vec<Finding> {
        let has_label = record
            .tags
            .iter()
            .any(|tag| tag.as_str().starts_with(T3_LABEL_PREFIX));
        if has_label {
            Vec::new()
        } else {
            parity_gap_finding!(
                record,
                format!(
                    "T3 rule is missing the mandatory verbatim label `{T3_LABEL_PREFIX} + <reason>` in its tags"
                ),
            )
            .into_iter()
            .collect()
        }
    }

    /// A doc anchor "resolves" when the file portion (before an optional
    /// `#fragment`) exists on disk, and — when a fragment is present — the
    /// file's text contains that fragment literally (mirrors how the
    /// legacy `.mjs` doc-anchor checker treated Markdown anchors: a
    /// resolvable target, not a full CommonMark heading-slug parse).
    fn check_doc_anchor(&self, record: &RuleRecord) -> Option<Finding> {
        let (file_part, fragment) = match record.doc_anchor.as_str().split_once('#') {
            Some((file, fragment)) => (file, Some(fragment)),
            None => (record.doc_anchor.as_str(), None),
        };
        // ALLOC-JUSTIFICATION: the validated relative path must own its text after this anchor borrow ends.
        let rel_path = match RelPath::try_from(file_part.to_owned()) {
            Ok(path) => path,
            Err(source) => {
                return parity_gap_finding!(
                    record,
                    format!(
                        "doc anchor `{}` is not a relative path: {source}",
                        record.doc_anchor
                    ),
                );
            }
        };
        let full_path = self.repo_root.resolve(&rel_path);
        let contents = match std::fs::read_to_string(&full_path) {
            Ok(contents) => contents,
            Err(source) => {
                return parity_gap_finding!(
                    record,
                    format!(
                        "doc anchor `{}` does not resolve: {source}",
                        record.doc_anchor
                    ),
                );
            }
        };
        if let Some(fragment) = fragment {
            if !contents.contains(fragment) {
                return parity_gap_finding!(
                    record,
                    format!(
                        "doc anchor `{}` file exists but fragment `{fragment}` was not found in its text",
                        record.doc_anchor
                    ),
                );
            }
        }
        None
    }

    /// Reverse direction: every artifact-claimed rule id in
    /// `orphan_candidates` that has NO matching registry record is an
    /// orphan validator/doc/fixture — a link with nothing on the other end.
    fn check_orphans(&self) -> Vec<Finding> {
        self.orphan_candidates
            .iter()
            .filter(|rule_id| self.registry.get(rule_id).is_none())
            .filter_map(|rule_id| {
                mechanization_finding!(
                    Clone::clone(rule_id),
                    "orphan mechanization artifact",
                    format!(
                        "a validator/doc/fixture artifact claims rule id `{rule_id}` but no registry record exists for it"
                    ),
                    rule_id_relpath(rule_id),
                )
            })
            .collect()
    }
}

/// [`RelPath`] for a bare [`RuleId`]. `RuleId`'s own charset (uppercase
/// ASCII/digits, `-`, `.` — see `crates/enforcer-domain/src/ids.rs`) is a
/// strict subset of what `RelPath` accepts, so this can never hit the
/// `..`/leading-separator rejection paths; falls back to a fixed diagnostic
/// path only if that shared invariant is broken.
fn rule_id_relpath(rule_id: &RuleId) -> RelPath {
    for candidate in [rule_id.as_str(), "rule-scaffold-parity/unresolved"] {
        if let Ok(path) = candidate.parse() {
            return path;
        }
    }
    // The fixed fallback is a valid relative path by construction. Keep the
    // function total without an unwrap/expect escape if that shared invariant
    // is ever changed accidentally.
    loop {
        if let Ok(path) = "rule-scaffold-parity/unresolved".parse() {
            return path;
        }
    }
}

/// A [`Validator`] wrapper so the registry-wide sweep can be invoked
/// through the same `Validator` contract every other rule uses (and, per
/// the workpack, from a future CLI `enforcer rule parity` command). Its
/// `rule_id()` is the synthetic id for the parity oracle rule itself;
/// `validate` ignores its file/source input and instead runs the full
/// registry sweep held by the wrapped [`ParityOracle`] plus `lookup`.
pub struct RuleScaffoldParityValidator<'a, 'b> {
    rule_id: RuleId,
    oracle: &'b ParityOracle<'a>,
    lookup: &'b dyn ValidatorLookup,
}

impl std::fmt::Debug for RuleScaffoldParityValidator<'_, '_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("RuleScaffoldParityValidator")
            .field("rule_id", &self.rule_id)
            .finish_non_exhaustive()
    }
}

impl<'a, 'b> RuleScaffoldParityValidator<'a, 'b> {
    /// Wrap `oracle` (with `lookup` for validator resolution) behind the
    /// `rule-scaffold-parity` rule id, so the whole-registry sweep can be
    /// driven through the standard `Validator::validate` entry point.
    pub fn new(
        rule_id: RuleId,
        oracle: &'b ParityOracle<'a>,
        lookup: &'b dyn ValidatorLookup,
    ) -> Self {
        Self {
            rule_id,
            oracle,
            lookup,
        }
    }
}

impl Validator for RuleScaffoldParityValidator<'_, '_> {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, _input: ValidationInput<'_>) -> Vec<Finding> {
        self.oracle.sweep(self.lookup)
    }
}
