# enforcer installer (c10 release pipeline) -- Windows PowerShell, no
# Rust toolchain required. Downloads the matching release binary,
# checksum-verifies it, and installs it to a bin dir. Defaults to the
# `lite` variant for CI use; set $env:ENFORCER_VARIANT = "full"
# to opt into the full (coordination+UI) build.
$ErrorActionPreference = "Stop"

$Version = if ($env:ENFORCER_VERSION) { $env:ENFORCER_VERSION } else { "0.1.0" }
$Variant = if ($env:ENFORCER_VARIANT) { $env:ENFORCER_VARIANT } else { "lite" }
$InstallDir = if ($env:ENFORCER_INSTALL_DIR) { $env:ENFORCER_INSTALL_DIR } else { "$env:USERPROFILE\.local\bin" }
$ReleaseBaseUrl = if ($env:ENFORCER_RELEASE_BASE_URL) { $env:ENFORCER_RELEASE_BASE_URL } else { "https://github.com/ocentra/enforcer/releases/download" }

$Triple = "x86_64-pc-windows-msvc"
$Asset = "enforcer-v$Version-$Variant-$Triple.zip"
$ChecksumAsset = "$Asset.sha256"
$Url = "$ReleaseBaseUrl/v$Version/$Asset"
$ChecksumUrl = "$ReleaseBaseUrl/v$Version/$ChecksumAsset"

$TmpDir = Join-Path $env:TEMP ([System.Guid]::NewGuid().ToString())
New-Item -ItemType Directory -Path $TmpDir -Force | Out-Null
try {
    $AssetPath = Join-Path $TmpDir $Asset
    $ChecksumPath = Join-Path $TmpDir $ChecksumAsset

    Write-Host "enforcer installer: downloading $Asset"
    Invoke-WebRequest -Uri $Url -OutFile $AssetPath
    Invoke-WebRequest -Uri $ChecksumUrl -OutFile $ChecksumPath

    $ExpectedSum = (Get-Content $ChecksumPath).Split(" ")[0].Trim()
    $ActualSum = (Get-FileHash -Algorithm SHA256 -Path $AssetPath).Hash.ToLower()
    if ($ExpectedSum.ToLower() -ne $ActualSum) {
        Write-Error "enforcer installer: checksum mismatch for $Asset -- refusing to install (expected $ExpectedSum, got $ActualSum)"
        exit 1
    }

    New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
    Expand-Archive -Path $AssetPath -DestinationPath $TmpDir -Force
    Copy-Item -Path (Join-Path $TmpDir "enforcer.exe") -Destination (Join-Path $InstallDir "enforcer.exe") -Force

    Write-Host "enforcer installer: installed $InstallDir\enforcer.exe ($Variant, v$Version, $Triple)"
}
finally {
    Remove-Item -Recurse -Force $TmpDir -ErrorAction SilentlyContinue
}
