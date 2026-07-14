//! Integration proof for workpack x04 (main branch protection CI): the
//! branch-protection verifier fails closed against captured fixtures
//! (no required checks / bypassable / red-but-merge-eligible), attests
//! cleanly against the pass fixture, and the emitter's `gh api` payload
//! matches a pinned golden fixture with symbolically-resolved check
//! contexts (never a hardcoded stale context string).
//!
//! Disjoint from `ci_fixtures.rs` (c10), which proves the release
//! pipeline / GitHub Action / installer-script artifacts; this file
//! proves the `branch_protection` module's own fixtures under
//! `tests/fixtures/branch_protection/**`, which c10 does not own.

use enforcer_install::ci::branch_protection::{
    emit_payload, resolve_contexts, verify_and_report, BranchProtectionReport, DesiredProtection,
    GhApiPayload, LiveProtectionState, Verdict, WorkflowJob,
};
use std::path::{Path, PathBuf};

fn fixture_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/branch_protection")
        .join(name)
}

fn load_live(name: &str) -> Result<LiveProtectionState, Box<dyn std::error::Error>> {
    let raw = std::fs::read_to_string(fixture_path(name))?;
    Ok(serde_json::from_str(&raw)?)
}

fn load_report(name: &str) -> Result<BranchProtectionReport, Box<dyn std::error::Error>> {
    let raw = std::fs::read_to_string(fixture_path(name))?;
    Ok(serde_json::from_str(&raw)?)
}

/// The desired configuration for THIS repo's `main`: both CI workflows
/// that push-trigger on `main` today -- `Rust CI`'s `rust-ci` job
/// (fmt/clippy/test/deny/audit, matrixed across OSes) and `Ocentra
/// Enforcer`'s `ocentra-enforcer` job (the self-scan/dogfood gate) --
/// matching `.github/workflows/{ci,ocentra-enforcer}.yml` and
/// `.github/BRANCH_PROTECTION.md`.
fn desired() -> DesiredProtection {
    DesiredProtection::baseline(vec![
        WorkflowJob {
            workflow_name: "Rust CI".to_owned(),
            job_id: "rust-ci".to_owned(),
            matrix: vec![
                "ubuntu-latest".to_owned(),
                "windows-latest".to_owned(),
                "macos-latest".to_owned(),
            ],
        },
        WorkflowJob {
            workflow_name: "Ocentra Enforcer".to_owned(),
            job_id: "ocentra-enforcer".to_owned(),
            matrix: vec![],
        },
    ])
}

#[test]
fn pass_fixture_attests_protection() -> Result<(), Box<dyn std::error::Error>> {
    let live = load_live("pass.json")?;
    let verdict = enforcer_install::ci::branch_protection::verify(&desired(), &live);
    if !verdict.is_attested() {
        return Err(format!("expected Attested, got {verdict:?}").into());
    }
    assert_eq!(verdict.exit_code(), 0);
    Ok(())
}

#[test]
fn pass_fixture_emits_the_pinned_installer_ci_report() -> Result<(), Box<dyn std::error::Error>> {
    let live = load_live("pass.json")?;
    let report = verify_and_report(&desired(), &live);
    assert_eq!(report, load_report("pass_report.golden.json")?);
    let json = serde_json::to_string(&report)?;
    let round_tripped: BranchProtectionReport = serde_json::from_str(&json)?;
    assert_eq!(round_tripped, report);
    Ok(())
}

#[test]
fn fail_fixture_no_required_checks_refuses_non_zero() -> Result<(), Box<dyn std::error::Error>> {
    let live = load_live("fail_no_required_checks.json")?;
    let verdict = enforcer_install::ci::branch_protection::verify(&desired(), &live);
    match verdict {
        Verdict::Refused(_) => {
            assert!(verdict.exit_code() > 0);
            Ok(())
        }
        Verdict::Attested => Err("fail fixture must not attest".into()),
    }
}

#[test]
fn fail_fixture_bypassable_refuses_non_zero() -> Result<(), Box<dyn std::error::Error>> {
    let live = load_live("fail_bypassable.json")?;
    let verdict = enforcer_install::ci::branch_protection::verify(&desired(), &live);
    match verdict {
        Verdict::Refused(_) => {
            assert!(verdict.exit_code() > 0);
            Ok(())
        }
        Verdict::Attested => Err("bypassable fail fixture must not attest".into()),
    }
}

#[test]
fn bypassable_fixture_reports_every_refusal_with_stable_codes(
) -> Result<(), Box<dyn std::error::Error>> {
    let live = load_live("fail_bypassable.json")?;
    let report = verify_and_report(&desired(), &live);
    assert!(!report.attested);
    assert!(report.exit_code > 0);
    assert_eq!(
        report
            .refusals
            .iter()
            .map(|refusal| refusal.code.as_str())
            .collect::<Vec<_>>(),
        vec!["admin_override_allowed", "force_push_allowed"]
    );
    Ok(())
}

#[test]
fn fail_fixture_red_merge_eligible_refuses_non_zero() -> Result<(), Box<dyn std::error::Error>> {
    let live = load_live("fail_red_merge_eligible.json")?;
    let verdict = enforcer_install::ci::branch_protection::verify(&desired(), &live);
    match verdict {
        Verdict::Refused(_) => {
            assert!(verdict.exit_code() > 0);
            Ok(())
        }
        Verdict::Attested => Err("red-but-merge-eligible fail fixture must not attest".into()),
    }
}

#[test]
fn emitter_payload_matches_the_pinned_golden_fixture() -> Result<(), Box<dyn std::error::Error>> {
    let raw = std::fs::read_to_string(fixture_path("emitted_payload.golden.json"))?;
    let golden: GhApiPayload = serde_json::from_str(&raw)?;
    let emitted = emit_payload(&desired());
    assert_eq!(
        emitted, golden,
        "emit_payload() drifted from the pinned golden fixture -- update the golden only if the desired protection legitimately changed"
    );
    Ok(())
}

#[test]
fn emitter_deduplicates_repeated_workflow_contexts() {
    let repeated = WorkflowJob {
        workflow_name: "Rust CI".to_owned(),
        job_id: "rust-ci".to_owned(),
        matrix: vec!["ubuntu-latest".to_owned(), "ubuntu-latest".to_owned()],
    };
    let desired = DesiredProtection::baseline(vec![repeated.clone(), repeated]);

    let contexts = desired.required_contexts();
    assert_eq!(contexts, vec!["Rust CI / rust-ci (ubuntu-latest)".to_owned()]);
    assert_eq!(
        emit_payload(&desired)
            .required_status_checks
            .expect("nonempty desired protection emits required checks")
            .contexts,
        contexts
    );
}

#[test]
fn emitted_payload_applied_as_live_state_yields_the_pass_fixture(
) -> Result<(), Box<dyn std::error::Error>> {
    // The emitter fixture's core claim: applying the emitted config (as if
    // `gh api` had accepted it and a subsequent read-back reported it,
    // plus required checks green) reproduces exactly the pass-fixture
    // state, and the verifier attests it.
    let payload = emit_payload(&desired());
    let live = LiveProtectionState {
        required_status_checks: payload.required_status_checks,
        enforce_admins: payload.enforce_admins,
        required_pull_request: payload.required_pull_request,
        allow_force_pushes: payload.allow_force_pushes,
        allow_deletions: payload.allow_deletions,
        required_checks_passing: Some(true),
    };
    let pass_fixture = load_live("pass.json")?;
    assert_eq!(live, pass_fixture);
    let verdict = enforcer_install::ci::branch_protection::verify(&desired(), &live);
    assert!(verdict.is_attested());
    Ok(())
}

#[test]
fn check_contexts_resolve_symbolically_never_a_hardcoded_stale_string(
) -> Result<(), Box<dyn std::error::Error>> {
    // Reconciliation check: the desired configuration's contexts must be
    // derived from the ACTUAL workflow files' declared name/job-id/matrix,
    // not a literal string authored independently of those files. This
    // guards against the exact drift the legacy `docs/BRANCH_PROTECTION.md`
    // suffered (it named `Ocentra Enforcer / ocentra-enforcer (*)` WITH a
    // matrix suffix, a pre-rename context nobody ever reconciled against
    // the real workflow -- which today has no matrix at all).
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");

    let rust_ci_job = WorkflowJob {
        workflow_name: "Rust CI".to_owned(),
        job_id: "rust-ci".to_owned(),
        matrix: vec![
            "ubuntu-latest".to_owned(),
            "windows-latest".to_owned(),
            "macos-latest".to_owned(),
        ],
    };
    let ci_yml = std::fs::read_to_string(repo_root.join(".github/workflows/ci.yml"))?;
    assert!(
        ci_yml.lines().any(|line| line.trim() == "name: Rust CI"),
        "ci.yml must declare the Rust CI workflow name exactly"
    );
    assert!(
        ci_yml.lines().any(|line| line.trim() == "rust-ci:"),
        "ci.yml must declare the rust-ci job exactly"
    );
    for context in resolve_contexts(&rust_ci_job) {
        assert!(context.starts_with("Rust CI / rust-ci ("));
    }

    let enforcer_job = WorkflowJob {
        workflow_name: "Ocentra Enforcer".to_owned(),
        job_id: "ocentra-enforcer".to_owned(),
        matrix: vec![],
    };
    let enforcer_yml =
        std::fs::read_to_string(repo_root.join(".github/workflows/ocentra-enforcer.yml"))?;
    assert!(
        enforcer_yml
            .lines()
            .any(|line| line.trim() == "name: Ocentra Enforcer"),
        "ocentra-enforcer.yml must declare the Ocentra Enforcer workflow name exactly"
    );
    assert!(
        enforcer_yml
            .lines()
            .any(|line| line.trim() == "ocentra-enforcer:"),
        "ocentra-enforcer.yml must declare the ocentra-enforcer job exactly"
    );
    // This workflow has NO matrix today -- resolve_contexts must render a
    // single bare context, not a stale parenthesized-OS suffix (the exact
    // shape the legacy doc got wrong).
    assert_eq!(
        resolve_contexts(&enforcer_job),
        vec!["Ocentra Enforcer / ocentra-enforcer".to_owned()]
    );
    Ok(())
}
