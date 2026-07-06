param(
    [ValidateSet('plan', 'download', 'probe')]
    [string]$Mode = 'plan',

    [string]$RepoRoot = '',
    [string]$ModelCache = '',
    [string]$LlamaCli = '',
    [string]$ImportLlamaCliPath = '',
    [string]$ChatModelPath = '',
    [string]$ImportChatModelPath = '',
    [string]$ChatModelId = '',
    [string]$ProbeFilter = 'chat',
    [string]$ProofOut = 'proof/memory/x06-models-chat-auto-gpu.json',

    [ValidateSet('cpu', 'gpu', 'npu', 'auto')]
    [string]$Acceleration = 'gpu',

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
    $candidate = Get-ChildItem -LiteralPath (Join-Path $RepoRoot 'model\bin') -Recurse -Filter 'llama-cli.exe' -ErrorAction SilentlyContinue |
        Select-Object -First 1
    if ($null -ne $candidate) {
        $LlamaCli = $candidate.FullName
    }
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
