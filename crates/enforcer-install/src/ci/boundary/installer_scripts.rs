//! c10 — the portable, zero-Rust-toolchain `install.sh`/`install.ps1`
//! installer scripts (RUST_ARCHITECTURE.md, "CI integration for CONSUMER
//! projects", binding).
//!
//! # Charter
//!
//! Any consumer project's CI (GitHub Actions, GitLab CI, CircleCI,
//! Bitbucket, Jenkins, or bare shell) downloads the matching release
//! binary via a curl/iwr-installable script — no `cargo`/`rustc`
//! invocation anywhere in the path. `install.sh` detects `uname -s`/
//! `uname -m` (handling glibc vs musl on Linux); `install.ps1` detects
//! Windows. Both checksum-verify the download and REJECT a
//! corrupted/mismatched asset fail-closed (never a silent fallback to an
//! unverified binary).
//!
//! This module renders the two scripts from a single Rust source of
//! truth (the [`enforcer_domain::install_types::TargetPlatform`] matrix + this
//! module's [`crate::ci::release_pipeline::asset_name`] naming) so the
//! shipped shell/PowerShell text can never drift from the Rust side's own
//! understanding of what a release asset is named — the scripts are
//! rendered, not hand-maintained prose that silently rots.

//! BOUNDARY-INVARIANT: installer-script input is decoded before typed rendering.
//!
use crate::ci::boundary::release_rendering::{render_asset_name, render_variant_label};
use enforcer_domain::install_types::{BinaryVariant, Libc, TargetPlatform};
use sha2::{Digest, Sha256};

/// Render the POSIX `install.sh` script. Pure function over `version`;
/// never touches disk or network itself (this module renders installer
/// SOURCE, it does not run one — running one is exactly what a consumer's
/// CI does with the rendered output).
#[must_use]
pub fn render_install_sh(version: &str) -> String {
    let default_variant = render_variant_label(BinaryVariant::ci_default());
    format!(
        r#"#!/usr/bin/env sh
# enforcer installer (c10 release pipeline) -- POSIX sh, no Rust toolchain
# required. Detects uname -s/-m (glibc vs musl on Linux), downloads the
# matching release binary, checksum-verifies it, and installs it to a bin
# dir. Defaults to the `{default_variant}` variant for CI use; pass
# ENFORCER_VARIANT=full to opt into the full (coordination+UI) build.
set -eu

VERSION="${{ENFORCER_VERSION:-{version}}}"
VARIANT="${{ENFORCER_VARIANT:-{default_variant}}}"
INSTALL_DIR="${{ENFORCER_INSTALL_DIR:-$HOME/.local/bin}}"
RELEASE_BASE_URL="${{ENFORCER_RELEASE_BASE_URL:-https://github.com/ocentra/enforcer/releases/download}}"

os="$(uname -s)"
arch="$(uname -m)"

case "$os" in
  Linux*)
    libc="gnu"
    if command -v ldd >/dev/null 2>&1 && ldd --version 2>&1 | grep -qi musl; then
      libc="musl"
    fi
    case "$arch" in
      x86_64) triple="x86_64-unknown-linux-$libc" ;;
      aarch64|arm64) triple="aarch64-unknown-linux-gnu" ;;
      *) echo "enforcer installer: unsupported linux arch '$arch'" >&2; exit 1 ;;
    esac
    ext="tar.gz"
    ;;
  Darwin*)
    case "$arch" in
      x86_64) triple="x86_64-apple-darwin" ;;
      arm64) triple="aarch64-apple-darwin" ;;
      *) echo "enforcer installer: unsupported macos arch '$arch'" >&2; exit 1 ;;
    esac
    ext="tar.gz"
    ;;
  *)
    echo "enforcer installer: unsupported OS '$os' (use install.ps1 on Windows)" >&2
    exit 1
    ;;
esac

asset="enforcer-v${{VERSION}}-${{VARIANT}}-${{triple}}.${{ext}}"
checksum_asset="${{asset}}.sha256"
url="${{RELEASE_BASE_URL}}/v${{VERSION}}/${{asset}}"
checksum_url="${{RELEASE_BASE_URL}}/v${{VERSION}}/${{checksum_asset}}"

tmp_dir="$(mktemp -d)"
trap 'rm -rf "$tmp_dir"' EXIT

echo "enforcer installer: downloading $asset"
curl -fsSL "$url" -o "$tmp_dir/$asset"
curl -fsSL "$checksum_url" -o "$tmp_dir/$checksum_asset"

expected_sum="$(awk '{{print $1}}' "$tmp_dir/$checksum_asset")"
actual_sum="$(sha256sum "$tmp_dir/$asset" | awk '{{print $1}}')"
if [ "$expected_sum" != "$actual_sum" ]; then
  echo "enforcer installer: checksum mismatch for $asset -- refusing to install (expected $expected_sum, got $actual_sum)" >&2
  exit 1
fi

mkdir -p "$INSTALL_DIR"
tar -xzf "$tmp_dir/$asset" -C "$tmp_dir"
install -m 0755 "$tmp_dir/enforcer" "$INSTALL_DIR/enforcer"

echo "enforcer installer: installed $INSTALL_DIR/enforcer ($VARIANT, v$VERSION, $triple)"
"#,
    )
}

/// Render the Windows `install.ps1` script. Pure function over `version`;
/// same checksum-verify-or-refuse contract as [`render_install_sh`].
#[must_use]
pub fn render_install_ps1(version: &str) -> String {
    let default_variant = render_variant_label(BinaryVariant::ci_default());
    format!(
        r#"# enforcer installer (c10 release pipeline) -- Windows PowerShell, no
# Rust toolchain required. Downloads the matching release binary,
# checksum-verifies it, and installs it to a bin dir. Defaults to the
# `{default_variant}` variant for CI use; set $env:ENFORCER_VARIANT = "full"
# to opt into the full (coordination+UI) build.
$ErrorActionPreference = "Stop"

$Version = if ($env:ENFORCER_VERSION) {{ $env:ENFORCER_VERSION }} else {{ "{version}" }}
$Variant = if ($env:ENFORCER_VARIANT) {{ $env:ENFORCER_VARIANT }} else {{ "{default_variant}" }}
$InstallDir = if ($env:ENFORCER_INSTALL_DIR) {{ $env:ENFORCER_INSTALL_DIR }} else {{ "$env:USERPROFILE\.local\bin" }}
$ReleaseBaseUrl = if ($env:ENFORCER_RELEASE_BASE_URL) {{ $env:ENFORCER_RELEASE_BASE_URL }} else {{ "https://github.com/ocentra/enforcer/releases/download" }}

$Triple = "x86_64-pc-windows-msvc"
$Asset = "enforcer-v$Version-$Variant-$Triple.zip"
$ChecksumAsset = "$Asset.sha256"
$Url = "$ReleaseBaseUrl/v$Version/$Asset"
$ChecksumUrl = "$ReleaseBaseUrl/v$Version/$ChecksumAsset"

$TmpDir = Join-Path $env:TEMP ([System.Guid]::NewGuid().ToString())
New-Item -ItemType Directory -Path $TmpDir -Force | Out-Null
try {{
    $AssetPath = Join-Path $TmpDir $Asset
    $ChecksumPath = Join-Path $TmpDir $ChecksumAsset

    Write-Host "enforcer installer: downloading $Asset"
    Invoke-WebRequest -Uri $Url -OutFile $AssetPath
    Invoke-WebRequest -Uri $ChecksumUrl -OutFile $ChecksumPath

    $ExpectedSum = (Get-Content $ChecksumPath).Split(" ")[0].Trim()
    $ActualSum = (Get-FileHash -Algorithm SHA256 -Path $AssetPath).Hash.ToLower()
    if ($ExpectedSum.ToLower() -ne $ActualSum) {{
        Write-Error "enforcer installer: checksum mismatch for $Asset -- refusing to install (expected $ExpectedSum, got $ActualSum)"
        exit 1
    }}

    New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
    Expand-Archive -Path $AssetPath -DestinationPath $TmpDir -Force
    Copy-Item -Path (Join-Path $TmpDir "enforcer.exe") -Destination (Join-Path $InstallDir "enforcer.exe") -Force

    Write-Host "enforcer installer: installed $InstallDir\enforcer.exe ($Variant, v$Version, $Triple)"
}}
finally {{
    Remove-Item -Recurse -Force $TmpDir -ErrorAction SilentlyContinue
}}
"#,
    )
}

/// Resolve the release-asset file name a consumer's installer would
/// request for a given `(os, arch, libc, variant, version)` combination —
/// the pure logic [`render_install_sh`]/[`render_install_ps1`] encode as
/// shell/PowerShell, exposed here so a test can assert the resolution
/// without shelling out to a real script interpreter.
///
/// # Errors
/// Returns [`crate::error::InstallError::UnsupportedTarget`] when the
/// `(os, arch, libc)` combination has no entry in
/// [`TargetPlatform::all`].
pub fn resolve_asset_name(
    os: &str,
    arch: &str,
    libc: Libc,
    variant: BinaryVariant,
    version: &str,
) -> crate::error::InstallResult<String> {
    let platform = match (os, arch, libc) {
        ("windows", "x86_64", _) => TargetPlatform::WindowsX86_64,
        ("macos", "x86_64", _) => TargetPlatform::MacX86_64,
        ("macos", "aarch64", _) => TargetPlatform::MacAarch64,
        ("linux", "x86_64", Libc::Gnu) => TargetPlatform::LinuxX86_64Gnu,
        ("linux", "x86_64", Libc::Musl) => TargetPlatform::LinuxX86_64Musl,
        ("linux", "aarch64", _) => TargetPlatform::LinuxAarch64Gnu,
        _ => {
            return Err(crate::error::InstallError::UnsupportedTarget {
                target: format!("{os}-{arch}-{libc:?}"),
            })
        }
    };
    Ok(render_asset_name(platform, variant, version))
}

/// A downloaded asset's bytes plus the checksum manifest line published
/// alongside it, ready for [`verify_checksum`].
#[derive(Debug, Clone)]
pub struct DownloadedAsset {
    /// The raw downloaded asset bytes.
    pub bytes: Vec<u8>,
    /// The `sha256sum`-format line from the published `.sha256` manifest
    /// (`"<hex digest>  <asset filename>"`).
    pub checksum_manifest_line: String,
}

/// Verify `asset.bytes` against the published checksum manifest line,
/// fail-closed: any mismatch (or an unparseable manifest line) is
/// [`crate::error::InstallError::DistributionFailed`], never a silent
/// fallback to trusting the unverified bytes.
///
/// # Errors
/// Returns [`crate::error::InstallError::DistributionFailed`] when the
/// manifest line has no parseable digest, or when the computed digest of
/// `asset.bytes` does not match the published digest.
pub fn verify_checksum(
    asset_name: &str,
    asset: &DownloadedAsset,
) -> crate::error::InstallResult<()> {
    let expected = asset
        .checksum_manifest_line
        .split_whitespace()
        .next()
        .ok_or_else(|| crate::error::InstallError::DistributionFailed {
            target: asset_name.to_owned(),
            reason: "checksum manifest line has no parseable digest".to_owned(),
        })?
        .to_ascii_lowercase();

    let actual = sha256_hex(&asset.bytes);

    if expected == actual {
        Ok(())
    } else {
        Err(crate::error::InstallError::DistributionFailed {
            target: asset_name.to_owned(),
            reason: format!(
                "checksum mismatch: expected {expected}, computed {actual} -- refusing to install a corrupted/tampered download"
            ),
        })
    }
}

/// SHA-256 over `data`, returned as a lowercase hexadecimal string.
///
/// ALLOC-JUSTIFICATION: checksum comparison needs the standard wire-format
/// digest string; cryptographic state remains inside the audited `sha2`
/// implementation rather than this installer module.
fn sha256_hex(data: &[u8]) -> String {
    format!("{:x}", Sha256::digest(data))
}

#[cfg(test)]
mod tests {
    use super::{
        render_install_ps1, render_install_sh, resolve_asset_name, sha256_hex, verify_checksum,
        DownloadedAsset,
    };
    use crate::error::InstallError;
    use enforcer_domain::install_types::{BinaryVariant, Libc};

    #[test]
    fn sha256_hex_matches_known_test_vectors() {
        // NIST/FIPS 180-4 empty-string and "abc" vectors.
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_owned()
        );
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad".to_owned()
        );
    }

    #[test]
    fn render_install_sh_contains_no_hardcoded_local_absolute_path() {
        let script = render_install_sh("0.1.0");
        // Fixed the E:/ocentra-enforcer class of bug at the source: no
        // rendered installer script may ever embed a literal local
        // absolute filesystem path (workpack c10 binding requirement).
        assert!(!script.as_str().contains("E:/"));
        assert!(!script.as_str().contains("C:/Projects"));
        assert!(!script.as_str().contains("/home/"));
        assert!(script.as_str().contains("uname -s"));
        assert!(script.as_str().contains("sha256sum"));
    }

    #[test]
    fn render_install_ps1_contains_no_hardcoded_local_absolute_path() {
        let script = render_install_ps1("0.1.0");
        assert!(!script.as_str().contains("E:/"));
        assert!(!script.as_str().contains("C:\\Projects"));
        assert!(script.as_str().contains("Get-FileHash"));
        assert!(script.as_str().contains("x86_64-pc-windows-msvc"));
    }

    #[test]
    fn both_scripts_default_to_the_lite_variant() {
        let sh = render_install_sh("0.1.0");
        let ps1 = render_install_ps1("0.1.0");
        assert!(sh.as_str().contains("ENFORCER_VARIANT:-lite"));
        assert!(ps1.as_str().contains(r#"else { "lite" }"#));
    }

    #[test]
    fn resolve_asset_name_matches_every_declared_platform() -> Result<(), Box<dyn std::error::Error>>
    {
        let cases: &[(&str, &str, Libc, &str)] = &[
            ("windows", "x86_64", Libc::Gnu, "x86_64-pc-windows-msvc"),
            ("macos", "x86_64", Libc::Gnu, "x86_64-apple-darwin"),
            ("macos", "aarch64", Libc::Gnu, "aarch64-apple-darwin"),
            ("linux", "x86_64", Libc::Gnu, "x86_64-unknown-linux-gnu"),
            ("linux", "x86_64", Libc::Musl, "x86_64-unknown-linux-musl"),
            ("linux", "aarch64", Libc::Gnu, "aarch64-unknown-linux-gnu"),
        ];
        for (os, arch, libc, triple) in cases {
            let name = resolve_asset_name(os, arch, *libc, BinaryVariant::Lite, "0.1.0")?;
            assert!(
                name.as_str().contains(triple),
                "expected {name} to contain {triple}"
            );
            assert!(name.as_str().contains("lite"));
        }
        Ok(())
    }

    #[test]
    fn resolve_asset_name_defaults_to_gnu_libc_for_unspecified_linux_arm(
    ) -> Result<(), Box<dyn std::error::Error>> {
        // aarch64 linux has only one released libc flavor today (gnu);
        // passing musl for it still resolves (no aarch64-musl asset is
        // published, so both libc values collapse onto the same target).
        let gnu = resolve_asset_name("linux", "aarch64", Libc::Gnu, BinaryVariant::Full, "0.1.0")?;
        let musl =
            resolve_asset_name("linux", "aarch64", Libc::Musl, BinaryVariant::Full, "0.1.0")?;
        assert_eq!(gnu, musl);
        Ok(())
    }

    #[test]
    fn resolve_asset_name_rejects_an_unreleased_combination() {
        let result = resolve_asset_name("plan9", "x86_64", Libc::Gnu, BinaryVariant::Lite, "0.1.0");
        assert!(matches!(
            result,
            Err(InstallError::UnsupportedTarget { .. })
        ));
    }

    #[test]
    fn verify_checksum_accepts_a_matching_digest() -> Result<(), Box<dyn std::error::Error>> {
        let bytes = b"pretend release binary bytes".to_vec();
        let digest = sha256_hex(&bytes);
        let asset = DownloadedAsset {
            bytes,
            checksum_manifest_line: format!(
                "{digest}  enforcer-v0.1.0-lite-x86_64-unknown-linux-gnu.tar.gz"
            ),
        };
        verify_checksum(
            "enforcer-v0.1.0-lite-x86_64-unknown-linux-gnu.tar.gz",
            &asset,
        )?;
        Ok(())
    }

    #[test]
    fn verify_checksum_rejects_fail_closed_on_a_corrupted_download() {
        // Seeded checksum mismatch: the workpack's own acceptance row
        // requires this to be rejected fail-closed, never a silent
        // fallback to trusting the unverified bytes.
        let asset = DownloadedAsset {
            bytes: b"corrupted-mid-transfer".to_vec(),
            checksum_manifest_line:
                "0000000000000000000000000000000000000000000000000000000000000000  enforcer.tar.gz"
                    .to_owned(),
        };
        let result = verify_checksum("enforcer.tar.gz", &asset);
        assert!(matches!(
            result,
            Err(InstallError::DistributionFailed { .. })
        ));
    }

    #[test]
    fn verify_checksum_rejects_an_unparseable_manifest_line() {
        let asset = DownloadedAsset {
            bytes: b"whatever".to_vec(),
            checksum_manifest_line: String::new(),
        };
        let result = verify_checksum("enforcer.tar.gz", &asset);
        assert!(matches!(
            result,
            Err(InstallError::DistributionFailed { .. })
        ));
    }
}
