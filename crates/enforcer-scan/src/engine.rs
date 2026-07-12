//! The rayon-based parallel scan engine: walks a resolved scope, routes
//! each file to its language family's [`enforcer_validator::validator::
//! Validator`]s, and folds every family's findings into one
//! [`enforcer_domain::findings::Report`].
//!
//! Wired families (every landed `enforcer-lang-*` crate as of arc-15):
//! rust (arc-06, baseline validators only — no aggregate registry landed
//! yet), typescript (arc-07), python (arc-08), common/generic-scanner
//! (arc-09, security's `SEC-2` slice is that crate's own, common owns the
//! rest), security (arc-10), iac (arc-11), k8s (arc-12). The scored
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

use enforcer_domain::findings::{Finding, Report, ScanScope, Violation};
use enforcer_domain::paths::{RelPath, RepoRoot};
use enforcer_validator::validator::{ValidationInput, Validator};

use crate::cargo_workspace_policy;
use crate::router::{classify, LanguageFamily};
use crate::scope::ResolvedScope;

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
    iac: Vec<Box<dyn Validator>>,
    k8s: Vec<Box<dyn Validator>>,
}

impl FamilyValidators {
    /// The validators applicable to a given [`LanguageFamily`]. `common`
    /// (arc-09's cross-language generic-scanner slice) and `security`
    /// (arc-10, `SEC-*`) run over every family, matching the legacy `.mjs`
    /// engine's posture that content-line rules are not extension-gated —
    /// only the AST/syntax-specific families (`rust`/`typescript`/
    /// `python`/`iac`/`k8s`) are extension-gated by [`LanguageFamily`].
    fn applicable(&self, family: LanguageFamily) -> Vec<&dyn Validator> {
        let mut out: Vec<&dyn Validator> = Vec::new();
        out.extend(self.common.iter().map(std::convert::AsRef::as_ref));
        out.extend(self.security.iter().map(std::convert::AsRef::as_ref));
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

/// Build the full [`FamilyValidators`] set from every landed
/// `enforcer-lang-*` crate's registry.
///
/// # Errors
/// Returns [`enforcer_core::error::DecodeError`] if any family registry
/// fails to build (a malformed rule spec, a duplicate rule id) — fails
/// closed rather than silently running with a partial registry.
pub fn build_family_validators() -> Result<FamilyValidators, enforcer_core::error::DecodeError> {
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
        iac,
        k8s,
    })
}

fn read_file_utf8(root: &RepoRoot, rel: &RelPath) -> Option<String> {
    let path = std::path::Path::new(root.as_str()).join(rel.as_str());
    std::fs::read_to_string(path).ok()
}

/// Run the engine over an already-resolved, already-walked file set.
///
/// The read phase fans out across `rayon`'s pool; the classify+validate
/// phase is sequential over the (already order-preserved) read results.
/// This keeps the result deterministic (parallel read == serial read,
/// byte-identical `Report` across repeated runs) without requiring
/// `Box<dyn Validator>` to cross the `rayon` thread boundary.
pub fn run(scope: &ResolvedScope, files: &[RelPath], validators: &FamilyValidators) -> Report {
    let sources: Vec<(RelPath, Option<String>)> = files
        .par_iter()
        // CLONE-JUSTIFICATION: the parallel read phase must retain each
        // relative path alongside its independently read source text.
        .map(|file| (file.clone(), read_file_utf8(&scope.repo_root, file)))
        .collect();

    let mut all_findings = Vec::new();
    for (file, source) in &sources {
        let Some(source) = source else { continue };
        let family = classify(&file);
        let mut per_file = Vec::new();
        for validator in validators.applicable(family) {
            let input = ValidationInput {
                file,
                source,
                scope: scope.kind,
            };
            per_file.extend(validator.validate(input));
        }
        per_file.sort_by(|a, b| (&a.file, a.line, &a.rule_id).cmp(&(&b.file, b.line, &b.rule_id)));
        all_findings.extend(per_file);
    }

    all_findings.extend(cargo_workspace_policy::findings_for_sources(&sources));

    fold_report(scope.kind, all_findings)
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
        ok: violations.is_empty(),
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
    use enforcer_domain::findings::{Finding, ScanScope};
    use enforcer_domain::paths::RepoRoot;
    use enforcer_domain::severity::Severity;

    use crate::router::LanguageFamily;
    use crate::scope::{resolve, ScopeRequest};
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
        assert!(!validators.applicable(LanguageFamily::Rust).is_empty());
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
            !report.ok,
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
    fn pass_fixture_produces_empty_report() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        write_file(temp.path(), "src/lib.rs", "fn good() -> i32 { 42 }")?;
        let root: RepoRoot = temp.path().to_string_lossy().parse()?;
        let resolved = resolve(&ScopeRequest::All, &root)?;
        let files = walk(temp.path(), &IgnoreRules::default())?;
        let validators = build_family_validators()?;
        let report = run(&resolved, &files, &validators);
        assert!(report.ok, "clean tree must produce an empty (ok) report");
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
            // ALLOC-JUSTIFICATION: this test constructs owned report fields
            // to verify report partitioning independent of source lifetime.
            title: "t".to_owned(),
            detail: "d".to_owned(),
            file: "src/lib.rs".parse()?,
            line: 1,
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
        assert!(!report.ok);
        Ok(())
    }
}
