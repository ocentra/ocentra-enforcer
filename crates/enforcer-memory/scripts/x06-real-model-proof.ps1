param(
    [ValidateSet('plan', 'download', 'probe')]
    [string]$Mode = 'plan',

    [string]$RepoRoot = '',
    [string]$ModelCache = '',
    [string]$LlamaCli = '',
    [string]$LlamaEmbedding = '',
    [string]$DownloadLlamaUrl = '',
    [string]$DownloadLlamaArchiveName = '',
    [string]$ImportLlamaCliPath = '',
    [string]$ImportLlamaToolchainPath = '',
    [string]$ChatModelPath = '',
    [string]$ImportChatModelPath = '',
    [string]$ChatModelId = '',
    [string]$ProbeFilter = 'chat',
    [string]$ProofOut = 'proof/memory/x06-models-chat-auto-gpu.json',

    [ValidateSet('cpu', 'gpu', 'npu', 'auto')]
    [string]$Acceleration = 'cpu',

    [double]$MinChatTokensPerSecond = 10.0,
    [double]$TargetChatTokensPerSecondLow = 40.0,
    [double]$TargetChatTokensPerSecondHigh = 60.0,
    [int]$MaxTokens = 16,
    [int]$TimeoutMs = 120000,
    [switch]$AllowMultiProbe
)

$ErrorActionPreference = 'Stop'

if ([string]::IsNullOrWhiteSpace($RepoRoot)) {
    $RepoRoot = (Get-Location).Path
}

if (-not (Test-Path -LiteralPath $RepoRoot)) {
    throw "Repo root does not exist: $RepoRoot"
}

Set-Location -LiteralPath $RepoRoot

function Find-RepoLlamaBinary {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Root,
        [Parameter(Mandatory = $true)]
        [string]$BinaryName
    )

    $candidate = Get-ChildItem -LiteralPath $Root -Recurse -Filter $BinaryName -File -ErrorAction SilentlyContinue |
        Select-Object -First 1
    if ($null -eq $candidate) {
        return $null
    }
    return $candidate.FullName
}

function Import-LlamaToolchain {
    param(
        [Parameter(Mandatory = $true)]
        [string]$SourcePath,
        [Parameter(Mandatory = $true)]
        [string]$RepoRoot
    )

    if (-not (Test-Path -LiteralPath $SourcePath)) {
        throw "Import llama toolchain path does not exist: $SourcePath"
    }

    $binDir = Join-Path $RepoRoot 'model\bin'
    New-Item -ItemType Directory -Force -Path $binDir | Out-Null

    if ((Get-Item -LiteralPath $SourcePath).PSIsContainer) {
        Get-ChildItem -LiteralPath $SourcePath -Recurse -Include 'llama-*.exe', '*.dll' -File -ErrorAction Stop |
            ForEach-Object {
                Copy-Item -LiteralPath $_.FullName -Destination (Join-Path $binDir $_.Name) -Force
            }
    } else {
        Copy-Item -LiteralPath $SourcePath -Destination (Join-Path $binDir ([System.IO.Path]::GetFileName($SourcePath))) -Force
        Get-ChildItem -LiteralPath (Split-Path $SourcePath) -Filter '*.dll' -File -ErrorAction SilentlyContinue |
            ForEach-Object {
                Copy-Item -LiteralPath $_.FullName -Destination (Join-Path $binDir $_.Name) -Force
            }
    }

    return @{
        LlamaCli = Find-RepoLlamaBinary -Root $binDir -BinaryName 'llama-cli.exe'
        LlamaEmbedding = Find-RepoLlamaBinary -Root $binDir -BinaryName 'llama-embedding.exe'
    }
}

if ([string]::IsNullOrWhiteSpace($LlamaCli)) {
    if (-not [string]::IsNullOrWhiteSpace($DownloadLlamaUrl)) {
        $archiveName = $DownloadLlamaArchiveName
        if ([string]::IsNullOrWhiteSpace($archiveName)) {
            $archiveName = [System.IO.Path]::GetFileName(([Uri]$DownloadLlamaUrl).AbsolutePath)
        }
        if ([string]::IsNullOrWhiteSpace($archiveName)) {
            throw "DownloadLlamaArchiveName could not be inferred from DownloadLlamaUrl"
        }
        $archiveBase = [System.IO.Path]::GetFileNameWithoutExtension($archiveName)
        $binRoot = Join-Path $RepoRoot 'model\bin'
        $downloadRoot = Join-Path $binRoot 'downloads'
        $archivePath = Join-Path $downloadRoot $archiveName
        $extractDir = Join-Path $binRoot $archiveBase
        New-Item -ItemType Directory -Force -Path $binRoot | Out-Null
        New-Item -ItemType Directory -Force -Path $downloadRoot | Out-Null
        if (-not (Test-Path -LiteralPath $archivePath)) {
            Invoke-WebRequest -Uri $DownloadLlamaUrl -OutFile $archivePath
        }
        if (-not (Test-Path -LiteralPath (Join-Path $extractDir 'llama-cli.exe'))) {
            if (Test-Path -LiteralPath $extractDir) {
                Remove-Item -LiteralPath $extractDir -Recurse -Force
            }
            New-Item -ItemType Directory -Force -Path $extractDir | Out-Null
            Expand-Archive -LiteralPath $archivePath -DestinationPath $extractDir -Force
        }
        $toolchain = Import-LlamaToolchain -SourcePath $extractDir -RepoRoot $RepoRoot
        if ([string]::IsNullOrWhiteSpace($LlamaCli)) {
            $LlamaCli = $toolchain.LlamaCli
        }
        if ([string]::IsNullOrWhiteSpace($LlamaEmbedding)) {
            $LlamaEmbedding = $toolchain.LlamaEmbedding
        }
    }
}

if ([string]::IsNullOrWhiteSpace($LlamaCli)) {
    if (-not [string]::IsNullOrWhiteSpace($ImportLlamaToolchainPath)) {
        $toolchain = Import-LlamaToolchain -SourcePath $ImportLlamaToolchainPath -RepoRoot $RepoRoot
        $LlamaCli = $toolchain.LlamaCli
        if ([string]::IsNullOrWhiteSpace($LlamaEmbedding)) {
            $LlamaEmbedding = $toolchain.LlamaEmbedding
        }
    }
}

if ([string]::IsNullOrWhiteSpace($LlamaCli)) {
    if (-not [string]::IsNullOrWhiteSpace($ImportLlamaCliPath)) {
        if (-not (Test-Path -LiteralPath $ImportLlamaCliPath)) {
            throw "Import llama-cli path does not exist: $ImportLlamaCliPath"
        }
        $binDir = Join-Path $RepoRoot 'model\bin'
        New-Item -ItemType Directory -Force -Path $binDir | Out-Null
        $LlamaCli = Join-Path $binDir 'llama-cli.exe'
        Copy-Item -LiteralPath $ImportLlamaCliPath -Destination $LlamaCli -Force
        Get-ChildItem -LiteralPath (Split-Path $ImportLlamaCliPath) -Filter '*.dll' -File -ErrorAction SilentlyContinue |
            ForEach-Object {
                Copy-Item -LiteralPath $_.FullName -Destination (Join-Path $binDir $_.Name) -Force
            }
        if ([string]::IsNullOrWhiteSpace($LlamaEmbedding)) {
            $LlamaEmbedding = Find-RepoLlamaBinary -Root $binDir -BinaryName 'llama-embedding.exe'
        }
    }
}

if ([string]::IsNullOrWhiteSpace($ChatModelPath)) {
    if (-not [string]::IsNullOrWhiteSpace($ImportChatModelPath)) {
        if (-not (Test-Path -LiteralPath $ImportChatModelPath)) {
            throw "Import chat model path does not exist: $ImportChatModelPath"
        }
        $chatDir = Join-Path $RepoRoot 'model\local\chat'
        New-Item -ItemType Directory -Force -Path $chatDir | Out-Null
        $ChatModelPath = Join-Path $chatDir ([System.IO.Path]::GetFileName($ImportChatModelPath))
        if (-not (Test-Path -LiteralPath $ChatModelPath)) {
            Copy-Item -LiteralPath $ImportChatModelPath -Destination $ChatModelPath
        }
    }
}

if ([string]::IsNullOrWhiteSpace($LlamaCli)) {
    $LlamaCli = Find-RepoLlamaBinary -Root (Join-Path $RepoRoot 'model\bin') -BinaryName 'llama-cli.exe'
}
if ([string]::IsNullOrWhiteSpace($LlamaEmbedding)) {
    $LlamaEmbedding = Find-RepoLlamaBinary -Root (Join-Path $RepoRoot 'model\bin') -BinaryName 'llama-embedding.exe'
}

$env:ENFORCER_X06_RUNTIME_MODE = $Mode
if (-not [string]::IsNullOrWhiteSpace($ModelCache)) {
    $env:ENFORCER_X06_MODEL_CACHE = $ModelCache
} else {
    Remove-Item Env:\ENFORCER_X06_MODEL_CACHE -ErrorAction SilentlyContinue
}
$env:ENFORCER_X06_PROBE_FILTER = $ProbeFilter
$env:ENFORCER_X06_ALLOW_MULTI_PROBE = if ($AllowMultiProbe.IsPresent) { '1' } else { '0' }
$env:ENFORCER_X06_AUTO_CHAT_MODEL = '1'
$env:ENFORCER_X06_ALLOW_NETWORK = if ($Mode -eq 'plan') { '0' } else { '1' }
$env:ENFORCER_X06_STREAMING_SIDECARS = '0'
$env:ENFORCER_X06_PROOF_OUT = $ProofOut
if (-not [string]::IsNullOrWhiteSpace($LlamaCli)) {
    $env:ENFORCER_X06_LLAMA_CLI = $LlamaCli
} else {
    Remove-Item Env:\ENFORCER_X06_LLAMA_CLI -ErrorAction SilentlyContinue
}
Remove-Item Env:\ENFORCER_X06_LLAMA_SERVER -ErrorAction SilentlyContinue
if (-not [string]::IsNullOrWhiteSpace($LlamaEmbedding)) {
    $env:ENFORCER_X06_LLAMA_EMBEDDING = $LlamaEmbedding
} else {
    Remove-Item Env:\ENFORCER_X06_LLAMA_EMBEDDING -ErrorAction SilentlyContinue
}
if (-not [string]::IsNullOrWhiteSpace($ChatModelPath)) {
    $env:ENFORCER_X06_CHAT_MODEL_PATH = $ChatModelPath
} else {
    Remove-Item Env:\ENFORCER_X06_CHAT_MODEL_PATH -ErrorAction SilentlyContinue
}
if (-not [string]::IsNullOrWhiteSpace($ChatModelId)) {
    $env:ENFORCER_X06_CHAT_MODEL_ID = $ChatModelId
} else {
    Remove-Item Env:\ENFORCER_X06_CHAT_MODEL_ID -ErrorAction SilentlyContinue
}
$env:ENFORCER_X06_LLAMA_ACCELERATION = $Acceleration
$env:ENFORCER_X06_LLAMA_MAX_TOKENS = [string]$MaxTokens
$env:ENFORCER_X06_LLAMA_TIMEOUT_MS = [string]$TimeoutMs
$env:ENFORCER_X06_ORT_TIMEOUT_MS = [string]$TimeoutMs
$env:ENFORCER_X06_MIN_CHAT_TOKENS_PER_SECOND = [string]$MinChatTokensPerSecond
$env:ENFORCER_X06_TARGET_CHAT_TOKENS_PER_SECOND_LOW = [string]$TargetChatTokensPerSecondLow
$env:ENFORCER_X06_TARGET_CHAT_TOKENS_PER_SECOND_HIGH = [string]$TargetChatTokensPerSecondHigh
$env:ENFORCER_X06_STRICT_CACHE_HASH = '0'
$env:ENFORCER_X06_LLAMA_FIT = '1'
$env:ENFORCER_X06_LLAMA_SPLIT_MODE = 'layer'
$env:ENFORCER_X06_LLAMA_MAIN_GPU = '0'

cargo build -p enforcer-memory --features real-models --example x06_model_runtime_probe
if ($LASTEXITCODE -ne 0) {
    exit $LASTEXITCODE
}

$probe = Join-Path $RepoRoot 'target\debug\examples\x06_model_runtime_probe.exe'
if (-not (Test-Path -LiteralPath $probe)) {
    throw "Compiled probe not found: $probe"
}

& $probe
exit $LASTEXITCODE
