//! Typed release-gating domain for Enforcer's own binary distribution.
//!
//! Archive names, version tags, and CI input stay in
//! [`super::boundary::release_rendering`]. This module only decides whether
//! the typed platform-and-variant assets are safe to publish.

use crate::distribution::TargetPlatform;

/// The Cargo-feature binary variants released for each target platform.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BinaryVariant {
    /// Includes the coordination hub and UI.
    Full,
    /// Headless CI-oriented binary with the smaller feature surface.
    Lite,
}

impl BinaryVariant {
    /// Every released variant in deterministic CI order.
    #[must_use]
    pub fn all() -> &'static [Self] {
        &[Self::Lite, Self::Full]
    }

    /// The variant selected when CI has no explicit feature choice.
    #[must_use]
    pub fn ci_default() -> Self {
        Self::Lite
    }
}

/// A typed asset that must pass a smoke run before publication.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseAsset {
    /// Target platform selected by the distribution matrix.
    pub platform: TargetPlatform,
    /// Binary feature variant selected by the distribution matrix.
    pub variant: BinaryVariant,
}

/// The typed result of one asset's smoke run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmokeResult {
    /// Asset tested by the smoke adapter.
    pub asset: ReleaseAsset,
    /// Explicit result, never an ambiguous boolean flag.
    pub outcome: SmokeOutcome,
}

/// The decision-relevant smoke outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmokeOutcome {
    /// Both release fixtures completed with their required outcomes.
    Passed,
    /// A fixture panicked, hung, or returned an unexpected exit status.
    Failed,
}

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

/// The pre-publish decision and any assets that require repair.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReleaseGateVerdict {
    /// Every release asset passed its smoke run.
    Publish,
    /// At least one release asset failed its smoke run.
    Blocked {
        /// Assets that prevented publication.
        failing: Vec<ReleaseAsset>,
    },
}

impl ReleaseGateVerdict {
    fn from_failing(failing: Vec<ReleaseAsset>) -> Self {
        if failing.is_empty() {
            Self::Publish
        } else {
            Self::Blocked { failing }
        }
    }

    /// The typed permission derived from the complete smoke sweep.
    #[must_use]
    pub fn publication_status(&self) -> PublicationStatus {
        match self {
            Self::Publish => PublicationStatus::Approved,
            Self::Blocked { .. } => PublicationStatus::Blocked,
        }
    }
}

/// Whether the release may proceed to publication.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PublicationStatus {
    /// Every required smoke run passed.
    Approved,
    /// One or more required smoke runs failed.
    Blocked,
}

#[cfg(test)]
mod tests {
    use super::{
        gate_release, BinaryVariant, PublicationStatus, ReleaseAsset, ReleaseGateVerdict,
        SmokeOutcome, SmokeResult,
    };
    use crate::distribution::TargetPlatform;

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
