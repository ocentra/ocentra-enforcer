use std::collections::BTreeSet;

use enforcer_domain::ids::{
    BuiltInCfmlRule, BuiltInDartRule, BuiltInIacRule, BuiltInK8sRule, GitHubBranchName,
    GitHubCheckContext,
};

#[test]
fn built_in_cfml_rules_have_unique_ids_and_valid_titles() -> Result<(), Box<dyn std::error::Error>>
{
    let ids: BTreeSet<_> = BuiltInCfmlRule::ALL
        .into_iter()
        .map(BuiltInCfmlRule::id)
        .collect();
    assert_eq!(ids.len(), BuiltInCfmlRule::ALL.len());
    for rule in BuiltInCfmlRule::ALL {
        assert!(!rule.finding_title()?.as_str().is_empty());
    }
    Ok(())
}

#[test]
fn built_in_dart_rules_have_unique_ids_and_valid_titles() -> Result<(), Box<dyn std::error::Error>>
{
    let ids: BTreeSet<_> = BuiltInDartRule::ALL
        .into_iter()
        .map(BuiltInDartRule::id)
        .collect();
    assert_eq!(ids.len(), BuiltInDartRule::ALL.len());
    for rule in BuiltInDartRule::ALL {
        assert!(!rule.finding_title()?.as_str().is_empty());
    }
    Ok(())
}

#[test]
fn built_in_iac_rules_have_unique_ids_and_valid_titles() -> Result<(), Box<dyn std::error::Error>> {
    let ids: BTreeSet<_> = BuiltInIacRule::ALL
        .into_iter()
        .map(BuiltInIacRule::id)
        .collect();
    assert_eq!(ids.len(), BuiltInIacRule::ALL.len());
    for rule in BuiltInIacRule::ALL {
        assert!(!rule.finding_title()?.as_str().is_empty());
    }
    Ok(())
}

#[test]
fn built_in_k8s_rules_have_unique_ids_and_valid_titles() -> Result<(), Box<dyn std::error::Error>> {
    let ids: BTreeSet<_> = BuiltInK8sRule::ALL
        .into_iter()
        .map(BuiltInK8sRule::id)
        .collect();
    assert_eq!(ids.len(), BuiltInK8sRule::ALL.len());
    for rule in BuiltInK8sRule::ALL {
        assert!(!rule.finding_title()?.as_str().is_empty());
    }
    Ok(())
}

#[test]
fn github_check_context_rejects_line_breaks() {
    assert_eq!(
        GitHubCheckContext::try_from("Rust CI / rust-ci\nspoof".to_owned())
            .as_ref()
            .err()
            .map(|error| error.path.as_str()),
        Some("githubCheckContext")
    );
}

#[test]
fn github_branch_name_rejects_git_ref_special_characters() {
    for invalid in ["main..backup", "release candidate"] {
        assert_eq!(
            GitHubBranchName::try_from(invalid.to_owned())
                .as_ref()
                .err()
                .map(|error| error.path.as_str()),
            Some("githubBranchName")
        );
    }
}

#[test]
fn github_branch_protection_values_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    let context = GitHubCheckContext::try_from("Rust CI / rust-ci (windows-latest)".to_owned())?;
    let branch = GitHubBranchName::try_from("main".to_owned())?;
    assert_eq!(context.as_str(), "Rust CI / rust-ci (windows-latest)");
    assert_eq!(branch.as_str(), "main");
    Ok(())
}
