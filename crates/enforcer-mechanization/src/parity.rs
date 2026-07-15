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
use std::path::Path;

use enforcer_domain::findings::{Finding, ScanScope};
use enforcer_domain::ids::RuleId;
use enforcer_domain::paths::RelPath;
use enforcer_domain::severity::{Severity, Tier};
use enforcer_rules::registry::{RuleRecord, RuleRegistry};
use enforcer_validator::harness::run_fixture_parity;
use enforcer_validator::validator::{ValidationInput, Validator};

/// Verbatim T3 label prefix. A T3 record is only accepted if at least one
/// of its `tags` starts with this exact string (doctrine: the label is
/// `advisory, no mechanization possible + <reason>`, so the reason varies
/// but the prefix must match byte-for-byte).
pub const T3_LABEL_PREFIX: &str = "advisory, no mechanization possible";

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
    repo_root: std::path::PathBuf,
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
        repo_root: &Path,
        orphan_candidates: BTreeSet<RuleId>,
    ) -> Self {
        Self {
            registry,
            repo_root: repo_root.to_path_buf(),
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
            findings.push(gap_finding(
                record,
                "no validator wired for this rule id — parity requires a firing validator",
            ));
            return findings;
        };

        if validator.rule_id() != &record.rule_id {
            findings.push(gap_finding(
                record,
                &format!(
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
            findings.push(gap_finding(record, &source.to_string()));
        }

        findings
    }

    /// T3: no behavioral fixture required, but the verbatim label MUST be
    /// present in `tags` — the label's presence is itself T1-enforced.
    fn check_labeled_record(&self, record: &RuleRecord) -> Vec<Finding> {
        let has_label = record
            .tags
            .iter()
            .any(|tag| tag.starts_with(T3_LABEL_PREFIX));
        if has_label {
            Vec::new()
        } else {
            vec![gap_finding(
                record,
                &format!(
                    "T3 rule is missing the mandatory verbatim label `{T3_LABEL_PREFIX} + <reason>` in its tags"
                ),
            )]
        }
    }

    /// A doc anchor "resolves" when the file portion (before an optional
    /// `#fragment`) exists on disk, and — when a fragment is present — the
    /// file's text contains that fragment literally (mirrors how the
    /// legacy `.mjs` doc-anchor checker treated Markdown anchors: a
    /// resolvable target, not a full CommonMark heading-slug parse).
    fn check_doc_anchor(&self, record: &RuleRecord) -> Option<Finding> {
        let (file_part, fragment) = match record.doc_anchor.split_once('#') {
            Some((file, fragment)) => (file, Some(fragment)),
            None => (record.doc_anchor.as_str(), None),
        };
        let full_path = self.repo_root.join(file_part);
        let contents = match std::fs::read_to_string(&full_path) {
            Ok(contents) => contents,
            Err(source) => {
                return Some(gap_finding(
                    record,
                    &format!(
                        "doc anchor `{}` does not resolve: {source}",
                        record.doc_anchor
                    ),
                ));
            }
        };
        if let Some(fragment) = fragment {
            if !contents.contains(fragment) {
                return Some(gap_finding(
                    record,
                    &format!(
                        "doc anchor `{}` file exists but fragment `{fragment}` was not found in its text",
                        record.doc_anchor
                    ),
                ));
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
            .map(|rule_id| Finding {
                rule_id: rule_id.clone(),
                severity: Severity::Error,
                title: "orphan mechanization artifact".to_owned(),
                detail: format!(
                    "a validator/doc/fixture artifact claims rule id `{rule_id}` but no registry record exists for it"
                ),
                file: rule_id_relpath(rule_id),
                line: 1,
                snippet: None,
            })
            .collect()
    }
}

fn gap_finding(record: &RuleRecord, detail: &str) -> Finding {
    Finding {
        rule_id: record.rule_id.clone(),
        severity: Severity::Error,
        title: format!("rule-scaffold-parity gap: {}", record.title),
        detail: detail.to_owned(),
        file: doc_or_fallback_relpath(&record.doc_anchor),
        line: 1,
        snippet: None,
    }
}

/// Fixed diagnostic [`RelPath`] used as `Finding::file` when the string on
/// hand does not itself parse as a repo-relative path. Built from a
/// literal that is structurally guaranteed valid under `RelPath::try_from`
/// (non-empty, no leading separator, no `..` segment — see
/// `crates/enforcer-domain/src/paths.rs`), so the `expect` below can never
/// actually fire; scoped `#[allow(clippy::expect_used)]` on a provably
/// infallible literal parse is the established pattern in this workspace
/// (e.g. `enforcer-coordination`, `enforcer-lang-security`) for exactly
/// this situation, rather than threading a fallible `Option<RelPath>`
/// through every `Finding` builder for a path that is diagnostic-only.
#[allow(clippy::expect_used)]
fn diagnostic_relpath() -> RelPath {
    "rule-scaffold-parity/unresolved"
        .parse()
        .expect("literal is a structurally valid RelPath")
}

/// Best-effort [`RelPath`] for a `Finding::file` slot when all we have is a
/// doc-anchor-shaped string: strip any `#fragment` and parse the rest,
/// falling back to [`diagnostic_relpath`] if it does not parse as a
/// repo-relative path.
fn doc_or_fallback_relpath(raw: &str) -> RelPath {
    let file_part = raw.split('#').next().unwrap_or(raw);
    file_part.parse().unwrap_or_else(|_| diagnostic_relpath())
}

/// [`RelPath`] for a bare [`RuleId`]. `RuleId`'s own charset (uppercase
/// ASCII/digits, `-`, `.` — see `crates/enforcer-domain/src/ids.rs`) is a
/// strict subset of what `RelPath` accepts, so this can never hit the
/// `..`/leading-separator rejection paths; falls back to
/// [`diagnostic_relpath`] on the same defensive footing as
/// [`doc_or_fallback_relpath`] rather than asserting the invariant inline.
fn rule_id_relpath(rule_id: &RuleId) -> RelPath {
    rule_id
        .as_str()
        .parse()
        .unwrap_or_else(|_| diagnostic_relpath())
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

#[allow(dead_code)]
fn _scan_scope_hint() -> ScanScope {
    // Sanity anchor: registry-wide sweeps ignore ValidationInput entirely,
    // so document that a caller may pass ScanScope::Workspace safely; kept
    // as a private function (not a test) purely to give this fact a single
    // discoverable home the compiler still checks type-correctness of.
    ScanScope::Workspace
}

#[cfg(test)]
mod tests {
    use super::{ParityOracle, RuleScaffoldParityValidator, ValidatorLookup, T3_LABEL_PREFIX};
    use enforcer_domain::findings::{Finding, ScanScope};
    use enforcer_domain::ids::RuleId;
    use enforcer_domain::paths::RelPath;
    use enforcer_domain::severity::{Severity, Tier};
    use enforcer_rules::registry::{FixtureRef, RuleRecord, RuleRegistry, ValidatorRef};
    use enforcer_validator::validator::{ValidationInput, Validator};
    use std::collections::BTreeSet;

    fn manifest_dir() -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    fn record(
        rule_id: &str,
        tier: Tier,
        tags: Vec<String>,
    ) -> Result<RuleRecord, enforcer_domain::boundary::decode_error::DecodeError> {
        Ok(RuleRecord {
            rule_id: rule_id.parse()?,
            version: 1,
            title: "Sample rule".to_owned(),
            tier,
            validator: ValidatorRef {
                crate_name: "enforcer-mechanization".to_owned(),
                path: "parity::MarkerValidator".to_owned(),
            },
            fixtures: FixtureRef {
                fail: "fixtures/scaffold/fail.txt".to_owned(),
                pass: "fixtures/scaffold/pass.txt".to_owned(),
            },
            doc_anchor: "tests/fixtures/parity/docs/SAMPLE.md#SAMPLE-ANCHOR".to_owned(),
            tags,
            params: serde_json::Value::Null,
        })
    }

    struct MarkerValidator {
        rule_id: RuleId,
    }

    impl Validator for MarkerValidator {
        fn rule_id(&self) -> &RuleId {
            &self.rule_id
        }

        fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
            if input.source.contains("SCAFFOLD_MARKER") {
                vec![Finding {
                    rule_id: self.rule_id.clone(),
                    severity: Severity::Error,
                    title: "marker present".to_owned(),
                    detail: "found SCAFFOLD_MARKER".to_owned(),
                    file: input.file.clone(),
                    line: 1,
                    snippet: None,
                }]
            } else {
                Vec::new()
            }
        }
    }

    struct SingleLookup<'a>(&'a RuleId, &'a dyn Validator);

    impl ValidatorLookup for SingleLookup<'_> {
        fn resolve(&self, rule_id: &RuleId) -> Option<&dyn Validator> {
            if rule_id == self.0 {
                Some(self.1)
            } else {
                None
            }
        }
    }

    struct EmptyLookup;

    impl ValidatorLookup for EmptyLookup {
        fn resolve(&self, _rule_id: &RuleId) -> Option<&dyn Validator> {
            None
        }
    }

    #[test]
    fn sweep_is_clean_for_a_fully_wired_t1_record() -> Result<(), Box<dyn std::error::Error>> {
        let record = record("RR-95.1", Tier::T1, vec![])?;
        let rule_id = record.rule_id.clone();
        let registry = RuleRegistry::from_records(vec![record])?;
        let validator = MarkerValidator {
            rule_id: rule_id.clone(),
        };
        let lookup = SingleLookup(&rule_id, &validator);
        let oracle = ParityOracle::new(&registry, &manifest_dir(), BTreeSet::new());
        assert!(oracle.sweep(&lookup).is_empty());
        Ok(())
    }

    #[test]
    fn sweep_flags_missing_validator_for_t1_record() -> Result<(), Box<dyn std::error::Error>> {
        let record = record("RR-95.2", Tier::T1, vec![])?;
        let registry = RuleRegistry::from_records(vec![record])?;
        let oracle = ParityOracle::new(&registry, &manifest_dir(), BTreeSet::new());
        let findings = oracle.sweep(&EmptyLookup);
        assert_eq!(findings.len(), 1);
        assert!(findings[0].detail.contains("no validator wired"));
        Ok(())
    }

    #[test]
    fn sweep_flags_dangling_doc_anchor() -> Result<(), Box<dyn std::error::Error>> {
        let mut record = record("RR-95.3", Tier::T1, vec![])?;
        record.doc_anchor = "tests/fixtures/parity/docs/DOES-NOT-EXIST.md#NOPE".to_owned();
        let rule_id = record.rule_id.clone();
        let registry = RuleRegistry::from_records(vec![record])?;
        let validator = MarkerValidator {
            rule_id: rule_id.clone(),
        };
        let lookup = SingleLookup(&rule_id, &validator);
        let oracle = ParityOracle::new(&registry, &manifest_dir(), BTreeSet::new());
        let findings = oracle.sweep(&lookup);
        assert!(findings
            .iter()
            .any(|f| f.detail.contains("does not resolve")));
        Ok(())
    }

    #[test]
    fn sweep_flags_missing_fixture_for_t1_record() -> Result<(), Box<dyn std::error::Error>> {
        let mut record = record("RR-95.4", Tier::T1, vec![])?;
        record.fixtures.fail = "fixtures/scaffold/does-not-exist.txt".to_owned();
        let rule_id = record.rule_id.clone();
        let registry = RuleRegistry::from_records(vec![record])?;
        let validator = MarkerValidator {
            rule_id: rule_id.clone(),
        };
        let lookup = SingleLookup(&rule_id, &validator);
        let oracle = ParityOracle::new(&registry, &manifest_dir(), BTreeSet::new());
        let findings = oracle.sweep(&lookup);
        assert_eq!(findings.len(), 1);
        Ok(())
    }

    #[test]
    fn sweep_flags_validator_rule_id_mismatch() -> Result<(), Box<dyn std::error::Error>> {
        let record = record("RR-95.5", Tier::T1, vec![])?;
        let rule_id = record.rule_id.clone();
        let registry = RuleRegistry::from_records(vec![record])?;
        let other_id: RuleId = "RR-95.6".parse()?;
        let validator = MarkerValidator { rule_id: other_id };
        let lookup = SingleLookup(&rule_id, &validator);
        let oracle = ParityOracle::new(&registry, &manifest_dir(), BTreeSet::new());
        let findings = oracle.sweep(&lookup);
        assert_eq!(findings.len(), 1);
        assert!(findings[0].detail.contains("resolved validator implements"));
        Ok(())
    }

    #[test]
    fn sweep_is_clean_for_t3_record_with_label() -> Result<(), Box<dyn std::error::Error>> {
        let record = record(
            "RR-95.7",
            Tier::T3,
            vec![format!(
                "{T3_LABEL_PREFIX} + judgment call, no deterministic check exists"
            )],
        )?;
        let registry = RuleRegistry::from_records(vec![record])?;
        let oracle = ParityOracle::new(&registry, &manifest_dir(), BTreeSet::new());
        assert!(oracle.sweep(&EmptyLookup).is_empty());
        Ok(())
    }

    #[test]
    fn sweep_flags_t3_record_missing_label() -> Result<(), Box<dyn std::error::Error>> {
        let record = record("RR-95.8", Tier::T3, vec!["unrelated-tag".to_owned()])?;
        let registry = RuleRegistry::from_records(vec![record])?;
        let oracle = ParityOracle::new(&registry, &manifest_dir(), BTreeSet::new());
        let findings = oracle.sweep(&EmptyLookup);
        assert_eq!(findings.len(), 1);
        assert!(findings[0].detail.contains("mandatory verbatim label"));
        Ok(())
    }

    #[test]
    fn sweep_flags_orphan_artifact_not_in_registry() -> Result<(), Box<dyn std::error::Error>> {
        let registry = RuleRegistry::from_records(vec![])?;
        let orphan: RuleId = "RR-95.9".parse()?;
        let mut orphans = BTreeSet::new();
        orphans.insert(orphan.clone());
        let oracle = ParityOracle::new(&registry, &manifest_dir(), orphans);
        let findings = oracle.sweep(&EmptyLookup);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].rule_id, orphan);
        assert!(findings[0].title.contains("orphan"));
        Ok(())
    }

    #[test]
    fn orphan_with_matching_registry_record_is_not_flagged(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let record = record("RR-95.10", Tier::T1, vec![])?;
        let rule_id = record.rule_id.clone();
        let registry = RuleRegistry::from_records(vec![record])?;
        let validator = MarkerValidator {
            rule_id: rule_id.clone(),
        };
        let lookup = SingleLookup(&rule_id, &validator);
        let mut candidates = BTreeSet::new();
        candidates.insert(rule_id.clone());
        let oracle = ParityOracle::new(&registry, &manifest_dir(), candidates);
        assert!(oracle.sweep(&lookup).is_empty());
        Ok(())
    }

    #[test]
    fn validator_wrapper_delegates_to_sweep() -> Result<(), Box<dyn std::error::Error>> {
        let registry = RuleRegistry::from_records(vec![])?;
        let orphan: RuleId = "RR-95.11".parse()?;
        let mut orphans = BTreeSet::new();
        orphans.insert(orphan);
        let oracle = ParityOracle::new(&registry, &manifest_dir(), orphans);
        let wrapper_id: RuleId = "MECH-PARITY.1".parse()?;
        let wrapper = RuleScaffoldParityValidator::new(wrapper_id, &oracle, &EmptyLookup);
        let file: RelPath = "fixtures/scaffold/fail.txt".parse()?;
        let findings = wrapper.validate(ValidationInput {
            file: &file,
            source: "",
            scope: ScanScope::Workspace,
        });
        assert_eq!(findings.len(), 1);
        Ok(())
    }
}
