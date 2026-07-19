//! The rayon-based parallel scan engine: walks a resolved scope, routes
//! each file to its language family's [`enforcer_validator::validator::
//! Validator`]s, and folds every family's findings into one
//! [`enforcer_domain::findings::Report`].
//!
//! Wired families (every landed `enforcer-lang-*` crate as of arc-15):
//! rust (arc-06, baseline validators only — no aggregate registry landed
//! yet), typescript (arc-07), python (arc-08), common/generic-scanner
//! (arc-09, security's `SEC-2` slice is that crate's own, common owns the
//! rest), security and cyberskills (arc-10/h11), iac (arc-11), k8s
//! (arc-12). The scored
//! literal scanner (arc-13) is NOT wired here: `enforcer-literal-scan`
//! emits its OWN `Finding`/`ScanReport` shape (pre-dating
//! `enforcer-domain`'s `Finding`), and the adapter that bridges the two
//! (`e01`, per `TEST_PROOF_EXPECTATIONS.md`'s arc-13 row: "this row proves
//! the folded scanner engine only, not the bridge") has not landed. Once
//! `e01`'s bridge exists, wiring it into [`run`] is a one-line addition —
//! this engine's fan-out/fold shape does not change.
//!
//! Parallelism boundary: the CPU/IO-bound phase — reading each file's
//! source text off disk — runs on `rayon`'s pool via `par_iter`. Trait
//! objects returned by the family registries (`Box<dyn Validator>`, not
//! `Send + Sync`-qualified by any registry this crate depends on) cannot
//! safely cross the `rayon` thread boundary without prohibited low-level
//! escape hatches, so validator dispatch itself
//! runs on the results of the parallel read phase, not inside it. This is
//! still the CPU-bound fan-out the workpack asks for on the actual
//! bottleneck (disk I/O across a large tree); it is not a fan-out over
//! validator CPU work per file.
//!
//! Determinism (the idempotency guard): [`crate::walk::walk`] already
//! returns files in sorted order; `par_iter().map(...).collect::<Vec<_>>()`
//! preserves input order regardless of which worker thread finishes a
//! given read first, so the parallel read phase's output order is
//! identical to a serial read's, and the subsequent sequential dispatch
//! produces byte-identical `Report`s across repeated runs.

use rayon::iter::ParallelIterator;
use rayon::prelude::IntoParallelRefIterator;
use std::num::NonZeroU32;

use enforcer_domain::boundary::validation::ValidationSourceText;
use enforcer_domain::config_types::InlineTestPolicy;
use enforcer_domain::findings::{
    Finding, FindingDetail, FindingLine, FindingSnippet, FindingTitle, Report, ReportOutcome,
    ScanScope, Violation,
};
use enforcer_domain::paths::{RelPath, RepoRoot};
use enforcer_domain::scan_types::{LanguageFamily, ResolvedScope};
use enforcer_domain::severity::Severity;
use enforcer_domain::telemetry_types::SourceLine;
use enforcer_validator::validator::{ValidationInput, Validator};

use crate::cargo_workspace_policy;
use crate::router::classify;

/// Every family's validators, grouped by [`LanguageFamily`] so [`run`]
/// only invokes the validators actually applicable to a given file.
/// Built once per scan (not per file) — family registries that can fail
/// (`Result<Vec<RegistryRow>, DecodeError>`) are resolved eagerly in
/// [`build_family_validators`], never lazily inside the per-file dispatch
/// loop.
pub struct FamilyValidators {
    rust: Vec<Box<dyn Validator>>,
    typescript: Vec<Box<dyn Validator>>,
    python: Vec<Box<dyn Validator>>,
    common: Vec<Box<dyn Validator>>,
    security: Vec<Box<dyn Validator>>,
    cyberskills: Vec<Box<dyn Validator>>,
    iac: Vec<Box<dyn Validator>>,
    k8s: Vec<Box<dyn Validator>>,
}

impl std::fmt::Debug for FamilyValidators {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("FamilyValidators")
            .field("rust", &self.rust.len())
            .field("typescript", &self.typescript.len())
            .field("python", &self.python.len())
            .field("common", &self.common.len())
            .field("security", &self.security.len())
            .field("cyberskills", &self.cyberskills.len())
            .field("iac", &self.iac.len())
            .field("k8s", &self.k8s.len())
            .finish()
    }
}

impl FamilyValidators {
    /// The validators applicable to a given [`LanguageFamily`]. `common`
    /// (arc-09's cross-language generic-scanner slice) and `security`
    /// (arc-10, `SEC-*`) and `cyberskills` (`CYBER-*`) run over every
    /// family, matching the legacy `.mjs`
    /// engine's posture that content-line rules are not extension-gated —
    /// only the AST/syntax-specific families (`rust`/`typescript`/
    /// `python`/`iac`/`k8s`) are extension-gated by [`LanguageFamily`].
    fn applicable(
        &self,
        family: LanguageFamily,
        file: &RelPath,
        scope: &ResolvedScope,
    ) -> Vec<&dyn Validator> {
        let mut out: Vec<&dyn Validator> = Vec::new();
        if !is_native_detector_authoring_surface(file, scope) {
            out.extend(self.common.iter().map(std::convert::AsRef::as_ref));
            out.extend(self.security.iter().map(std::convert::AsRef::as_ref));
            out.extend(self.cyberskills.iter().map(std::convert::AsRef::as_ref));
        }
        match family {
            LanguageFamily::Rust => {
                out.extend(self.rust.iter().map(std::convert::AsRef::as_ref));
            }
            LanguageFamily::TypeScript => {
                out.extend(self.typescript.iter().map(std::convert::AsRef::as_ref));
            }
            LanguageFamily::Python => {
                out.extend(self.python.iter().map(std::convert::AsRef::as_ref));
            }
            LanguageFamily::Terraform => {
                out.extend(self.iac.iter().map(std::convert::AsRef::as_ref));
            }
            LanguageFamily::YamlOrConfig => {
                out.extend(self.iac.iter().map(std::convert::AsRef::as_ref));
                out.extend(self.k8s.iter().map(std::convert::AsRef::as_ref));
            }
            LanguageFamily::Unknown => {}
        }
        out
    }
}

/// Native rule implementations and their signature catalogs are evidence
/// *about* dangerous text, rather than product code that executes it. Running
/// content-line detectors over these named authoring roots makes a detector
/// report its own regexes, IDs, and deliberately vulnerable corpus examples
/// as findings.
///
/// This is intentionally a small path-role table, not a broad language, test,
/// or scanner-source exclusion. An explicitly requested path always runs every
/// content detector, so fixture and rule-authoring validation remains possible
/// on demand. Language-specific structural validators always run.
fn is_native_detector_authoring_surface(file: &RelPath, scope: &ResolvedScope) -> bool {
    let path = file.as_str();
    if explicit_scope_includes_file(scope, file) {
        return false;
    }
    [
        "crates/enforcer-lang-common/src/families/",
        "crates/enforcer-lang-security/src/rules/",
        "crates/enforcer-lang-security/tests/",
        "crates/enforcer-rules/rules/",
        "crates/enforcer-security/src/rules/",
    ]
    .iter()
    .any(|root| path.starts_with(root))
        || matches!(
            path,
            "crates/enforcer-lang-py/src/source_scan.rs"
                | "crates/enforcer-lang-py/src/boundary/line_marker.rs"
        )
}

/// An explicit scan target can be either one file or a directory. The latter
/// must carry the same detector-dispatch guarantee to every contained file.
fn explicit_scope_includes_file(scope: &ResolvedScope, file: &RelPath) -> bool {
    scope.explicit_paths.iter().any(|requested| {
        requested == file
            || file
                .as_str()
                .strip_prefix(requested.as_str())
                .is_some_and(|suffix| suffix.starts_with('/'))
    })
}

/// Build the full [`FamilyValidators`] set from every landed
/// `enforcer-lang-*` crate's registry.
///
/// # Errors
/// Returns [`enforcer_domain::boundary::decode_error::DecodeError`] if any family registry
/// fails to build (a malformed rule spec, a duplicate rule id) — fails
/// closed rather than silently running with a partial registry.
pub fn build_family_validators(
) -> Result<FamilyValidators, enforcer_domain::boundary::decode_error::DecodeError> {
    // `enforcer-lang-rust` (arc-06) has not landed an aggregate registry
    // yet (only the two hosted baseline validators exist as concrete
    // modules) — wire them directly; a later arc-06 feature pack adding
    // `all_validators()` collapses this to one call, matching the
    // `enforcer-lang-py` shape.
    let rust: Vec<Box<dyn Validator>> = vec![
        Box::new(enforcer_lang_rust::rules::no_reexports::NoReexportsValidator::new()?),
        Box::new(enforcer_lang_rust::rules::error_handling::ErrorHandlingValidator::new()?),
    ];

    let typescript: Vec<Box<dyn Validator>> = enforcer_lang_ts::rules::registry::build_all()?
        .into_iter()
        .map(|row| row.validator)
        .collect();

    let python: Vec<Box<dyn Validator>> = enforcer_lang_py::all_validators()?;

    let common: Vec<Box<dyn Validator>> = enforcer_lang_common::registry::all(
        enforcer_lang_common::port_platform::DeclaredScope::Undeclared,
    );

    let security: Vec<Box<dyn Validator>> = enforcer_lang_security::rules::registry::build_all()?
        .into_iter()
        .map(|row| row.validator)
        .collect();

    let cyberskills: Vec<Box<dyn Validator>> =
        enforcer_lang_security::rules::cyberskills::registry::build_all()?
            .into_iter()
            .map(|row| row.validator)
            .collect();

    let iac: Vec<Box<dyn Validator>> = enforcer_lang_iac::rules::registry::build_all()?
        .into_iter()
        .map(|row| row.validator)
        .collect();

    let k8s: Vec<Box<dyn Validator>> = enforcer_lang_k8s::rules::registry::build_all()?
        .into_iter()
        .map(|row| row.validator)
        .collect();

    Ok(FamilyValidators {
        rust,
        typescript,
        python,
        common,
        security,
        cyberskills,
        iac,
        k8s,
    })
}

fn read_file_utf8(root: &RepoRoot, rel: &RelPath) -> Option<ValidationSourceText> {
    let path = std::path::Path::new(root.as_str()).join(rel.as_str());
    let Ok(source) = std::fs::read_to_string(path) else {
        return None;
    };
    Some(ValidationSourceText::try_new(source))
}

/// Run the engine over an already-resolved, already-walked file set.
///
/// The read phase fans out across `rayon`'s pool; the classify+validate
/// phase is sequential over the (already order-preserved) read results.
/// This keeps the result deterministic (parallel read == serial read,
/// byte-identical `Report` across repeated runs) without requiring
/// `Box<dyn Validator>` to cross the `rayon` thread boundary.
pub fn run(scope: &ResolvedScope, files: &[RelPath], validators: &FamilyValidators) -> Report {
    run_with_inline_test_policy(scope, files, validators, InlineTestPolicy::Forbid)
}

/// Run the engine with the resolved policy for non-Rust inline tests declared
/// in production modules. Rust's `#[cfg(test)]` modules are idiomatic unit
/// tests and are exempt from `TEST-2.2`; [`InlineTestPolicy::Forbid`] makes a
/// TypeScript/JavaScript/Python placement blocking, [`InlineTestPolicy::Warn`]
/// keeps it visible without failing the scan, and [`InlineTestPolicy::Allow`]
/// intentionally emits nothing.
pub fn run_with_inline_test_policy(
    scope: &ResolvedScope,
    files: &[RelPath],
    validators: &FamilyValidators,
    inline_test_policy: InlineTestPolicy,
) -> Report {
    let mut sources: Vec<(RelPath, Option<ValidationSourceText>)> = files
        .par_iter()
        // CLONE-JUSTIFICATION: the parallel read phase must retain each
        // relative path alongside its independently read source text.
        .map(|file| (file.clone(), read_file_utf8(&scope.repo_root, file)))
        .collect();
    sources.retain(|(file, _)| should_scan_source(scope, file));

    if let Ok(workspace_manifest) = RelPath::try_new("Cargo.toml") {
        if !sources.iter().any(|(file, _)| file == &workspace_manifest) {
            sources.push((
                workspace_manifest.clone(),
                read_file_utf8(&scope.repo_root, &workspace_manifest),
            ));
        }
    }

    let mut all_findings = Vec::new();
    for (file, source) in &sources {
        let Some(source) = source else { continue };
        let family = classify(file);
        let mut per_file = Vec::new();
        for validator in validators.applicable(family, file, scope) {
            let input = ValidationInput {
                file,
                source: source.as_source(),
                scope: scope.kind,
            };
            per_file.extend(validator.validate(input));
        }
        per_file.sort_by(|a, b| (&a.file, a.line, &a.rule_id).cmp(&(&b.file, b.line, &b.rule_id)));
        all_findings.extend(per_file);
    }

    let workspace_inventory = workspace_manifest_inventory(&scope.repo_root);
    all_findings.extend(cargo_workspace_policy::findings_for_sources_with_inventory(
        &sources,
        &workspace_inventory,
    ));
    all_findings.extend(inline_test_findings(&sources, inline_test_policy));

    fold_report(scope.kind, all_findings)
}

fn should_scan_source(scope: &ResolvedScope, file: &RelPath) -> bool {
    let path = file.as_str();
    let is_fixture = path.starts_with("tests/fixtures/") || path.contains("/tests/fixtures/");
    !is_fixture
        || scope
            .explicit_paths
            .iter()
            .any(|requested| requested == file)
}

fn workspace_manifest_inventory(root: &RepoRoot) -> Vec<(RelPath, Option<ValidationSourceText>)> {
    let root_manifest: RelPath = match "Cargo.toml".parse() {
        Ok(path) => path,
        Err(_) => return Vec::new(),
    };
    let Some(root_source) = read_file_utf8(root, &root_manifest) else {
        return Vec::new();
    };
    let mut manifests = workspace_member_entries(root_source.as_source().as_str())
        .into_iter()
        .flat_map(|entry| expand_workspace_member(root, &entry))
        .collect::<Vec<_>>();
    manifests.sort();
    manifests.dedup();
    manifests
        .into_iter()
        .map(|manifest| {
            let source = read_file_utf8(root, &manifest);
            (manifest, source)
        })
        .collect()
}

fn workspace_member_entries(source: &str) -> Vec<String> {
    let mut in_workspace = false;
    let mut collecting_members = false;
    let mut members_source = String::new();
    for line in source.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            in_workspace = trimmed == "[workspace]";
            collecting_members = false;
            continue;
        }
        if !in_workspace {
            continue;
        }
        if collecting_members {
            members_source.push_str(trimmed);
            if trimmed.contains(']') {
                break;
            }
            continue;
        }
        let Some((key, value)) = trimmed.split_once('=') else {
            continue;
        };
        if key.trim() != "members" {
            continue;
        }
        members_source.push_str(value.trim());
        collecting_members = !value.contains(']');
        if !collecting_members {
            break;
        }
    }
    members_source
        .split('"')
        .skip(1)
        .step_by(2)
        .filter(|entry| !entry.is_empty())
        .map(str::to_owned)
        .collect()
}

fn expand_workspace_member(root: &RepoRoot, entry: &str) -> Vec<RelPath> {
    let entry = entry.replace('\\', "/");
    let Some((prefix, suffix)) = entry.split_once('*') else {
        return member_manifest_path(&entry).into_iter().collect();
    };
    if suffix.contains('*') {
        return Vec::new();
    }
    let directory = std::path::Path::new(root.as_str()).join(prefix.trim_end_matches('/'));
    let Ok(children) = std::fs::read_dir(directory) else {
        return Vec::new();
    };
    children
        .filter_map(Result::ok)
        .filter(|child| child.file_type().is_ok_and(|kind| kind.is_dir()))
        .filter_map(|child| {
            let name = child.file_name();
            let name = name.to_str()?;
            member_manifest_path(&format!("{prefix}{name}{suffix}"))
        })
        .collect()
}

fn member_manifest_path(member: &str) -> Option<RelPath> {
    let member = member.trim_matches('/');
    let manifest = if member.is_empty() {
        "Cargo.toml".to_owned()
    } else {
        format!("{member}/Cargo.toml")
    };
    manifest.parse().ok()
}

fn inline_test_findings(
    sources: &[(RelPath, Option<ValidationSourceText>)],
    policy: InlineTestPolicy,
) -> Vec<Finding> {
    let severity = match policy {
        InlineTestPolicy::Forbid => Severity::Error,
        InlineTestPolicy::Warn => Severity::Warning,
        InlineTestPolicy::Allow => return Vec::new(),
    };
    let rule_id: enforcer_domain::ids::RuleId = match "TEST-2.2".parse() {
        Ok(rule_id) => rule_id,
        Err(_) => return Vec::new(),
    };
    let mut findings = Vec::new();
    for (file, source) in sources {
        if !is_inline_test_candidate(file) {
            continue;
        }
        let Some(source) = source else { continue };
        let source = source.as_source().as_str();
        let Some((line_index, line)) = source
            .lines()
            .enumerate()
            .find(|(_, line)| is_inline_test_marker(file, line))
        else {
            continue;
        };
        let Ok(line_number) = u32::try_from(line_index + 1) else {
            continue;
        };
        let Some(line_number) = NonZeroU32::new(line_number) else {
            continue;
        };
        let source_line = SourceLine::try_new(line_number);
        findings.push(Finding {
            // CLONE-JUSTIFICATION: each finding owns its rule id after this
            // borrowed source inventory is released.
            rule_id: rule_id.clone(),
            severity,
            // ALLOC-JUSTIFICATION: finding titles are owned wire values and
            // this static wording is copied per independent finding.
            title: match FindingTitle::new("inline test in production source".to_owned()) {
                Ok(title) => title,
                Err(_) => continue,
            },
            // ALLOC-JUSTIFICATION: each detail is an independent owned
            // remediation message carried by the finding.
            detail: match FindingDetail::new("move this test into the crate's tests/ directory, or configure inlineTestPolicy to warn or allow deliberately".to_owned()) {
                Ok(detail) => detail,
                Err(_) => continue,
            },
            // CLONE-JUSTIFICATION: findings outlive the borrowed source list
            // and therefore own their canonical path.
            file: file.clone(),
            line: FindingLine::known(source_line),
            snippet: snippet_from_line(line),
        });
    }
    findings
}

fn snippet_from_line(line: &str) -> Option<FindingSnippet> {
    // ALLOC-JUSTIFICATION: snippets are owned report fields and must outlive
    // the source line borrowed from the scan buffer.
    let Ok(snippet) = FindingSnippet::new(line.to_owned()) else {
        return None;
    };
    Some(snippet)
}

fn is_inline_test_candidate(file: &RelPath) -> bool {
    let source_path = file.as_str();
    !source_path.split('/').any(|segment| segment == "tests")
        && !source_path.ends_with("_test.rs")
        && !source_path.ends_with("_tests.rs")
        && (source_path.ends_with(".ts")
            || source_path.ends_with(".tsx")
            || source_path.ends_with(".js")
            || source_path.ends_with(".jsx")
            || source_path.ends_with(".py"))
}

fn is_inline_test_marker(file: &RelPath, line: &str) -> bool {
    let trimmed = line.trim_start();
    if file.as_str().ends_with(".py") {
        return trimmed.starts_with("def test_") || trimmed.starts_with("async def test_");
    }
    ["describe(", "it(", "test("]
        .iter()
        .any(|marker| trimmed.starts_with(marker))
}

/// Fold a flat findings stream into a [`Report`]: partitions into
/// `violations` (severity `error`) vs `warnings` (everything else),
/// leaves `waived` empty (waiver application is a config-boundary concern
/// this skeleton does not own), and sets `ok` false iff any violation
/// exists.
fn fold_report(scope: ScanScope, findings: impl IntoIterator<Item = Finding>) -> Report {
    let mut all_findings = Vec::new();
    let mut violations = Vec::new();
    let mut warnings = Vec::new();

    for finding in findings {
        // CLONE-JUSTIFICATION: the report retains every finding while the
        // severity partition owns its corresponding violation or warning.
        all_findings.push(finding.clone());
        match Violation::try_from(finding.clone()) {
            Ok(violation) => violations.push(violation),
            Err(_) => warnings.push(finding),
        }
    }

    Report {
        ok: if violations.is_empty() {
            ReportOutcome::Clean
        } else {
            ReportOutcome::Violations
        },
        scope,
        violations,
        warnings,
        waived: Vec::new(),
        findings: all_findings,
    }
}

#[cfg(test)]
mod tests {
    use super::{build_family_validators, fold_report, run};
    use enforcer_domain::findings::{Finding, FindingLine, ReportOutcome, ScanScope};
    use enforcer_domain::paths::RepoRoot;
    use enforcer_domain::scan_types::ScopeRequest;
    use enforcer_domain::severity::Severity;
    use enforcer_domain::telemetry_types::SourceLine;

    use crate::scope::resolve;
    use crate::walk::{walk, IgnoreRules};

    fn write_file(root: &std::path::Path, rel: &str, contents: &str) -> std::io::Result<()> {
        let path = root.join(rel);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, contents)
    }

    #[test]
    fn family_validators_build_cleanly() -> Result<(), Box<dyn std::error::Error>> {
        let validators = build_family_validators()?;
        assert!(!validators.rust.is_empty());
        for expected in [
            "CYBER-FILELESS-MALWARE.1",
            "CYBER-FILELESS-TELEMETRY.1",
            "CYBER-FILELESS-REPORT.1",
        ] {
            assert!(
                validators
                    .cyberskills
                    .iter()
                    .any(|validator| validator.rule_id().as_str() == expected),
                "production scan registry is missing {expected}"
            );
        }
        Ok(())
    }

    #[test]
    fn fail_fixture_trips_a_finding() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        write_file(
            temp.path(),
            "src/lib.rs",
            "fn bad() { let x: Option<i32> = None; x.unwrap(); }",
        )?;
        let root: RepoRoot = temp.path().to_string_lossy().parse()?;
        let resolved = resolve(&ScopeRequest::All, &root)?;
        let files = walk(temp.path(), &IgnoreRules::default())?;
        let validators = build_family_validators()?;
        let report = run(&resolved, &files, &validators);
        assert!(
            report.ok == ReportOutcome::Violations,
            "unwrap() in first-party code must trip a violation"
        );
        assert!(report
            .findings
            .iter()
            .any(|f| f.file.as_str() == "src/lib.rs"));
        Ok(())
    }

    #[test]
    fn workspace_scan_rejects_an_external_cargo_path_dependency(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        write_file(
            temp.path(),
            "Cargo.toml",
            "[workspace]\nmembers = [\"crates/core\", \"crates/app\"]\n\n[workspace.dependencies]\ncore = { path = \"crates/core\" }\n",
        )?;
        write_file(
            temp.path(),
            "crates/core/Cargo.toml",
            "[package]\nname = \"core\"\nversion = \"0.1.0\"\n",
        )?;
        write_file(
            temp.path(),
            "crates/app/Cargo.toml",
            "[package]\nname = \"app\"\nversion = \"0.1.0\"\n\n[dependencies]\ncore = { path = \"../core\" }\noutside = { path = \"../outside\" }\n",
        )?;
        let root: RepoRoot = temp.path().to_string_lossy().parse()?;
        let resolved = resolve(&ScopeRequest::All, &root)?;
        let files = walk(temp.path(), &IgnoreRules::default())?;
        let validators = build_family_validators()?;
        let report = run(&resolved, &files, &validators);
        assert!(report.findings.iter().any(|finding| {
            finding.rule_id.as_str() == "RR-9.3" && finding.file.as_str() == "crates/app/Cargo.toml"
        }));
        Ok(())
    }

    #[test]
    fn scoped_scan_recognizes_unlisted_workspace_member_dependencies(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        write_file(
            temp.path(),
            "Cargo.toml",
            "[workspace]\nmembers = [\"crates/*\"]\n",
        )?;
        write_file(
            temp.path(),
            "crates/core/Cargo.toml",
            "[package]\nname = \"core\"\nversion = \"0.1.0\"\n",
        )?;
        write_file(
            temp.path(),
            "crates/app/Cargo.toml",
            "[package]\nname = \"app\"\nversion = \"0.1.0\"\n\n[dependencies]\ncore = { path = \"../core\" }\noutside = { path = \"../outside\" }\n",
        )?;
        let root: RepoRoot = temp.path().to_string_lossy().parse()?;
        let resolved = resolve(&ScopeRequest::Paths(vec!["crates/app".into()]), &root)?;
        let files = vec!["crates/app/Cargo.toml".parse()?];
        let validators = build_family_validators()?;
        let report = run(&resolved, &files, &validators);
        let cargo_findings = report
            .findings
            .iter()
            .filter(|finding| finding.rule_id.as_str() == "RR-9.3")
            .collect::<Vec<_>>();
        assert_eq!(cargo_findings.len(), 1);
        assert_eq!(cargo_findings[0].file.as_str(), "crates/app/Cargo.toml");
        assert!(cargo_findings[0]
            .snippet
            .as_ref()
            .is_some_and(|snippet| snippet.as_str().contains("outside")));
        Ok(())
    }

    #[test]
    fn directory_scope_excludes_seeded_test_fixtures() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        write_file(
            temp.path(),
            "crates/example/tests/fixtures/fail.rs",
            "fn seeded_failure() { let value: Option<i32> = None; value.unwrap(); }",
        )?;
        let root: RepoRoot = temp.path().to_string_lossy().parse()?;
        let resolved = resolve(&ScopeRequest::Paths(vec!["crates/example".into()]), &root)?;
        let files = walk(temp.path(), &IgnoreRules::default())?;
        let validators = build_family_validators()?;
        let report = run(&resolved, &files, &validators);
        assert!(report
            .findings
            .iter()
            .all(|finding| { finding.file.as_str() != "crates/example/tests/fixtures/fail.rs" }));
        Ok(())
    }

    #[test]
    fn exact_fixture_scope_preserves_fixture_validation() -> Result<(), Box<dyn std::error::Error>>
    {
        let temp = tempfile::tempdir()?;
        let fixture = "crates/example/tests/fixtures/fail.rs";
        write_file(
            temp.path(),
            fixture,
            "fn seeded_failure() { let value: Option<i32> = None; value.unwrap(); }",
        )?;
        let root: RepoRoot = temp.path().to_string_lossy().parse()?;
        let resolved = resolve(&ScopeRequest::Paths(vec![fixture.into()]), &root)?;
        let files = walk(temp.path(), &IgnoreRules::default())?;
        let validators = build_family_validators()?;
        let report = run(&resolved, &files, &validators);
        assert!(report.findings.iter().any(|finding| {
            finding.file.as_str() == fixture && finding.severity == Severity::Error
        }));
        Ok(())
    }

    #[test]
    fn pass_fixture_produces_empty_report() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        write_file(temp.path(), "src/lib.rs", "fn good() -> i32 { 42 }")?;
        let root: RepoRoot = temp.path().to_string_lossy().parse()?;
        let resolved = resolve(&ScopeRequest::All, &root)?;
        let files = walk(temp.path(), &IgnoreRules::default())?;
        let validators = build_family_validators()?;
        let report = run(&resolved, &files, &validators);
        assert_eq!(
            report.ok,
            ReportOutcome::Clean,
            "clean tree must produce an empty (ok) report"
        );
        assert!(report.violations.is_empty());
        Ok(())
    }

    #[test]
    fn repeated_runs_are_byte_identical() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        write_file(
            temp.path(),
            "src/a.rs",
            "fn a() { let x: Option<i32> = None; x.unwrap(); }",
        )?;
        write_file(temp.path(), "src/b.rs", "fn b() -> i32 { 1 }")?;
        write_file(temp.path(), "src/c.rs", "fn c() { panic!(\"no\"); }")?;
        let root: RepoRoot = temp.path().to_string_lossy().parse()?;
        let resolved = resolve(&ScopeRequest::All, &root)?;
        let files = walk(temp.path(), &IgnoreRules::default())?;
        let validators = build_family_validators()?;

        let first = run(&resolved, &files, &validators);
        let second = run(&resolved, &files, &validators);
        assert_eq!(
            first, second,
            "two runs over the same scope must be byte-identical"
        );
        Ok(())
    }

    #[test]
    fn fold_report_partitions_by_severity() -> Result<(), Box<dyn std::error::Error>> {
        let error_finding = Finding {
            rule_id: "RR-6.1".parse()?,
            severity: Severity::Error,
            title: "t".parse()?,
            detail: "d".parse()?,
            file: "src/lib.rs".parse()?,
            line: FindingLine::known(SourceLine::try_new(std::num::NonZeroU32::MIN)),
            snippet: None,
        };
        let warning_finding = Finding {
            severity: Severity::Warning,
            // CLONE-JUSTIFICATION: the error case is reused unchanged except
            // for severity to prove partitioning preserves report fields.
            ..error_finding.clone()
        };
        let report = fold_report(ScanScope::Files, vec![error_finding, warning_finding]);
        assert_eq!(report.violations.len(), 1);
        assert_eq!(report.warnings.len(), 1);
        assert_eq!(report.ok, ReportOutcome::Violations);
        Ok(())
    }
}
