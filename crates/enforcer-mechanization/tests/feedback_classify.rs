use enforcer_domain::harness_types::{
    HarnessDiagnosticMessage, HarnessDiagnosticPath, HarnessExternalRuleId, HarnessLanguage,
    HarnessRunId, HarnessSourceLine, HarnessToolName,
};
use enforcer_domain::mechanization_types::MechanizationClassification;
use enforcer_domain::severity::Severity;
use enforcer_harness::parsers::HarnessDiagnostic;
use enforcer_mechanization::feedback::classify::classify;

fn diagnostic(tool: &str, rule_id: &str) -> HarnessDiagnostic {
    HarnessDiagnostic {
        run_id: HarnessRunId::from_adapter("run-1"),
        tool: HarnessToolName::from_adapter(tool),
        language: HarnessLanguage::Rust,
        severity: Severity::Error,
        rule_id: HarnessExternalRuleId::from_adapter(rule_id),
        file: HarnessDiagnosticPath::from_adapter("src/lib.rs"),
        line: HarnessSourceLine::from_external(1),
        message: HarnessDiagnosticMessage::from_adapter("boom"),
        source: None,
        fingerprint: None,
    }
}

#[test]
fn static_analysis_diagnostics_are_preventable() {
    assert_eq!(
        classify(&diagnostic("cargo", "E0308")),
        MechanizationClassification::Prevent
    );
    assert_eq!(
        classify(&diagnostic("eslint", "no-unused-vars")),
        MechanizationClassification::Prevent
    );
}

#[test]
fn runtime_and_non_specific_diagnostics_are_detect_only() {
    assert_eq!(
        classify(&diagnostic("pytest", "pytest")),
        MechanizationClassification::Detect
    );
    assert_eq!(
        classify(&diagnostic("cflint", "HAR-2.4")),
        MechanizationClassification::Detect
    );
    assert_eq!(
        classify(&diagnostic("some-custom-runner", "CUSTOM-1")),
        MechanizationClassification::Detect
    );
}
