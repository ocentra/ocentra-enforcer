//! One dispatch function per subcommand. Delegates all engine work to the
//! owning crate (`enforcer-scan`, `enforcer-mcp`, `enforcer-literal-scan`,
//! ...); this module never reimplements validator/engine logic, only
//! wires scope resolution -> engine call -> [`crate::output`] rendering
//! and maps the outcome to an [`ExitCode`].

use std::path::Path;

use enforcer_domain::core_types::ExitCode;
use enforcer_domain::findings::ReportOutcome;
use enforcer_domain::paths::RepoRoot;
use enforcer_domain::scan_types::{IgnoreDirectorySegment, ResolvedScope};
use enforcer_scan::{engine, walk};

use crate::cli::{
    AiRuleIndexArgs, ArchitectureAction, ArchitectureCheckArgs, GeneratedArtifactsArgs,
    MutationRiskArgs, PolicyAction, RequiredTestsArgs, SbomArgs, ScopeArgs,
    SingleSourceContractsArgs, VerifyArgs,
};
use crate::output;

/// Resolve the repo root to scan/check against: the current working
/// directory, canonicalized and normalized. A failure here is an
/// internal error (the enforcer cannot even locate its own working
/// directory), never a usage error.
fn current_repo_root() -> Result<RepoRoot, String> {
    let cwd = std::env::current_dir().map_err(|e| format!("cannot read current directory: {e}"))?;
    cwd.to_string_lossy()
        .parse::<RepoRoot>()
        .map_err(|e| e.to_string())
}

/// Resolve a scope request into the concrete file list the engine should
/// read. Explicit paths are user-selected scan targets and therefore bypass
/// discovery-only ignore rules for the requested file; workspace and diff
/// discovery continue to honor those rules. `All` walks the whole tree and
/// `Diff` resolves the Git range to changed paths before filtering.
fn resolve_files(
    root: &RepoRoot,
    resolved: &ResolvedScope,
    ignore_rules: &walk::IgnoreRules,
) -> std::io::Result<Vec<enforcer_domain::paths::RelPath>> {
    let root_path = Path::new(root.as_str());
    if resolved.kind == enforcer_domain::findings::ScanScope::Diff {
        let all_files = walk::walk(root_path, ignore_rules)?;
        let Some((base, head)) = &resolved.diff_range else {
            return Ok(Vec::new());
        };
        let output = std::process::Command::new("git")
            .args(["diff", "--name-only", base.as_str(), head.as_str()])
            .current_dir(root.as_str())
            .output()?;
        if !output.status.success() {
            return Err(std::io::Error::other("git diff --name-only failed"));
        }
        let changed = String::from_utf8_lossy(&output.stdout);
        let changed: std::collections::BTreeSet<_> = changed.lines().collect();
        return Ok(all_files
            .into_iter()
            .filter(|file| changed.contains(file.as_str()))
            .collect());
    }
    if !resolved.explicit_paths.is_empty() {
        return walk::expand_explicit(root_path, &resolved.explicit_paths, ignore_rules);
    }
    walk::walk(root_path, ignore_rules)
}

/// Load the authoritative policy-source exclusions once at the CLI boundary.
///
/// Project configuration owns its declared exclusions. Native policies also
/// exclude every test tree: tests intentionally contain counterfeit secrets
/// and other negative cases, rather than deployable product source.
fn load_policy_ignore_rules(root: &RepoRoot) -> Result<walk::IgnoreRules, String> {
    let config_path = Path::new(root.as_str()).join("ocentra-enforcer.config.json");
    let config = enforcer_config::load_project_config_with_env(&config_path)
        .map_err(|error| error.to_string())?;
    let test_tree = IgnoreDirectorySegment::try_new(String::from("tests"))
        .map_err(|error| error.to_string())?;
    let mut ignore_dirs = config.ignore_dirs;
    if !ignore_dirs.iter().any(|directory| directory == &test_tree) {
        ignore_dirs.push(test_tree);
    }
    Ok(walk::IgnoreRules::new(
        ignore_dirs,
        config.ignore_file_globs,
    ))
}

/// Run the scan engine over a resolved [`ScopeArgs`] and render the
/// report. Shared by `check`/`scan`/`verify`.
pub fn run_scoped_check(scope_args: &ScopeArgs) -> ExitCode {
    let request = match crate::scope::resolve_request(scope_args) {
        Ok(request) => request,
        Err(message) => {
            output::print_usage_error(&message);
            return ExitCode::UsageError;
        }
    };
    let root = match current_repo_root() {
        Ok(root) => root,
        Err(message) => {
            output::print_internal_error(&message);
            return ExitCode::InternalError;
        }
    };
    let config_path = Path::new(root.as_str()).join("ocentra-enforcer.config.json");
    let config = match enforcer_config::load_project_config_with_env(&config_path) {
        Ok(config) => config,
        Err(err) => {
            output::print_config_error(&err.to_string());
            return ExitCode::ConfigError;
        }
    };
    let resolved = match enforcer_scan::scope::resolve(&request, &root) {
        Ok(resolved) => resolved,
        Err(err) => {
            output::print_usage_error(&err.to_string());
            return ExitCode::UsageError;
        }
    };
    let ignore_rules =
        walk::IgnoreRules::new(config.ignore_dirs.clone(), config.ignore_file_globs.clone());
    let files = match resolve_files(&root, &resolved, &ignore_rules) {
        Ok(files) => files,
        Err(err) => {
            output::print_internal_error(&format!("walk failed: {err}"));
            return ExitCode::InternalError;
        }
    };
    let validators = match engine::build_family_validators() {
        Ok(validators) => validators,
        Err(err) => {
            output::print_internal_error(&format!("failed to build validator registry: {err}"));
            return ExitCode::InternalError;
        }
    };
    let report = engine::run_with_inline_test_policy(
        &resolved,
        &files,
        &validators,
        config.rust_scan_scope.inline_test_policy,
    );
    output::print_report(&report);
    if report.ok == ReportOutcome::Clean {
        ExitCode::Success
    } else {
        ExitCode::Violations
    }
}

/// `enforcer verify --mode <mode> <scope>`. `--mode` selection only
/// changes which profile of checks conceptually applies; this skeleton
/// runs the same engine as `check` for every mode (no per-mode check
/// subsetting has landed in `enforcer-scan` yet) so the mode always
/// parses and routes, matching the workpack's fixture intent, without
/// pretending a subsetting engine exists when it does not.
pub fn run_verify(args: &VerifyArgs) -> ExitCode {
    run_scoped_check(&args.scope)
}

/// `enforcer policy dependency-policy`: run the native Cargo local-path
/// dependency policy against the whole current workspace.
pub fn run_policy(action: &PolicyAction) -> ExitCode {
    match action {
        PolicyAction::DependencyPolicy => run_dependency_policy(),
        PolicyAction::Secrets => run_secret_policy(),
        PolicyAction::Sbom(args) => run_sbom_policy(args),
        PolicyAction::RequiredTests(args) => run_required_tests_policy(args),
        PolicyAction::GeneratedArtifacts(args) => run_generated_artifacts_policy(args),
        PolicyAction::MutationRisk(args) => run_mutation_risk_policy(args),
        PolicyAction::SingleSourceContracts(args) => run_single_source_contracts_policy(args),
        PolicyAction::AiRuleIndex(args) => run_ai_rule_index_policy(args),
    }
}
fn run_mutation_risk_policy(args: &MutationRiskArgs) -> ExitCode {
    let request = match crate::scope::resolve_request(&args.scope) {
        Ok(enforcer_domain::scan_types::ScopeRequest::All) => {
            output::print_usage_error("mutation-risk requires explicit paths or --base <sha> --head <sha>; --all is not a mutation set");
            return ExitCode::UsageError;
        }
        Ok(request) => request,
        Err(error) => {
            output::print_usage_error(&error);
            return ExitCode::UsageError;
        }
    };
    let root = match current_repo_root() {
        Ok(root) => root,
        Err(error) => {
            output::print_internal_error(&error);
            return ExitCode::InternalError;
        }
    };
    let resolved = match enforcer_scan::scope::resolve(&request, &root) {
        Ok(value) => value,
        Err(error) => {
            output::print_usage_error(&error.to_string());
            return ExitCode::UsageError;
        }
    };
    let files = match resolve_files(&root, &resolved, &walk::IgnoreRules::default()) {
        Ok(files) => files,
        Err(error) => {
            output::print_internal_error(&format!("mutation-risk file resolution failed: {error}"));
            return ExitCode::InternalError;
        }
    };
    let proof_validation = enforcer_proof::boundary::mutation_risk::validate(
        Path::new(root.as_str()),
        resolved.diff_range.as_ref().map(|(_, head)| head.as_str()),
    );
    let proof = enforcer_scan::mutation_risk::MutationRiskProofState::from_accepted(
        proof_validation.is_accepted(),
    );
    match enforcer_scan::mutation_risk::check(
        resolved.kind,
        &files,
        &enforcer_scan::mutation_risk::MutationRiskPolicy::default(),
        proof,
    ) {
        Ok(report) => {
            output::print_report(&report);
            if report.ok == ReportOutcome::Clean {
                ExitCode::Success
            } else {
                ExitCode::Violations
            }
        }
        Err(error) => {
            output::print_internal_error(&format!("mutation-risk failed: {error}"));
            ExitCode::InternalError
        }
    }
}
fn run_ai_rule_index_policy(args: &AiRuleIndexArgs) -> ExitCode {
    let root = match current_repo_root() {
        Ok(root) => root,
        Err(error) => {
            output::print_internal_error(&error);
            return ExitCode::InternalError;
        }
    };
    match enforcer_scan::ai_rule_index::check(
        &root,
        enforcer_domain::findings::ScanScope::Workspace,
        args.max_lines,
    ) {
        Ok(report) => {
            output::print_report(&report);
            if report.ok == ReportOutcome::Clean {
                ExitCode::Success
            } else {
                ExitCode::Violations
            }
        }
        Err(error) => {
            output::print_internal_error(&error);
            ExitCode::InternalError
        }
    }
}

fn run_single_source_contracts_policy(args: &SingleSourceContractsArgs) -> ExitCode {
    let root = match current_repo_root() {
        Ok(value) => value,
        Err(error) => {
            output::print_internal_error(&error);
            return ExitCode::InternalError;
        }
    };
    let resolved =
        match enforcer_scan::scope::resolve(&enforcer_domain::scan_types::ScopeRequest::All, &root)
        {
            Ok(value) => value,
            Err(error) => {
                output::print_usage_error(&error.to_string());
                return ExitCode::UsageError;
            }
        };
    let files = match walk::walk(Path::new(root.as_str()), &walk::IgnoreRules::default()) {
        Ok(value) => value,
        Err(error) => {
            output::print_internal_error(&format!("walk failed: {error}"));
            return ExitCode::InternalError;
        }
    };
    let config = args
        .config_path
        .as_ref()
        .map(|path| path.to_string_lossy().to_string());
    match enforcer_scan::single_source_contracts::check(
        &root,
        resolved.kind,
        &files,
        config.as_deref(),
    ) {
        Ok(report) => {
            output::print_report(&report);
            if report.ok == ReportOutcome::Clean {
                ExitCode::Success
            } else {
                ExitCode::Violations
            }
        }
        Err(error) => {
            output::print_internal_error(&error);
            ExitCode::InternalError
        }
    }
}

fn run_generated_artifacts_policy(args: &GeneratedArtifactsArgs) -> ExitCode {
    let root = match current_repo_root() {
        Ok(root) => root,
        Err(message) => {
            output::print_internal_error(&message);
            return ExitCode::InternalError;
        }
    };
    let config_path = Path::new(root.as_str()).join("ocentra-enforcer.config.json");
    let config = match enforcer_config::load_project_config_with_env(&config_path) {
        Ok(config) => config,
        Err(error) => {
            output::print_config_error(&error.to_string());
            return ExitCode::ConfigError;
        }
    };
    let request = enforcer_domain::scan_types::ScopeRequest::All;
    let resolved = match enforcer_scan::scope::resolve(&request, &root) {
        Ok(resolved) => resolved,
        Err(error) => {
            output::print_usage_error(&error.to_string());
            return ExitCode::UsageError;
        }
    };
    let ignore_rules =
        walk::IgnoreRules::new(config.ignore_dirs.clone(), config.ignore_file_globs.clone());
    let files = match resolve_files(&root, &resolved, &ignore_rules) {
        Ok(files) => files,
        Err(error) => {
            output::print_internal_error(&format!("walk failed: {error}"));
            return ExitCode::InternalError;
        }
    };
    let tracked = args.tracked
        || matches!(
            config.generated_artifacts_mode,
            enforcer_domain::config_types::GeneratedArtifactsMode::Tracked
        );
    let allowlist = config
        .generated_artifacts_allowlist
        .iter()
        .map(|glob| glob.as_str().to_owned())
        .collect::<Vec<_>>();
    let report = match enforcer_scan::generated_artifacts::check(
        &root,
        resolved.kind,
        &files,
        tracked,
        &allowlist,
    ) {
        Ok(report) => report,
        Err(error) => {
            output::print_internal_error(&format!("generated-artifacts failed: {error}"));
            return ExitCode::InternalError;
        }
    };
    output::print_report(&report);
    if report.ok == ReportOutcome::Clean {
        ExitCode::Success
    } else {
        ExitCode::Violations
    }
}

fn run_required_tests_policy(args: &RequiredTestsArgs) -> ExitCode {
    let root = match current_repo_root() {
        Ok(root) => root,
        Err(message) => {
            output::print_internal_error(&message);
            return ExitCode::InternalError;
        }
    };
    let config_path = Path::new(root.as_str()).join("ocentra-enforcer.config.json");
    let config = match enforcer_config::load_project_config_with_env(&config_path) {
        Ok(config) => config,
        Err(error) => {
            output::print_config_error(&error.to_string());
            return ExitCode::ConfigError;
        }
    };
    let request = match crate::scope::resolve_request(&args.scope) {
        Ok(request) => request,
        Err(message) => {
            output::print_usage_error(&message);
            return ExitCode::UsageError;
        }
    };
    let resolved = match enforcer_scan::scope::resolve(&request, &root) {
        Ok(resolved) => resolved,
        Err(error) => {
            output::print_usage_error(&error.to_string());
            return ExitCode::UsageError;
        }
    };
    let ignore_rules =
        walk::IgnoreRules::new(config.ignore_dirs.clone(), config.ignore_file_globs.clone());
    let files = match resolve_files(&root, &resolved, &ignore_rules) {
        Ok(files) => files,
        Err(error) => {
            output::print_internal_error(&format!("walk failed: {error}"));
            return ExitCode::InternalError;
        }
    };
    let report = enforcer_scan::engine::run_required_test_policy(
        &resolved,
        &files,
        args.strict_empty_test_trees || config.strict_empty_test_trees.requires_nonempty(),
        &config.private_rust_test_module_allowlist,
    );
    output::print_report(&report);
    if report.ok == ReportOutcome::Clean {
        ExitCode::Success
    } else {
        ExitCode::Violations
    }
}

fn run_dependency_policy() -> ExitCode {
    let root = match current_repo_root() {
        Ok(root) => root,
        Err(message) => {
            output::print_internal_error(&message);
            return ExitCode::InternalError;
        }
    };
    let request = enforcer_domain::scan_types::ScopeRequest::All;
    let ignore_rules = match load_policy_ignore_rules(&root) {
        Ok(rules) => rules,
        Err(err) => {
            output::print_config_error(&err);
            return ExitCode::ConfigError;
        }
    };
    let resolved = match enforcer_scan::scope::resolve(&request, &root) {
        Ok(resolved) => resolved,
        Err(err) => {
            output::print_usage_error(&err.to_string());
            return ExitCode::UsageError;
        }
    };
    let files = match resolve_files(&root, &resolved, &ignore_rules) {
        Ok(files) => files,
        Err(err) => {
            output::print_internal_error(&format!("walk failed: {err}"));
            return ExitCode::InternalError;
        }
    };
    let report = enforcer_scan::engine::run_dependency_policy(&resolved, &files);
    output::print_report(&report);
    if report.ok == ReportOutcome::Clean {
        ExitCode::Success
    } else {
        ExitCode::Violations
    }
}

/// `enforcer policy secrets`: run only the native secret validators over the
/// current workspace. It deliberately does not piggyback on the full scan,
/// so unrelated language findings cannot obscure the credential verdict.
fn run_secret_policy() -> ExitCode {
    let root = match current_repo_root() {
        Ok(root) => root,
        Err(message) => {
            output::print_internal_error(&message);
            return ExitCode::InternalError;
        }
    };
    let request = enforcer_domain::scan_types::ScopeRequest::All;
    let ignore_rules = match load_policy_ignore_rules(&root) {
        Ok(rules) => rules,
        Err(err) => {
            output::print_config_error(&err);
            return ExitCode::ConfigError;
        }
    };
    let resolved = match enforcer_scan::scope::resolve(&request, &root) {
        Ok(resolved) => resolved,
        Err(err) => {
            output::print_usage_error(&err.to_string());
            return ExitCode::UsageError;
        }
    };
    let files = match resolve_files(&root, &resolved, &ignore_rules) {
        Ok(files) => files,
        Err(err) => {
            output::print_internal_error(&format!("walk failed: {err}"));
            return ExitCode::InternalError;
        }
    };
    let report = match enforcer_scan::engine::run_secret_policy(&resolved, &files) {
        Ok(report) => report,
        Err(err) => {
            output::print_internal_error(&format!(
                "failed to build secret validator registry: {err}"
            ));
            return ExitCode::InternalError;
        }
    };
    output::print_report(&report);
    if report.ok == ReportOutcome::Clean {
        ExitCode::Success
    } else {
        ExitCode::Violations
    }
}

/// `enforcer policy sbom --output <directory>`: emit an Enforcer-owned
/// document bound to the exact locked Cargo resolution.
fn run_sbom_policy(args: &SbomArgs) -> ExitCode {
    let root = match current_repo_root() {
        Ok(root) => root,
        Err(message) => {
            output::print_internal_error(&message);
            return ExitCode::InternalError;
        }
    };
    let root_path = Path::new(root.as_str());
    match enforcer_scan::sbom_policy::generate_current_workspace(root_path, &args.output) {
        Ok(artifact) => {
            // The output sink intentionally owns process output. The artifact
            // path is not a finding; it is the successful command result.
            output::print_artifact_path(&artifact);
            ExitCode::Success
        }
        Err(error) => {
            output::print_internal_error(&format!("native SBOM generation failed: {error}"));
            ExitCode::InternalError
        }
    }
}

/// `enforcer advise literals`. Routes to `enforcer-literal-scan`'s
/// `run_scan`, preserving the single-target constraint upstream in
/// `crate::advise::AdviseTarget` (unknown targets never parse to begin
/// with).
pub fn run_advise_literals() -> ExitCode {
    let root = match current_repo_root() {
        Ok(root) => root,
        Err(message) => {
            output::print_internal_error(&message);
            return ExitCode::InternalError;
        }
    };
    let opts = enforcer_literal_scan::CliOptions {
        root: std::path::PathBuf::from(root.as_str()).into(),
        ..enforcer_literal_scan::CliOptions::default()
    };
    match enforcer_literal_scan::run_scan(&opts) {
        Ok(report) => {
            let ok = report.ok;
            output::print_literal_scan_report(&report);
            if matches!(ok, enforcer_domain::findings::ReportOutcome::Clean) {
                ExitCode::Success
            } else {
                ExitCode::Violations
            }
        }
        Err(err) => {
            output::print_internal_error(&format!("literal-risk scan failed: {err}"));
            ExitCode::InternalError
        }
    }
}

/// `enforcer architecture check --language <lang> <scope>`. The selected
/// language is a compatibility argument while the configured aggregate owns
/// its own native rule families and reports every configured member.
pub fn run_architecture(action: &ArchitectureAction) -> ExitCode {
    match action {
        ArchitectureAction::Check(args) => run_architecture_check(args),
    }
}

fn run_architecture_check(args: &ArchitectureCheckArgs) -> ExitCode {
    let _language = args.language;
    let request = match crate::scope::resolve_request(&args.scope) {
        Ok(request) => request,
        Err(message) => {
            output::print_usage_error(&message);
            return ExitCode::UsageError;
        }
    };
    let root = match current_repo_root() {
        Ok(root) => root,
        Err(message) => {
            output::print_internal_error(&message);
            return ExitCode::InternalError;
        }
    };
    let config_path = args
        .config
        .clone()
        .unwrap_or_else(|| Path::new(root.as_str()).join("ocentra-enforcer.config.json"));
    let config = match enforcer_config::load_project_config_with_env(&config_path) {
        Ok(config) => config,
        Err(error) => {
            output::print_config_error(&error.to_string());
            return ExitCode::ConfigError;
        }
    };
    let resolved = match enforcer_scan::scope::resolve(&request, &root) {
        Ok(resolved) => resolved,
        Err(error) => {
            output::print_usage_error(&error.to_string());
            return ExitCode::UsageError;
        }
    };
    let ignore_rules =
        walk::IgnoreRules::new(config.ignore_dirs.clone(), config.ignore_file_globs.clone());
    let files = match resolve_files(&root, &resolved, &ignore_rules) {
        Ok(files) => files,
        Err(error) => {
            output::print_internal_error(&format!("walk failed: {error}"));
            return ExitCode::InternalError;
        }
    };
    match enforcer_scan::architecture_policy::execute(&root, resolved.kind, &files, &config) {
        Ok(result) => {
            if args.json {
                output::print_report_json(&result.report);
            } else {
                output::print_report(&result.report);
            }
            if result.report.ok == ReportOutcome::Clean {
                ExitCode::Success
            } else {
                ExitCode::Violations
            }
        }
        Err(error) => {
            output::print_internal_error(&format!("architecture-policy failed: {error}"));
            ExitCode::InternalError
        }
    }
}

#[cfg(test)]
mod tests {
    use super::current_repo_root;

    #[test]
    fn current_repo_root_resolves_in_a_real_process() -> Result<(), std::io::Error> {
        let root = current_repo_root().map_err(std::io::Error::other)?;
        assert!(!root.as_str().is_empty());
        Ok(())
    }
}
