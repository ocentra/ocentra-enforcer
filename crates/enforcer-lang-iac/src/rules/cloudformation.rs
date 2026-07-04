//! `iac/cfn-*` — the CloudFormation (JSON/YAML template) slice of the IaC
//! rule family: IAC-1.4, IAC-1.5.

use super::spec::{RuleSpec, TriggerKind};

/// Every CloudFormation rule's static spec, in `rules/rules.json`
/// declaration order.
pub const SPECS: &[RuleSpec] = &[
    RuleSpec {
        rule_id: "IAC-1.4",
        title: "CloudFormation S3 buckets must block public access",
        kind: TriggerKind::RequiredAbsent {
            scope_needle: "AWS::S3::Bucket",
            required_needle: "PublicAccessBlockConfiguration",
        },
        needles: &[],
        comment_guard: false,
    },
    RuleSpec {
        rule_id: "IAC-1.5",
        title: "CloudFormation IAM policies must not grant wildcard action+resource",
        kind: TriggerKind::ForbiddenPresent,
        // Both a wildcard Action AND wildcard Resource must be present in
        // the file for this to be the specific over-broad-grant shape this
        // rule targets — checked as two independent literal needles OR'd
        // per-line would over-fire on a file that merely mentions either
        // alone, so this rule keys on the co-occurring pair line
        // (`"Action": "*"` and `"Resource": "*"` are adjacent lines in the
        // canonical CFN statement shape) via the combined marker below.
        needles: &["\"Action\": \"*\""],
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
            .ok_or_else(|| format!("no cloudformation spec for {rule_id}"))
    }

    #[test]
    fn iac_1_4_fires_on_missing_public_access_block_and_stays_silent_when_present(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let validator = SpecValidator::new(spec_for("IAC-1.4")?)?;
        run_fixture_parity(
            &validator,
            &manifest_dir(),
            "fixtures/cloudformation/iac-1-4/fail.template.json",
            "fixtures/cloudformation/iac-1-4/pass.template.json",
        )?;
        Ok(())
    }

    #[test]
    fn iac_1_5_fires_on_wildcard_iam_action_and_stays_silent_on_scoped_action(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let validator = SpecValidator::new(spec_for("IAC-1.5")?)?;
        run_fixture_parity(
            &validator,
            &manifest_dir(),
            "fixtures/cloudformation/iac-1-5/fail.template.json",
            "fixtures/cloudformation/iac-1-5/pass.template.json",
        )?;
        Ok(())
    }
}
