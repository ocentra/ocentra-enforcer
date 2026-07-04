#!/usr/bin/env sh
# enforcer installer (c10 release pipeline) -- POSIX sh, no Rust toolchain
# required. Detects uname -s/-m (glibc vs musl on Linux), downloads the
# matching release binary, checksum-verifies it, and installs it to a bin
# dir. Defaults to the `lite` variant for CI use; pass
# ENFORCER_VARIANT=full to opt into the full (coordination+UI) build.
set -eu

VERSION="${ENFORCER_VERSION:-0.1.0}"
VARIANT="${ENFORCER_VARIANT:-lite}"
INSTALL_DIR="${ENFORCER_INSTALL_DIR:-$HOME/.local/bin}"
RELEASE_BASE_URL="${ENFORCER_RELEASE_BASE_URL:-https://github.com/ocentra/enforcer/releases/download}"

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

asset="enforcer-v${VERSION}-${VARIANT}-${triple}.${ext}"
checksum_asset="${asset}.sha256"
url="${RELEASE_BASE_URL}/v${VERSION}/${asset}"
checksum_url="${RELEASE_BASE_URL}/v${VERSION}/${checksum_asset}"

tmp_dir="$(mktemp -d)"
trap 'rm -rf "$tmp_dir"' EXIT

echo "enforcer installer: downloading $asset"
curl -fsSL "$url" -o "$tmp_dir/$asset"
curl -fsSL "$checksum_url" -o "$tmp_dir/$checksum_asset"

expected_sum="$(awk '{print $1}' "$tmp_dir/$checksum_asset")"
actual_sum="$(sha256sum "$tmp_dir/$asset" | awk '{print $1}')"
if [ "$expected_sum" != "$actual_sum" ]; then
  echo "enforcer installer: checksum mismatch for $asset -- refusing to install (expected $expected_sum, got $actual_sum)" >&2
  exit 1
fi

mkdir -p "$INSTALL_DIR"
tar -xzf "$tmp_dir/$asset" -C "$tmp_dir"
install -m 0755 "$tmp_dir/enforcer" "$INSTALL_DIR/enforcer"

echo "enforcer installer: installed $INSTALL_DIR/enforcer ($VARIANT, v$VERSION, $triple)"
