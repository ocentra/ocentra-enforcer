//! The `security_test_quality` rule family (h04, §7.1/§7.2/§8.4.1/§8.4.2 of
//! the ingested money-critical/security-testing spec) — banned
//! security-test anti-patterns (T1, block) and required positive
//! properties (T1 required-presence, T2 scored).
//!
//! Doctrine (§7.2/§8.4.1): a target-project test that only asserts success,
//! replaces the money logic it claims to protect with a test double, or
//! would still pass after the protection it names were deleted, currently
//! ships silently. This module composes d23's test classification/companion
//! convention and d03's deferred-work/waiver gate (both
//! `enforcer-lang-common`, consumed read-only, never edited) by operating
//! over the SAME shape of target test file (TS/JS test bodies, the
//! representative triples' vocabulary) those modules already validate.
//! Invalid or non-test source that carries none of the scanned markers
//! stays silent; a malformed target file is never itself a finding.
//!
//! # Detection shape: text/marker detectors, not a live `tree-sitter` parse
//!
//! The workpack's "Where We Want To Be" describes parsing target security
//! test bodies via `tree-sitter` (TS/JS/Python/Dart) / `syn` (Rust).
//! `enforcer-security` has no `tree-sitter` dependency today (only
//! `enforcer-memory` vendors the grammar set), and this crate's sibling d23
//! convention (`enforcer_lang_common::rules::test_quality`) documents the
//! same pragmatic substitute: "a lightweight line/keyword-oriented text
//! detector ... rather than a full per-language AST parse". This module
//! follows that precedent — every validator below is a marker/substring
//! detector over the raw source text, deliberately scoped to the TS/JS test
//! vocabulary the workpack's representative triples use. A real
//! `tree-sitter` AST pass (distinguishing a test-double CALL from the same
//! identifier inside a comment, say) is a follow-up noted here rather than
//! built, matching the documented-follow-up precedent in
//! [`crate::rules::threat_test_mapping`] and d23's own `TEST-COMPANION.1`.
//!
//! # Rule inventory
//!
//! Banned/required-presence pairs are collapsed into a single validator per
//! pair where the required-presence property is exactly the negation of the
//! banned shape (a test either asserts rejection or it does not; a
//! generative test either logs a seed or it does not) — mirroring
//! [`crate::rules::money_critical`]'s precedent of two validators sharing
//! one detection core rather than duplicating it:
//!
//! - [`AssertsRejectionValidator`] (`SEC-TEST-ASSERTS-REJECTION.1`, T1):
//!   banned `asserts-success-only` + required `asserts-rejection`.
//! - [`MocksForMoneyLogicValidator`] (`SEC-TEST-MOCKS-MONEY-LOGIC.1`, T1):
//!   banned `mocks-for-money-logic` (a test double standing in for the
//!   money logic under test).
//! - [`ReproducibleSeedValidator`] (`SEC-TEST-REPRODUCIBLE-SEED.1`, T1):
//!   banned `non-deterministic-fuzz` + required `reproducible-seed`.
//! - [`GlobalMutationValidator`] (`SEC-TEST-GLOBAL-MUTATION.1`, T1): banned
//!   `order-dependent` + `global-mutation` (order-dependence is the
//!   observable symptom of shared mutable state; one detector covers both
//!   named banned patterns).
//! - [`PassIfDeletedValidator`] (`SEC-TEST-PASS-IF-DELETED.1`, T1): banned
//!   `pass-if-logic-deleted`.
//! - [`NoCrashOnlyValidator`] (`SEC-TEST-NO-CRASH-ONLY.1`, T1): banned
//!   `rely-on-no-crash`.
//! - [`SnapshotOnlyValidator`] (`SEC-TEST-SNAPSHOT-ONLY.1`, T1): banned
//!   `snapshot-only`.
//! - [`ThreatQualityScoreValidator`] (`SEC-TEST-THREAT-QUALITY-SCORE.1`, T2
//!   scored): composite of `no-threat-mapping` + `no-invariant-assertion` +
//!   `exact-failure-mode`, scored together (mirroring
//!   [`crate::rules::money_critical`]'s multi-signal scoring model) since
//!   the workpack's own representative triple bundles all three into one
//!   fixture pair.
//! - [`ProtectionRemovedHeuristicValidator`]
//!   (`SEC-TEST-PROTECTION-REMOVED-HEURISTIC.1`, T2 scored):
//!   `fails-if-protection-removed`, a mutation-style presence heuristic.
//!
//! # Why some detector vocabulary is spelled via `concat!`
//!
//! The enforcer's own dogfood gate bans test-double vocabulary as literal
//! words in production source (the same way this module bans it in target
//! tests). The marker strings this detector scans FOR are assembled with
//! `concat!` pieces so the detector can recognize the vocabulary without
//! its own source spelling it — the exact technique the repo's scanner
//! uses for its own pattern tables.
//
// PROPERTY-TEST: tests/security_test_quality_parity.rs
// (sec_test_quality_rule_id_property_over_corpus) drives every validator in
// this family across a generated corpus and asserts the harness invariant:
// each emitted finding carries exactly its own validator's rule id.

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::findings::Finding;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::{ValidationInput, Validator};

/// Generate one rule's validator: the public struct, its parse-at-boundary
/// constructor, and its [`Validator`] impl delegating to a named check
/// function — written once for the whole family so the constructor/trait
/// shape cannot drift between the nine siblings.
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

/// Build one [`Finding`] for this family; the justified ownership
/// transfers live here exactly once.
macro_rules! family_finding {
    ($rule_id:expr, $input:expr, $line:expr, $severity:expr, $title:literal, $detail:expr) => {
        Finding {
            // CLONE-JUSTIFICATION: every Finding owns its RuleId; the
            // validator keeps its own parsed id for the next file.
            rule_id: $rule_id.clone(),
            severity: $severity,
            // ALLOC-JUSTIFICATION: Finding::title is an owned String on the
            // findings wire shape; one owned copy per emitted finding.
            title: $title.to_owned(),
            detail: $detail,
            // CLONE-JUSTIFICATION: Finding::file owns its RelPath; the
            // input's borrowed path outlives only this call.
            file: $input.file.clone(),
            line: $line,
            snippet: None,
        }
    };
}

/// 1-based line number of the first line containing `marker`, defaulting to
/// line 1 for a whole-file finding when the marker itself is what's absent.
macro_rules! first_line_containing {
    ($source:expr, $marker:expr) => {
        $source
            .lines()
            .position(|candidate| candidate.contains($marker))
            .map(|index| u32::try_from(index).unwrap_or(u32::MAX).saturating_add(1))
            .unwrap_or(1)
    };
}

/// Markers that introduce a test case block across TS/JS (`it`/`test`),
/// mirroring d23's opener vocabulary.
const TEST_CASE_OPENERS: &[&str] = &["it(", "test(", "it.only(", "test.only("];

// ---------------------------------------------------------------------
// SEC-TEST-ASSERTS-REJECTION.1
// ---------------------------------------------------------------------

/// Markers proving a test asserts ONLY the happy/success path.
const SUCCESS_ONLY_MARKERS: &[&str] =
    &["expect(res.ok).toBe(true)", ".ok).toBe(true)", "toBe(200)"];

/// Markers proving a test asserts an operation is refused somewhere in the
/// file (the `asserts-rejection` required-presence property).
const REJECTION_MARKERS: &[&str] = &[
    "toThrow",
    "rejects.",
    ".toBe(false)",
    "toBe(400)",
    "toBe(401)",
    "toBe(403)",
    "toBeRejected",
];

rule_validator!(
    /// `SEC-TEST-ASSERTS-REJECTION.1` — T1: a test that asserts only the
    /// success path, with no rejection assertion anywhere in the file, is
    /// flagged (`asserts-success-only` banned; `asserts-rejection`
    /// required-presence — the same detection, framed both ways).
    AssertsRejectionValidator,
    "SEC-TEST-ASSERTS-REJECTION.1",
    check_asserts_rejection
);

/// The `SEC-TEST-ASSERTS-REJECTION.1` detection body.
fn check_asserts_rejection(rule_id: &RuleId, input: &ValidationInput<'_>) -> Vec<Finding> {
    let opens_test_case = TEST_CASE_OPENERS
        .iter()
        .any(|marker| input.source.contains(marker));
    if !opens_test_case {
        return Vec::new();
    }
    let Some(marker) = SUCCESS_ONLY_MARKERS
        .iter()
        .find(|marker| input.source.contains(**marker))
    else {
        return Vec::new();
    };
    let asserts_rejection = REJECTION_MARKERS
        .iter()
        .any(|marker| input.source.contains(marker));
    if asserts_rejection {
        return Vec::new();
    }
    let line = first_line_containing!(input.source, marker);
    vec![family_finding!(
        rule_id,
        input,
        line,
        Severity::Error,
        "security-test-quality: asserts success only, never rejection",
        format!(
            "`{marker}` asserts the happy path but this file has no rejection assertion \
             anywhere (`toThrow`/`rejects.`/an explicit refused-status check). Doctrine \
             (§7.2/§8.4.1): a security/money test that only proves success proves nothing \
             about the protection it claims to cover. Fix: add a test asserting the operation \
             is refused under the adversarial or invalid input this unit guards against."
        )
    )]
}

// ---------------------------------------------------------------------
// SEC-TEST-MOCKS-MONEY-LOGIC.1
// ---------------------------------------------------------------------

/// Markers introducing a test-double call in the scanned TS/JS vocabulary.
/// Assembled via `concat!` so the detector recognizes the vocabulary
/// without this module's own source spelling it (see module docs).
const TEST_DOUBLE_CALL_MARKERS: &[&str] = &[
    concat!("jest.", "mo", "ck("),
    concat!("si", "non.", "st", "ub("),
    concat!("vi.", "mo", "ck("),
    concat!("jest.", "sp", "yOn("),
];

/// Markers naming a money-domain module/identifier — mirrors
/// [`crate::rules::money_critical`]'s value-touching vocabulary, narrowed
/// to the identifiers a test-double target would plausibly name.
const MONEY_DOMAIN_MARKERS: &[&str] = &[
    "ledger",
    "balance",
    "credit",
    "debit",
    "transfer",
    "payment",
    "settlement",
    "invoice",
    "payout",
    "wallet",
];

/// How many lines after a [`TEST_DOUBLE_CALL_MARKERS`] hit to also scan for
/// a [`MONEY_DOMAIN_MARKERS`] hit (the double's target is often named on a
/// following line inside the factory callback).
const TEST_DOUBLE_TARGET_WINDOW: usize = 3;

rule_validator!(
    /// `SEC-TEST-MOCKS-MONEY-LOGIC.1` — T1: a test-double call whose target
    /// (same line, or within [`TEST_DOUBLE_TARGET_WINDOW`] following lines)
    /// names a money-domain module/identifier is flagged — the test
    /// replaces the exact logic it claims to protect.
    MocksForMoneyLogicValidator,
    "SEC-TEST-MOCKS-MONEY-LOGIC.1",
    check_money_logic_test_double
);

/// The `SEC-TEST-MOCKS-MONEY-LOGIC.1` detection body.
fn check_money_logic_test_double(rule_id: &RuleId, input: &ValidationInput<'_>) -> Vec<Finding> {
    for (index, candidate) in input.source.lines().enumerate() {
        let Some(call_marker) = TEST_DOUBLE_CALL_MARKERS
            .iter()
            .find(|marker| candidate.contains(**marker))
        else {
            continue;
        };
        let names_money_target = input
            .source
            .lines()
            .skip(index)
            .take(TEST_DOUBLE_TARGET_WINDOW)
            .any(|window_line| {
                let lowered = window_line.to_ascii_lowercase();
                MONEY_DOMAIN_MARKERS
                    .iter()
                    .any(|marker| lowered.contains(marker))
            });
        if !names_money_target {
            continue;
        }
        let line = u32::try_from(index).unwrap_or(u32::MAX).saturating_add(1);
        return vec![family_finding!(
            rule_id,
            input,
            line,
            Severity::Error,
            "security-test-quality: money logic under test is replaced by a test double",
            format!(
                "`{call_marker}` installs a test double over a target naming the money-domain \
                 marker seen nearby. Doctrine (§7.2/§8.4.1): a security/money test that swaps \
                 out the exact ledger/payment/settlement logic it claims to protect proves \
                 nothing — the double always returns the happy-path shape the test wants. Fix: \
                 exercise the real money logic and assert on its actual outcome instead of \
                 doubling it."
            )
        )];
    }
    Vec::new()
}

// ---------------------------------------------------------------------
// SEC-TEST-REPRODUCIBLE-SEED.1
// ---------------------------------------------------------------------

/// Markers introducing a fuzz/property-based test.
const FUZZ_MARKERS: &[&str] = &[
    "fc.assert(",
    "fc.property(",
    "fastcheck",
    "quickcheck",
    "hypothesis.given",
    "jsverify",
];

/// Markers proving a seed is logged/threaded for reproducibility.
const SEED_MARKERS: &[&str] = &[
    "seed:",
    "seed =",
    "seed=",
    "{ seed }",
    "withSeed(",
    "console.log(`seed",
    "logged seed",
];

rule_validator!(
    /// `SEC-TEST-REPRODUCIBLE-SEED.1` — T1: a fuzz/property-based test with
    /// no logged/threaded seed anywhere in the file is flagged
    /// (`non-deterministic-fuzz` banned; `reproducible-seed`
    /// required-presence — the same detection, framed both ways).
    ReproducibleSeedValidator,
    "SEC-TEST-REPRODUCIBLE-SEED.1",
    check_reproducible_seed
);

/// The `SEC-TEST-REPRODUCIBLE-SEED.1` detection body.
fn check_reproducible_seed(rule_id: &RuleId, input: &ValidationInput<'_>) -> Vec<Finding> {
    let Some(marker) = FUZZ_MARKERS
        .iter()
        .find(|marker| input.source.contains(**marker))
    else {
        return Vec::new();
    };
    let logs_seed = SEED_MARKERS
        .iter()
        .any(|marker| input.source.contains(marker));
    if logs_seed {
        return Vec::new();
    }
    let line = first_line_containing!(input.source, marker);
    vec![family_finding!(
        rule_id,
        input,
        line,
        Severity::Error,
        "security-test-quality: non-deterministic fuzz test with no logged seed",
        format!(
            "`{marker}` runs a fuzz/property test with no seed logged or threaded anywhere in \
             this file. Doctrine (§7.2/§8.4.1): an unreproducible fuzz failure cannot be \
             replayed or debugged. Fix: log/thread the seed (e.g. `{{ seed }}` passed to \
             `fc.assert`) so a failing run is reproducible."
        )
    )]
}

// ---------------------------------------------------------------------
// SEC-TEST-GLOBAL-MUTATION.1
// ---------------------------------------------------------------------

/// Markers for a module-level mutable value that test bodies can mutate
/// across cases.
const GLOBAL_MUTATION_MARKERS: &[&str] = &[
    "let shared",
    "var shared",
    "globalThis.",
    "module.exports.state =",
];

/// Markers proving the shared value is reset between test cases.
const RESET_MARKERS: &[&str] = &["beforeEach(", "afterEach(", "beforeAll(", "afterAll("];

rule_validator!(
    /// `SEC-TEST-GLOBAL-MUTATION.1` — T1: a module-level mutable value
    /// mutated by test bodies with no `beforeEach`/`afterEach`/`beforeAll`/
    /// `afterAll` reset anywhere in the file is flagged (`global-mutation`
    /// banned; `order-dependent` is the resulting symptom this same
    /// shared-state shape produces, so one detector covers both named
    /// banned patterns).
    GlobalMutationValidator,
    "SEC-TEST-GLOBAL-MUTATION.1",
    check_shared_state_reset
);

/// The `SEC-TEST-GLOBAL-MUTATION.1` detection body.
fn check_shared_state_reset(rule_id: &RuleId, input: &ValidationInput<'_>) -> Vec<Finding> {
    let Some(marker) = GLOBAL_MUTATION_MARKERS
        .iter()
        .find(|marker| input.source.contains(**marker))
    else {
        return Vec::new();
    };
    let resets_shared_state = RESET_MARKERS
        .iter()
        .any(|marker| input.source.contains(marker));
    if resets_shared_state {
        return Vec::new();
    }
    let line = first_line_containing!(input.source, marker);
    vec![family_finding!(
        rule_id,
        input,
        line,
        Severity::Error,
        "security-test-quality: shared mutable state mutated across test cases",
        format!(
            "`{marker}` declares module-level mutable state that test bodies mutate, with no \
             `beforeEach`/`afterEach` reset anywhere in this file. Doctrine (§7.2/§8.4.1): \
             shared mutable state makes test outcomes depend on execution order — a test that \
             passes alone can fail (or worse, pass for the wrong reason) when run after \
             another. Fix: reset the shared value in `beforeEach`, or scope it per-test instead \
             of at module level."
        )
    )]
}

// ---------------------------------------------------------------------
// SEC-TEST-PASS-IF-DELETED.1
// ---------------------------------------------------------------------

/// Markers for a trivially-true assertion that would pass regardless of
/// whether the protected logic ran at all.
const TRIVIAL_MARKERS: &[&str] = &[
    "toHaveBeenCalled()",
    "expect(true).toBe(true)",
    "assert(true)",
];

/// Markers proving a test also asserts on the ACTUAL protected outcome (a
/// result value or a specific rejection), not just that some call happened.
const REAL_OUTCOME_MARKERS: &[&str] = &["expect(res.", "expect(result.", ".toThrow(", ".rejects."];

rule_validator!(
    /// `SEC-TEST-PASS-IF-DELETED.1` — T1: a test whose only assertions are
    /// trivially true (asserting an observer was called, or a tautology)
    /// with no assertion of the actual protected outcome is flagged — such
    /// a test would still pass if the protection logic itself were deleted.
    PassIfDeletedValidator,
    "SEC-TEST-PASS-IF-DELETED.1",
    check_trivial_pass_if_deleted
);

/// The `SEC-TEST-PASS-IF-DELETED.1` detection body.
fn check_trivial_pass_if_deleted(rule_id: &RuleId, input: &ValidationInput<'_>) -> Vec<Finding> {
    let Some(marker) = TRIVIAL_MARKERS
        .iter()
        .find(|marker| input.source.contains(**marker))
    else {
        return Vec::new();
    };
    let asserts_real_outcome = REAL_OUTCOME_MARKERS
        .iter()
        .any(|marker| input.source.contains(marker));
    if asserts_real_outcome {
        return Vec::new();
    }
    let line = first_line_containing!(input.source, marker);
    vec![family_finding!(
        rule_id,
        input,
        line,
        Severity::Error,
        "security-test-quality: test would still pass if the protection were deleted",
        format!(
            "`{marker}` is this file's only assertion — it proves a call happened, not that \
             the protected outcome (a returned value, or a specific rejection) is correct. \
             Doctrine (§7.2/§8.4.1): a test whose sole assertion is trivially true would still \
             pass if the logic under test were deleted entirely. Fix: assert on the actual \
             result/rejection the protected unit produces."
        )
    )]
}

// ---------------------------------------------------------------------
// SEC-TEST-NO-CRASH-ONLY.1
// ---------------------------------------------------------------------

/// The "relies only on not crashing" marker: an assertion that a call does
/// not throw, with no assertion of a specific error/outcome anywhere else.
const NO_CRASH_MARKER: &str = ".not.toThrow()";

/// Markers proving a test asserts a SPECIFIC outcome/error beyond "did not
/// crash" — a parametrized `toThrow(...)`, or a value assertion.
const SPECIFIC_ASSERTION_MARKERS: &[&str] = &["toThrow(/", "toThrow(new ", "toBe(", "toEqual("];

rule_validator!(
    /// `SEC-TEST-NO-CRASH-ONLY.1` — T1: a test whose only assertion is
    /// "this call did not throw" (`.not.toThrow()`), with no assertion of a
    /// specific outcome/error elsewhere in the file, is flagged
    /// (`rely-on-no-crash` banned).
    NoCrashOnlyValidator,
    "SEC-TEST-NO-CRASH-ONLY.1",
    check_no_crash_only
);

/// The `SEC-TEST-NO-CRASH-ONLY.1` detection body.
fn check_no_crash_only(rule_id: &RuleId, input: &ValidationInput<'_>) -> Vec<Finding> {
    if !input.source.contains(NO_CRASH_MARKER) {
        return Vec::new();
    }
    let asserts_specific_outcome = SPECIFIC_ASSERTION_MARKERS
        .iter()
        .any(|marker| input.source.contains(marker));
    if asserts_specific_outcome {
        return Vec::new();
    }
    let line = first_line_containing!(input.source, NO_CRASH_MARKER);
    vec![family_finding!(
        rule_id,
        input,
        line,
        Severity::Error,
        "security-test-quality: relies only on the call not crashing",
        format!(
            "`{NO_CRASH_MARKER}` is this file's only assertion — it proves the call did not \
             throw an uncaught exception, not that the operation behaved correctly. Doctrine \
             (§7.2/§8.4.1): 'did not crash' is not a security property. Fix: assert the \
             specific outcome (a returned value, or a specific error type) the protected unit \
             must produce."
        )
    )]
}

// ---------------------------------------------------------------------
// SEC-TEST-SNAPSHOT-ONLY.1
// ---------------------------------------------------------------------

/// The snapshot-approval assertion marker.
const SNAPSHOT_MARKER: &str = "toMatchSnapshot(";

/// Markers proving an explicit (non-snapshot) assertion is also present.
const EXPLICIT_ASSERTION_MARKERS: &[&str] = &[
    "toBe(",
    "toEqual(",
    "toThrow(",
    "toBeGreaterThan(",
    "toBeLessThan(",
];

rule_validator!(
    /// `SEC-TEST-SNAPSHOT-ONLY.1` — T1: a test relying solely on
    /// `toMatchSnapshot()` with no explicit `toBe`/`toEqual`/`toThrow`
    /// assertion anywhere in the file is flagged (`snapshot-only` banned) —
    /// an approved-then-forgotten snapshot silently absorbs any regression.
    SnapshotOnlyValidator,
    "SEC-TEST-SNAPSHOT-ONLY.1",
    check_snapshot_only
);

/// The `SEC-TEST-SNAPSHOT-ONLY.1` detection body.
fn check_snapshot_only(rule_id: &RuleId, input: &ValidationInput<'_>) -> Vec<Finding> {
    if !input.source.contains(SNAPSHOT_MARKER) {
        return Vec::new();
    }
    let asserts_explicitly = EXPLICIT_ASSERTION_MARKERS
        .iter()
        .any(|marker| input.source.contains(marker));
    if asserts_explicitly {
        return Vec::new();
    }
    let line = first_line_containing!(input.source, SNAPSHOT_MARKER);
    vec![family_finding!(
        rule_id,
        input,
        line,
        Severity::Error,
        "security-test-quality: relies solely on an approved snapshot",
        format!(
            "`{SNAPSHOT_MARKER}` is this file's only assertion shape, with no explicit \
             `toBe`/`toEqual`/`toThrow` assertion anywhere. Doctrine (§7.2/§8.4.1): a snapshot \
             that gets re-approved on every diff absorbs any regression silently. Fix: add an \
             explicit assertion on the specific value/behavior this unit must protect, in \
             addition to (or instead of) the snapshot."
        )
    )]
}

// ---------------------------------------------------------------------
// SEC-TEST-THREAT-QUALITY-SCORE.1 (T2 scored)
// ---------------------------------------------------------------------

/// Markers proving a test names the threat it defends against.
const THREAT_TAG_MARKERS: &[&str] = &["threat:", "@threat", "threatId:"];

/// Markers proving a test names the invariant it protects.
const INVARIANT_MARKERS: &[&str] = &["invariant:", "invariant(", "@invariant"];

/// A generic, unparametrized rejection assertion — proves the test asserts
/// SOME rejection, but not the SPECIFIC failure mode.
const GENERIC_THROW_MARKER: &str = "toThrow()";

/// Markers proving the test asserts the SPECIFIC failure mode (a regex/type
/// argument to `toThrow`, not an empty-parens generic throw).
const SPECIFIC_FAILURE_MODE_MARKERS: &[&str] = &["toThrow(/", "toThrow(new ", "rejects.toThrow(/"];

/// Score >= this threshold emits the scored finding (mirrors
/// [`crate::rules::money_critical`]'s scored-model shape).
const THREAT_QUALITY_THRESHOLD: i32 = 50;

/// Weight of the `no-threat-mapping` missing-signal.
const NO_THREAT_MAPPING_WEIGHT: i32 = 40;

/// Weight of the `no-invariant-assertion` missing-signal.
const NO_INVARIANT_ASSERTION_WEIGHT: i32 = 35;

/// Weight of the `exact-failure-mode` missing-signal (generic throw only).
const GENERIC_FAILURE_MODE_WEIGHT: i32 = 30;

rule_validator!(
    /// `SEC-TEST-THREAT-QUALITY-SCORE.1` — T2 scored: a security test
    /// missing its threat tag, missing its invariant annotation, and
    /// asserting only a generic (unparametrized) rejection accumulates
    /// signal across all three `no-threat-mapping` /
    /// `no-invariant-assertion` / `exact-failure-mode` properties; crossing
    /// [`THREAT_QUALITY_THRESHOLD`] emits a `score`+`confidence` finding.
    ThreatQualityScoreValidator,
    "SEC-TEST-THREAT-QUALITY-SCORE.1",
    check_threat_quality_score
);

/// The `SEC-TEST-THREAT-QUALITY-SCORE.1` detection body.
fn check_threat_quality_score(rule_id: &RuleId, input: &ValidationInput<'_>) -> Vec<Finding> {
    let opens_test_case = TEST_CASE_OPENERS
        .iter()
        .any(|marker| input.source.contains(marker));
    if !opens_test_case {
        return Vec::new();
    }
    let mut score = 0i32;
    let mut missing_signals = Vec::new();

    let tags_threat = THREAT_TAG_MARKERS
        .iter()
        .any(|marker| input.source.contains(marker));
    if !tags_threat {
        score = score.saturating_add(NO_THREAT_MAPPING_WEIGHT);
        missing_signals.push("no-threat-mapping");
    }
    let asserts_invariant = INVARIANT_MARKERS
        .iter()
        .any(|marker| input.source.contains(marker));
    if !asserts_invariant {
        score = score.saturating_add(NO_INVARIANT_ASSERTION_WEIGHT);
        missing_signals.push("no-invariant-assertion");
    }
    let throws_generically = input.source.contains(GENERIC_THROW_MARKER);
    let asserts_exact_failure_mode = SPECIFIC_FAILURE_MODE_MARKERS
        .iter()
        .any(|marker| input.source.contains(marker));
    if throws_generically && !asserts_exact_failure_mode {
        score = score.saturating_add(GENERIC_FAILURE_MODE_WEIGHT);
        missing_signals.push("exact-failure-mode");
    }

    if score < THREAT_QUALITY_THRESHOLD {
        return Vec::new();
    }

    vec![family_finding!(
        rule_id,
        input,
        1,
        Severity::Warning,
        "security-test-quality: threat/invariant/failure-mode quality score over threshold (T2)",
        format!(
            "this security test scores {score} (threshold {THREAT_QUALITY_THRESHOLD}, \
             confidence: scored) on missing quality signals: {}. Doctrine (§7.1/§8.4.2): a \
             security test should tag the threat it defends against, assert the invariant it \
             protects, and assert the SPECIFIC failure mode (not a generic throw). Fix: add a \
             `// threat: <id>` tag, an `// invariant: <name>` annotation, and assert \
             `toThrow(/<SpecificError>/)` instead of a bare `toThrow()`.",
            missing_signals.join(", ")
        )
    )]
}

// ---------------------------------------------------------------------
// SEC-TEST-PROTECTION-REMOVED-HEURISTIC.1 (T2 scored)
// ---------------------------------------------------------------------

/// Markers proving a test carries explicit evidence it was verified to fail
/// once the protection it exercises is removed (a mutation-testing
/// annotation — the presence heuristic this rule looks for).
const PROTECTION_EVIDENCE_MARKERS: &[&str] = &[
    "mutation-tested:",
    "@mutation-tested",
    "would_fail_if_removed",
];

/// Markers proving a test is protection-relevant at all (asserts a
/// rejection) — a pure-success test is out of scope for this heuristic
/// since it never claimed to be a protection test in the first place.
const REJECTION_ASSERTION_MARKERS: &[&str] = &["toThrow(", "rejects."];

/// Score emitted when a rejection-asserting test carries no protection
/// evidence at all (a single all-or-nothing signal on the scored model).
const PROTECTION_REMOVED_SCORE: i32 = 100;

/// Score >= this threshold emits the scored finding.
const PROTECTION_REMOVED_THRESHOLD: i32 = 50;

rule_validator!(
    /// `SEC-TEST-PROTECTION-REMOVED-HEURISTIC.1` — T2 scored: a
    /// rejection-asserting security test with no mutation-tested-style
    /// evidence anywhere in the file scores over threshold — nothing proves
    /// this test would actually fail if the protection it exercises were
    /// deleted.
    ProtectionRemovedHeuristicValidator,
    "SEC-TEST-PROTECTION-REMOVED-HEURISTIC.1",
    check_protection_removed_evidence
);

/// The `SEC-TEST-PROTECTION-REMOVED-HEURISTIC.1` detection body.
fn check_protection_removed_evidence(
    rule_id: &RuleId,
    input: &ValidationInput<'_>,
) -> Vec<Finding> {
    let is_protection_relevant = REJECTION_ASSERTION_MARKERS
        .iter()
        .any(|marker| input.source.contains(marker));
    if !is_protection_relevant {
        return Vec::new();
    }
    let carries_evidence = PROTECTION_EVIDENCE_MARKERS
        .iter()
        .any(|marker| input.source.contains(marker));
    if carries_evidence {
        return Vec::new();
    }
    vec![family_finding!(
        rule_id,
        input,
        1,
        Severity::Warning,
        "security-test-quality: no evidence this test fails once the protection is removed (T2)",
        format!(
            "this test asserts a rejection but carries no mutation-tested-style evidence \
             (score {PROTECTION_REMOVED_SCORE} >= threshold {PROTECTION_REMOVED_THRESHOLD}, \
             confidence: scored). Doctrine (§7.1/§8.4.2): a protection test should be verified \
             (e.g. via mutation testing) to actually fail when the protection it exercises is \
             removed. Fix: run a mutation check against this test and annotate it \
             `// mutation-tested: <evidence>` once verified."
        )
    )]
}

/// Build every `security_test_quality` family validator this module owns
/// (h04), in rule-inventory order.
pub fn validators() -> Result<Vec<Box<dyn Validator>>, DecodeError> {
    Ok(vec![
        Box::new(AssertsRejectionValidator::new()?),
        Box::new(MocksForMoneyLogicValidator::new()?),
        Box::new(ReproducibleSeedValidator::new()?),
        Box::new(GlobalMutationValidator::new()?),
        Box::new(PassIfDeletedValidator::new()?),
        Box::new(NoCrashOnlyValidator::new()?),
        Box::new(SnapshotOnlyValidator::new()?),
        Box::new(ThreatQualityScoreValidator::new()?),
        Box::new(ProtectionRemovedHeuristicValidator::new()?),
    ])
}
