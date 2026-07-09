use enforcer_install::emitters::git_hooks::{plan, HookFlavor};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn hook_templates_default_to_staged_scope_with_workspace_opt_in() -> TestResult {
    for flavor in [
        HookFlavor::PlainGitHook,
        HookFlavor::Husky,
        HookFlavor::Lefthook,
    ] {
        let writes = plan(std::path::Path::new("repo-root"), flavor);
        let template = writes
            .first()
            .ok_or("git hook flavor must produce one planned write")?
            .contents;

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
