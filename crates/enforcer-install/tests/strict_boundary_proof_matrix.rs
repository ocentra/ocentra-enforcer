use enforcer_domain::{
    install_types::{
        ArtifactKind, CommandName, DryRun, InstallCommand, InstallOutputMode,
        InstallRequestContext, InstallScope, InstallVerifyCheck, InstallVerifyReport,
        ObservedBranchProtection, PlannedInstallChange, PluginPublishContract, SkillAsset,
        SkillAssetManifest, UninstallCommand, Verification,
    },
    paths::RepoRoot,
};
use enforcer_install::{
    ci::boundary::{
        branch_protection::BranchProtectionReportDto,
        branch_protection_payload::{
            BranchProtectionWriteDto, LiveProtectionStateDto, RequiredStatusChecksDto,
        },
    },
    command_envelope::CommandEnvelopeDto,
    install_requests::{InstallRequest, UninstallRequest},
    migrate_legacy_name::{MigrationFindingDto, RewrittenFileDto},
    report::{
        AppliedChangeDto, ApplyResultDto, InstallReportDto, PlannedChangeDto,
        PluginPublishContractDto, SkillAssetDto, SkillAssetManifestDto, VerifyCheckDto,
        VerifyReportDto,
    },
    request_context::RequestContextDto,
};

fn path(value: &str) -> Result<RepoRoot, Box<dyn std::error::Error>> {
    Ok(value.to_owned().try_into()?)
}

fn context(binary_path: &str) -> RequestContextDto {
    RequestContextDto {
        scope: InstallScope::User,
        dry_run: DryRun::Disabled,
        output: InstallOutputMode::Human,
        binary_path: binary_path.into(),
    }
}

fn change() -> Result<PlannedChangeDto, Box<dyn std::error::Error>> {
    Ok(PlannedChangeDto {
        harness: "claude".to_owned(),
        kind: ArtifactKind::McpRegistration,
        path: path("/opt/claude.json")?,
        description: "register enforcer".to_owned(),
        is_update: false,
    })
}

fn check() -> VerifyCheckDto {
    VerifyCheckDto {
        harness: "claude".to_owned(),
        name: "mcp-registration-present".to_owned(),
        passed: true,
        detail: String::new(),
    }
}

#[test]
fn install_boundary_dtos_round_trip_through_their_external_json_contracts(
) -> Result<(), Box<dyn std::error::Error>> {
    let command: CommandEnvelopeDto = CommandEnvelopeDto::new(CommandName::Doctor, vec![check()]);
    let command_wire = serde_json::to_string(&command)?;
    let command_back: CommandEnvelopeDto = serde_json::from_str(&command_wire)?;
    assert_eq!(command_back, command);

    let finding: MigrationFindingDto = MigrationFindingDto {
        harness: "codex".to_owned(),
        path: "/home/user/.codex/config.toml".to_owned(),
        kind: enforcer_domain::install_types::FindingKind::LegacyServerRegistration,
        detail: "legacy registration".to_owned(),
    };
    let finding_wire = serde_json::to_string(&finding)?;
    let finding_back: MigrationFindingDto = serde_json::from_str(&finding_wire)?;
    assert_eq!(finding_back, finding);

    let rewritten: RewrittenFileDto = RewrittenFileDto {
        path: "/home/user/.codex/config.toml".to_owned(),
        backup_path: "/home/user/.codex/config.toml.bak".to_owned(),
    };
    let rewritten_wire = serde_json::to_string(&rewritten)?;
    let rewritten_back: RewrittenFileDto = serde_json::from_str(&rewritten_wire)?;
    assert_eq!(rewritten_back, rewritten);

    let planned: PlannedChangeDto = change()?;
    let planned_wire = serde_json::to_string(&planned)?;
    let planned_back: PlannedChangeDto = serde_json::from_str(&planned_wire)?;
    assert_eq!(planned_back, planned);

    let applied: AppliedChangeDto = AppliedChangeDto {
        change: change()?,
        succeeded: true,
        backup_path: None,
    };
    let applied_wire = serde_json::to_string(&applied)?;
    let applied_back: AppliedChangeDto = serde_json::from_str(&applied_wire)?;
    assert_eq!(applied_back, applied);

    let verify: VerifyCheckDto = check();
    let verify_wire = serde_json::to_string(&verify)?;
    let verify_back: VerifyCheckDto = serde_json::from_str(&verify_wire)?;
    assert_eq!(verify_back, verify);

    let asset: SkillAssetDto = SkillAssetDto {
        skill_name: "ocentra-enforcer".to_owned(),
        asset_path: "skills/ocentra-enforcer/SKILL.md".to_owned(),
    };
    let asset_wire = serde_json::to_string(&asset)?;
    let asset_back: SkillAssetDto = serde_json::from_str(&asset_wire)?;
    assert_eq!(asset_back, asset);

    let contract: PluginPublishContractDto = PluginPublishContractDto {
        manifest_path: ".codex-plugin/plugin.json".to_owned(),
        field: "skills".to_owned(),
        expected_value: "./skills/".to_owned(),
    };
    let contract_wire = serde_json::to_string(&contract)?;
    let contract_back: PluginPublishContractDto = serde_json::from_str(&contract_wire)?;
    assert_eq!(contract_back, contract);

    let request: RequestContextDto = context("/opt/enforcer");
    let request_wire = serde_json::to_string(&request)?;
    let request_back: RequestContextDto = serde_json::from_str(&request_wire)?;
    assert_eq!(request_back, request);

    let session: enforcer_install::hooks::sessionstart::SessionStartHookConfigDto =
        enforcer_install::hooks::sessionstart::sessionstart_hook_config(std::path::Path::new(
            "/opt/enforcer",
        ));
    let session_wire = serde_json::to_string(&session)?;
    let session_back: enforcer_install::hooks::sessionstart::SessionStartHookConfigDto =
        serde_json::from_str(&session_wire)?;
    assert_eq!(session_back, session);

    let required: RequiredStatusChecksDto = RequiredStatusChecksDto {
        strict: true,
        contexts: vec!["Rust CI / rust-ci".to_owned()],
    };
    let required_wire = serde_json::to_string(&required)?;
    let required_back: RequiredStatusChecksDto = serde_json::from_str(&required_wire)?;
    assert_eq!(required_back, required);

    let write: BranchProtectionWriteDto = BranchProtectionWriteDto {
        required_status_checks: RequiredStatusChecksDto {
            strict: true,
            contexts: vec!["Rust CI / rust-ci".to_owned()],
        },
        enforce_admins: true,
        required_pull_request: true,
        allow_force_pushes: false,
        allow_deletions: false,
    };
    let write_wire = serde_json::to_string(&write)?;
    let write_back: BranchProtectionWriteDto = serde_json::from_str(&write_wire)?;
    assert_eq!(write_back, write);

    let branch: BranchProtectionReportDto = BranchProtectionReportDto {
        branch: "main".to_owned(),
        expected_contexts: vec!["Rust CI / rust-ci".to_owned()],
        observed_contexts: vec!["Rust CI / rust-ci".to_owned()],
        attested: true,
        exit_code: 0,
        refusal_codes: Vec::new(),
    };
    let branch_wire = serde_json::to_string(&branch)?;
    let branch_back: BranchProtectionReportDto = serde_json::from_str(&branch_wire)?;
    assert_eq!(branch_back, branch);
    Ok(())
}

#[test]
fn session_start_hook_config_dto_rejects_an_empty_command() -> Result<(), Box<dyn std::error::Error>>
{
    use enforcer_domain::install_types::{HookEvent, SessionStartHookConfig};
    use enforcer_install::hooks::sessionstart::SessionStartHookConfigDto;

    let result = SessionStartHookConfig::try_from(SessionStartHookConfigDto {
        event: HookEvent::SessionStart,
        matcher: String::new(),
        command: String::new(),
        reminder_body: "reminder".to_owned(),
    });
    assert!(result.is_err(), "empty hook commands are rejected");
    let error = result.err().ok_or("empty hook commands are rejected")?;

    assert_eq!(error.path, "command");
    Ok(())
}

#[test]
fn install_boundary_try_from_rejects_invalid_dtos_at_each_domain_entrypoint(
) -> Result<(), Box<dyn std::error::Error>> {
    let command_error = InstallVerifyReport::try_from(CommandEnvelopeDto {
        command: CommandName::Doctor,
        ok: true,
        checks: vec![VerifyCheckDto {
            passed: false,
            ..check()
        }],
    })
    .err()
    .ok_or("aggregate command status must agree with its checks")?;
    assert_eq!(command_error.path, "ok");

    let install_error = InstallCommand::try_from(InstallRequest {
        context: context("relative/enforcer"),
        only_harnesses: Vec::new(),
    })
    .err()
    .ok_or("install requires an absolute binary path")?;
    assert_eq!(install_error.path, "installBinaryPath");
    let uninstall_error = UninstallCommand::try_from(UninstallRequest {
        context: context("relative/enforcer"),
        only_harnesses: Vec::new(),
    })
    .err()
    .ok_or("uninstall requires an absolute binary path")?;
    assert_eq!(uninstall_error.path, "installBinaryPath");
    let context_error = InstallRequestContext::try_from(context("relative/enforcer"))
        .err()
        .ok_or("request context requires an absolute binary path")?;
    assert_eq!(context_error.path, "installBinaryPath");

    let mut invalid_change = change()?;
    invalid_change.harness.clear();
    let planned_error = PlannedInstallChange::try_from(invalid_change.clone())
        .err()
        .ok_or("planned changes require a harness id")?;
    assert_eq!(planned_error.path, "harnessId");
    let report_error = enforcer_domain::install_types::InstallReport::try_from(InstallReportDto {
        planned_changes: vec![invalid_change.clone()],
        warnings: Vec::new(),
    })
    .err()
    .ok_or("reports reject an invalid nested change")?;
    assert_eq!(report_error.path, "harnessId");
    let applied_error =
        enforcer_domain::install_types::AppliedInstallChange::try_from(AppliedChangeDto {
            change: invalid_change.clone(),
            succeeded: true,
            backup_path: None,
        })
        .err()
        .ok_or("applied changes reject an invalid nested change")?;
    assert_eq!(applied_error.path, "harnessId");
    let apply_error = enforcer_domain::install_types::ApplyResult::try_from(ApplyResultDto {
        applied: vec![AppliedChangeDto {
            change: invalid_change,
            succeeded: true,
            backup_path: None,
        }],
    })
    .err()
    .ok_or("apply results reject an invalid nested change")?;
    assert_eq!(apply_error.path, "harnessId");

    let invalid_check = VerifyCheckDto {
        harness: String::new(),
        ..check()
    };
    let verify_check_error = InstallVerifyCheck::try_from(invalid_check.clone())
        .err()
        .ok_or("verification checks require a harness id")?;
    assert_eq!(verify_check_error.path, "harnessId");
    let verify_report_error = InstallVerifyReport::try_from(VerifyReportDto {
        checks: vec![invalid_check],
    })
    .err()
    .ok_or("verification reports reject an invalid nested check")?;
    assert_eq!(verify_report_error.path, "harnessId");

    let asset_error = SkillAsset::try_from(SkillAssetDto {
        skill_name: "invalid\0skill".to_owned(),
        asset_path: "skills/x/SKILL.md".to_owned(),
    })
    .err()
    .ok_or("skill assets require a non-empty name")?;
    assert_eq!(asset_error.path, "installReportText");
    let contract_error = PluginPublishContract::try_from(PluginPublishContractDto {
        manifest_path: ".codex-plugin/plugin.json".to_owned(),
        field: "invalid\0field".to_owned(),
        expected_value: "./skills/".to_owned(),
    })
    .err()
    .ok_or("publish contracts require a field name")?;
    assert_eq!(contract_error.path, "installReportText");
    let manifest_error = SkillAssetManifest::try_from(SkillAssetManifestDto {
        assets: vec![SkillAssetDto {
            skill_name: "invalid\0skill".to_owned(),
            asset_path: "skills/x/SKILL.md".to_owned(),
        }],
        plugin_contracts: Vec::new(),
    })
    .err()
    .ok_or("manifests reject invalid nested assets")?;
    assert_eq!(manifest_error.path, "installReportText");

    let live_error = ObservedBranchProtection::try_from(LiveProtectionStateDto {
        required_status_checks: Some(RequiredStatusChecksDto {
            strict: true,
            contexts: vec!["Rust CI / rust-ci\nspoof".to_owned()],
        }),
        enforce_admins: true,
        required_pull_request: true,
        allow_force_pushes: false,
        allow_deletions: false,
        required_checks_passing: Some(true),
    })
    .err()
    .ok_or("GitHub check contexts cannot include line breaks")?;
    assert_eq!(live_error.path, "githubCheckContext");
    let branch_error = Verification::try_from(BranchProtectionReportDto {
        branch: "main".to_owned(),
        expected_contexts: Vec::new(),
        observed_contexts: Vec::new(),
        attested: false,
        exit_code: 1,
        refusal_codes: vec!["unknown".to_owned()],
    })
    .err()
    .ok_or("unknown refusal codes are rejected")?;
    assert_eq!(branch_error.path, "refusalCode");
    Ok(())
}
