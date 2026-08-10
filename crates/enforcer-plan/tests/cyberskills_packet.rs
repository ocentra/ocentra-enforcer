//! BOUNDARY-INVARIANT: CP05 tests exercise the public typed factory only.
//! NEGATIVE-TEST: incomplete and unapproved drafts are rejected without file writes.

use std::error::Error;

use enforcer_domain::ids::RuleId;
use enforcer_plan::cyberskills_packet::{
    ComponentApproval, ComponentDraft, ComponentId, FixturePath, FixtureSet, LicenseName,
    NotProvedText, PacketFactory, PacketPath, PacketPaths, PredicateText, SourceAnchor,
    SourceIdentity, SourcePath, SourceSha256,
};
use serde_json::Value;

const SOURCE_PATH: &str =
    "vendor/anthropic-cybersecurity-skills/skills/exploiting-mass-assignment-in-rest-apis/SKILL.md";
const SOURCE_SHA256: &str = "23393f7b14703375d6487e099a3108186d299b359fab45e7c09281e41b479af9";
const SOURCE_ANCHOR: &str = "### Step 2 — Test Privilege Escalation via Role Fields:L73";

type TestResult = Result<(), Box<dyn Error>>;

fn approved_draft() -> Result<ComponentDraft, enforcer_domain::boundary::decode_error::DecodeError>
{
    Ok(ComponentDraft {
        approval: ComponentApproval::Approved,
        component_id: Some(ComponentId::try_from(
            "native-predicate-exploiting-mass-assignment".to_owned(),
        )?),
        source: Some(SourceIdentity::new(
            SourcePath::try_from(SOURCE_PATH.to_owned())?,
            SourceSha256::try_from(SOURCE_SHA256.to_owned())?,
            SourceAnchor::try_from(SOURCE_ANCHOR.to_owned())?,
            LicenseName::try_from("Apache-2.0".to_owned())?,
        )),
        rule_id: Some("CYBER-MASS-ASSIGN.1".parse::<RuleId>()?),
        predicate: Some(PredicateText::try_from(
            "Static source detection of whole untrusted request-object binding without a field allowlist."
                .to_owned(),
        )?),
        not_proved: Some(NotProvedText::try_from(
            "Does not prove live endpoint discovery, authorization behavior, or exploitation outcomes."
                .to_owned(),
        )?),
        fixtures: Some(FixtureSet::new(
            FixturePath::try_from(
                "crates/enforcer-lang-security/tests/fixtures/cyberskills/web.mass-assignment/bad/vuln.py"
                    .to_owned(),
            )?,
            FixturePath::try_from(
                "crates/enforcer-lang-security/tests/fixtures/cyberskills/web.mass-assignment/good/safe.py"
                    .to_owned(),
            )?,
        )?),
        paths: Some(PacketPaths::new(
            PacketPath::try_from("crates/enforcer-rules/rules/cyberskills.json".to_owned())?,
            PacketPath::try_from(
                "crates/enforcer-lang-security/src/rules/cyberskills/mass_assignment.rs"
                    .to_owned(),
            )?,
            PacketPath::try_from("proof/cyberskills/cp05/mass-assignment-packet.json".to_owned())?,
        )),
    })
}

#[test]
fn approved_mass_assignment_emits_the_clerical_skeleton() -> TestResult {
    let expected: Value = serde_json::from_str(include_str!(
        "fixtures/cyberskills_packet/approved_mass_assignment.json"
    ))?;
    let actual = PacketFactory::build(approved_draft()?)?.to_json();
    assert_eq!(actual, expected);
    assert_eq!(actual["generated"]["writesFiles"], false);
    assert_eq!(
        actual["generated"]["securityMeaning"],
        "supplied-input-only"
    );
    Ok(())
}

#[test]
fn unapproved_component_is_rejected() -> TestResult {
    let mut draft = approved_draft()?;
    draft.approval = ComponentApproval::Unapproved;
    let error = PacketFactory::build(draft).expect_err("unapproved draft must fail closed");
    assert_eq!(error.path, "approval");
    Ok(())
}

#[test]
fn missing_source_hash_fixture_is_rejected_at_the_typed_boundary() -> TestResult {
    let fixture: Value = serde_json::from_str(include_str!(
        "fixtures/cyberskills_packet/missing_source_sha256.json"
    ))?;
    assert!(fixture["sourceSha256"].is_string());
    assert_eq!(fixture["sourceSha256"].as_str(), Some(""));
    let error =
        SourceSha256::try_from(String::new()).expect_err("empty source hash must fail closed");
    assert_eq!(error.path, "sourceSha256");

    let mut draft = approved_draft()?;
    draft.source = None;
    let error = PacketFactory::build(draft).expect_err("missing source must fail closed");
    assert_eq!(error.path, "source");
    Ok(())
}

#[test]
fn protected_source_and_duplicate_fixtures_are_rejected() -> TestResult {
    let protected = SourcePath::try_from(
        "vendor/anthropic-cybersecurity-skills/skills/detecting-fileless-malware-techniques/SKILL.md"
            .to_owned(),
    )
    .expect_err("protected source must remain outside the factory");
    assert_eq!(protected.path, "sourcePath");

    let fixture = FixturePath::try_from(
        "crates/enforcer-lang-security/tests/fixtures/cyberskills/web.mass-assignment/bad/vuln.py"
            .to_owned(),
    )?;
    let duplicate =
        FixtureSet::new(fixture.clone(), fixture).expect_err("fixture roles must differ");
    assert_eq!(duplicate.path, "fixtures");
    Ok(())
}
