//! Platform binary distribution/download seam (RUST_ARCHITECTURE.md,
//! "Distribution (codebase-memory model)"): resolve the correct released
//! binary for the current OS/arch (win/mac/linux incl. musl +
//! apple-silicon) and support the `enforcer install`/`enforcer update`
//! entrypoints. No runtime toolchain required by consumers.
//!
//! This module is the SKELETON only: target-triple resolution and the
//! release-asset naming convention are pure/testable here; the actual HTTP
//! fetch (a real download client, checksum verification against a
//! published manifest, and the atomic swap-in described in "Update UX")
//! is a follow-on fill per the adapter packs' sequencing — this seam gives
//! them one place to plug a [`Downloader`] implementation in without
//! touching [`crate::core`].

//! BOUNDARY-INVARIANT: release transport input is converted to canonical domain values.
//!
use crate::error::{InstallError, InstallResult};
use enforcer_domain::install_types::{ReleaseVersion, ResolvedBinary, TargetPlatform};

/// A released target platform. Mirrors the CI release matrix
/// (win/mac/linux incl. musl + apple-silicon) — RUST_ARCHITECTURE.md,
/// "Distribution (codebase-memory model)".
/// Resolve a target triple to its canonical common platform.
pub fn from_triple(triple: &str) -> InstallResult<TargetPlatform> {
    TargetPlatform::all()
        .iter()
        .copied()
        .find(|platform| platform.target_triple() == triple)
        .ok_or_else(|| InstallError::UnsupportedTarget {
            target: triple.to_owned(),
        })
}

/// Detect the current host's canonical release platform.
pub fn detect_host() -> InstallResult<TargetPlatform> {
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;
    match (os, arch) {
        ("windows", "x86_64") => Ok(TargetPlatform::WindowsX86_64),
        ("macos", "x86_64") => Ok(TargetPlatform::MacX86_64),
        ("macos", "aarch64") => Ok(TargetPlatform::MacAarch64),
        ("linux", "x86_64") => Ok(TargetPlatform::LinuxX86_64Gnu),
        ("linux", "aarch64") => Ok(TargetPlatform::LinuxAarch64Gnu),
        _ => Err(InstallError::UnsupportedTarget {
            target: format!("{os}-{arch}"),
        }),
    }
}

/// The seam a real downloader implements: fetch and checksum-verify a
/// release asset for `platform`/`version`, then place the extracted
/// binary at `install_path`. This crate's skeleton does not ship a live
/// HTTP implementation — that lands with the adapter packs per the
/// workpack's sequencing — but every caller (`core::install`,
/// `core::update`) is written against this trait so a deterministic offline
/// implementation can prove the install/update flow in tests today.
pub trait Downloader {
    /// Fetch, verify, and install the binary for `platform`/`version` to
    /// `install_path`.
    ///
    /// # Errors
    /// Returns [`InstallError::DistributionFailed`] on any network,
    /// checksum-mismatch, or extraction failure.
    fn fetch(
        &self,
        platform: TargetPlatform,
        version: &ReleaseVersion,
        install_path: &std::path::Path,
    ) -> InstallResult<ResolvedBinary>;
}

#[cfg(test)]
mod tests {
    use super::{
        detect_host, from_triple, Downloader, InstallError, ReleaseVersion, TargetPlatform,
    };
    use enforcer_domain::install_types::{InstallBinaryPath, ResolvedBinary};
    use std::path::{Path, PathBuf};

    #[test]
    fn every_platform_round_trips_through_its_target_triple(
    ) -> Result<(), Box<dyn std::error::Error>> {
        for platform in TargetPlatform::all() {
            let triple = platform.target_triple();
            let back = from_triple(triple)?;
            assert_eq!(back, *platform);
        }
        Ok(())
    }

    #[test]
    fn unknown_triple_is_an_unsupported_target_error() {
        let result = from_triple("sparc-unknown-solaris");
        assert!(matches!(
            result,
            Err(InstallError::UnsupportedTarget { .. })
        ));
    }

    #[test]
    fn windows_asset_name_uses_zip_extension() -> Result<(), Box<dyn std::error::Error>> {
        let version = ReleaseVersion::try_from("0.1.0".to_owned())
            .map_err(|error| format!("release version fixture is invalid: {error:?}"))?;
        let name = TargetPlatform::WindowsX86_64.asset_name(&version);
        assert_eq!(name, "enforcer-v0.1.0-x86_64-pc-windows-msvc.zip");
        Ok(())
    }

    #[test]
    fn unix_asset_names_use_tar_gz_extension() -> Result<(), Box<dyn std::error::Error>> {
        let version = ReleaseVersion::try_from("0.1.0".to_owned())
            .map_err(|error| format!("release version fixture is invalid: {error:?}"))?;
        let name = TargetPlatform::LinuxX86_64Musl.asset_name(&version);
        assert_eq!(name, "enforcer-v0.1.0-x86_64-unknown-linux-musl.tar.gz");

        let name = TargetPlatform::MacAarch64.asset_name(&version);
        assert_eq!(name, "enforcer-v0.1.0-aarch64-apple-darwin.tar.gz");
        Ok(())
    }

    #[test]
    fn detect_host_resolves_to_a_supported_platform_on_this_ci_matrix(
    ) -> Result<(), Box<dyn std::error::Error>> {
        // This crate's CI matrix (win/mac/linux x86_64/aarch64) is exactly
        // the released matrix, so detection must succeed wherever the
        // test suite itself runs.
        let platform = detect_host()?;
        assert!(TargetPlatform::all().contains(&platform));
        Ok(())
    }

    /// A deterministic downloader that always succeeds, for exercising callers of
    /// [`Downloader`] without a real network.
    struct DeterministicDownloader;

    impl Downloader for DeterministicDownloader {
        fn fetch(
            &self,
            platform: TargetPlatform,
            version: &ReleaseVersion,
            install_path: &Path,
        ) -> Result<ResolvedBinary, InstallError> {
            Ok(ResolvedBinary {
                platform,
                version: version.clone(),
                install_path: InstallBinaryPath::try_from(install_path.to_path_buf())?,
            })
        }
    }

    /// A deterministic downloader that always fails, for exercising the
    /// distribution-failure error path.
    struct RejectingDownloader;

    impl Downloader for RejectingDownloader {
        fn fetch(
            &self,
            platform: TargetPlatform,
            _version: &ReleaseVersion,
            _install_path: &Path,
        ) -> Result<ResolvedBinary, InstallError> {
            Err(InstallError::DistributionFailed {
                target: platform.target_triple().to_owned(),
                reason: "simulated network failure".to_owned(),
            })
        }
    }

    #[test]
    fn fake_downloader_resolves_a_binary() -> Result<(), Box<dyn std::error::Error>> {
        let downloader = DeterministicDownloader;
        let resolved = downloader.fetch(
            TargetPlatform::LinuxX86_64Gnu,
            &ReleaseVersion::try_from("0.1.0".to_owned())
                .map_err(|error| format!("release version fixture is invalid: {error:?}"))?,
            &std::env::temp_dir().join("enforcer"),
        )?;
        assert_eq!(resolved.platform, TargetPlatform::LinuxX86_64Gnu);
        assert_eq!(resolved.version.as_str(), "0.1.0");
        Ok(())
    }

    #[test]
    fn failing_downloader_surfaces_a_distribution_failed_error(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let downloader = RejectingDownloader;
        let version = ReleaseVersion::try_from("0.1.0".to_owned())
            .map_err(|error| format!("release version fixture is invalid: {error:?}"))?;
        let result = downloader.fetch(
            TargetPlatform::WindowsX86_64,
            &version,
            &PathBuf::from("C:/Program Files/enforcer/enforcer.exe"),
        );
        assert!(matches!(
            result,
            Err(InstallError::DistributionFailed { .. })
        ));
        Ok(())
    }
}
