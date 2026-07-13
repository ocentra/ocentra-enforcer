use enforcer_harness::parsers::HarnessDiagnostic;
use enforcer_mechanization::feedback::classify::{classify, Classification};

fn diagnostic(tool: &str, rule_id: &str) -> HarnessDiagnostic {
    HarnessDiagnostic {
        run_id: "run-1".to_owned(),
        tool: tool.to_owned(),
        language: "rust".to_owned(),
        severity: "error".to_owned(),
        rule_id: rule_id.to_owned(),
        file: "src/lib.rs".to_owned(),
        line: 1,
        message: "boom".to_owned(),
        source: None,
        fingerprint: None,
    }
}

#[test]
fn static_analysis_diagnostics_are_preventable() {
    assert_eq!(classify(&diagnostic("cargo", "E0308")), Classification::Prevent);
    assert_eq!(
        classify(&diagnostic("eslint", "no-unused-vars")),
        Classification::Prevent
    );
}

#[test]
fn runtime_and_non_specific_diagnostics_are_detect_only() {
    assert_eq!(classify(&diagnostic("pytest", "pytest")), Classification::Detect);
    assert_eq!(classify(&diagnostic("cflint", "HAR-2.4")), Classification::Detect);
    assert_eq!(classify(&diagnostic("cargo", "")), Classification::Detect);
    assert_eq!(
        classify(&diagnostic("some-custom-runner", "CUSTOM-1")),
        Classification::Detect
    );
}
