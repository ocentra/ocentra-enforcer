use enforcer_domain::install_types::HookFlavor;
use enforcer_domain::install_types::{InstallRootPath, OverwriteMode};
use enforcer_install::emitters::git_hooks::{apply, plan, verify};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn hook_templates_default_to_staged_scope_with_workspace_opt_in() -> TestResult {
    for flavor in [
        HookFlavor::PlainGitHook,
        HookFlavor::Husky,
        HookFlavor::Lefthook,
    ] {
        let root =
            enforcer_domain::install_types::InstallRootPath::try_from(std::env::current_dir()?)?;
        let writes = plan(&root, flavor)?;
        let template = writes
            .first()
            .ok_or("git hook flavor must produce one planned write")?
            .contents
            .as_str();

        assert!(
            template.contains("OCENTRA_ENFORCER_PRECOMMIT_SCOPE:-staged"),
            "{flavor:?} must default pre-commit enforcement to staged scope"
        );
        assert!(
            template.contains("OCENTRA_ENFORCER_PRECOMMIT_SCOPE:-staged}\" = \"workspace\""),
            "{flavor:?} must keep full workspace pre-commit as an explicit opt-in"
        );
        assert!(
            !template.contains("--workspace"),
            "{flavor:?} must not use the retired workspace-scope flag"
        );
    }
    Ok(())
}

#[test]
fn linked_worktree_plain_hook_is_per_worktree_and_never_common_hooks() -> TestResult {
    let root = tempfile::tempdir()?;
    let common = root.path().join("common-git");
    let worktree_git = root.path().join("worktree-git");
    std::fs::create_dir_all(common.join("hooks"))?;
    std::fs::create_dir_all(&worktree_git)?;
    std::fs::write(
        common.join("config"),
        "[extensions]\n\tworktreeConfig = true\n[ocentra]\n\tmarker = keep\n",
    )?;
    std::fs::write(worktree_git.join("commondir"), "../common-git\n")?;
    std::fs::write(
        worktree_git.join("config.worktree"),
        "[ocentra]\n\tmarker = keep\n",
    )?;
    std::fs::write(
        common.join("hooks/pre-commit"),
        "shared hook must remain unchanged\n",
    )?;
    std::fs::write(
        root.path().join(".git"),
        format!("gitdir: {}\n", worktree_git.display()),
    )?;

    let install_root = InstallRootPath::try_from(root.path().to_path_buf())?;
    let planned = plan(
        &install_root,
        enforcer_domain::install_types::HookFlavor::PlainGitHook,
    )?;
    let planned_path = planned
        .first()
        .ok_or("plain hook must produce one planned write")?
        .path
        .as_path()
        .to_path_buf();
    assert_eq!(planned_path, root.path().join(".enforcer/hooks/pre-commit"));
    assert_ne!(planned_path, common.join("hooks/pre-commit"));

    apply(
        &install_root,
        enforcer_domain::install_types::HookFlavor::PlainGitHook,
        OverwriteMode::PreserveExisting,
    )?;
    assert!(planned_path.is_file());
    assert_eq!(
        std::fs::read_to_string(common.join("hooks/pre-commit"))?,
        "shared hook must remain unchanged\n"
    );
    let worktree_config = std::fs::read_to_string(worktree_git.join("config.worktree"))?;
    assert!(worktree_config.contains("hooksPath = .enforcer/hooks"));
    assert!(verify(
        &install_root,
        enforcer_domain::install_types::HookFlavor::PlainGitHook,
    )?
    .iter()
    .all(|check| matches!(
        check.status,
        enforcer_domain::install_types::CheckStatus::Passed
    )));
    Ok(())
}

#[test]
fn linked_worktree_without_worktree_config_refuses_shared_config_mutation() -> TestResult {
    let root = tempfile::tempdir()?;
    let common = root.path().join("common-git");
    let worktree_git = root.path().join("worktree-git");
    std::fs::create_dir_all(&common)?;
    std::fs::create_dir_all(&worktree_git)?;
    let common_config = "[extensions]\n\tworktreeConfig = false\n";
    std::fs::write(common.join("config"), common_config)?;
    std::fs::write(worktree_git.join("commondir"), "../common-git\n")?;
    std::fs::write(
        root.path().join(".git"),
        format!("gitdir: {}\n", worktree_git.display()),
    )?;

    let install_root = InstallRootPath::try_from(root.path().to_path_buf())?;
    let result = apply(
        &install_root,
        enforcer_domain::install_types::HookFlavor::PlainGitHook,
        OverwriteMode::PreserveExisting,
    );
    assert!(matches!(
        result,
        Err(enforcer_install::error::InstallError::MalformedConfig { ref path, ref reason })
            if (path.ends_with("common-git\\config") || path.ends_with("common-git/config"))
                && reason.contains("refusing to mutate the shared config")
    ));
    assert_eq!(
        std::fs::read_to_string(common.join("config"))?,
        common_config
    );
    assert!(!root.path().join(".enforcer/hooks/pre-commit").exists());
    Ok(())
}
