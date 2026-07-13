use std::path::PathBuf;

use enforcer_lang_iac::rules::{
    cloudformation, kubernetes, terraform,
    spec::{RuleSpec, SpecValidator},
};
use enforcer_validator::harness::run_fixture_parity;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn spec_for(specs: &[RuleSpec], rule_id: &str) -> Result<RuleSpec, String> {
    specs
        .iter()
        .find(|spec| spec.rule_id == rule_id)
        .copied()
        .ok_or_else(|| format!("missing IaC rule spec {rule_id}"))
}

fn assert_fixture(specs: &[RuleSpec], rule_id: &str, fail: &str, pass: &str) -> Result<(), Box<dyn std::error::Error>> {
    let validator = SpecValidator::new(spec_for(specs, rule_id)?)?;
    run_fixture_parity(&validator, &manifest_dir(), fail, pass)?;
    Ok(())
}

#[test]
fn cloudformation_rule_fixtures_preserve_fail_and_pass_contracts() -> Result<(), Box<dyn std::error::Error>> {
    assert_fixture(
        cloudformation::SPECS,
        "IAC-1.4",
        "fixtures/cloudformation/iac-1-4/fail.template.json",
        "fixtures/cloudformation/iac-1-4/pass.template.json",
    )?;
    assert_fixture(
        cloudformation::SPECS,
        "IAC-1.5",
        "fixtures/cloudformation/iac-1-5/fail.template.json",
        "fixtures/cloudformation/iac-1-5/pass.template.json",
    )?;
    Ok(())
}

#[test]
fn kubernetes_rule_fixture_preserves_fail_and_pass_contract() -> Result<(), Box<dyn std::error::Error>> {
    assert_fixture(
        kubernetes::SPECS,
        "IAC-1.8",
        "fixtures/kubernetes/iac-1-8/fail.k8s.yaml",
        "fixtures/kubernetes/iac-1-8/pass.k8s.yaml",
    )
}

#[test]
fn terraform_rule_fixtures_preserve_fail_and_pass_contracts() -> Result<(), Box<dyn std::error::Error>> {
    for (rule_id, fail, pass) in [
        ("IAC-1.1", "fixtures/terraform/iac-1-1/fail.tf", "fixtures/terraform/iac-1-1/pass.tf"),
        ("IAC-1.2", "fixtures/terraform/iac-1-2/fail.tf", "fixtures/terraform/iac-1-2/pass.tf"),
        ("IAC-1.3", "fixtures/terraform/iac-1-3/fail.tf", "fixtures/terraform/iac-1-3/pass.tf"),
        ("IAC-1.6", "fixtures/terraform/iac-1-6/fail.tf", "fixtures/terraform/iac-1-6/pass.tf"),
        ("IAC-1.7", "fixtures/terraform/iac-1-7/fail.tf", "fixtures/terraform/iac-1-7/pass.tf"),
    ] {
        assert_fixture(terraform::SPECS, rule_id, fail, pass)?;
    }
    Ok(())
}
