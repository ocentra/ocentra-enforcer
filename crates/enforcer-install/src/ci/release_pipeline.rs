//! c10 — the enforcer's OWN release pipeline (cargo-dist-style):
//! plan -> build -> host -> publish across the win/mac/linux (+musl,
//! +apple-silicon) matrix declared in RUST_ARCHITECTURE.md's
//! "Distribution" section, producing checksummed release binaries for
//! BOTH the `full` and `lite` (arc-22 Cargo feature split) variants.
//!
//! # Charter — this repo's release, not a consumer's CI
//!
//! "CI integration for CONSUMER projects (c10 -- a different surface from
//! AI-harness install)" (RUST_ARCHITECTURE.md, binding): *this* repo's own
//! CI builds and PUBLISHES the release; every consumer project's CI merely
//! POINTS AT it. This module is the single producer side: it computes the
//! release matrix, the per-asset naming (delegating platform/triple
//! resolution to [`crate::distribution::TargetPlatform`], which already
//! owns that seam — this module does not duplicate it), the pre-publish
//! cross-platform smoke gate contract, and the version-pin manifest a
//! consumer records in its own repo.
//!
//! Distinct from [`crate::emitters::consumer_ci`] (c07), which writes the
//! PER-CONSUMER workflow set (codeql/secret-scan/sbom/etc.) for a repo that
//! *installs* the enforcer. This module never emits a consumer-workflow
//! file, and that module never emits a release-pipeline or Action file.

use crate::distribution::TargetPlatform;

/// The two Cargo-feature-split binary variants (arc-22) built and
/// released per platform. CI tooling (installer/GH Action/npm wrapper)
/// defaults to `Lite`; `Full` is an explicit opt-in
/// (RUST_ARCHITECTURE.md, "`full` vs `lite` binary variants").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BinaryVariant {
    /// Ships the coordination hub (`enforcer-coordination`) and the UI
    /// (`enforcer-ui`) — the interactive/human-driven surface.
    Full,
    /// Excludes the coordination hub and UI: no lanes, no mail, no Tauri
    /// surface to serve. Smaller, faster, smaller attack surface. The
    /// default fetched variant for headless mechanical CI use.
    Lite,
}

impl BinaryVariant {
    /// Every released variant, in a fixed order (`Lite` first — it is the
    /// CI-default and should sort ahead of `Full` in generated matrices).
    #[must_use]
    pub fn all() -> &'static [Self] {
        &[Self::Lite, Self::Full]
    }

    /// The variant CI tooling defaults to when the caller does not
    /// explicitly opt into `full` (RUST_ARCHITECTURE.md, binding).
    #[must_use]
    pub fn ci_default() -> Self {
        Self::Lite
    }

    /// The label used in release-asset file names and the Cargo
    /// `--features` invocation (`""` for `full`, since `full` is already
    /// the crate's `default` feature set per arc-22's `Cargo.toml`).
    #[must_use]
    pub fn asset_label(self) -> &'static str {
        match self {
            Self::Full => "full",
            Self::Lite => "lite",
        }
    }

    /// The `cargo build` feature flags this variant compiles with,
    /// mirroring `enforcer-cli/Cargo.toml`'s `default = ["full"]` /
    /// `lite = []` feature split.
    #[must_use]
    pub fn cargo_features(self) -> &'static [&'static str] {
        match self {
            Self::Full => &["full"],
            Self::Lite => &["lite"],
        }
    }
}

/// One release asset the pipeline plans to build+publish: a
/// (platform, variant) pair and the exact asset file name a consumer's
/// installer resolves.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseAsset {
    /// Target platform.
    pub platform: TargetPlatform,
    /// Binary variant (`full`/`lite`).
    pub variant: BinaryVariant,
    /// The release-asset file name, e.g.
    /// `enforcer-v0.1.0-lite-x86_64-pc-windows-msvc.zip`.
    pub asset_name: String,
    /// The checksum-manifest file name published alongside the asset
    /// (`<asset_name>.sha256`) — one manifest entry per asset, never a
    /// single repo-wide checksum file a consumer must parse to find its
    /// own entry.
    pub checksum_name: String,
}

/// Compute the release-asset name for a `(platform, variant, version)`
/// triple. Delegates the platform/extension half to
/// [`TargetPlatform::asset_name`] and inserts the variant label so `full`
/// and `lite` assets never collide on disk or in a GitHub Release's asset
/// list.
#[must_use]
pub fn asset_name(platform: TargetPlatform, variant: BinaryVariant, version: &str) -> String {
    // Built directly from the same (triple, extension) facts
    // `TargetPlatform::asset_name` uses, rather than parsing that
    // function's rendered string back apart -- so asset names sort and
    // glob predictably: `enforcer-v{version}-{variant}-{triple}.{ext}`.
    let ext = if matches!(platform, TargetPlatform::WindowsX86_64) {
        "zip"
    } else {
        "tar.gz"
    };
    format!(
        "enforcer-v{version}-{}-{}.{ext}",
        variant.asset_label(),
        platform.target_triple()
    )
}

/// Compute the FULL release matrix: every declared [`TargetPlatform`]
/// crossed with every [`BinaryVariant`] — this is what CI's
/// plan/build/host/publish pipeline iterates to build+publish every
/// asset, and what the pre-publish smoke gate iterates to test every
/// asset before the release goes live.
#[must_use]
pub fn release_matrix(version: &str) -> Vec<ReleaseAsset> {
    let mut assets = Vec::with_capacity(TargetPlatform::all().len() * BinaryVariant::all().len());
    for platform in TargetPlatform::all() {
        for variant in BinaryVariant::all() {
            let name = asset_name(*platform, *variant, version);
            assets.push(ReleaseAsset {
                platform: *platform,
                variant: *variant,
                asset_name: name.clone(),
                checksum_name: format!("{name}.sha256"),
            });
        }
    }
    assets
}

/// Outcome of the pre-publish cross-platform smoke gate for one release
/// asset: did the built binary run at all (both variants, every target
/// platform) against a minimal fail/pass smoke fixture, per
/// RUST_ARCHITECTURE.md's binding "a release is never published broken"
/// requirement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmokeResult {
    /// Which asset this result is for.
    pub asset: ReleaseAsset,
    /// `true` only if the binary ran the fail fixture and reported a
    /// blocking violation (non-panic, exit code
    /// [`enforcer_core::exit_codes::ExitCode::Violations`]) AND ran the
    /// pass fixture cleanly (exit code
    /// [`enforcer_core::exit_codes::ExitCode::Success`]). A panic, a
    /// hang, or a fail-fixture that comes back clean all count as `false`.
    pub passed: bool,
    /// Human-readable detail; empty when `passed`.
    pub detail: String,
}

/// Gate a release given a full smoke-test sweep. A release is blocked the
/// moment ANY asset's smoke result is not `passed` — never averaged,
/// never "most platforms passed" (RUST_ARCHITECTURE.md, binding: "a
/// broken/panicking binary on ANY platform blocks the release entirely").
#[must_use]
pub fn gate_release(results: &[SmokeResult]) -> ReleaseGateVerdict {
    let failing: Vec<ReleaseAsset> = results
        .iter()
        .filter(|r| !r.passed)
        .map(|r| r.asset.clone())
        .collect();
    if failing.is_empty() {
        ReleaseGateVerdict::Publish
    } else {
        ReleaseGateVerdict::Blocked { failing }
    }
}

/// The pre-publish gate's verdict.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReleaseGateVerdict {
    /// Every asset's smoke result passed; the release may be published.
    Publish,
    /// At least one asset failed its smoke test; the release is blocked.
    /// Carries the failing assets so the CI log names them explicitly.
    Blocked {
        /// The asset(s) whose smoke test did not pass.
        failing: Vec<ReleaseAsset>,
    },
}

impl ReleaseGateVerdict {
    /// `true` iff the verdict is [`Self::Publish`].
    #[must_use]
    pub fn may_publish(&self) -> bool {
        matches!(self, Self::Publish)
    }
}

/// A consumer's pinned-version record — the lockfile-style version stamp
/// the `github-actions`/generic install adapter writes into a consumer's
/// own repo (RUST_ARCHITECTURE.md, binding: "Version-pinning guidance,
/// not blind latest"). Pin-by-default; `channel: Latest` is an explicit
/// opt-in a consumer chooses, never the default.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionPin {
    /// The release channel this consumer's install resolves against.
    pub channel: ReleaseChannel,
}

/// Which release a consumer's installer resolves to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReleaseChannel {
    /// A fixed version tag (e.g. `"0.1.0"`) — the DEFAULT. An
    /// enforcer-side rule change cannot silently break this consumer's CI
    /// until the consumer deliberately bumps the pin.
    Pinned {
        /// The pinned version string.
        version: String,
    },
    /// Always resolves to the newest published release. An explicit,
    /// deliberate opt-in — never the installer's default.
    Latest,
}

impl Default for VersionPin {
    /// The default pin is a channel that still requires an explicit
    /// version string to be meaningful; callers construct
    /// [`ReleaseChannel::Pinned`] with a real version rather than relying
    /// on this `Default` for anything but "no channel chosen yet" in a
    /// builder context. Defaulting to [`ReleaseChannel::Latest`] would
    /// violate the binding pin-by-default contract, so this intentionally
    /// has no meaningful zero-value and callers must supply a version.
    fn default() -> Self {
        Self {
            channel: ReleaseChannel::Pinned {
                version: String::new(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        asset_name, gate_release, release_matrix, BinaryVariant, ReleaseAsset, ReleaseGateVerdict,
        SmokeResult,
    };
    use crate::distribution::TargetPlatform;

    #[test]
    fn ci_default_variant_is_lite() {
        assert_eq!(BinaryVariant::ci_default(), BinaryVariant::Lite);
    }

    #[test]
    fn asset_name_embeds_variant_between_version_and_triple() {
        let name = asset_name(TargetPlatform::WindowsX86_64, BinaryVariant::Lite, "0.1.0");
        assert_eq!(name, "enforcer-v0.1.0-lite-x86_64-pc-windows-msvc.zip");

        let name = asset_name(TargetPlatform::MacAarch64, BinaryVariant::Full, "0.1.0");
        assert_eq!(name, "enforcer-v0.1.0-full-aarch64-apple-darwin.tar.gz");
    }

    #[test]
    fn release_matrix_covers_every_platform_times_every_variant() {
        let matrix = release_matrix("0.1.0");
        assert_eq!(
            matrix.len(),
            TargetPlatform::all().len() * BinaryVariant::all().len()
        );
        // every asset name is unique -- full/lite never collide.
        let mut names: Vec<&str> = matrix.iter().map(|a| a.asset_name.as_str()).collect();
        let unique_count = {
            names.sort_unstable();
            names.dedup();
            names.len()
        };
        assert_eq!(unique_count, matrix.len());
    }

    #[test]
    fn release_matrix_checksum_name_is_asset_name_plus_sha256_suffix() {
        let matrix = release_matrix("0.1.0");
        for asset in &matrix {
            assert_eq!(asset.checksum_name, format!("{}.sha256", asset.asset_name));
        }
    }

    fn sample_asset(passed_platform: TargetPlatform) -> ReleaseAsset {
        ReleaseAsset {
            platform: passed_platform,
            variant: BinaryVariant::Lite,
            asset_name: asset_name(passed_platform, BinaryVariant::Lite, "0.1.0"),
            checksum_name: "irrelevant.sha256".to_owned(),
        }
    }

    #[test]
    fn gate_release_publishes_when_every_smoke_result_passed() {
        let results = vec![
            SmokeResult {
                asset: sample_asset(TargetPlatform::WindowsX86_64),
                passed: true,
                detail: String::new(),
            },
            SmokeResult {
                asset: sample_asset(TargetPlatform::MacAarch64),
                passed: true,
                detail: String::new(),
            },
        ];
        assert_eq!(gate_release(&results), ReleaseGateVerdict::Publish);
        assert!(gate_release(&results).may_publish());
    }

    #[test]
    fn gate_release_blocks_when_any_single_platform_smoke_fails(
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Seeded violation: every platform but one passes -- the release
        // must still be blocked entirely, never "mostly published".
        let broken = sample_asset(TargetPlatform::LinuxX86_64Musl);
        let results = vec![
            SmokeResult {
                asset: sample_asset(TargetPlatform::WindowsX86_64),
                passed: true,
                detail: String::new(),
            },
            SmokeResult {
                asset: sample_asset(TargetPlatform::MacX86_64),
                passed: true,
                detail: String::new(),
            },
            SmokeResult {
                asset: broken.clone(),
                passed: false,
                detail: "binary panicked on the fail-fixture smoke run".to_owned(),
            },
        ];
        let verdict = gate_release(&results);
        assert!(!verdict.may_publish());
        match verdict {
            ReleaseGateVerdict::Blocked { failing } => {
                assert_eq!(failing, vec![broken]);
            }
            ReleaseGateVerdict::Publish => return Err("expected Blocked".into()),
        }
        Ok(())
    }
}
