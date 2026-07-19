//! `REQ-TESTCAT-SEVEN.1` + `REQ-TESTCAT-MAP.1` (both T1) — the required
//! test-categories coverage gate (h02, §4/§8.3 of the ingested
//! money-critical/security-testing spec).
//!
//! Doctrine (§4/§8.3): every money-critical unit (as classified by h01,
//! consumed here read-only, never redefined) MUST carry all seven required
//! test categories — negative, replay, concurrency, rollback/compensation,
//! economic-exhaustion, time-based, and signing/verification. Missing any
//! one of the seven is a build-blocking gap. This module implements a
//! `Validator` over a typed `REQUIRED_TEST_CATEGORIES` record (deserialized
//! in the `boundary` submodule, mirroring
//! [`crate::rules::threat_test_mapping`]'s THREAT_MAP shape) rather than a
//! heuristic AST scan: category membership is a structured,
//! explicitly-tagged field on the record (the d23 companion/category
//! convention, consumed read-only from `enforcer-lang-common`), never
//! guessed from prose. A malformed or invalid record stays silent — the
//! same contract h01/h03 document for unparseable source.
//!
//! # Two validators, one record shape
//!
//! - [`RequiredTestCategoriesSevenValidator`] (`REQ-TESTCAT-SEVEN.1`): every
//!   unit listed under `units` must carry at least one test id in EACH of
//!   the seven required categories — any missing category on any listed
//!   unit is a `Finding` naming exactly which categories are absent. This
//!   validator only inspects units already present in the record; a unit
//!   entirely absent from `units` is
//!   [`RequiredTestCategoriesMapValidator`]'s concern, not this one's
//!   (mirrors h03's coverage-vs-unmapped split).
//! - [`RequiredTestCategoriesMapValidator`] (`REQ-TESTCAT-MAP.1`): every
//!   entry in the h01-shaped `moneyCriticalUnits` manifest snapshot must
//!   resolve to a `units` entry carrying at least one test id in at least
//!   one category — a unit with no entry at all, or an entry with every
//!   category empty, is "unresolvable" and flagged regardless of whether
//!   the units that DO resolve are fully seven-category complete (that
//!   completeness check is [`RequiredTestCategoriesSevenValidator`]'s job).
//!
//! GENERIC across any value system (fiat, Stripe, an internal ledger, or the
//! optional crypto/Anchor instance) — the fixtures use a deliberately
//! neutral `endpoint_a` vocabulary, never a crypto-only one.
//!
//! # `REQUIRED_TEST_CATEGORIES` wire shape
//!
//! ```jsonc
//! {
//!   "moneyCriticalUnits": ["endpoint_a"],
//!   "units": [
//!     {
//!       "unit": "endpoint_a",
//!       "tests": {
//!         "negative": ["neg_endpoint_a_rejects_invalid_amount"],
//!         "replay": ["replay_endpoint_a_idempotent_retry"],
//!         "concurrency": ["conc_endpoint_a_no_lost_update"],
//!         "rollback": ["rollback_endpoint_a_compensates_on_failure"],
//!         "economic_exhaustion": ["econ_endpoint_a_rejects_pool_exhaustion"],
//!         "time_based": ["time_endpoint_a_expires_after_window"],
//!         "signing": ["sign_endpoint_a_verifies_signature"]
//!       }
//!     }
//!   ]
//! }
//! ```
//!
//! `moneyCriticalUnits` is the h01-shaped money-critical manifest SNAPSHOT
//! this fixture format carries inline (a real pipeline wiring would resolve
//! this list from h01's `#[money_critical(registered)]` scan instead; this
//! record format lets the category-coverage check be exercised standalone
//! against a fixture, exactly like h03's THREAT_MAP representative triples).
//!
//! # Deviation from the workpack's `syn`/`tree-sitter` wording
//!
//! The workpack's "Where We Want To Be" describes resolving each unit's
//! test files via filesystem/module check with test bodies read via
//! `syn`/`tree-sitter`. Mirroring h03's precedent (a typed record consumed
//! at a serde boundary rather than a live filesystem walk — the base
//! `Validator` contract inspects one file's text in isolation and has no
//! filesystem access, see [`crate::rules::threat_test_mapping`]'s module
//! docs), this module validates the SAME record shape: a `units` entry's
//! `tests.<category>` array is the mechanical, non-heuristic proof that a
//! category-tagged test exists (satisfying REQ-TESTCAT-CATEGORY-TAGGING's
//! "tag, not heuristic guessing" doctrine structurally — category
//! membership is an explicit named array field, never inferred from
//! filename or body text). Real filesystem resolution of the record itself
//! from h01's manifest + on-disk test files is a follow-up for the
//! `enforcer-scan` orchestrator, which unlike this per-file `Validator` can
//! walk the whole tree and cross-reference multiple paths.
//
// PROPERTY-TEST: tests/required_test_categories_parity.rs
// (req_testcat_seven_property_over_all_category_subsets) proves the gate
// over every one of the 128 category-presence subsets, plus malformed and
// invalid records staying silent.

mod boundary;

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::findings::Finding;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

/// Generate one rule's validator: the public struct, its parse-at-boundary
/// constructor, and its [`Validator`] impl delegating to a named check
/// function — written once for the whole family so the constructor/trait
/// shape cannot drift between siblings.
macro_rules! rule_validator {
    ($(#[$doc:meta])* $name:ident, $rule_id:literal, $check:path) => {
        $(#[$doc])*
        #[derive(Debug)]
        pub struct $name {
            rule_id: RuleId,
        }

        impl $name {
            /// Build the validator, parsing its own `RuleId` literal at
            /// construction (parse-at-boundary).
            pub fn new() -> Result<Self, DecodeError> {
                Ok(Self {
                    rule_id: $rule_id.parse()?,
                })
            }
        }

        impl Validator for $name {
            fn rule_id(&self) -> &RuleId {
                &self.rule_id
            }

            fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
                $check(&self.rule_id, &input)
            }
        }
    };
}

rule_validator!(
    /// `REQ-TESTCAT-SEVEN.1` — T1 seven-category coverage gate (§4/§8.3).
    ///
    /// Every unit listed under `units` must carry at least one test id in
    /// EACH of the seven required categories. This validator only inspects
    /// units already present in the record — a unit entirely absent from
    /// `units` is [`RequiredTestCategoriesMapValidator`]'s concern, not
    /// this one's.
    RequiredTestCategoriesSevenValidator,
    "REQ-TESTCAT-SEVEN.1",
    check_seven_categories
);

rule_validator!(
    /// `REQ-TESTCAT-MAP.1` — T1 unit-resolution gate (§4/§8.3).
    ///
    /// Any unit named in the h01-shaped `moneyCriticalUnits` manifest
    /// snapshot that has no corresponding `units` entry — or whose entry
    /// carries zero test ids across every category — is "unresolvable" and
    /// flagged, independent of whether units that DO resolve are fully
    /// seven-category complete (that completeness check is
    /// [`RequiredTestCategoriesSevenValidator`]'s job).
    RequiredTestCategoriesMapValidator,
    "REQ-TESTCAT-MAP.1",
    check_unit_resolution
);

/// The `REQ-TESTCAT-SEVEN.1` detection body: flag any mapped unit whose
/// category coverage is incomplete.
fn check_seven_categories(rule_id: &RuleId, input: &ValidationInput<'_>) -> Vec<Finding> {
    let Some(record) = boundary::parse_record(input.source.as_str()) else {
        return Vec::new();
    };

    let mut findings = Vec::new();
    for (index, unit) in record.units().iter().enumerate() {
        let missing = unit.missing_category_labels();
        if missing.is_empty() {
            continue;
        }
        let line = u32::try_from(index).unwrap_or(u32::MAX).saturating_add(1);
        findings.extend(canonical_finding! {
            // CLONE-JUSTIFICATION: every Finding owns its RuleId; the
            // validator keeps its own parsed id for the next file.
            rule_id: rule_id.clone(),
            severity: Severity::Error,
            // ALLOC-JUSTIFICATION: Finding::title is an owned String on the
            // findings wire shape; one owned copy per emitted finding.
            title: "money-critical unit is missing required test categories (T1)".to_owned(),
            detail: format!(
                "unit `{}` is missing: {}. Doctrine (§4/§8.3): every money-critical unit MUST \
                 carry a negative + replay + concurrency + rollback/compensation + \
                 economic-exhaustion + time-based + signing/verification test. Fix: add the \
                 missing category test(s) to this unit's REQUIRED_TEST_CATEGORIES entry.",
                unit.name(),
                missing.join(", ")
            ),
            // CLONE-JUSTIFICATION: Finding::file owns its RelPath; the
            // input's borrowed path outlives only this call.
            file: input.file.clone(),
            line: line,
            snippet: None,
        });
    }
    findings
}

/// The `REQ-TESTCAT-MAP.1` detection body: flag any classified unit that
/// resolves to zero category-tagged tests.
fn check_unit_resolution(rule_id: &RuleId, input: &ValidationInput<'_>) -> Vec<Finding> {
    let Some(record) = boundary::parse_record(input.source.as_str()) else {
        return Vec::new();
    };

    let mut findings = Vec::new();
    for (index, classified_unit) in record.money_critical_units().iter().enumerate() {
        if record.unit_resolves(classified_unit) {
            continue;
        }
        let line = u32::try_from(index).unwrap_or(u32::MAX).saturating_add(1);
        findings.extend(canonical_finding! {
            // CLONE-JUSTIFICATION: every Finding owns its RuleId; the
            // validator keeps its own parsed id for the next file.
            rule_id: rule_id.clone(),
            severity: Severity::Error,
            // ALLOC-JUSTIFICATION: Finding::title is an owned String on the
            // findings wire shape; one owned copy per emitted finding.
            title: "money-critical unit does not resolve to any test (T1)".to_owned(),
            detail: format!(
                "unit `{classified_unit}` is classified money-critical but resolves to zero \
                 category-tagged tests in REQUIRED_TEST_CATEGORIES `units`. Doctrine (§4/§8.3): \
                 every h01-classified unit MUST resolve to at least one test file bearing a \
                 recognized category tag. Fix: add a `units` entry for `{classified_unit}` with \
                 at least one category-tagged test id."
            ),
            // CLONE-JUSTIFICATION: Finding::file owns its RelPath; the
            // input's borrowed path outlives only this call.
            file: input.file.clone(),
            line: line,
            snippet: None,
        });
    }
    findings
}
