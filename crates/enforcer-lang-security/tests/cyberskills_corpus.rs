//! Pen-test-grade labeled corpus for the h11 cyberskills source-pattern
//! families. Each `_corpus/<family>.json` holds many minimal inputs labeled
//! by VENDOR behavior: `flag` (a real misconfig/attack the family must
//! detect — no false negatives) or `clean` (a benign near-miss the family
//! must NOT flag — no false positives). Every case runs against the whole
//! family (union of its validators); a `flag` case must yield >=1 finding,
//! a `clean` case must yield 0. This is the detection/prevention proof: it
//! exercises hundreds of variants, not one happy-path fixture pair.
//!
//! The corpus was generated from the vendored skills and then RECONCILED:
//! every case was run against the validators and each mismatch adjudicated
//! against the vendor source (a mislabeled case was corrected to the vendor
//! verdict; a real detection gap was fixed in the rule). Cases for vendor
//! checks outside h11's named thin-slice (e.g. the additional Azure
//! encryption/network checks) were removed and are tracked as follow-ups.

use std::path::PathBuf;

use enforcer_domain::findings::ScanScope;
use enforcer_domain::paths::RelPath;
use enforcer_lang_security::rules::cyberskills::cloud_azure::{
    AzureStorageMinTls12Validator, AzureStoragePublicBlobValidator,
    AzureStorageRequireHttpsValidator,
};
use enforcer_lang_security::rules::cyberskills::dependency_confusion::DependencyConfusionClaimableValidator;
use enforcer_lang_security::rules::cyberskills::dockerfile_hardening::DockerfileHardeningValidator;
use enforcer_lang_security::rules::cyberskills::iac_terraform::{
    IamNoWildcardActionValidator, S3EncryptionRequiredValidator, SgNoPublicSshIngressValidator,
};
use enforcer_lang_security::rules::cyberskills::k8s_pod_security::K8sPodSecurityValidator;
use enforcer_lang_security::rules::cyberskills::waf_sqli::WafSqliSignatureValidator;
use enforcer_lang_security::rules::cyberskills::web_headers::{
    CookieSecureHttponlySamesiteValidator, CspMissingValidator, HstsMissingOrWeakValidator,
};
use enforcer_validator::validator::{ValidationInput, Validator};

#[derive(serde::Deserialize)]
struct Case {
    name: String,
    input: String,
    expect: String,
    #[serde(default)]
    branch: String,
    #[serde(default)]
    reason: String,
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn load(corpus_file: &str) -> Result<Vec<Case>, Box<dyn std::error::Error>> {
    let path = manifest_dir()
        .join("tests/fixtures/cyberskills/_corpus")
        .join(corpus_file);
    let raw = std::fs::read_to_string(&path)
        .map_err(|e| format!("cannot read corpus {}: {e}", path.display()))?;
    Ok(serde_json::from_str(&raw)?)
}

/// Run every case in `corpus_file` against the whole `family` (union of
/// validators) and assert the flag/clean label holds. All mismatches are
/// collected and reported together so reconciliation sees the full set.
fn assert_family(
    corpus_file: &str,
    file_name: &str,
    family: &[Box<dyn Validator>],
) -> Result<(), Box<dyn std::error::Error>> {
    let cases = load(corpus_file)?;
    assert!(!cases.is_empty(), "empty corpus {corpus_file}");
    let file: RelPath = file_name.parse()?;

    let mut mismatches = Vec::new();
    for case in &cases {
        let findings: usize = family
            .iter()
            .map(|validator| {
                validator
                    .validate(ValidationInput {
                        file: &file,
                        source: &case.input,
                        scope: ScanScope::Files,
                    })
                    .len()
            })
            .sum();
        let flagged = findings > 0;
        let want_flag = match case.expect.as_str() {
            "flag" => true,
            "clean" => false,
            other => return Err(format!("case `{}`: bad expect `{other}`", case.name).into()),
        };
        if flagged != want_flag {
            mismatches.push(format!(
                "  [{}] expected {} but got {} findings ({}). reason: {}",
                case.name,
                case.expect,
                findings,
                if flagged { "flagged" } else { "clean" },
                case.reason
            ));
        }
    }

    assert!(
        mismatches.is_empty(),
        "{corpus_file}: {} of {} cases mismatched:\n{}",
        mismatches.len(),
        cases.len(),
        mismatches.join("\n")
    );
    Ok(())
}

#[test]
fn corpus_iac_terraform() -> Result<(), Box<dyn std::error::Error>> {
    let family: Vec<Box<dyn Validator>> = vec![
        Box::new(S3EncryptionRequiredValidator::new()?),
        Box::new(IamNoWildcardActionValidator::new()?),
        Box::new(SgNoPublicSshIngressValidator::new()?),
    ];
    assert_family("iac_terraform.json", "main.tf", &family)
}

#[test]
fn corpus_cloud_azure() -> Result<(), Box<dyn std::error::Error>> {
    let family: Vec<Box<dyn Validator>> = vec![
        Box::new(AzureStoragePublicBlobValidator::new()?),
        Box::new(AzureStorageRequireHttpsValidator::new()?),
        Box::new(AzureStorageMinTls12Validator::new()?),
    ];
    assert_family("cloud_azure.json", "account.json", &family)
}

/// Headers is the one family whose three checks each evaluate the SAME
/// response independently, so a fixture crafted to isolate one header
/// legitimately omits the others (which would otherwise trip their
/// "missing" branch). Each case is therefore routed to the ONE validator
/// its name/branch names, not the union.
#[test]
fn corpus_web_headers() -> Result<(), Box<dyn std::error::Error>> {
    let hsts = HstsMissingOrWeakValidator::new()?;
    let csp = CspMissingValidator::new()?;
    let cookie = CookieSecureHttponlySamesiteValidator::new()?;

    let cases = load("web_headers.json")?;
    assert!(!cases.is_empty(), "empty web_headers corpus");
    let file: RelPath = "response.json".parse()?;

    let mut mismatches = Vec::new();
    for case in &cases {
        let tag = format!("{} {}", case.name, case.branch).to_ascii_lowercase();
        // A case naming one header is routed to that validator; a combined
        // response case (all headers present) runs the whole family.
        // Route by which header CATEGORY the case names (samesite belongs
        // to the cookie category, not a separate one). A case naming one
        // category runs only that validator — its fixture legitimately omits
        // the other headers; a "combined" case (whole response) runs the
        // full family.
        let is_hsts = tag.contains("hsts");
        let is_csp = tag.contains("csp");
        let is_cookie = tag.contains("cookie") || tag.contains("samesite");
        let categories = [is_hsts, is_csp, is_cookie].iter().filter(|b| **b).count();
        let selected: Vec<&dyn Validator> = if tag.contains("combined") || categories != 1 {
            vec![&hsts, &csp, &cookie]
        } else if is_hsts {
            vec![&hsts]
        } else if is_csp {
            vec![&csp]
        } else {
            vec![&cookie]
        };
        let findings: usize = selected
            .iter()
            .map(|validator| {
                validator
                    .validate(ValidationInput {
                        file: &file,
                        source: &case.input,
                        scope: ScanScope::Files,
                    })
                    .len()
            })
            .sum();
        let flagged = findings > 0;
        let want_flag = match case.expect.as_str() {
            "flag" => true,
            "clean" => false,
            other => return Err(format!("case `{}`: bad expect `{other}`", case.name).into()),
        };
        if flagged != want_flag {
            mismatches.push(format!(
                "  [{}] expected {} but got {} findings ({}). reason: {}",
                case.name,
                case.expect,
                findings,
                if flagged { "flagged" } else { "clean" },
                case.reason
            ));
        }
    }
    assert!(
        mismatches.is_empty(),
        "web_headers.json: {} of {} cases mismatched:\n{}",
        mismatches.len(),
        cases.len(),
        mismatches.join("\n")
    );
    Ok(())
}

#[test]
fn corpus_dependency_confusion() -> Result<(), Box<dyn std::error::Error>> {
    let family: Vec<Box<dyn Validator>> =
        vec![Box::new(DependencyConfusionClaimableValidator::new()?)];
    assert_family("dependency_confusion.json", "package.json", &family)
}

#[test]
fn corpus_waf_sqli() -> Result<(), Box<dyn std::error::Error>> {
    let family: Vec<Box<dyn Validator>> = vec![Box::new(WafSqliSignatureValidator::new()?)];
    assert_family("waf_sqli.json", "access.log", &family)
}

#[test]
fn corpus_k8s_pod_security() -> Result<(), Box<dyn std::error::Error>> {
    let family: Vec<Box<dyn Validator>> = vec![Box::new(K8sPodSecurityValidator::new()?)];
    assert_family("k8s_pod.json", "workload.yaml", &family)
}

#[test]
fn corpus_dockerfile_hardening() -> Result<(), Box<dyn std::error::Error>> {
    let family: Vec<Box<dyn Validator>> = vec![Box::new(DockerfileHardeningValidator::new()?)];
    assert_family("dockerfile.json", "Dockerfile", &family)
}
