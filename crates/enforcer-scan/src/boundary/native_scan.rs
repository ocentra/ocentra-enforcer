//! BOUNDARY-INVARIANT: raw caller choices convert only into one typed native
//! scope and curated language filter before reaching the scan engine.
//! Negative invalid-input coverage rejects empty file scopes and unknown crate
//! names; no request may silently widen its scope or discard a filter.
//!
//! Typed native scan request boundary.
//!
//! This is the one execution entry point for the native scanner's supported
//! narrowing modes.  It intentionally has no catch-all option map: callers
//! must select exactly one typed scope and every requested language is either
//! applied or rejected before the engine runs.

use std::path::PathBuf;
use std::process::Command;

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::config_types::CrateName;
use enforcer_domain::findings::{Report, ScanScope};
use enforcer_domain::paths::{RelPath, RepoRoot};
use enforcer_domain::scan_types::{CommitRef, LanguageFamily, ScopeRequest};

use crate::engine::{self, build_family_validators};
use crate::router::classify;
use crate::scope;
use crate::walk::{self, IgnoreRules};

/// Language filters supported by the native scan engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeScanLanguage {
    Rust,
    TypeScript,
    Python,
    Terraform,
    YamlOrConfig,
}

impl NativeScanLanguage {
    fn matches(self, path: &RelPath) -> bool {
        matches!(
            (self, classify(path)),
            (Self::Rust, LanguageFamily::Rust)
                | (Self::TypeScript, LanguageFamily::TypeScript)
                | (Self::Python, LanguageFamily::Python)
                | (Self::Terraform, LanguageFamily::Terraform)
                | (Self::YamlOrConfig, LanguageFamily::YamlOrConfig)
        )
    }
}

/// Exactly one supported native scan scope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeScanScope {
    Files(Vec<PathBuf>),
    Workspace,
    Crate(CrateName),
    Diff { base: CommitRef, head: CommitRef },
}

/// Fully typed native scan request. There are no ignored options or fallback
/// scope: unsupported input must fail while being decoded into this shape.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeScanRequest {
    pub scope: NativeScanScope,
    pub languages: Vec<NativeScanLanguage>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NativeScanSelection {
    scope: NativeScanScope,
    languages: Vec<NativeScanLanguage>,
}

impl From<&NativeScanRequest> for NativeScanSelection {
    fn from(request: &NativeScanRequest) -> Self {
        Self {
            scope: request.scope.clone(),
            languages: request.languages.clone(),
        }
    }
}

/// Result produced by the native request path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeScanResult {
    pub scope: ScanScope,
    pub scanned_files: Vec<RelPath>,
    pub report: Report,
}

/// Failure from the native request boundary. Every variant is terminal: this
/// contract never widens a bad scope or drops an unsupported filter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeScanError {
    Decode(DecodeError),
    Io {
        operation: &'static str,
        reason: String,
    },
    UnsupportedCrate {
        name: CrateName,
    },
    UnsupportedLanguagePolicy {
        reason: String,
    },
    GitDiff {
        base: CommitRef,
        head: CommitRef,
        reason: String,
    },
}

impl std::fmt::Display for NativeScanError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Decode(error) => write!(formatter, "native scan input rejected: {error}"),
            Self::Io { operation, reason } => {
                write!(formatter, "native scan {operation} failed: {reason}")
            }
            Self::UnsupportedCrate { name } => {
                write!(formatter, "native scan crate `{name}` was not found")
            }
            Self::UnsupportedLanguagePolicy { reason } => formatter.write_str(reason),
            Self::GitDiff { base, head, reason } => write!(
                formatter,
                "native scan diff `{}..{}` could not be resolved: {reason}",
                base.as_str(),
                head.as_str()
            ),
        }
    }
}

impl std::error::Error for NativeScanError {}

impl From<DecodeError> for NativeScanError {
    fn from(value: DecodeError) -> Self {
        Self::Decode(value)
    }
}

/// Execute a typed native scan request against one repository root.
pub fn execute(
    request: &NativeScanRequest,
    repo_root: &RepoRoot,
) -> Result<NativeScanResult, NativeScanError> {
    let (resolved, files) = resolve_files(request, repo_root)?;
    let validators = build_family_validators()?;
    let report = engine::run(&resolved, &files, &validators);
    Ok(NativeScanResult {
        scope: resolved.kind,
        scanned_files: files,
        report,
    })
}

/// Execute only the native Cargo local-path dependency policy over the typed
/// request scope. This is deliberately distinct from a filtered full scan:
/// callers receive the real workspace-inventory policy without unrelated
/// language findings.
pub fn execute_dependency_policy(
    request: &NativeScanRequest,
    repo_root: &RepoRoot,
) -> Result<NativeScanResult, NativeScanError> {
    let (resolved, files) = resolve_files(request, repo_root)?;
    let report = engine::run_dependency_policy(&resolved, &files);
    Ok(NativeScanResult {
        scope: resolved.kind,
        scanned_files: files,
        report,
    })
}

/// Execute only the native secret-policy registry over the typed request
/// scope. This keeps the named `secrets` check tied to the concrete SEC
/// validators rather than to a broad scan filtered after the fact.
pub fn execute_secret_policy(
    request: &NativeScanRequest,
    repo_root: &RepoRoot,
) -> Result<NativeScanResult, NativeScanError> {
    let (resolved, files) = resolve_files(request, repo_root)?;
    let report = engine::run_secret_policy(&resolved, &files).map_err(NativeScanError::Decode)?;
    Ok(NativeScanResult {
        scope: resolved.kind,
        scanned_files: files,
        report,
    })
}

/// Execute the native secret policy over files currently staged in Git.
///
/// This mirrors the standalone `secrets --staged` contract: Git selects the
/// paths, while validation reads the working-tree source at those paths. The
/// selection deliberately excludes deletions because there is no source left
/// to validate.
pub fn execute_staged_secret_policy(
    request: &NativeScanRequest,
    repo_root: &RepoRoot,
) -> Result<NativeScanResult, NativeScanError> {
    let paths = staged_files(repo_root, &IgnoreRules::default())?;
    let staged_request = NativeScanRequest {
        scope: NativeScanScope::Files(
            paths
                .into_iter()
                .map(|path| PathBuf::from(path.as_str()))
                .collect(),
        ),
        languages: request.languages.clone(),
    };
    if let NativeScanScope::Files(paths) = &staged_request.scope {
        if paths.is_empty() {
            let mut resolved = scope::resolve(&ScopeRequest::All, repo_root)?;
            resolved.kind = ScanScope::Files;
            let report =
                engine::run_secret_policy(&resolved, &[]).map_err(NativeScanError::Decode)?;
            return Ok(NativeScanResult {
                scope: ScanScope::Files,
                scanned_files: Vec::new(),
                report,
            });
        }
    }
    execute_secret_policy(&staged_request, repo_root)
}

/// Execute the dedicated TypeScript import-boundary rule only.
pub fn execute_import_boundaries_policy(
    request: &NativeScanRequest,
    repo_root: &RepoRoot,
    config: &enforcer_domain::config_types::EffectiveConfig,
) -> Result<NativeScanResult, NativeScanError> {
    let (resolved, files) = resolve_files(request, repo_root)?;
    let report = crate::import_boundaries::check(repo_root, resolved.kind, &files, config)
        .map_err(|reason| NativeScanError::Io {
            operation: "import-boundaries check",
            reason,
        })?;
    Ok(NativeScanResult {
        scope: resolved.kind,
        scanned_files: files,
        report,
    })
}
/// Executes the Rust re-export policy for an explicitly typed scan request.
pub fn execute_reexports_policy(
    request: &NativeScanRequest,
    repo_root: &RepoRoot,
) -> Result<NativeScanResult, NativeScanError> {
    let (resolved, files) = resolve_files(request, repo_root)?;
    let report =
        engine::run_reexports_policy(&resolved, &files).map_err(NativeScanError::Decode)?;
    Ok(NativeScanResult {
        scope: resolved.kind,
        scanned_files: files,
        report,
    })
}

/// Execute frozen-MJS `mutation-risk` parity over exactly the typed request
/// scope. This is a dedicated path policy, never a filtered broad scan.
pub fn execute_mutation_risk_policy(
    request: &NativeScanRequest,
    repo_root: &RepoRoot,
    policy: &crate::mutation_risk::MutationRiskPolicy,
) -> Result<NativeScanResult, NativeScanError> {
    let (resolved, files) = resolve_files(request, repo_root)?;
    let report = crate::mutation_risk::check(resolved.kind, &files, policy).map_err(|reason| {
        NativeScanError::Io {
            operation: "mutation-risk check",
            reason,
        }
    })?;
    Ok(NativeScanResult {
        scope: resolved.kind,
        scanned_files: files,
        report,
    })
}

/// Execute the standalone documentation-completeness policy. Its authority is
/// the checked-in registry and rule-doc tree, so scope only labels the report.
pub fn execute_docs_completeness(
    request: &NativeScanRequest,
    repo_root: &RepoRoot,
) -> Result<NativeScanResult, NativeScanError> {
    let (resolved, files) = resolve_files(request, repo_root)?;
    let report = crate::docs_completeness::check(repo_root, resolved.kind).map_err(|reason| {
        NativeScanError::Io {
            operation: "docs-completeness check",
            reason,
        }
    })?;
    Ok(NativeScanResult {
        scope: resolved.kind,
        scanned_files: files,
        report,
    })
}

/// Validates project configuration lockdown for the requested repository scope.
pub fn execute_config_lockdown(
    request: &NativeScanRequest,
    repo_root: &RepoRoot,
) -> Result<NativeScanResult, NativeScanError> {
    let (resolved, files) = resolve_files(request, repo_root)?;
    let root = std::path::Path::new(repo_root.as_str());
    let report = crate::config_lockdown::check_config_lockdown(
        &root.join("ocentra-enforcer.config.json"),
        root,
        resolved.kind,
    )
    .map_err(|reason| NativeScanError::Io {
        operation: "config-lockdown check",
        reason,
    })?;
    Ok(NativeScanResult {
        scope: resolved.kind,
        scanned_files: files,
        report,
    })
}

/// Execute the native waiver governance policy. This deliberately shares the
/// config diagnostic ingress with `config-lockdown` but has its own report
/// family and never falls back to a broad source scan.
pub fn execute_waiver_policy(
    request: &NativeScanRequest,
    repo_root: &RepoRoot,
) -> Result<NativeScanResult, NativeScanError> {
    let (resolved, files) = resolve_files(request, repo_root)?;
    let root = std::path::Path::new(repo_root.as_str());
    let report = crate::config_lockdown::check_waiver_policy(
        &root.join("ocentra-enforcer.config.json"),
        root,
        resolved.kind,
    )
    .map_err(|reason| NativeScanError::Io {
        operation: "waiver-policy check",
        reason,
    })?;
    Ok(NativeScanResult {
        scope: resolved.kind,
        scanned_files: files,
        report,
    })
}

/// Execute the Rust-only implementation of `no-naked-domain-strings`.
pub fn execute_rust_string_boundaries_policy(
    request: &NativeScanRequest,
    repo_root: &RepoRoot,
    config: &enforcer_domain::config_types::EffectiveConfig,
) -> Result<NativeScanResult, NativeScanError> {
    if request
        .languages
        .iter()
        .any(|language| !matches!(language, NativeScanLanguage::Rust))
    {
        return Err(NativeScanError::UnsupportedLanguagePolicy {
            reason: "no-naked-domain-strings native implementation currently supports only rust; TypeScript and Python remain explicitly unsupported".to_owned(),
        });
    }
    let (resolved, files) = resolve_files(request, repo_root)?;
    let report =
        engine::run_rust_string_boundaries_policy(&resolved, &files, config).map_err(|reason| {
            NativeScanError::Io {
                operation: "rust string-boundaries check",
                reason,
            }
        })?;
    Ok(NativeScanResult {
        scope: resolved.kind,
        scanned_files: files,
        report,
    })
}

/// Execute the structural test-tree policy. `strict_empty_test_trees` is a
/// typed policy choice, not an ignored MCP option.
pub fn execute_required_test_policy(
    request: &NativeScanRequest,
    repo_root: &RepoRoot,
    config: &enforcer_domain::config_types::EffectiveConfig,
    strict_empty_test_trees: bool,
) -> Result<NativeScanResult, NativeScanError> {
    let (resolved, files) = resolve_files(request, repo_root)?;
    let report = engine::run_required_test_policy(
        &resolved,
        &files,
        strict_empty_test_trees || config.strict_empty_test_trees.requires_nonempty(),
        &config.private_rust_test_module_allowlist,
    );
    Ok(NativeScanResult {
        scope: resolved.kind,
        scanned_files: files,
        report,
    })
}

/// Execute the path-based generated-artifact policy.  Tracked mode obtains
/// its own Git inventory so generated directories excluded from the ordinary
/// source walker are still checked, matching the frozen MJS check.
pub fn execute_generated_artifacts(
    request: &NativeScanRequest,
    repo_root: &RepoRoot,
    config: &enforcer_domain::config_types::EffectiveConfig,
    tracked_override: bool,
) -> Result<NativeScanResult, NativeScanError> {
    let (resolved, files) = resolve_files(request, repo_root)?;
    let tracked = tracked_override
        || matches!(
            config.generated_artifacts_mode,
            enforcer_domain::config_types::GeneratedArtifactsMode::Tracked
        );
    let allowlist = config
        .generated_artifacts_allowlist
        .iter()
        .map(|glob| glob.as_str().to_owned())
        .collect::<Vec<_>>();
    let report =
        crate::generated_artifacts::check(repo_root, resolved.kind, &files, tracked, &allowlist)
            .map_err(|reason| NativeScanError::Io {
                operation: "generated-artifacts check",
                reason,
            })?;
    Ok(NativeScanResult {
        scope: resolved.kind,
        scanned_files: files,
        report,
    })
}

/// Checks the single-source-of-truth contracts for an explicit repository root.
pub fn execute_single_source_contracts(
    request: &NativeScanRequest,
    repo_root: &RepoRoot,
    config_path: Option<&str>,
) -> Result<NativeScanResult, NativeScanError> {
    let (resolved, files) = resolve_files(request, repo_root)?;
    let report =
        crate::single_source_contracts::check(repo_root, resolved.kind, &files, config_path)
            .map_err(|reason| NativeScanError::Io {
                operation: "single-source-contracts check",
                reason,
            })?;
    Ok(NativeScanResult {
        scope: resolved.kind,
        scanned_files: files,
        report,
    })
}

/// Validates the generated AI rule index against its reviewed source inputs.
pub fn execute_ai_rule_index(
    request: &NativeScanRequest,
    repo_root: &RepoRoot,
    max_lines: Option<usize>,
) -> Result<NativeScanResult, NativeScanError> {
    let (resolved, files) = resolve_files(request, repo_root)?;
    let report =
        crate::ai_rule_index::check(repo_root, resolved.kind, max_lines).map_err(|reason| {
            NativeScanError::Io {
                operation: "ai-rule-index check",
                reason,
            }
        })?;
    Ok(NativeScanResult {
        scope: resolved.kind,
        scanned_files: files,
        report,
    })
}

/// Execute the resolved, config-driven source-shape policy.  This is kept
/// separate from the broad family scan because the policy selects files by
/// configured roots and extensions, then applies ordered path overrides.
pub fn execute_source_shape_policy(
    request: &NativeScanRequest,
    repo_root: &RepoRoot,
    config: &enforcer_domain::config_types::EffectiveConfig,
) -> Result<NativeScanResult, NativeScanError> {
    let (resolved, files) = resolve_files(request, repo_root)?;
    let report =
        crate::source_shape::check(repo_root, resolved.kind, &files, config).map_err(|reason| {
            NativeScanError::Io {
                operation: "source-shape check",
                reason,
            }
        })?;
    Ok(NativeScanResult {
        scope: resolved.kind,
        scanned_files: files,
        report,
    })
}

/// Execute every configured architecture-policy member and retain its
/// per-member outcomes for CLI/MCP aggregation.
pub fn execute_architecture_policy(
    request: &NativeScanRequest,
    repo_root: &RepoRoot,
    config: &enforcer_domain::config_types::EffectiveConfig,
) -> Result<crate::architecture_policy::ArchitecturePolicyAggregate, NativeScanError> {
    let (resolved, files) = resolve_files(request, repo_root)?;
    crate::architecture_policy::execute(repo_root, resolved.kind, &files, config).map_err(
        |reason| NativeScanError::Io {
            operation: "architecture-policy check",
            reason,
        },
    )
}

/// Execute one named family already owned by the architecture-policy engine.
pub fn execute_architecture_rule_family(
    request: &NativeScanRequest,
    repo_root: &RepoRoot,
    check: &str,
) -> Result<NativeScanResult, NativeScanError> {
    let (resolved, files) = resolve_files(request, repo_root)?;
    let report =
        crate::architecture_policy::execute_rule_family(repo_root, resolved.kind, &files, check)
            .map_err(|reason| NativeScanError::Io {
                operation: "architecture named check",
                reason,
            })?;
    Ok(NativeScanResult {
        scope: resolved.kind,
        scanned_files: files,
        report,
    })
}

pub(crate) fn resolve_files(
    request: &NativeScanRequest,
    repo_root: &RepoRoot,
) -> Result<(enforcer_domain::scan_types::ResolvedScope, Vec<RelPath>), NativeScanError> {
    resolve_files_with_rules(request, repo_root, &IgnoreRules::default())
}

/// Resolve a native request through caller-supplied, already-validated ignore
/// rules.  This is shared by scan execution and readiness doctoring so both
/// select the same files for the same typed scope.
pub(crate) fn resolve_files_with_rules(
    request: &NativeScanRequest,
    repo_root: &RepoRoot,
    rules: &IgnoreRules,
) -> Result<(enforcer_domain::scan_types::ResolvedScope, Vec<RelPath>), NativeScanError> {
    let selection = NativeScanSelection::from(request);
    let (scope_request, files, kind) =
        match &selection.scope {
            NativeScanScope::Files(paths) => {
                if paths.is_empty() {
                    return Err(
                        DecodeError::new("scan.files", "must contain at least one path").into(),
                    );
                }
                let scope_request = ScopeRequest::Paths(paths.clone());
                let resolved = scope::resolve(&scope_request, repo_root)?;
                let files = walk::expand_explicit(
                    std::path::Path::new(repo_root.as_str()),
                    &resolved.explicit_paths,
                    rules,
                )
                .map_err(|error| {
                    DecodeError::new(
                        "scan.files",
                        format!(
                        "each requested path must identify an existing file or directory: {error}"
                    ),
                    )
                })?;
                (scope_request, files, ScanScope::Files)
            }
            NativeScanScope::Workspace => {
                let scope_request = ScopeRequest::All;
                let files = walk::walk(std::path::Path::new(repo_root.as_str()), rules).map_err(
                    |error| NativeScanError::Io {
                        operation: "workspace walk",
                        reason: error.to_string(),
                    },
                )?;
                (scope_request, files, ScanScope::Workspace)
            }
            NativeScanScope::Crate(name) => {
                let scope_request = ScopeRequest::All;
                let all_files = walk::walk(std::path::Path::new(repo_root.as_str()), rules)
                    .map_err(|error| NativeScanError::Io {
                        operation: "crate discovery walk",
                        reason: error.to_string(),
                    })?;
                let crate_root = find_crate_root(repo_root, &all_files, name)?;
                let files = all_files
                    .into_iter()
                    .filter(|path| {
                        path == &crate_root
                            || path
                                .as_str()
                                .starts_with(&format!("{}/", crate_root.as_str()))
                    })
                    .collect();
                (scope_request, files, ScanScope::Crate)
            }
            NativeScanScope::Diff { base, head } => {
                let scope_request = ScopeRequest::Diff {
                    base: base.clone(),
                    head: head.clone(),
                };
                let files = diff_files(repo_root, base, head, rules)?;
                (scope_request, files, ScanScope::Diff)
            }
        };
    let mut resolved = scope::resolve(&scope_request, repo_root)?;
    resolved.kind = kind;
    let files = filter_languages(files, &selection.languages);
    Ok((resolved, files))
}

fn filter_languages(files: Vec<RelPath>, languages: &[NativeScanLanguage]) -> Vec<RelPath> {
    if languages.is_empty() {
        return files;
    }
    files
        .into_iter()
        .filter(|path| {
            languages
                .iter()
                .copied()
                .any(|language| language.matches(path))
        })
        .collect()
}

fn find_crate_root(
    repo_root: &RepoRoot,
    files: &[RelPath],
    crate_name: &CrateName,
) -> Result<RelPath, NativeScanError> {
    for manifest in files
        .iter()
        .filter(|path| path.as_str().ends_with("Cargo.toml"))
    {
        let path = std::path::Path::new(repo_root.as_str()).join(manifest.as_str());
        let Ok(contents) = std::fs::read_to_string(path) else {
            continue;
        };
        if contents
            .lines()
            .any(|line| line.trim() == format!("name = \"{}\"", crate_name.as_str()))
        {
            let root = manifest.as_str().strip_suffix("/Cargo.toml").unwrap_or("");
            if root.is_empty() {
                return "Cargo.toml".parse().map_err(Into::into);
            }
            return root.parse().map_err(Into::into);
        }
    }
    Err(NativeScanError::UnsupportedCrate {
        name: crate_name.clone(),
    })
}

fn diff_files(
    repo_root: &RepoRoot,
    base: &CommitRef,
    head: &CommitRef,
    rules: &IgnoreRules,
) -> Result<Vec<RelPath>, NativeScanError> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_root.as_str())
        .args(["diff", "--name-only", "--diff-filter=ACMR"])
        .arg(base.as_str())
        .arg(head.as_str())
        .output()
        .map_err(|error| NativeScanError::GitDiff {
            base: base.clone(),
            head: head.clone(),
            reason: error.to_string(),
        })?;
    if !output.status.success() {
        return Err(NativeScanError::GitDiff {
            base: base.clone(),
            head: head.clone(),
            reason: String::from_utf8_lossy(&output.stderr).trim().to_owned(),
        });
    }
    let paths = String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::parse)
        .collect::<Result<Vec<RelPath>, DecodeError>>()?;
    Ok(walk::filter_explicit(&paths, rules))
}

fn staged_files(
    repo_root: &RepoRoot,
    rules: &IgnoreRules,
) -> Result<Vec<RelPath>, NativeScanError> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_root.as_str())
        .args([
            "diff",
            "--cached",
            "--name-only",
            "--diff-filter=ACMR",
            "-z",
        ])
        .output()
        .map_err(|error| NativeScanError::Io {
            operation: "staged file discovery",
            reason: error.to_string(),
        })?;
    if !output.status.success() {
        return Err(NativeScanError::Io {
            operation: "staged file discovery",
            reason: String::from_utf8_lossy(&output.stderr).trim().to_owned(),
        });
    }
    let paths = output
        .stdout
        .split(|byte| *byte == b'\0')
        .filter(|path| !path.is_empty())
        .map(|path| {
            std::str::from_utf8(path)
                .map_err(|error| DecodeError::new("staged path", error.to_string()))
        })
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .map(str::parse)
        .collect::<Result<Vec<RelPath>, DecodeError>>()?;
    Ok(walk::filter_explicit(&paths, rules))
}

#[cfg(test)]
mod tests {
    use super::{
        execute, execute_dependency_policy, execute_secret_policy, execute_staged_secret_policy,
        resolve_files, NativeScanError, NativeScanLanguage, NativeScanRequest, NativeScanScope,
    };
    use enforcer_domain::config_types::CrateName;
    use enforcer_domain::findings::ScanScope;
    use enforcer_domain::paths::RepoRoot;
    use std::path::Path;
    use std::process::Command;

    fn root(path: &Path) -> Result<RepoRoot, Box<dyn std::error::Error>> {
        Ok(path.to_string_lossy().parse()?)
    }

    fn write(
        root: &Path,
        relative: &str,
        contents: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let path = root.join(relative);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, contents)?;
        Ok(())
    }

    #[test]
    fn files_scope_applies_language_filter_before_engine_execution(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        write(temp.path(), "src/lib.rs", "pub fn native_contract() {}")?;
        write(
            temp.path(),
            "web/app.ts",
            "export const nativeContract = true;",
        )?;
        let request = NativeScanRequest {
            scope: NativeScanScope::Files(vec!["src/lib.rs".into(), "web/app.ts".into()]),
            languages: vec![NativeScanLanguage::Rust],
        };
        let (resolved, files) = resolve_files(&request, &root(temp.path())?)?;
        assert_eq!(resolved.kind, ScanScope::Files);
        assert_eq!(
            files.iter().map(|path| path.as_str()).collect::<Vec<_>>(),
            ["src/lib.rs"]
        );
        Ok(())
    }

    #[test]
    fn files_scope_expands_declared_directories_with_repo_relative_paths(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        write(temp.path(), "src/lib.rs", "pub fn native_contract() {}")?;
        write(temp.path(), "src/nested/mod.rs", "pub fn nested() {}")?;
        let request = NativeScanRequest {
            scope: NativeScanScope::Files(vec!["src".into()]),
            languages: vec![NativeScanLanguage::Rust],
        };
        let (resolved, files) = resolve_files(&request, &root(temp.path())?)?;
        assert_eq!(resolved.kind, ScanScope::Files);
        assert_eq!(
            files.iter().map(|path| path.as_str()).collect::<Vec<_>>(),
            ["src/lib.rs", "src/nested/mod.rs"]
        );
        Ok(())
    }

    #[test]
    fn workspace_and_crate_scopes_are_distinct_and_crate_is_package_name_based(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        write(
            temp.path(),
            "Cargo.toml",
            "[workspace]\nmembers = [\"nested/member\"]\n",
        )?;
        write(
            temp.path(),
            "nested/member/Cargo.toml",
            "[package]\nname = \"native-member\"\nversion = \"0.1.0\"\n",
        )?;
        write(
            temp.path(),
            "nested/member/src/lib.rs",
            "pub fn member() {}",
        )?;
        write(temp.path(), "other/src/lib.rs", "pub fn other() {}")?;
        let repo_root = root(temp.path())?;
        let workspace = NativeScanRequest {
            scope: NativeScanScope::Workspace,
            languages: Vec::new(),
        };
        let crate_request = NativeScanRequest {
            scope: NativeScanScope::Crate("native-member".parse::<CrateName>()?),
            languages: Vec::new(),
        };
        let (workspace_scope, workspace_files) = resolve_files(&workspace, &repo_root)?;
        let (crate_scope, crate_files) = resolve_files(&crate_request, &repo_root)?;
        assert_eq!(workspace_scope.kind, ScanScope::Workspace);
        assert_eq!(crate_scope.kind, ScanScope::Crate);
        assert!(workspace_files.len() > crate_files.len());
        assert_eq!(
            crate_files
                .iter()
                .map(|path| path.as_str())
                .collect::<Vec<_>>(),
            ["nested/member/Cargo.toml", "nested/member/src/lib.rs"]
        );
        Ok(())
    }

    #[test]
    fn diff_scope_uses_git_changed_files_and_execute_returns_report(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        assert!(Command::new("git")
            .arg("-C")
            .arg(temp.path())
            .arg("init")
            .status()?
            .success());
        assert!(Command::new("git")
            .arg("-C")
            .arg(temp.path())
            .args(["config", "user.email", "native@example.test"])
            .status()?
            .success());
        assert!(Command::new("git")
            .arg("-C")
            .arg(temp.path())
            .args(["config", "user.name", "Native Scan"])
            .status()?
            .success());
        write(temp.path(), "Cargo.toml", "[workspace]\nmembers = []\n")?;
        write(temp.path(), "src/old.rs", "pub fn old() {}")?;
        assert!(Command::new("git")
            .arg("-C")
            .arg(temp.path())
            .args(["add", "."])
            .status()?
            .success());
        assert!(Command::new("git")
            .arg("-C")
            .arg(temp.path())
            .args(["commit", "-m", "base"])
            .status()?
            .success());
        write(temp.path(), "src/new.rs", "pub fn new_file() {}")?;
        assert!(Command::new("git")
            .arg("-C")
            .arg(temp.path())
            .args(["add", "."])
            .status()?
            .success());
        assert!(Command::new("git")
            .arg("-C")
            .arg(temp.path())
            .args(["commit", "-m", "head"])
            .status()?
            .success());
        let request = NativeScanRequest {
            scope: NativeScanScope::Diff {
                base: "HEAD~1".parse()?,
                head: "HEAD".parse()?,
            },
            languages: vec![NativeScanLanguage::Rust],
        };
        let result = execute(&request, &root(temp.path())?)?;
        assert_eq!(result.scope, ScanScope::Diff);
        assert_eq!(
            result
                .scanned_files
                .iter()
                .map(|path| path.as_str())
                .collect::<Vec<_>>(),
            ["src/new.rs"]
        );
        assert_eq!(result.report.scope, ScanScope::Diff);
        Ok(())
    }

    #[test]
    fn empty_or_unknown_crate_scope_fails_closed() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let repo_root = root(temp.path())?;
        let empty = NativeScanRequest {
            scope: NativeScanScope::Files(Vec::new()),
            languages: Vec::new(),
        };
        assert!(matches!(
            resolve_files(&empty, &repo_root),
            Err(NativeScanError::Decode(_))
        ));
        let unknown = NativeScanRequest {
            scope: NativeScanScope::Crate("missing".parse()?),
            languages: Vec::new(),
        };
        assert!(matches!(
            resolve_files(&unknown, &repo_root),
            Err(NativeScanError::UnsupportedCrate { .. })
        ));
        Ok(())
    }

    #[test]
    fn dedicated_secret_policy_uses_the_sec_registry_without_full_scan_noise(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let github_token = ["ghp_", "abcdefghijklmnopqrstuvwxyz0123456789"].concat();
        write(
            temp.path(),
            "src/config.rs",
            &format!(
                "const SECRET = \"0123456789abcdefghijklmnop\";\nconst GH = \"{github_token}\";"
            ),
        )?;
        let request = NativeScanRequest {
            scope: NativeScanScope::Files(vec!["src/config.rs".into()]),
            languages: Vec::new(),
        };
        let result = execute_secret_policy(&request, &root(temp.path())?)?;
        assert!(result.report.violations.iter().all(|finding| {
            matches!(finding.finding().rule_id.as_str(), "SEC-1.1" | "SEC-1.2")
        }));
        assert!(result
            .report
            .violations
            .iter()
            .any(|finding| finding.finding().rule_id.as_str() == "SEC-1.1"));
        Ok(())
    }

    #[test]
    fn staged_secret_policy_scans_only_staged_paths() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        assert!(Command::new("git")
            .arg("init")
            .arg(temp.path())
            .status()?
            .success());
        write(
            temp.path(),
            "src/staged.rs",
            "const SECRET = \"0123456789abcdefghijklmnop\";",
        )?;
        write(
            temp.path(),
            "src/unstaged.rs",
            "const SECRET = \"abcdefghijklmnop0123456789\";",
        )?;
        assert!(Command::new("git")
            .arg("-C")
            .arg(temp.path())
            .args(["add", "src/staged.rs"])
            .status()?
            .success());
        let request = NativeScanRequest {
            scope: NativeScanScope::Workspace,
            languages: vec![NativeScanLanguage::Rust],
        };
        let result = execute_staged_secret_policy(&request, &root(temp.path())?)?;
        assert_eq!(result.scope, ScanScope::Files);
        assert_eq!(
            result
                .scanned_files
                .iter()
                .map(|path| path.as_str())
                .collect::<Vec<_>>(),
            ["src/staged.rs"]
        );
        assert!(result.report.violations.iter().all(|finding| {
            finding.finding().file.as_str() == "src/staged.rs"
                && finding.finding().rule_id.as_str().starts_with("SEC-")
        }));
        assert_eq!(result.report.violations.len(), 1);
        Ok(())
    }

    #[test]
    fn dedicated_dependency_policy_uses_workspace_inventory(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        write(
            temp.path(),
            "Cargo.toml",
            "[workspace]\nmembers = [\"crates/app\"]\n",
        )?;
        write(
            temp.path(),
            "crates/app/Cargo.toml",
            "[package]\nname = \"app\"\nversion = \"0.1.0\"\n\n[dependencies]\noutside = { path = \"../../outside\" }\n",
        )?;
        let request = NativeScanRequest {
            scope: NativeScanScope::Workspace,
            languages: Vec::new(),
        };
        let result = execute_dependency_policy(&request, &root(temp.path())?)?;
        assert_eq!(result.report.violations.len(), 1);
        assert_eq!(
            result.report.violations[0].finding().rule_id.as_str(),
            "RR-9.3"
        );
        Ok(())
    }
}
