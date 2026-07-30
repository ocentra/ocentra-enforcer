//! One dispatch function per subcommand. Delegates all engine work to the
//! owning crate (`enforcer-scan`, `enforcer-mcp`, `enforcer-literal-scan`,
//! ...); this module never reimplements validator/engine logic, only
//! wires scope resolution -> engine call -> [`crate::output`] rendering
//! and maps the outcome to an [`ExitCode`].

use std::path::Path;

use enforcer_domain::core_types::ExitCode;
use enforcer_domain::findings::ReportOutcome;
use enforcer_domain::paths::RepoRoot;
use enforcer_domain::scan_types::ResolvedScope;
use enforcer_scan::{engine, walk};

use crate::cli::{
    ArchitectureAction, ArchitectureCheckArgs, PolicyAction, SbomArgs, ScopeArgs, VerifyArgs,
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
/// read. `Paths` mode intersects the walked tree with the caller's
/// explicit paths (files pass through directly; directories are expanded
/// by re-walking rooted at the directory); `All`/`Diff` walk the whole
/// tree (a `Diff` git-range restriction is not implemented by
/// `enforcer-scan` yet -- this skeleton walks the full tree for `Diff`
/// too rather than silently scanning zero files, and callers relying on
/// true diff-scoping should track that gap, not assume it is silently
/// narrower).
fn resolve_files(
    root: &RepoRoot,
    resolved: &ResolvedScope,
) -> std::io::Result<Vec<enforcer_domain::paths::RelPath>> {
    let root_path = Path::new(root.as_str());
    let all_files = walk::walk(root_path, &walk::IgnoreRules::default())?;
    if resolved.explicit_paths.is_empty() {
        return Ok(all_files);
    }
    Ok(all_files
        .into_iter()
        .filter(|file| {
            resolved
                .explicit_paths
                .iter()
                .any(|explicit| file.as_str().starts_with(explicit.as_str()))
        })
        .collect())
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
    let files = match resolve_files(&root, &resolved) {
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
    let resolved = match enforcer_scan::scope::resolve(&request, &root) {
        Ok(resolved) => resolved,
        Err(err) => {
            output::print_usage_error(&err.to_string());
            return ExitCode::UsageError;
        }
    };
    let files = match resolve_files(&root, &resolved) {
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
    let resolved = match enforcer_scan::scope::resolve(&request, &root) {
        Ok(resolved) => resolved,
        Err(err) => {
            output::print_usage_error(&err.to_string());
            return ExitCode::UsageError;
        }
    };
    let files = match resolve_files(&root, &resolved) {
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

/// `enforcer architecture check --language <lang> <scope>`. Routes
/// `typescript` to the real `import-boundaries` validator; `rust` has no
/// landed architecture validator yet (see module docs on
/// `crate::architecture`) and reports that gap through the internal-error
/// class rather than a false "clean" result.
pub fn run_architecture(action: &ArchitectureAction) -> ExitCode {
    match action {
        ArchitectureAction::Check(args) => run_architecture_check(args),
    }
}

fn run_architecture_check(args: &ArchitectureCheckArgs) -> ExitCode {
    use crate::architecture::ArchitectureLanguage;
    match args.language {
        ArchitectureLanguage::TypeScript => run_scoped_check(&args.scope),
        ArchitectureLanguage::Rust => {
            output::print_internal_error(
                "architecture-policy has no landed Rust validator yet (named-check parity \
                 entry only, per enforcer-mcp::registry); this is a real gap, not a clean run",
            );
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
