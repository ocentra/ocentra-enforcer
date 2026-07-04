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
//! truth (the [`crate::distribution::TargetPlatform`] matrix + this
//! module's [`crate::ci::release_pipeline::asset_name`] naming) so the
//! shipped shell/PowerShell text can never drift from the Rust side's own
//! understanding of what a release asset is named — the scripts are
//! rendered, not hand-maintained prose that silently rots.

use crate::ci::release_pipeline::{self, BinaryVariant};
use crate::distribution::TargetPlatform;

/// Render the POSIX `install.sh` script. Pure function over `version`;
/// never touches disk or network itself (this module renders installer
/// SOURCE, it does not run one — running one is exactly what a consumer's
/// CI does with the rendered output).
#[must_use]
pub fn render_install_sh(version: &str) -> String {
    let default_variant = BinaryVariant::ci_default().asset_label();
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
    let default_variant = BinaryVariant::ci_default().asset_label();
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
    Ok(release_pipeline::asset_name(platform, variant, version))
}

/// Which C runtime a Linux host links against — the installer's own
/// `ldd --version` sniff, exposed as a typed enum so callers/tests never
/// pass a bare bool whose meaning is easy to invert by accident.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Libc {
    /// glibc (the default Linux target).
    Gnu,
    /// musl (Alpine and similar).
    Musl,
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

/// Minimal, dependency-free SHA-256 over `data`, returned as a lowercase
/// hex string. This crate intentionally carries no network/crypto crate
/// dependency for the live-download path yet (see
/// [`crate::distribution::Downloader`]'s module docs — that lands with
/// the adapter packs); this implementation exists purely so
/// [`verify_checksum`]'s fail-closed contract is provable in unit tests
/// today without adding a dependency this skeleton does not otherwise
/// need.
fn sha256_hex(data: &[u8]) -> String {
    // Public-domain-equivalent, textbook SHA-256 (FIPS 180-4). Not
    // performance-tuned -- this path runs once per downloaded asset in a
    // one-shot installer/CI-action invocation, not a hot loop.
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];

    let mut msg = data.to_vec();
    let bit_len = (data.len() as u64) * 8;
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&bit_len.to_be_bytes());

    for chunk in msg.chunks(64) {
        let mut w = [0u32; 64];
        for (i, word) in chunk.chunks(4).enumerate() {
            w[i] = u32::from_be_bytes([word[0], word[1], word[2], word[3]]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }

        let (mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh) =
            (h[0], h[1], h[2], h[3], h[4], h[5], h[6], h[7]);

        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = hh
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);

            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }

        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
        h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g);
        h[7] = h[7].wrapping_add(hh);
    }

    h.iter().map(|word| format!("{word:08x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::{
        render_install_ps1, render_install_sh, resolve_asset_name, sha256_hex, verify_checksum,
        DownloadedAsset, Libc,
    };
    use crate::ci::release_pipeline::BinaryVariant;
    use crate::error::InstallError;

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
        assert!(!script.contains("E:/"));
        assert!(!script.contains("C:/Projects"));
        assert!(!script.contains("/home/"));
        assert!(script.contains("uname -s"));
        assert!(script.contains("sha256sum"));
    }

    #[test]
    fn render_install_ps1_contains_no_hardcoded_local_absolute_path() {
        let script = render_install_ps1("0.1.0");
        assert!(!script.contains("E:/"));
        assert!(!script.contains("C:\\Projects"));
        assert!(script.contains("Get-FileHash"));
        assert!(script.contains("x86_64-pc-windows-msvc"));
    }

    #[test]
    fn both_scripts_default_to_the_lite_variant() {
        let sh = render_install_sh("0.1.0");
        let ps1 = render_install_ps1("0.1.0");
        assert!(sh.contains("ENFORCER_VARIANT:-lite"));
        assert!(ps1.contains(r#"else { "lite" }"#));
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
            assert!(name.contains(triple), "expected {name} to contain {triple}");
            assert!(name.contains("lite"));
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
