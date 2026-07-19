use enforcer_domain::ids::BuiltInIacRule;
use enforcer_domain::paths::{RelPath, RepoRoot};
use enforcer_lang_iac::rules::registry::build_all;
use enforcer_validator::harness::run_fixture_parity;

fn manifest_root() -> Result<RepoRoot, enforcer_domain::boundary::decode_error::DecodeError> {
    RepoRoot::try_from(std::path::Path::new(env!("CARGO_MANIFEST_DIR")))
}

fn assert_fixture(
    rule: BuiltInIacRule,
    fail: &RelPath,
    pass: &RelPath,
) -> Result<(), Box<dyn std::error::Error>> {
    let rule_id = rule.id();
    let rows = build_all()?;
    let Some(row) = rows.iter().find(|row| row.rule_id == rule_id) else {
        return Err(
            std::io::Error::other(format!("missing built-in IaC validator for {rule_id}")).into(),
        );
    };
    run_fixture_parity(row.validator.as_ref(), &manifest_root()?, fail, pass)?;
    Ok(())
}

#[test]
fn cloudformation_rule_fixtures_preserve_fail_and_pass_contracts(
) -> Result<(), Box<dyn std::error::Error>> {
    assert_fixture(
        BuiltInIacRule::CloudFormationPublicAccess,
        &"fixtures/cloudformation/iac-1-4/fail.template.json".parse()?,
        &"fixtures/cloudformation/iac-1-4/pass.template.json".parse()?,
    )?;
    assert_fixture(
        BuiltInIacRule::CloudFormationWildcardIam,
        &"fixtures/cloudformation/iac-1-5/fail.template.json".parse()?,
        &"fixtures/cloudformation/iac-1-5/pass.template.json".parse()?,
    )?;
    Ok(())
}

#[test]
fn kubernetes_rule_fixture_preserves_fail_and_pass_contract(
) -> Result<(), Box<dyn std::error::Error>> {
    assert_fixture(
        BuiltInIacRule::KubernetesPrivilegedContainer,
        &"fixtures/kubernetes/iac-1-8/fail.k8s.yaml".parse()?,
        &"fixtures/kubernetes/iac-1-8/pass.k8s.yaml".parse()?,
    )
}

#[test]
fn terraform_rule_fixtures_preserve_fail_and_pass_contracts(
) -> Result<(), Box<dyn std::error::Error>> {
    for (rule, fail, pass) in [
        (
            BuiltInIacRule::TerraformS3Encryption,
            "fixtures/terraform/iac-1-1/fail.tf".parse()?,
            "fixtures/terraform/iac-1-1/pass.tf".parse()?,
        ),
        (
            BuiltInIacRule::TerraformOpenIngress,
            "fixtures/terraform/iac-1-2/fail.tf".parse()?,
            "fixtures/terraform/iac-1-2/pass.tf".parse()?,
        ),
        (
            BuiltInIacRule::TerraformHardcodedSecrets,
            "fixtures/terraform/iac-1-3/fail.tf".parse()?,
            "fixtures/terraform/iac-1-3/pass.tf".parse()?,
        ),
        (
            BuiltInIacRule::TerraformProviderVersion,
            "fixtures/terraform/iac-1-6/fail.tf".parse()?,
            "fixtures/terraform/iac-1-6/pass.tf".parse()?,
        ),
        (
            BuiltInIacRule::TerraformRemoteStateEncryption,
            "fixtures/terraform/iac-1-7/fail.tf".parse()?,
            "fixtures/terraform/iac-1-7/pass.tf".parse()?,
        ),
    ] {
        assert_fixture(rule, &fail, &pass)?;
    }
    Ok(())
}
