//! Fixture-backed proof that GitHub wire values are converted at the boundary
//! before typed branch-protection policy evaluates them.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use enforcer_domain::ids::GitHubCheckContext;
use enforcer_domain::install_types::{
    BypassAllowance, ContextRequirement, DesiredProtection, ObservedBranchProtection,
    PullRequestRequirement, RequiredChecksHealth, UpToDateRequirement, Verification,
};
use enforcer_install::ci::boundary::{
    branch_protection::report,
    branch_protection_payload::{
        BranchProtectionWriteDto, LiveProtectionStateDto, RequiredStatusChecksDto,
    },
    branch_protection_workflow::WorkflowJobDeclaration,
};
use enforcer_install::ci::branch_protection::verify;

fn fixture_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/branch_protection")
        .join(name)
}

fn desired() -> Result<DesiredProtection, Box<dyn std::error::Error>> {
    let rust_ci = BTreeSet::try_from(WorkflowJobDeclaration {
        workflow_name: "Rust CI".to_owned(),
        job_id: "rust-ci".to_owned(),
        matrix: vec![
            "ubuntu-latest".to_owned(),
            "windows-latest".to_owned(),
            "macos-latest".to_owned(),
        ],
    })?;
    let enforcer = BTreeSet::try_from(WorkflowJobDeclaration {
        workflow_name: "Ocentra Enforcer".to_owned(),
        job_id: "ocentra-enforcer".to_owned(),
        matrix: Vec::new(),
    })?;
    Ok(DesiredProtection::baseline(
        rust_ci.union(&enforcer).cloned().collect(),
    ))
}

fn live(name: &str) -> Result<ObservedBranchProtection, Box<dyn std::error::Error>> {
    let raw = std::fs::read_to_string(fixture_path(name))?;
    Ok(ObservedBranchProtection::try_from(serde_json::from_str::<
        LiveProtectionStateDto,
    >(&raw)?)?)
}

#[test]
fn github_live_fixture_maps_to_typed_observation() -> Result<(), Box<dyn std::error::Error>> {
    let observed = live("pass.json")?;
    let context = GitHubCheckContext::try_from("Rust CI / rust-ci (windows-latest)".to_owned())?;
    assert_eq!(
        observed.context_requirement(&context),
        ContextRequirement::Present
    );
    assert_eq!(observed.up_to_date(), UpToDateRequirement::Required);
    assert_eq!(observed.pull_requests(), PullRequestRequirement::Required);
    assert_eq!(observed.administrator_bypass(), BypassAllowance::Denied);
    assert_eq!(observed.force_push(), BypassAllowance::Denied);
    assert_eq!(observed.deletion(), BypassAllowance::Denied);
    assert_eq!(observed.required_checks(), RequiredChecksHealth::Passing);
    Ok(())
}

#[test]
fn fixture_policy_attests_only_the_safe_observation() -> Result<(), Box<dyn std::error::Error>> {
    let desired = desired()?;
    assert!(matches!(
        verify(&desired, &live("pass.json")?),
        Verification::Attested
    ));
    assert!(matches!(
        verify(&desired, &live("fail_no_required_checks.json")?),
        Verification::Refused(_)
    ));
    assert!(matches!(
        verify(&desired, &live("fail_bypassable.json")?),
        Verification::Refused(_)
    ));
    assert!(matches!(
        verify(&desired, &live("fail_red_merge_eligible.json")?),
        Verification::Refused(_)
    ));
    Ok(())
}

#[test]
fn typed_policy_emits_and_reports_through_the_boundary() -> Result<(), Box<dyn std::error::Error>> {
    let desired = desired()?;
    let observed = live("pass.json")?;
    let verification = verify(&desired, &observed);
    let payload = BranchProtectionWriteDto::from(&desired);
    assert!(payload.enforce_admins);
    assert!(payload.required_pull_request);
    assert!(!payload.allow_force_pushes);
    assert!(!payload.allow_deletions);
    assert_eq!(payload.required_status_checks.contexts.len(), 4);
    let rendered = report(&desired, &observed, &verification);
    assert!(rendered.attested);
    assert_eq!(rendered.exit_code, 0);
    assert!(rendered.refusal_codes.is_empty());
    Ok(())
}

#[test]
fn branch_protection_dtos_round_trip_through_json() -> Result<(), Box<dyn std::error::Error>> {
    let write = BranchProtectionWriteDto::from(&desired()?);
    let write_wire = serde_json::to_string(&write)?;
    let round_trip_write: BranchProtectionWriteDto = serde_json::from_str(&write_wire)?;
    let round_trip_checks: &RequiredStatusChecksDto = &round_trip_write.required_status_checks;
    assert_eq!(round_trip_checks.contexts.len(), 4);

    let raw = std::fs::read_to_string(fixture_path("pass.json"))?;
    let live: LiveProtectionStateDto = serde_json::from_str(&raw)?;
    let live_wire = serde_json::to_string(&live)?;
    let round_trip_live: LiveProtectionStateDto = serde_json::from_str(&live_wire)?;
    assert_eq!(round_trip_live, live);
    Ok(())
}
