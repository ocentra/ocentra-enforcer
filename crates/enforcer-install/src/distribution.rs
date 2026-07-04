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

use crate::error::{InstallError, InstallResult};

/// A released target platform. Mirrors the CI release matrix
/// (win/mac/linux incl. musl + apple-silicon) — RUST_ARCHITECTURE.md,
/// "Distribution (codebase-memory model)".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TargetPlatform {
    /// `x86_64-pc-windows-msvc`.
    WindowsX86_64,
    /// `x86_64-apple-darwin`.
    MacX86_64,
    /// `aarch64-apple-darwin` (apple-silicon).
    MacAarch64,
    /// `x86_64-unknown-linux-gnu`.
    LinuxX86_64Gnu,
    /// `x86_64-unknown-linux-musl`.
    LinuxX86_64Musl,
    /// `aarch64-unknown-linux-gnu`.
    LinuxAarch64Gnu,
}

impl TargetPlatform {
    /// The Rust target-triple string this platform corresponds to, used
    /// both to name release assets and to resolve the current host.
    #[must_use]
    pub fn target_triple(self) -> &'static str {
        match self {
            Self::WindowsX86_64 => "x86_64-pc-windows-msvc",
            Self::MacX86_64 => "x86_64-apple-darwin",
            Self::MacAarch64 => "aarch64-apple-darwin",
            Self::LinuxX86_64Gnu => "x86_64-unknown-linux-gnu",
            Self::LinuxX86_64Musl => "x86_64-unknown-linux-musl",
            Self::LinuxAarch64Gnu => "aarch64-unknown-linux-gnu",
        }
    }

    /// Every platform in the released matrix, for exhaustive iteration in
    /// tests and doctor-style "is my platform supported" checks.
    #[must_use]
    pub fn all() -> &'static [Self] {
        &[
            Self::WindowsX86_64,
            Self::MacX86_64,
            Self::MacAarch64,
            Self::LinuxX86_64Gnu,
            Self::LinuxX86_64Musl,
            Self::LinuxAarch64Gnu,
        ]
    }

    /// Resolve a target-triple string to a known [`TargetPlatform`].
    ///
    /// # Errors
    /// Returns [`InstallError::UnsupportedTarget`] if `triple` does not
    /// match any entry in [`Self::all`] (the current OS/arch has no
    /// published binary).
    pub fn from_triple(triple: &str) -> InstallResult<Self> {
        Self::all()
            .iter()
            .copied()
            .find(|p| p.target_triple() == triple)
            .ok_or_else(|| InstallError::UnsupportedTarget {
                target: triple.to_owned(),
            })
    }

    /// Detect the current build host's target platform from
    /// `cfg!`-visible OS/arch, preferring the glibc-linked Linux variant
    /// (musl is an explicit opt-in some consumers choose, not the
    /// autodetected default here).
    ///
    /// # Errors
    /// Returns [`InstallError::UnsupportedTarget`] if the running OS/arch
    /// combination has no entry in [`Self::all`] (e.g. an exotic
    /// architecture with no released binary) — no runtime toolchain is
    /// required by consumers, but an unreleased combination is a detected
    /// error, never a silent no-op.
    pub fn detect_host() -> InstallResult<Self> {
        let os = std::env::consts::OS;
        let arch = std::env::consts::ARCH;
        match (os, arch) {
            ("windows", "x86_64") => Ok(Self::WindowsX86_64),
            ("macos", "x86_64") => Ok(Self::MacX86_64),
            ("macos", "aarch64") => Ok(Self::MacAarch64),
            ("linux", "x86_64") => Ok(Self::LinuxX86_64Gnu),
            ("linux", "aarch64") => Ok(Self::LinuxAarch64Gnu),
            _ => Err(InstallError::UnsupportedTarget {
                target: format!("{os}-{arch}"),
            }),
        }
    }

    /// The release-asset file name for this platform, given a version
    /// tag (e.g. `enforcer-v0.1.0-x86_64-pc-windows-msvc.zip`). Windows
    /// ships a `.zip`; every other platform ships a `.tar.gz` — matching
    /// the c10 `cargo-dist`-driven release pipeline's own naming.
    #[must_use]
    pub fn asset_name(self, version: &str) -> String {
        let ext = if matches!(self, Self::WindowsX86_64) {
            "zip"
        } else {
            "tar.gz"
        };
        format!("enforcer-v{version}-{}.{ext}", self.target_triple())
    }
}

/// A resolved binary ready to install: the platform it targets, the
/// release-asset name it came from, and the absolute path it was
/// installed to (or will be installed to, for a `--dry-run` plan).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedBinary {
    /// The target platform this binary was resolved for.
    pub platform: TargetPlatform,
    /// The release version string (e.g. `"0.1.0"`).
    pub version: String,
    /// Absolute path the binary is (or will be) installed to.
    pub install_path: std::path::PathBuf,
}

/// The seam a real downloader implements: fetch and checksum-verify a
/// release asset for `platform`/`version`, then place the extracted
/// binary at `install_path`. This crate's skeleton does not ship a live
/// HTTP implementation — that lands with the adapter packs per the
/// workpack's sequencing — but every caller (`core::install`,
/// `core::update`) is written against this trait so a fake/offline
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
        version: &str,
        install_path: &std::path::Path,
    ) -> InstallResult<ResolvedBinary>;
}

#[cfg(test)]
mod tests {
    use super::{Downloader, InstallError, ResolvedBinary, TargetPlatform};
    use std::path::{Path, PathBuf};

    #[test]
    fn every_platform_round_trips_through_its_target_triple(
    ) -> Result<(), Box<dyn std::error::Error>> {
        for platform in TargetPlatform::all() {
            let triple = platform.target_triple();
            let back = TargetPlatform::from_triple(triple)?;
            assert_eq!(back, *platform);
        }
        Ok(())
    }

    #[test]
    fn unknown_triple_is_an_unsupported_target_error() {
        let result = TargetPlatform::from_triple("sparc-unknown-solaris");
        assert!(matches!(
            result,
            Err(InstallError::UnsupportedTarget { .. })
        ));
    }

    #[test]
    fn windows_asset_name_uses_zip_extension() {
        let name = TargetPlatform::WindowsX86_64.asset_name("0.1.0");
        assert_eq!(name, "enforcer-v0.1.0-x86_64-pc-windows-msvc.zip");
    }

    #[test]
    fn unix_asset_names_use_tar_gz_extension() {
        let name = TargetPlatform::LinuxX86_64Musl.asset_name("0.1.0");
        assert_eq!(name, "enforcer-v0.1.0-x86_64-unknown-linux-musl.tar.gz");

        let name = TargetPlatform::MacAarch64.asset_name("0.1.0");
        assert_eq!(name, "enforcer-v0.1.0-aarch64-apple-darwin.tar.gz");
    }

    #[test]
    fn detect_host_resolves_to_a_supported_platform_on_this_ci_matrix(
    ) -> Result<(), Box<dyn std::error::Error>> {
        // This crate's CI matrix (win/mac/linux x86_64/aarch64) is exactly
        // the released matrix, so detection must succeed wherever the
        // test suite itself runs.
        let platform = TargetPlatform::detect_host()?;
        assert!(TargetPlatform::all().contains(&platform));
        Ok(())
    }

    /// A fake downloader that always succeeds, for exercising callers of
    /// [`Downloader`] without a real network.
    struct FakeDownloader;

    impl Downloader for FakeDownloader {
        fn fetch(
            &self,
            platform: TargetPlatform,
            version: &str,
            install_path: &Path,
        ) -> Result<ResolvedBinary, InstallError> {
            Ok(ResolvedBinary {
                platform,
                version: version.to_owned(),
                install_path: install_path.to_path_buf(),
            })
        }
    }

    /// A fake downloader that always fails, for exercising the
    /// distribution-failure error path.
    struct FailingDownloader;

    impl Downloader for FailingDownloader {
        fn fetch(
            &self,
            platform: TargetPlatform,
            _version: &str,
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
        let downloader = FakeDownloader;
        let resolved = downloader.fetch(
            TargetPlatform::LinuxX86_64Gnu,
            "0.1.0",
            &PathBuf::from("/usr/local/bin/enforcer"),
        )?;
        assert_eq!(resolved.platform, TargetPlatform::LinuxX86_64Gnu);
        assert_eq!(resolved.version, "0.1.0");
        Ok(())
    }

    #[test]
    fn failing_downloader_surfaces_a_distribution_failed_error() {
        let downloader = FailingDownloader;
        let result = downloader.fetch(
            TargetPlatform::WindowsX86_64,
            "0.1.0",
            &PathBuf::from("C:/Program Files/enforcer/enforcer.exe"),
        );
        assert!(matches!(
            result,
            Err(InstallError::DistributionFailed { .. })
        ));
    }
}
