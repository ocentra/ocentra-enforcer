//! Typed release-gating domain for Enforcer's own binary distribution.
//!
//! Archive names, version tags, and CI input stay in
//! [`super::boundary::release_rendering`]. This module only decides whether
//! the typed platform-and-variant assets are safe to publish.

use enforcer_domain::install_types::{ReleaseGateVerdict, SmokeOutcome, SmokeResult};

/// Gate a completed smoke sweep. One failed asset blocks the whole release.
#[must_use]
pub fn gate_release(results: Vec<SmokeResult>) -> ReleaseGateVerdict {
    let failing = results
        .into_iter()
        .filter(|result| result.outcome == SmokeOutcome::Failed)
        .map(|result| result.asset)
        .collect();
    ReleaseGateVerdict::from_failing(failing)
}

#[cfg(test)]
mod tests {
    use super::gate_release;
    use enforcer_domain::install_types::{
        BinaryVariant, PublicationStatus, ReleaseAsset, ReleaseGateVerdict, SmokeOutcome,
        SmokeResult, TargetPlatform,
    };

    #[test]
    fn ci_default_variant_is_lite() {
        assert_eq!(BinaryVariant::ci_default(), BinaryVariant::Lite);
    }

    #[test]
    fn gate_release_publishes_when_every_smoke_result_passed() {
        let results = vec![SmokeResult {
            asset: ReleaseAsset {
                platform: TargetPlatform::WindowsX86_64,
                variant: BinaryVariant::Lite,
            },
            outcome: SmokeOutcome::Passed,
        }];
        let verdict = gate_release(results);
        assert_eq!(verdict, ReleaseGateVerdict::Publish);
        assert_eq!(verdict.publication_status(), PublicationStatus::Approved);
    }

    #[test]
    fn gate_release_blocks_when_any_single_platform_smoke_fails() {
        let broken = ReleaseAsset {
            platform: TargetPlatform::LinuxX86_64Musl,
            variant: BinaryVariant::Lite,
        };
        let verdict = gate_release(vec![
            SmokeResult {
                asset: ReleaseAsset {
                    platform: TargetPlatform::WindowsX86_64,
                    variant: BinaryVariant::Lite,
                },
                outcome: SmokeOutcome::Passed,
            },
            SmokeResult {
                asset: broken.clone(),
                outcome: SmokeOutcome::Failed,
            },
        ]);
        assert_eq!(verdict.publication_status(), PublicationStatus::Blocked);
        assert_eq!(
            verdict,
            ReleaseGateVerdict::Blocked {
                failing: vec![broken]
            }
        );
    }
}
