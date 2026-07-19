//! Integration test: scaffold a brand-new rule into a `tempdir`, assert
//! the five artifacts the workpack requires all land (a loadable
//! `RuleRecord`, a validator skeleton, a resolvable doc anchor, and both a
//! pass and a fail fixture), then round-trip that scaffolded output back
//! through the fail-closed parity oracle.
//!
//! This is deliberately an INTEGRATION test (writes real files to a real
//! tempdir, reads them back) rather than the crate-local unit tests in
//! `src/scaffold.rs`, which only exercise `scaffold_rule`'s in-memory
//! output — this file proves the on-disk round trip the CLI-facing
//! `enforcer rule new` command will eventually drive.

use std::fs;
use std::path::Path;

use enforcer_domain::findings::{Finding, FindingDetail, FindingLine, FindingTitle};
use enforcer_domain::ids::RuleId;
use enforcer_domain::paths::RepoRoot;
use enforcer_domain::severity::{Severity, Tier};
use enforcer_domain::telemetry_types::SourceLine;
use enforcer_mechanization::oracle::accept_rule;
use enforcer_mechanization::scaffold::{scaffold_rule, ScaffoldSpec};
use enforcer_validator::validator::{ValidationInput, Validator};

/// The scaffolder's generated validator skeleton always returns zero
/// findings by design (see `src/scaffold.rs`'s module doc) — a
/// freshly-scaffolded rule is INTENTIONALLY not yet parity-green. This
/// stand-in plays the role of "a human filled in the detection logic": it
/// fires on the literal marker `SCAFFOLD_MARKER`, matching the bad/good
/// fixture pair this test writes.
struct FilledInValidator {
    rule_id: RuleId,
}

impl Validator for FilledInValidator {
    fn rule_id(&self) -> &RuleId {
        &self.rule_id
    }

    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding> {
        if input.source.as_str().contains("SCAFFOLD_MARKER") {
            test_finding(
                self.rule_id.clone(),
                input.file.clone(),
                "scaffold marker present",
                "found SCAFFOLD_MARKER",
            )
            .into_iter()
            .collect()
        } else {
            Vec::new()
        }
    }
}

fn test_finding(
    rule_id: RuleId,
    file: enforcer_domain::paths::RelPath,
    title: &str,
    detail: &str,
) -> Option<Finding> {
    Some(Finding {
        rule_id,
        severity: Severity::Error,
        title: FindingTitle::new(title.to_owned()).ok()?,
        detail: FindingDetail::new(detail.to_owned()).ok()?,
        file,
        line: FindingLine::known(SourceLine::try_new(std::num::NonZeroU32::MIN)),
        snippet: None,
    })
}

fn sample_spec(
    fail_rel: &str,
    pass_rel: &str,
    doc_rel: &str,
) -> Result<ScaffoldSpec, enforcer_domain::boundary::decode_error::DecodeError> {
    Ok(ScaffoldSpec {
        rule_id: "RR-77.1".parse()?,
        title: "No temp-dir frobnicating".parse()?,
        tier: Tier::T1,
        validator_crate: "enforcer-mechanization".parse()?,
        validator_path: "scaffold_roundtrip::FilledInValidator".parse()?,
        fail_fixture_path: fail_rel.parse()?,
        pass_fixture_path: pass_rel.parse()?,
        doc_anchor: doc_rel.parse()?,
        tags: vec!["scaffold-roundtrip".parse()?],
    })
}

#[test]
fn scaffold_output_lands_five_artifacts_and_re_passes_parity(
) -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let repo_path: &Path = temp.path();
    let repo_root = RepoRoot::try_from(repo_path)?;

    let fail_rel = "fail_fixture.txt";
    let pass_rel = "pass_fixture.txt";
    let doc_rel = "SCAFFOLD_ROUNDTRIP.md#SCAFFOLD-ROUNDTRIP-1";

    let spec = sample_spec(fail_rel, pass_rel, doc_rel)?;
    let output = scaffold_rule(&spec)?;

    // Artifact 1: a loadable RuleRecord.
    let registry =
        enforcer_rules::registry::RuleRegistry::from_records(vec![output.record.clone()])?;
    assert_eq!(
        registry.count(),
        enforcer_domain::rules_types::RuleRecordCount::from_records([()])
    );

    // Artifact 2: the validator skeleton source text (written to disk as
    // the CLI-facing command would; this test does not compile it, since
    // that requires a real crate target — the skeleton's CONTENT is
    // asserted directly).
    let validator_path = repo_path.join("scaffolded_validator.rs");
    fs::write(&validator_path, output.validator_skeleton_source.as_str())?;
    assert!(validator_path.exists());
    let expected_contract = format!(
        "//! Freshly scaffolded validator for `{}` — {}.",
        spec.rule_id.as_str(),
        spec.title
    );
    assert_eq!(
        output.validator_skeleton_source.as_str().lines().next(),
        Some(expected_contract.as_str()),
        "the generated validator must identify the exact scaffolded rule in its module contract"
    );

    // Artifact 3: a resolvable doc anchor.
    let doc_path = repo_path.join("SCAFFOLD_ROUNDTRIP.md");
    fs::write(
        &doc_path,
        "# Scaffold Roundtrip Doc\n\n## SCAFFOLD-ROUNDTRIP-1\n\nAnchor text.\n",
    )?;
    assert!(doc_path.exists());

    // Artifacts 4 + 5: fail and pass fixture slots. The scaffolder's
    // starter slot content is inert (comment-only), so this test replaces
    // it with the "filled in" content a human would write — the slot
    // machinery (writing to the declared paths) is what is under test.
    fs::write(repo_path.join(fail_rel), "SCAFFOLD_MARKER present here\n")?;
    fs::write(repo_path.join(pass_rel), "clean content, no marker\n")?;
    assert!(repo_path.join(fail_rel).exists());
    assert!(repo_path.join(pass_rel).exists());

    // Round trip: scaffold -> load record -> parity passes, now that a
    // real validator implementation is supplied.
    let validator = FilledInValidator {
        rule_id: output.record.rule_id.clone(),
    };
    accept_rule(&output.record, Some(&validator), &repo_root)?;

    Ok(())
}

#[test]
fn scaffold_output_fails_parity_before_fixtures_are_filled_in(
) -> Result<(), Box<dyn std::error::Error>> {
    // Companion negative case: the scaffolder's OWN starter slot content
    // (comment-only, never trips anything) must fail the oracle — proving
    // scaffolding never silently produces an already-passing rule.
    let temp = tempfile::tempdir()?;
    let repo_path: &Path = temp.path();
    let repo_root = RepoRoot::try_from(repo_path)?;

    let fail_rel = "fail_fixture.txt";
    let pass_rel = "pass_fixture.txt";
    let doc_rel = "SCAFFOLD_ROUNDTRIP.md#SCAFFOLD-ROUNDTRIP-1";

    let spec = sample_spec(fail_rel, pass_rel, doc_rel)?;
    let output = scaffold_rule(&spec)?;

    // Write the scaffolder's own inert starter slot content verbatim.
    fs::write(repo_path.join(fail_rel), output.fail_fixture_slot.as_str())?;
    fs::write(repo_path.join(pass_rel), output.pass_fixture_slot.as_str())?;

    let validator = FilledInValidator {
        rule_id: output.record.rule_id.clone(),
    };
    let outcome = accept_rule(&output.record, Some(&validator), &repo_root);
    assert!(
        outcome.is_err(),
        "unfilled fail-fixture slot must not pass parity"
    );

    Ok(())
}
