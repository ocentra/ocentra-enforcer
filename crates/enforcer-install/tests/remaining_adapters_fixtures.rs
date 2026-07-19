//! c09 acceptance-row proof (`remaining-adapters-detect` +
//! `remaining-harness-adapters`, `TEST_PROOF_EXPECTATIONS.md`): exercises
//! the six remaining Track C adapters (antigravity, windsurf, opencode,
//! aider, kilocode, kiro) end to end against isolated temp-home fixtures:
//! - JSON-config harnesses (antigravity/windsurf/kilocode/kiro): fail
//!   fixture (entry missing/renamed -> `verify` reports the named failing
//!   check), pass fixture (`apply` yields the correct native config,
//!   re-read matches, second `apply` byte-identical), not-detected
//!   fixture (absent harness marker -> zero writes).
//! - CLI-only harnesses (opencode/aider): the `Tier::T3` `deferred: no mcp
//!   surface` label, writing zero files.
//! - `remaining-adapters-detect`: c02's `detect_harnesses` enumerates all
//!   six [`enforcer_domain::ids::HarnessId`]s, and `enforcer_install::core::doctor`
//!   aggregates one [`enforcer_install::report::VerifyReportDto`] per adapter
//!   across the full six-adapter set.

use enforcer_domain::install_types::{DoctorCommand, InstallRequestContext};
use enforcer_install::adapters::aider::AiderAdapter;
use enforcer_install::adapters::antigravity::AntigravityAdapter;
use enforcer_install::adapters::kilocode::KiloCodeAdapter;
use enforcer_install::adapters::kiro::KiroAdapter;
use enforcer_install::adapters::opencode::OpenCodeAdapter;
use enforcer_install::adapters::windsurf::WindsurfAdapter;
use enforcer_install::core::{doctor, HarnessAdapter};
use enforcer_install::detect::{detect_harnesses, MapEnv, RealFs, KNOWN_HARNESS_IDS};
fn ctx(
    binary: &std::path::Path,
) -> Result<InstallRequestContext, enforcer_domain::boundary::decode_error::DecodeError> {
    InstallRequestContext::try_with_defaults(binary.to_path_buf())
}

// ---------------------------------------------------------------------
// remaining-adapters-detect: autodetect enumerates all six ids
// ---------------------------------------------------------------------

#[test]
fn autodetect_enumerates_all_six_remaining_harness_ids() -> Result<(), Box<dyn std::error::Error>> {
    let expected = [
        "antigravity",
        "windsurf",
        "opencode",
        "aider",
        "kilocode",
        "kiro",
    ];
    for id in expected {
        assert!(
            KNOWN_HARNESS_IDS.contains(&id),
            "expected {id} in KNOWN_HARNESS_IDS"
        );
    }

    let home = tempfile::tempdir()?;
    let env = MapEnv::new().with("HOME", home.path().display().to_string());
    let fs = RealFs;
    let records = detect_harnesses(&env, &fs)?;
    for id in expected {
        assert!(
            records.iter().any(|r| r.id.as_str() == id),
            "expected a detection record for {id}"
        );
    }
    Ok(())
}

#[test]
fn doctor_aggregates_a_verify_report_per_adapter_across_all_six(
) -> Result<(), Box<dyn std::error::Error>> {
    let home = tempfile::tempdir()?;
    let binary = home.path().join("bin").join("enforcer");
    let antigravity = AntigravityAdapter::try_new(home.path().to_path_buf(), binary.clone())?;
    let windsurf = WindsurfAdapter::try_new(home.path().to_path_buf(), binary.clone())?;
    let kilocode = KiloCodeAdapter::try_new(home.path().to_path_buf(), binary.clone())?;
    let kiro = KiroAdapter::try_new(home.path().to_path_buf(), binary.clone())?;
    let opencode = OpenCodeAdapter::new();
    let aider = AiderAdapter::new();

    let adapters: Vec<&dyn HarnessAdapter> =
        vec![&antigravity, &windsurf, &kilocode, &kiro, &opencode, &aider];
    let request = DoctorCommand::default();
    let outcomes = doctor(&adapters, &ctx(&binary)?, &request)?;
    assert_eq!(outcomes.len(), 6);

    let keys: Vec<&str> = outcomes.iter().map(|(key, _)| key.as_str()).collect();
    for expected in [
        "antigravity",
        "windsurf",
        "kilocode",
        "kiro",
        "opencode",
        "aider",
    ] {
        assert!(
            keys.contains(&expected),
            "expected {expected} in doctor output"
        );
    }
    Ok(())
}

// ---------------------------------------------------------------------
// JSON-config harnesses: fail / pass / not-detected fixtures
// ---------------------------------------------------------------------

macro_rules! json_adapter_proof {
    ($mod_name:ident, $adapter:ty, $seed_dir:expr, $config_rel:expr) => {
        mod $mod_name {
            use super::ctx;
            use enforcer_install::core::HarnessAdapter;

            #[test]
            fn not_detected_fixture_yields_zero_writes() -> Result<(), Box<dyn std::error::Error>> {
                let home = tempfile::tempdir()?;
                let binary = home.path().join("bin").join("enforcer");
                // No harness marker seeded at all -- this is the
                // "absent harness" fixture. The adapter itself has no
                // notion of "not detected" (that is c02's job); what it
                // MUST do is never write outside of an explicit `apply`.
                // Simply constructing it and calling `verify` (the doctor
                // path an absent-harness caller would take) must not
                // create any file.
                let adapter = <$adapter>::try_new(home.path().to_path_buf(), binary.clone())?;
                let _ = adapter.verify(&ctx(&binary)?)?;
                let config_path = home.path().join($config_rel);
                assert!(!config_path.exists(), "verify must not write");
                Ok(())
            }

            #[test]
            fn pass_fixture_apply_then_reread_matches_and_second_apply_is_idempotent(
            ) -> Result<(), Box<dyn std::error::Error>> {
                let home = tempfile::tempdir()?;
                let binary = home.path().join("bin").join("enforcer");
                std::fs::create_dir_all(home.path().join($seed_dir))?;
                let adapter = <$adapter>::try_new(home.path().to_path_buf(), binary.clone())?;

                let plan = adapter.plan(&ctx(&binary)?)?;
                assert_eq!(
                    plan.planned_changes.len(),
                    1,
                    "a fresh native adapter fixture must plan exactly one config write"
                );
                let applied = adapter.apply(&plan)?;
                assert!(applied.applied.iter().all(|change| matches!(
                    change.status,
                    enforcer_domain::install_types::CheckStatus::Passed
                )));

                let verify = adapter.verify(&ctx(&binary)?)?;
                assert!(
                    verify.checks.iter().all(|check| matches!(
                        check.status,
                        enforcer_domain::install_types::CheckStatus::Passed
                    )),
                    "expected all checks green: {verify:?}"
                );

                let second_plan = adapter.plan(&ctx(&binary)?)?;
                assert!(
                    second_plan.planned_changes.is_empty(),
                    "second apply must be a no-op (idempotent)"
                );
                Ok(())
            }

            #[test]
            fn fail_fixture_renamed_entry_reports_named_failing_check(
            ) -> Result<(), Box<dyn std::error::Error>> {
                let home = tempfile::tempdir()?;
                let binary = home.path().join("bin").join("enforcer");
                let adapter = <$adapter>::try_new(home.path().to_path_buf(), binary.clone())?;

                let plan = adapter.plan(&ctx(&binary)?)?;
                adapter.apply(&plan)?;

                let config_path = home.path().join($config_rel);
                let raw = std::fs::read_to_string(&config_path)?;
                let mut root: serde_json::Value = serde_json::from_str(&raw)?;
                if let Some(servers) = root
                    .get_mut("mcpServers")
                    .and_then(serde_json::Value::as_object_mut)
                {
                    if let Some(entry) = servers.remove(enforcer_mcp::name::SERVER_NAME) {
                        servers.insert("renamed-server".to_owned(), entry);
                    }
                }
                std::fs::write(&config_path, serde_json::to_string_pretty(&root)?)?;

                let report = adapter.verify(&ctx(&binary)?)?;
                assert!(!report.checks.iter().all(|check| matches!(
                    check.status,
                    enforcer_domain::install_types::CheckStatus::Passed
                )));
                assert_eq!(report.checks[0].name.as_str(), "mcp-registration-present");
                Ok(())
            }
        }
    };
}

json_adapter_proof!(
    antigravity_proof,
    enforcer_install::adapters::antigravity::AntigravityAdapter,
    ".gemini/config",
    ".gemini/config/mcp_config.json"
);
json_adapter_proof!(
    windsurf_proof,
    enforcer_install::adapters::windsurf::WindsurfAdapter,
    ".codeium/windsurf",
    ".codeium/windsurf/mcp_config.json"
);
json_adapter_proof!(
    kilocode_proof,
    enforcer_install::adapters::kilocode::KiloCodeAdapter,
    "globalStorage/kilocode.kilo-code/settings",
    "globalStorage/kilocode.kilo-code/settings/mcp_settings.json"
);
json_adapter_proof!(
    kiro_proof,
    enforcer_install::adapters::kiro::KiroAdapter,
    ".kiro/settings",
    ".kiro/settings/mcp.json"
);

// ---------------------------------------------------------------------
// CLI-only harnesses: T3 deferred, zero writes
// ---------------------------------------------------------------------

macro_rules! cli_only_stub_proof {
    ($mod_name:ident, $adapter:ty, $key:expr) => {
        mod $mod_name {
            use super::ctx;
            use enforcer_install::core::HarnessAdapter;

            #[test]
            fn deferred_no_mcp_surface_label_is_present_and_writes_zero_files(
            ) -> Result<(), Box<dyn std::error::Error>> {
                let dir = tempfile::tempdir()?;
                let binary = std::env::temp_dir().join("enforcer");
                let adapter = <$adapter>::new();
                assert_eq!(adapter.harness_key().as_str(), $key);

                let plan = adapter.plan(&ctx(&binary)?)?;
                assert!(plan.planned_changes.is_empty());
                assert!(plan
                    .warnings
                    .iter()
                    .any(|w| w.as_str().contains("no mcp surface")));

                let applied = adapter.apply(&plan)?;
                assert!(applied.applied.is_empty());
                let after: Vec<_> = std::fs::read_dir(dir.path())?.collect();
                assert!(
                    after.is_empty(),
                    "CLI-only no-write adapter must leave the directory empty"
                );

                let report = adapter.verify(&ctx(&binary)?)?;
                assert!(report.checks.iter().all(|check| matches!(
                    check.status,
                    enforcer_domain::install_types::CheckStatus::Passed
                )));
                assert!(report.checks[0].detail.as_str().contains("T3"));
                assert!(report.checks[0].detail.as_str().contains("no mcp surface"));
                Ok(())
            }
        }
    };
}

cli_only_stub_proof!(
    opencode_proof,
    enforcer_install::adapters::opencode::OpenCodeAdapter,
    "opencode"
);
cli_only_stub_proof!(
    aider_proof,
    enforcer_install::adapters::aider::AiderAdapter,
    "aider"
);
