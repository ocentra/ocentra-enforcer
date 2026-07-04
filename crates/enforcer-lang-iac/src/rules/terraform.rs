//! `iac/terraform-*` — the Terraform (HCL) slice of the IaC rule family:
//! IAC-1.1, IAC-1.2, IAC-1.3, IAC-1.6, IAC-1.7.

use super::spec::{RuleSpec, TriggerKind};

/// Every Terraform rule's static spec, in `rules/rules.json` declaration
/// order.
pub const SPECS: &[RuleSpec] = &[
    RuleSpec {
        rule_id: "IAC-1.1",
        title: "Terraform S3 buckets must enable server-side encryption",
        kind: TriggerKind::RequiredAbsent {
            scope_needle: "aws_s3_bucket",
            required_needle: "server_side_encryption_configuration",
        },
        needles: &[],
        comment_guard: false,
    },
    RuleSpec {
        rule_id: "IAC-1.2",
        title: "Terraform security groups must not allow unrestricted ingress",
        kind: TriggerKind::ForbiddenPresent,
        needles: &["0.0.0.0/0"],
        comment_guard: true,
    },
    RuleSpec {
        rule_id: "IAC-1.3",
        title: "Terraform resources must not hardcode secrets or credentials",
        kind: TriggerKind::ForbiddenPresent,
        needles: &["aws_access_key_id", "aws_secret_access_key", "password ="],
        comment_guard: true,
    },
    RuleSpec {
        rule_id: "IAC-1.6",
        title: "Terraform provider blocks must pin an exact version",
        kind: TriggerKind::RequiredAbsent {
            scope_needle: "required_providers",
            required_needle: "version",
        },
        needles: &[],
        comment_guard: false,
    },
    RuleSpec {
        rule_id: "IAC-1.7",
        title: "Terraform remote state backends must enable encryption",
        kind: TriggerKind::RequiredAbsent {
            scope_needle: "backend \"s3\"",
            required_needle: "encrypt",
        },
        needles: &[],
        comment_guard: false,
    },
];

#[cfg(test)]
mod tests {
    use super::SPECS;
    use crate::rules::spec::SpecValidator;
    use enforcer_validator::harness::run_fixture_parity;
    use std::path::PathBuf;

    fn manifest_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    fn spec_for(rule_id: &str) -> Result<crate::rules::spec::RuleSpec, String> {
        SPECS
            .iter()
            .find(|spec| spec.rule_id == rule_id)
            .copied()
            .ok_or_else(|| format!("no terraform spec for {rule_id}"))
    }

    #[test]
    fn iac_1_1_fires_on_unencrypted_s3_bucket_and_stays_silent_when_encrypted(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let validator = SpecValidator::new(spec_for("IAC-1.1")?)?;
        run_fixture_parity(
            &validator,
            &manifest_dir(),
            "fixtures/terraform/iac-1-1/fail.tf",
            "fixtures/terraform/iac-1-1/pass.tf",
        )?;
        Ok(())
    }

    #[test]
    fn iac_1_2_fires_on_open_ingress_and_stays_silent_on_scoped_cidr(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let validator = SpecValidator::new(spec_for("IAC-1.2")?)?;
        run_fixture_parity(
            &validator,
            &manifest_dir(),
            "fixtures/terraform/iac-1-2/fail.tf",
            "fixtures/terraform/iac-1-2/pass.tf",
        )?;
        Ok(())
    }

    #[test]
    fn iac_1_3_fires_on_hardcoded_secret_and_stays_silent_without_one(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let validator = SpecValidator::new(spec_for("IAC-1.3")?)?;
        run_fixture_parity(
            &validator,
            &manifest_dir(),
            "fixtures/terraform/iac-1-3/fail.tf",
            "fixtures/terraform/iac-1-3/pass.tf",
        )?;
        Ok(())
    }

    #[test]
    fn iac_1_6_fires_on_unpinned_provider_and_stays_silent_when_pinned(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let validator = SpecValidator::new(spec_for("IAC-1.6")?)?;
        run_fixture_parity(
            &validator,
            &manifest_dir(),
            "fixtures/terraform/iac-1-6/fail.tf",
            "fixtures/terraform/iac-1-6/pass.tf",
        )?;
        Ok(())
    }

    #[test]
    fn iac_1_7_fires_on_unencrypted_state_backend_and_stays_silent_when_encrypted(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let validator = SpecValidator::new(spec_for("IAC-1.7")?)?;
        run_fixture_parity(
            &validator,
            &manifest_dir(),
            "fixtures/terraform/iac-1-7/fail.tf",
            "fixtures/terraform/iac-1-7/pass.tf",
        )?;
        Ok(())
    }
}
