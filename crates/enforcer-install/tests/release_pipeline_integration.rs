//! Executable public-behavior proof for the release rendering boundary and
//! typed publication gate.

use enforcer_install::ci::{
    boundary::release_rendering::{release_matrix, EmptyReleaseVersion, ReleaseVersionWire},
    release_pipeline::{
        gate_release, BinaryVariant, PublicationStatus, ReleaseAsset, SmokeOutcome, SmokeResult,
    },
};
use enforcer_install::distribution::TargetPlatform;

#[test]
fn release_boundary_rejects_blank_tags_and_renders_every_typed_asset() {
    assert_eq!(
        ReleaseVersionWire::from_raw_ci_tag(String::new()),
        Err(EmptyReleaseVersion)
    );

    let version = ReleaseVersionWire::from_raw_ci_tag("0.1.0".to_owned());
    let matrix = match version {
        Ok(version) => release_matrix(&version),
        Err(EmptyReleaseVersion) => Vec::new(),
    };
    assert_eq!(
        matrix.len(),
        TargetPlatform::all().len() * BinaryVariant::all().len()
    );
    assert!(matrix.iter().any(|record| {
        record.asset
            == ReleaseAsset {
                platform: TargetPlatform::WindowsX86_64,
                variant: BinaryVariant::Lite,
            }
            && record.asset_name == "enforcer-v0.1.0-lite-x86_64-pc-windows-msvc.zip"
            && record.checksum_name == "enforcer-v0.1.0-lite-x86_64-pc-windows-msvc.zip.sha256"
    }));
}

#[test]
fn typed_gate_blocks_a_release_when_one_rendered_asset_fails_smoke() {
    let verdict = gate_release(vec![
        SmokeResult {
            asset: ReleaseAsset {
                platform: TargetPlatform::MacAarch64,
                variant: BinaryVariant::Full,
            },
            outcome: SmokeOutcome::Passed,
        },
        SmokeResult {
            asset: ReleaseAsset {
                platform: TargetPlatform::LinuxX86_64Musl,
                variant: BinaryVariant::Lite,
            },
            outcome: SmokeOutcome::Failed,
        },
    ]);

    assert_eq!(verdict.publication_status(), PublicationStatus::Blocked);
}
