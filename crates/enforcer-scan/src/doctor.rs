//! Native repository-readiness doctor.
//!
//! This is deliberately distinct from `enforcer-install`'s harness registration
//! doctor: it checks a target repository, resolved policy, tool availability,
//! and the selected Rust scope without mutating the repository.

use std::process::Command;

use enforcer_domain::{
    config_types::{EffectiveConfig, RuleEnabled},
    paths::RepoRoot,
    scan_types::LanguageFamily,
};

use crate::{
    boundary::native_scan::{resolve_files_with_rules, NativeScanError, NativeScanRequest},
    router::classify,
    walk::IgnoreRules,
};

/// Fully resolved native repository-doctor input.
#[derive(Debug, Clone)]
pub struct DoctorRequest {
    repo_root: RepoRoot,
    scan: NativeScanRequest,
    config: EffectiveConfig,
}

impl DoctorRequest {
    /// Construct doctor input only after callers resolve config and scope.
    pub fn new(repo_root: RepoRoot, scan: NativeScanRequest, config: EffectiveConfig) -> Self {
        Self { repo_root, scan, config }
    }
}

/// One mechanical readiness check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DoctorCheck {
    name: &'static str,
    ok: bool,
    // BRAND-INVARIANT: detail is constructed only by this fixed doctor report vocabulary.
    detail: String,
}

impl DoctorCheck {
    pub fn name(&self) -> &'static str { self.name }
    pub const fn ok(&self) -> bool { self.ok }
    pub fn detail(&self) -> &str { &self.detail }
}

/// Native equivalent of the frozen MJS repository doctor report.
#[derive(Debug, Clone)]
pub struct DoctorReport {
    ok: bool,
    profile_name: String,
    checks: Vec<DoctorCheck>,
}

impl DoctorReport {
    pub const fn ok(&self) -> bool { self.ok }
    pub fn command(&self) -> &'static str { "doctor" }
    pub fn profile_name(&self) -> &str { &self.profile_name }
    pub fn checks(&self) -> &[DoctorCheck] { &self.checks }
}

/// Run the six frozen-doctor readiness checks using native Rust only.
pub fn run(request: &DoctorRequest) -> Result<DoctorReport, NativeScanError> {
    let root_exists = std::path::Path::new(request.repo_root.as_str()).is_dir();
    let rules = IgnoreRules::new(
        request.config.ignore_dirs.clone(),
        request.config.ignore_file_globs.clone(),
    );
    let (_, files) = resolve_files_with_rules(&request.scan, &request.repo_root, &rules)?;
    let rust_count = files.iter().filter(|path| classify(path) == LanguageFamily::Rust).count();
    let require_deny = request.config.cargo_dependency_policy.require_cargo_deny == RuleEnabled::Enabled;
    let checks = vec![
        check("root", root_exists, request.repo_root.as_str().to_owned()),
        check("config schema", true, format!("schemaVersion={}", request.config.schema_version)),
        check("cargo", command_exists("cargo"), "required for cargo gates and metadata dependency checks".to_owned()),
        check("git", command_exists("git"), "required for diff scopes".to_owned()),
        check("cargo-deny", !require_deny || command_exists("cargo-deny"), if require_deny { "required when requireCargoDeny=true".to_owned() } else { "not required by this profile".to_owned() }),
        check("scope files", rust_count > 0, format!("{rust_count} Rust file(s) selected")),
    ];
    let ok = checks.iter().all(DoctorCheck::ok);
    Ok(DoctorReport { ok, profile_name: request.config.profile_name.as_str().to_owned(), checks })
}

fn check(name: &'static str, ok: bool, detail: String) -> DoctorCheck { DoctorCheck { name, ok, detail } }

fn command_exists(command: &str) -> bool { Command::new(command).arg("--version").output().is_ok() }

#[cfg(test)]
mod tests {
    use super::{run, DoctorRequest};
    use crate::boundary::native_scan::{NativeScanRequest, NativeScanScope};
    use enforcer_domain::paths::RepoRoot;

    #[test]
    fn native_doctor_reports_the_frozen_six_checks() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        std::fs::create_dir_all(temp.path().join("src"))?;
        std::fs::write(temp.path().join("src/lib.rs"), "pub fn ready() {}\n")?;
        let root: RepoRoot = temp.path().to_string_lossy().parse()?;
        let config = enforcer_config::load_project_config(&temp.path().join("missing.json"))?;
        let request = DoctorRequest::new(root, NativeScanRequest { scope: NativeScanScope::Files(vec!["src/lib.rs".into()]), languages: Vec::new() }, config);
        let report = run(&request)?;
        assert_eq!(report.command(), "doctor");
        assert_eq!(report.checks().iter().map(|check| check.name()).collect::<Vec<_>>(), ["root", "config schema", "cargo", "git", "cargo-deny", "scope files"]);
        assert!(report.checks().iter().find(|check| check.name() == "scope files").is_some_and(|check| check.ok()));
        Ok(())
    }
}
