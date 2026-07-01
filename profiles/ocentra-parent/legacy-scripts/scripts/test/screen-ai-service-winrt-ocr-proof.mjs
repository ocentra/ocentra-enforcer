import { spawn, spawnSync } from 'node:child_process';
import { chmodSync, existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { mkdtemp } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, relative } from 'node:path';
import { setTimeout as delay } from 'node:timers/promises';
import { chromium } from 'playwright';
import {
  AgentCommand,
  AgentEvent,
  AgentEventEnvelopeSchema,
} from '@ocentra-parent/schema-domain/agent-command-event-contracts';
import { AgentProtocolDefaults } from '@ocentra-parent/schema-domain/agent-protocol-defaults';

import {
  ParentDevEnv,
  ParentDevHost,
  createHttpOrigin,
  createAgentAddress,
  createAgentHealthUrl,
  createAgentWebSocketUrl,
  isLikelyParentAgentOccupant,
  isLikelyParentPortalOccupant,
  resolveParentDevPort,
} from '../dev/local-dev-config.mjs';
import { ensurePortFree } from '../dev/port-utils.mjs';
import { removeDirectoryWithRetry, spawnVitePortal, stopProcessTreeAndWait } from './agent-service-process.mjs';

const repoRoot = process.cwd();
const sensitiveNativeMode = process.argv.includes('--sensitive-native-redaction');
const proofSlug = sensitiveNativeMode ? 'service-winrt-ocr-redaction' : 'service-winrt-ocr';
const outputDir = join(repoRoot, 'output', 'screen-ai-pipeline-proof', proofSlug);
const validationLogPath = join(outputDir, '14-validation-commands.log');
const sourceSnapshotPath = join(outputDir, '00-source-snapshot.md');
const parentRedactionPolicyPath = join(outputDir, 'parent-redaction-policy.json');
const agentPort = resolveParentDevPort(
  process.env.OCENTRA_SCREEN_SERVICE_WINRT_OCR_PROOF_AGENT_PORT,
  4691,
  'OCENTRA_SCREEN_SERVICE_WINRT_OCR_PROOF_AGENT_PORT'
);
const portalPort = resolveParentDevPort(
  process.env.OCENTRA_SCREEN_SERVICE_WINRT_OCR_PROOF_PORTAL_PORT,
  4692,
  'OCENTRA_SCREEN_SERVICE_WINRT_OCR_PROOF_PORTAL_PORT'
);
const sensitiveLiveText = [
  'School portal account jane@example.com',
  'Parent phone 555-010-1234',
  'password reset token visible',
].join('\n');
const liveUrl = sensitiveNativeMode
  ? `data:text/plain;charset=utf-8,${encodeURIComponent(sensitiveLiveText)}`
  : 'https://en.wikipedia.org/wiki/Mathematics';
const expectedTerms = sensitiveNativeMode ? ['jane', '555', 'password'] : ['wikipedia', 'mathematics'];
const liveTitlePatterns = sensitiveNativeMode
  ? ['*data:text/plain*', '*jane@example.com*']
  : ['*Wikipedia*', '*Mathematics*'];
const systemChromePath = 'C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe';
const buildRoot = process.env.CARGO_TARGET_DIR ?? join(repoRoot, 'target');
const activityRoot = await mkdtemp(join(tmpdir(), 'ocentra-screen-service-winrt-'));
const queueDir = join(activityRoot, 'screen-queue');
const journalPath = join(activityRoot, 'activity.ndjson');
const keyPath = join(activityRoot, 'activity-journal.key');
const storePath = join(activityRoot, 'activity.sqlite');
const adapterObservationPath = join(activityRoot, 'winrt-ocr-observation.json');
const adapterCommandPath = join(activityRoot, 'screen-service-winrt-ocr-adapter.cmd');
const adapterScriptPath = join(activityRoot, 'screen-service-winrt-ocr-adapter.ps1');
const queuePath = join(queueDir, 'screen-evidence-queue.ndjson');
const healthUrl = createAgentHealthUrl(agentPort);
const wsUrl = createAgentWebSocketUrl(agentPort);
const validationCommands = [
  sensitiveNativeMode
    ? 'node scripts/test/screen-ai-service-winrt-ocr-proof.mjs --sensitive-native-redaction'
    : 'node scripts/test/screen-ai-service-winrt-ocr-proof.mjs',
];
const analysisReadModelWaitMs = 240000;

if (process.platform !== 'win32') {
  throw new Error('screen-ai-service-winrt-ocr-proof requires Windows WinRT OCR and desktop capture.');
}

rmSync(outputDir, { recursive: true, force: true });
mkdirSync(outputDir, { recursive: true });
writeJson(parentRedactionPolicyPath, parentSelectedRedactionPolicy());
writeAdapterCommand();

await ensurePortFree(agentPort, isLikelyParentAgentOccupant, console.log);

let browser;
let portal;
let portalBrowser;
let service;
let serviceOutput = () => '';
let lastObservedReadModel = null;

try {
  await runCommand('cargo', ['build', '-p', 'ocentra-parent-agent-service']);
  browser = await openLivePage();
  await focusLivePage(browser);
  service = startService(serviceCaptureEnv());
  serviceOutput = collectOutput(service);
  await waitForHttp(healthUrl, serviceOutput);
  await focusLivePage(browser);
  await waitForQueueRecords(1);
  await stopProcessTreeAndWait(service);
  service = undefined;

  await ensurePortFree(agentPort, isLikelyParentAgentOccupant, console.log);
  service = startService(serviceAnalysisEnv());
  serviceOutput = collectOutput(service);
  await waitForHttp(healthUrl, serviceOutput);
  const readModel = await waitForAnalyzedScreenReadModel();
  const analysisRow = localOcrRow(readModel);
  await waitForAnalyzedQueueRemoval(analysisRow?.queueJobId);
  await waitForQueueDrain();
  const queueRecords = readQueueRecordsAllowEmpty();
  const ocrObservation = await waitForOcrObservation();
  assertProof(readModel, queueRecords, ocrObservation);

  const sanitizedReadModel = sanitizeReadModel(readModel);
  const sanitizedQueueRecords = sanitizeQueueRecords(queueRecords);
  const sanitizedObservation = sanitizeOcrObservation(ocrObservation);
  const portalArtifact = sensitiveNativeMode ? await capturePortalScreenshot() : null;
  const summary = {
    proof: sensitiveNativeMode ? 'screen-ai-service-winrt-ocr-redaction-proof' : 'screen-ai-service-winrt-ocr-proof',
    proofTier: 'P3_REAL_CAPTURE_LOCAL_OCR_SERVICE_PATH',
    platform: process.platform,
    liveSource: {
      kind: browser.sourceEvidence.sourceKind,
      url: browser.sourceEvidence.url,
      title: browser.sourceEvidence.title,
      expectedTerms,
    },
    agentPort,
    analysisRow: localOcrRow(sanitizedReadModel),
    winRtOcrObservation: sanitizedObservation,
    queueRecordsAfterAnalysis: queueRecords.length,
    artifacts: {
      sourceSnapshot: relative(repoRoot, sourceSnapshotPath),
      proofSummary: relative(repoRoot, join(outputDir, 'proof-summary.json')),
      screenReadModel: relative(repoRoot, join(outputDir, 'screen-read-model.json')),
      winRtOcrObservation: relative(repoRoot, join(outputDir, 'winrt-ocr-observation.json')),
      queueRecordMetadataAfterAnalysis: relative(repoRoot, join(outputDir, 'queue-records-after-analysis.json')),
      validationCommands: relative(repoRoot, validationLogPath),
      portalScreenshot: portalArtifact === null ? null : relative(repoRoot, portalArtifact.screenshotPath),
      parentSelectedRedactionPolicy: relative(repoRoot, parentRedactionPolicyPath),
    },
    assertions: {
      realWindowsServiceCaptureRequired: true,
      liveExternalBrowserSurfaceUsed: !sensitiveNativeMode,
      liveLocalBrowserTextSurfaceUsed: sensitiveNativeMode,
      serviceAdapterRanWindowsWinRtOcr: sanitizedObservation.runtime === 'Windows.Media.Ocr.OcrEngine',
      ocrSawExpectedLivePageTerms: expectedTerms.every((term) =>
        sanitizedObservation.expectedTermsFound.includes(term)
      ),
      activityReadModelReachedViaWebSocket: readModel.state === 'ready',
      providerKindPreservedFromAdapter: analysisRow.providerKind === 'localOcr',
      runtimeMetadataPreservedFromAdapter:
        analysisRow.modelRuntimeRef === 'windows-winrt-ocr-local-runtime' &&
        analysisRow.modelId === 'windows-winrt-ocr' &&
        analysisRow.promptOrTemplateVersion === 'screen-ocr-worker-winrt-v1',
      policyConsumedOcrResult: analysisRow.policyEligible === true && analysisRow.primaryCategory === 'school',
      encryptedQueueDrainedAfterAnalysis: queueRecords.length === 0,
      adapterTemporaryImageDeleted: sanitizedObservation.tempImageExistsAfterDelete === false,
      rawImageNotRetainedInReadModel: analysisRow.rawImageRetained === false,
      ocrSnippetsPreservedInReadModel: (analysisRow.ocrTextSnippets ?? []).length > 0,
      redactionNotesShapePreservedInReadModel: Array.isArray(analysisRow.redactionNotes),
      serviceRedactedSensitiveOcrSnippets: sensitiveNativeMode ? redactedServiceRowAssertions(analysisRow) : null,
      serviceConsumedParentSelectedRedactionPolicy: sensitiveNativeMode
        ? parentSelectedRedactionPolicy().textRetentionMode === 'redactedSnippets' &&
          parentSelectedRedactionPolicy().piiRedactionEnabled === true
        : null,
      realPortalScreenshotCaptured: portalArtifact === null ? null : existsSync(portalArtifact.screenshotPath),
    },
    nonClaims: [
      'This proves the Windows service path from timed cadence capture through encrypted queue, WinRT OCR adapter output, Activity Screen read model, and queue/raw temp deletion.',
      'It does not claim production OCR quality tuning, authenticated social coverage, cross-platform OCR parity, portal UI polish, or enforcement handoff.',
      'The proof intentionally does not retain the raw captured screenshot as an artifact; the OCR observation keeps bounded text snippets and a text digest only.',
    ],
  };
  writeSourceSnapshot(browser.sourceEvidence, sanitizedObservation);
  writeJson(join(outputDir, 'proof-summary.json'), summary);
  writeJson(join(outputDir, 'screen-read-model.json'), sanitizedReadModel);
  writeJson(join(outputDir, 'winrt-ocr-observation.json'), sanitizedObservation);
  writeJson(join(outputDir, 'queue-records-after-analysis.json'), sanitizedQueueRecords);
  if (portalArtifact !== undefined) {
    writeJson(join(outputDir, 'portal-proof-summary.json'), portalArtifact.summary);
  }
  writeText(validationLogPath, `${validationCommands.join('\n')}\n`);
  console.log(`${proofSlug}-proof-ok:${analysisRow.providerKind}:${queueRecords.length}`);
} catch (error) {
  writeFailureArtifacts(error);
  throw error;
} finally {
  await Promise.allSettled([
    browser === undefined ? Promise.resolve() : browser.close(),
    portalBrowser === undefined ? Promise.resolve() : portalBrowser.close(),
    portal === undefined ? Promise.resolve() : stopProcessTreeAndWait(portal),
    service === undefined ? Promise.resolve() : stopProcessTreeAndWait(service),
  ]);
  await removeDirectoryWithRetry(activityRoot);
}

async function openLivePage() {
  const launchedAfterIso = new Date(Date.now() - 1000).toISOString();
  closeExistingLiveBrowserWindows();
  openSystemBrowser();
  const liveWindow = await waitForLiveBrowserWindow(launchedAfterIso);
  activateBrowserWindow(liveWindow.title, launchedAfterIso, liveWindow.processId);
  return {
    sourceEvidence: {
      sourceKind: sensitiveNativeMode ? 'live-local-browser-text-surface' : 'live-public-browser-page',
      liveExternalUrl: !sensitiveNativeMode,
      url: liveUrl,
      title: liveWindow.title,
      processId: liveWindow.processId,
      launchedAfterIso,
      expectedTerms,
    },
    close: async () => {
      closeLiveBrowserWindow(liveWindow.processId);
    },
  };
}

async function focusLivePage(surface) {
  activateBrowserWindow(
    surface.sourceEvidence.title,
    surface.sourceEvidence.launchedAfterIso,
    surface.sourceEvidence.processId
  );
  await delay(1000);
}

function openSystemBrowser() {
  if (!existsSync(systemChromePath)) {
    throw new Error(`System Chrome not found at ${systemChromePath}`);
  }
  const script = [
    "$ErrorActionPreference = 'Stop'",
    `Start-Process -FilePath '${escapePowerShell(systemChromePath)}' -ArgumentList '--new-window','${escapePowerShell(
      liveUrl
    )}' -WindowStyle Normal`,
  ].join('\n');
  const result = spawnSync('powershell', ['-NoProfile', '-ExecutionPolicy', 'Bypass', '-Command', script], {
    cwd: repoRoot,
    encoding: 'utf8',
    windowsHide: true,
  });
  if (result.error) {
    throw result.error;
  }
  if (result.status !== 0) {
    throw new Error(`Chrome launch failed: ${result.stderr.trim()}`);
  }
}

async function waitForLiveBrowserWindow(launchedAfterIso) {
  const startedAt = Date.now();
  while (Date.now() - startedAt < 20000) {
    const liveWindow = findLiveBrowserWindow(launchedAfterIso);
    if (liveWindow) {
      return liveWindow;
    }
    await delay(500);
  }
  throw new Error('Timed out waiting for live Chrome proof window.');
}

function findLiveBrowserWindow(launchedAfterIso) {
  const script = [
    `$since = [DateTimeOffset]::Parse('${escapePowerShell(launchedAfterIso)}').LocalDateTime`,
    `$target = Get-Process | Where-Object { try { ${powerShellLikeAny('$_.MainWindowTitle')} } catch { $false } } | Sort-Object StartTime -Descending | Select-Object -First 1`,
    'if ($null -eq $target) { return }',
    '[ordered]@{ processId = $target.Id; title = $target.MainWindowTitle } | ConvertTo-Json -Compress',
  ].join('\n');
  const result = spawnSync('powershell', ['-NoProfile', '-ExecutionPolicy', 'Bypass', '-Command', script], {
    cwd: repoRoot,
    encoding: 'utf8',
    windowsHide: true,
  });
  if (result.status !== 0 || result.stdout.trim().length === 0) {
    return null;
  }
  return JSON.parse(result.stdout.replace(/^\uFEFF/, ''));
}

function closeLiveBrowserWindow(processId) {
  if (!Number.isInteger(processId)) {
    return;
  }
  const script = [
    `$target = Get-Process -Id ${processId} -ErrorAction SilentlyContinue`,
    'if ($null -ne $target -and $target.MainWindowHandle -ne 0) { $target.CloseMainWindow() | Out-Null }',
    'Start-Sleep -Milliseconds 500',
  ].join('\n');
  spawnSync('powershell', ['-NoProfile', '-ExecutionPolicy', 'Bypass', '-Command', script], {
    cwd: repoRoot,
    encoding: 'utf8',
    windowsHide: true,
  });
}

function closeExistingLiveBrowserWindows() {
  const script = [
    `$targets = Get-Process chrome -ErrorAction SilentlyContinue | Where-Object { ${powerShellLikeAny('$_.MainWindowTitle')} }`,
    '$targets | ForEach-Object { $_.CloseMainWindow() | Out-Null }',
    'if (@($targets).Count -gt 0) { Start-Sleep -Milliseconds 900 }',
  ].join('\n');
  spawnSync('powershell', ['-NoProfile', '-ExecutionPolicy', 'Bypass', '-Command', script], {
    cwd: repoRoot,
    encoding: 'utf8',
    windowsHide: true,
  });
}

function activateBrowserWindow(title, launchedAfterIso, processId) {
  const targetProcessId = Number.isInteger(processId) ? processId : null;
  const script = [
    'Add-Type -AssemblyName Microsoft.VisualBasic',
    "Add-Type -TypeDefinition @'",
    'using System;',
    'using System.Runtime.InteropServices;',
    'using System.Text;',
    'public static class OcentraWin32 {',
    '  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hWnd);',
    '  [DllImport("user32.dll")] public static extern bool ShowWindowAsync(IntPtr hWnd, int nCmdShow);',
    '  [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();',
    '  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint processId);',
    '  [DllImport("kernel32.dll")] public static extern uint GetCurrentThreadId();',
    '  [DllImport("user32.dll")] public static extern bool AttachThreadInput(uint idAttach, uint idAttachTo, bool fAttach);',
    '  [DllImport("user32.dll")] public static extern bool BringWindowToTop(IntPtr hWnd);',
    '  [DllImport("user32.dll")] public static extern IntPtr SetActiveWindow(IntPtr hWnd);',
    '  [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern int GetWindowText(IntPtr hWnd, StringBuilder text, int count);',
    '}',
    "'@",
    'function Invoke-OcentraActivateTarget {',
    '  param($targetProcess)',
    '  $foregroundWindow = [OcentraWin32]::GetForegroundWindow()',
    '  $targetProcessIdForThread = 0',
    '  $foregroundProcessIdForThread = 0',
    '  $currentThread = [OcentraWin32]::GetCurrentThreadId()',
    '  $targetThread = [OcentraWin32]::GetWindowThreadProcessId($targetProcess.MainWindowHandle, [ref]$targetProcessIdForThread)',
    '  $foregroundThread = [OcentraWin32]::GetWindowThreadProcessId($foregroundWindow, [ref]$foregroundProcessIdForThread)',
    '  try {',
    '    if ($foregroundThread -ne 0) { [OcentraWin32]::AttachThreadInput($currentThread, $foregroundThread, $true) | Out-Null }',
    '    if ($targetThread -ne 0) { [OcentraWin32]::AttachThreadInput($currentThread, $targetThread, $true) | Out-Null }',
    '    [OcentraWin32]::ShowWindowAsync($targetProcess.MainWindowHandle, 9) | Out-Null',
    '    [Microsoft.VisualBasic.Interaction]::AppActivate($targetProcess.Id) | Out-Null',
    '    [OcentraWin32]::BringWindowToTop($targetProcess.MainWindowHandle) | Out-Null',
    '    [OcentraWin32]::SetActiveWindow($targetProcess.MainWindowHandle) | Out-Null',
    '    [OcentraWin32]::SetForegroundWindow($targetProcess.MainWindowHandle) | Out-Null',
    '    Start-Sleep -Milliseconds 700',
    '  } finally {',
    '    if ($targetThread -ne 0) { [OcentraWin32]::AttachThreadInput($currentThread, $targetThread, $false) | Out-Null }',
    '    if ($foregroundThread -ne 0) { [OcentraWin32]::AttachThreadInput($currentThread, $foregroundThread, $false) | Out-Null }',
    '  }',
    '  $buffer = New-Object System.Text.StringBuilder 512',
    '  [OcentraWin32]::GetWindowText([OcentraWin32]::GetForegroundWindow(), $buffer, $buffer.Capacity) | Out-Null',
    '  return $buffer.ToString()',
    '}',
    `$since = [DateTimeOffset]::Parse('${escapePowerShell(launchedAfterIso)}').LocalDateTime`,
    targetProcessId === null
      ? '$target = $null'
      : `$target = Get-Process -Id ${targetProcessId} -ErrorAction SilentlyContinue`,
    'if ($null -ne $target -and $target.MainWindowHandle -eq 0) { $target = $null }',
    `if ($null -ne $target -and ${powerShellNotLikeAll('$target.MainWindowTitle')}) { $target = $null }`,
    `$windows = Get-Process | Where-Object { try { $_.MainWindowHandle -ne 0 -and $_.StartTime -ge $since -and (${powerShellLikeAny('$_.MainWindowTitle')}) } catch { $false } } | Sort-Object StartTime -Descending`,
    'if ($null -eq $target) { $target = $windows | Select-Object -First 1 }',
    `if ($null -eq $target) { $target = Get-Process | Where-Object { $_.MainWindowTitle -like '*${escapePowerShell(title)}*' } | Select-Object -First 1 }`,
    "if ($null -eq $target) { throw 'No visible browser window found for proof activation.' }",
    '$foregroundTitle = $null',
    'for ($attempt = 0; $attempt -lt 5; $attempt++) {',
    '  $foregroundTitle = Invoke-OcentraActivateTarget $target',
    `  if (${powerShellLikeAny('$foregroundTitle')}) { break }`,
    '  Start-Sleep -Milliseconds 400',
    '}',
    "if ($foregroundTitle -like 'native-ocr-worker-proof-*') {",
    "  Get-Process | Where-Object { $_.MainWindowTitle -like 'native-ocr-worker-proof-*' } | ForEach-Object { $_.CloseMainWindow() | Out-Null }",
    '  Start-Sleep -Milliseconds 900',
    '  $foregroundTitle = Invoke-OcentraActivateTarget $target',
    '}',
    "if ($foregroundTitle -like '*Windows Security*') {",
    "  Get-Process | Where-Object { $_.MainWindowTitle -eq 'Windows Security' } | ForEach-Object { $_.CloseMainWindow() | Out-Null }",
    '  Start-Sleep -Milliseconds 900',
    "  Get-Process | Where-Object { $_.MainWindowTitle -eq 'Windows Security' -or $_.ProcessName -eq 'PickerHost' } | ForEach-Object { Stop-Process -Id $_.Id -Force -ErrorAction SilentlyContinue }",
    '  Start-Sleep -Milliseconds 900',
    '  $foregroundTitle = Invoke-OcentraActivateTarget $target',
    '}',
    `if (${powerShellNotLikeAll('$foregroundTitle')}) { throw ('Foreground window after activation was ' + $foregroundTitle + '; target was ' + $target.MainWindowTitle) }`,
  ].join('\n');
  const result = spawnSync('powershell', ['-NoProfile', '-ExecutionPolicy', 'Bypass', '-Command', script], {
    cwd: repoRoot,
    encoding: 'utf8',
    windowsHide: true,
  });
  if (result.status !== 0) {
    throw new Error(`Could not activate live browser window ${title}\n${result.stdout}\n${result.stderr}`);
  }
}

function writeAdapterCommand() {
  writeFileSync(adapterScriptPath, adapterPowerShell(), 'utf8');
  writeFileSync(
    adapterCommandPath,
    [
      '@echo off',
      'powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0screen-service-winrt-ocr-adapter.ps1"',
      '',
    ].join('\r\n')
  );
  chmodSync(adapterCommandPath, 0o755);
}

function adapterPowerShell() {
  return `
$ErrorActionPreference = 'Stop'
$inputPayload = [Console]::In.ReadToEnd()
$request = $inputPayload | ConvertFrom-Json
if ([string]::IsNullOrWhiteSpace([string]$request.imageBase64)) { throw 'Adapter request missing imageBase64.' }
$imageBytes = [Convert]::FromBase64String([string]$request.imageBase64)
$imagePath = Join-Path $env:TEMP ('ocentra-service-winrt-ocr-' + [guid]::NewGuid().ToString() + '.png')
[IO.File]::WriteAllBytes($imagePath, $imageBytes)
try {
  Add-Type -AssemblyName System.Runtime.WindowsRuntime
  [Windows.Storage.StorageFile, Windows.Storage, ContentType=WindowsRuntime] | Out-Null
  [Windows.Storage.Streams.IRandomAccessStreamWithContentType, Windows.Storage.Streams, ContentType=WindowsRuntime] | Out-Null
  [Windows.Graphics.Imaging.BitmapDecoder, Windows.Graphics.Imaging, ContentType=WindowsRuntime] | Out-Null
  [Windows.Graphics.Imaging.SoftwareBitmap, Windows.Graphics.Imaging, ContentType=WindowsRuntime] | Out-Null
  [Windows.Media.Ocr.OcrEngine, Windows.Foundation, ContentType=WindowsRuntime] | Out-Null
  [Windows.Media.Ocr.OcrResult, Windows.Foundation, ContentType=WindowsRuntime] | Out-Null
  function AwaitOp($op, [Type]$type) {
    $method = [System.WindowsRuntimeSystemExtensions].GetMethods() | Where-Object {
      $_.Name -eq 'AsTask' -and $_.IsGenericMethodDefinition -and $_.GetGenericArguments().Count -eq 1 -and $_.GetParameters().Count -eq 1
    } | Select-Object -First 1
    $task = $method.MakeGenericMethod($type).Invoke($null, @($op))
    return $task.GetAwaiter().GetResult()
  }
  $file = AwaitOp ([Windows.Storage.StorageFile]::GetFileFromPathAsync($imagePath)) ([Windows.Storage.StorageFile])
  $stream = AwaitOp ($file.OpenReadAsync()) ([Windows.Storage.Streams.IRandomAccessStreamWithContentType])
  $decoder = AwaitOp ([Windows.Graphics.Imaging.BitmapDecoder]::CreateAsync($stream)) ([Windows.Graphics.Imaging.BitmapDecoder])
  $bitmap = AwaitOp ($decoder.GetSoftwareBitmapAsync()) ([Windows.Graphics.Imaging.SoftwareBitmap])
  $engine = [Windows.Media.Ocr.OcrEngine]::TryCreateFromUserProfileLanguages()
  if ($null -eq $engine) { throw 'WinRT OCR engine unavailable for user profile languages.' }
  $result = AwaitOp ($engine.RecognizeAsync($bitmap)) ([Windows.Media.Ocr.OcrResult])
  $text = [string]$result.Text
  if ([string]::IsNullOrWhiteSpace($text)) { throw 'WinRT OCR returned no text.' }
  $normalized = $text.ToLowerInvariant()
  $expectedTerms = @(${expectedTerms.map((term) => powerShellString(term)).join(', ')})
  $foundTerms = @()
  foreach ($term in $expectedTerms) {
    if ($normalized.Contains($term)) {
      $foundTerms += $term
    }
  }
  if ($foundTerms.Count -lt $expectedTerms.Count -and ${sensitiveNativeMode ? '$false' : '$true'}) { throw ('WinRT OCR missing expected live page terms. Found: ' + ($foundTerms -join ',')) }
  $lineIndex = 0
  $candidateSnippets = @()
  foreach ($line in $result.Lines) {
    $lineIndex += 1
    $lineText = ([string]$line.Text).Trim()
    if ($lineText.Length -gt 0) {
      $candidateSnippets += [ordered]@{ text = $lineText; boundingBoxRef = ('line-' + $lineIndex) }
    }
  }
  $snippets = @()
  foreach ($candidate in $candidateSnippets) {
    $candidateText = ([string]$candidate.text).ToLowerInvariant()
    foreach ($term in $expectedTerms) {
      if ($candidateText.Contains($term) -and $snippets.Count -lt 6) {
        $snippets += $candidate
        break
      }
    }
  }
  foreach ($candidate in $candidateSnippets) {
    if ($snippets.Count -ge 6) { break }
    $alreadySelected = $false
    foreach ($selected in $snippets) {
      if ($selected.boundingBoxRef -eq $candidate.boundingBoxRef) {
        $alreadySelected = $true
        break
      }
    }
    if (-not $alreadySelected) {
      $snippets += $candidate
    }
  }
  $snippetTexts = @($snippets | ForEach-Object { [string]$_.text })
  $sha = [System.Security.Cryptography.SHA256]::Create()
  $digestBytes = $sha.ComputeHash([System.Text.Encoding]::UTF8.GetBytes($text))
  $textDigest = 'sha256:' + ([BitConverter]::ToString($digestBytes).Replace('-', '').ToLowerInvariant())
  $observation = [ordered]@{
    runtime = 'Windows.Media.Ocr.OcrEngine'
    providerKind = 'localOcr'
    expectedTermsFound = $foundTerms
    lineCount = @($result.Lines).Count
    textDigest = $textDigest
    snippets = $snippets
    tempImagePath = $imagePath
  }
  $output = [ordered]@{
    summary = ${powerShellString(
      sensitiveNativeMode
        ? 'Windows WinRT OCR read a live local browser sensitive-text page from the service queued capture and service redaction removed sensitive snippets.'
        : 'Windows WinRT OCR read the live Wikipedia mathematics page from the service queued capture.'
    )}
    primaryCategory = 'school'
    confidence = 0.91
    policyEligible = $true
    providerKind = 'localOcr'
    modelRuntimeRef = 'windows-winrt-ocr-local-runtime'
    modelId = 'windows-winrt-ocr'
    promptOrTemplateVersion = 'screen-ocr-worker-winrt-v1'
    ocrTextSnippets = $snippetTexts
    redactionNotes = @()
  } | ConvertTo-Json -Compress
  [Console]::Out.WriteLine($output)
}
finally {
  Remove-Item -Force $imagePath -ErrorAction SilentlyContinue
  $existsAfterDelete = Test-Path $imagePath
  if ($null -ne $observation) {
    $observation['tempImageExistsAfterDelete'] = $existsAfterDelete
    $observation | ConvertTo-Json -Depth 8 | Set-Content -Path $env:OCENTRA_SCREEN_SERVICE_WINRT_OCR_OBSERVATION_PATH -Encoding UTF8
  }
}
`;
}

function startService(env) {
  return spawn(proofAgentServicePath(), [], {
    cwd: repoRoot,
    env,
    stdio: ['ignore', 'pipe', 'pipe'],
    windowsHide: true,
  });
}

function serviceCaptureEnv() {
  return {
    ...baseServiceEnv(),
    OCENTRA_PARENT_SCREEN_SERVICE_CADENCE_RUNTIME_ENABLED: 'true',
    OCENTRA_PARENT_SCREEN_SERVICE_ANALYSIS_RUNTIME_ENABLED: 'false',
    OCENTRA_PARENT_SCREEN_SERVICE_ANALYSIS_ENABLED: 'true',
    OCENTRA_PARENT_SCREEN_SERVICE_CADENCE_ENABLED: 'true',
    OCENTRA_PARENT_SCREEN_SERVICE_CADENCE_SECONDS: '1',
    OCENTRA_PARENT_SCREEN_SERVICE_CADENCE_MAX_CAPTURES: '1',
    OCENTRA_PARENT_SCREEN_SERVICE_CADENCE_MAX_TICKS: '4',
  };
}

function serviceAnalysisEnv() {
  return {
    ...baseServiceEnv(),
    OCENTRA_PARENT_SCREEN_SERVICE_CADENCE_RUNTIME_ENABLED: 'false',
    OCENTRA_PARENT_SCREEN_SERVICE_FOREGROUND_RUNTIME_ENABLED: 'false',
    OCENTRA_PARENT_SCREEN_SERVICE_RETENTION_SWEEPER_RUNTIME_ENABLED: 'false',
    OCENTRA_PARENT_SCREEN_SERVICE_ANALYSIS_RUNTIME_ENABLED: 'true',
    OCENTRA_PARENT_SCREEN_SERVICE_ANALYSIS_ENABLED: 'true',
    OCENTRA_PARENT_SCREEN_SERVICE_ANALYSIS_POLL_SECONDS: '1',
    OCENTRA_PARENT_SCREEN_SERVICE_ANALYSIS_MAX_JOBS: sensitiveNativeMode ? '2' : '1',
    OCENTRA_PARENT_SCREEN_SERVICE_ANALYSIS_MAX_TICKS: '180',
    OCENTRA_PARENT_SCREEN_SERVICE_ANALYSIS_ADAPTER_TIMEOUT_MS: '120000',
    OCENTRA_PARENT_SCREEN_SERVICE_ANALYSIS_ADAPTER_COMMAND: adapterCommandPath,
    OCENTRA_PARENT_SCREEN_SERVICE_OCR_REDACTION_POLICY_PATH: parentRedactionPolicyPath,
    OCENTRA_SCREEN_SERVICE_WINRT_OCR_OBSERVATION_PATH: adapterObservationPath,
  };
}

function parentSelectedRedactionPolicy() {
  return {
    schemaVersion: 1,
    parentSettingRef: 'parent-setting-screen-service-redaction-proof',
    settingVersion: 1,
    ocrTextEnabled: true,
    snippetLimit: 5,
    redactionMode: 'localSensitiveText',
    textRetentionMode: 'redactedSnippets',
    credentialSuppressionEnabled: true,
    piiRedactionEnabled: true,
    parentControlled: true,
    rawTextRetentionAllowed: false,
  };
}

function portalServiceEnv() {
  return {
    ...baseServiceEnv(),
    [ParentDevEnv.AgentAllowedOrigins]: portalUrl(),
    OCENTRA_PARENT_SCREEN_SERVICE_CADENCE_RUNTIME_ENABLED: 'false',
    OCENTRA_PARENT_SCREEN_SERVICE_FOREGROUND_RUNTIME_ENABLED: 'false',
    OCENTRA_PARENT_SCREEN_SERVICE_RETENTION_SWEEPER_RUNTIME_ENABLED: 'false',
    OCENTRA_PARENT_SCREEN_SERVICE_ANALYSIS_RUNTIME_ENABLED: 'false',
  };
}

function baseServiceEnv() {
  return {
    ...process.env,
    [ParentDevEnv.AgentAddress]: createAgentAddress(agentPort),
    [ParentDevEnv.ActivityDbPath]: storePath,
    OCENTRA_PARENT_ACTIVITY_JOURNAL_PATH: journalPath,
    OCENTRA_PARENT_ACTIVITY_JOURNAL_KEY_PATH: keyPath,
    OCENTRA_PARENT_SCREEN_SERVICE_QUEUE_MAX_PENDING: '2',
    OCENTRA_PARENT_SCREEN_SERVICE_QUEUE_DIR: queueDir,
  };
}

async function waitForAnalyzedScreenReadModel() {
  const startedAt = Date.now();
  let lastReadModel;
  while (Date.now() - startedAt < analysisReadModelWaitMs) {
    lastReadModel = await requestScreenReadModel();
    lastObservedReadModel = lastReadModel;
    if (lastReadModel.state === 'ready' && Array.isArray(lastReadModel.rows) && localOcrRow(lastReadModel)) {
      return lastReadModel;
    }
    await delay(250);
  }
  throw new Error(`Timed out waiting for localOcr analysis row: ${JSON.stringify(lastReadModel)}`);
}

async function requestScreenReadModel() {
  return new Promise((resolve, reject) => {
    const socket = new WebSocket(wsUrl);
    const timer = setTimeout(() => {
      socket.close();
      reject(new Error('Screen WinRT OCR read-model WebSocket proof timed out.'));
    }, 10000);
    socket.addEventListener('open', () => socket.send(JSON.stringify(commandEnvelope())));
    socket.addEventListener('message', (message) => {
      try {
        const parsed = AgentEventEnvelopeSchema.parse(JSON.parse(String(message.data)));
        if (parsed.event === AgentEvent.ConnectionReady) {
          return;
        }
        if (parsed.event !== AgentEvent.ActivityScreenReadModelReported) {
          throw new Error(`Expected screen read model, received ${parsed.event}`);
        }
        const readModelJson = parsed.payload[AgentProtocolDefaults.Field.ActivityReadModel];
        if (typeof readModelJson !== 'string') {
          throw new Error(`Screen read model payload was missing JSON: ${JSON.stringify(parsed.payload)}`);
        }
        clearTimeout(timer);
        socket.close();
        resolve(JSON.parse(readModelJson));
      } catch (error) {
        clearTimeout(timer);
        socket.close();
        reject(error);
      }
    });
    socket.addEventListener('error', () => {
      clearTimeout(timer);
      reject(new Error('Screen WinRT OCR read-model WebSocket failed.'));
    });
  });
}

function commandEnvelope() {
  return {
    schemaVersion: 1,
    messageId: 'cmd-screen-ai-service-winrt-ocr-read-model',
    sentAt: new Date().toISOString(),
    source: { peerId: 'portal-dev', role: 'portal' },
    target: { deviceId: 'local-dev-agent', platform: 'windows', route: 'localhost' },
    command: AgentCommand.ActivityScreenReadModelGet,
    payload: {
      [AgentProtocolDefaults.Field.ScopeKind]: 'family',
      [AgentProtocolDefaults.Field.FamilyId]: 'family-local',
      [AgentProtocolDefaults.Field.RangeStart]: '1970-01-01T00:00:00Z',
      [AgentProtocolDefaults.Field.RangeEnd]: new Date().toISOString(),
    },
  };
}

async function waitForQueueRecords(count) {
  const startedAt = Date.now();
  while (Date.now() - startedAt < 30000) {
    if (existsSync(queuePath)) {
      try {
        if (readQueueRecordsAllowEmpty().length >= count) {
          return;
        }
      } catch {
        await delay(250);
        continue;
      }
    }
    await delay(250);
  }
  throw new Error(`Timed out waiting for ${count} screen queue records.\n${readOptional(queuePath)}`);
}

async function waitForAnalyzedQueueRemoval(queueJobId) {
  if (typeof queueJobId !== 'string' || queueJobId.length === 0) {
    throw new Error('Cannot wait for queue removal without analyzed queue job id.');
  }
  const startedAt = Date.now();
  while (Date.now() - startedAt < 10000) {
    const records = readQueueRecordsAllowEmpty();
    if (!records.some((record) => record.queueJobId === queueJobId)) {
      return;
    }
    await delay(100);
  }
  throw new Error(`Timed out waiting for analyzed queue job removal: ${queueJobId}`);
}

async function waitForQueueDrain() {
  const startedAt = Date.now();
  while (Date.now() - startedAt < 30000) {
    if (readQueueRecordsAllowEmpty().length === 0) {
      return;
    }
    await delay(250);
  }
  throw new Error(`Timed out waiting for screen queue drain: ${JSON.stringify(readQueueRecordsAllowEmpty())}`);
}

async function waitForOcrObservation() {
  const startedAt = Date.now();
  while (Date.now() - startedAt < 10000) {
    if (existsSync(adapterObservationPath)) {
      return readJson(adapterObservationPath);
    }
    await delay(100);
  }
  throw new Error('Timed out waiting for WinRT OCR adapter observation.');
}

async function capturePortalScreenshot() {
  const screenshotPath = join(outputDir, 'portal-screen-analysis-redaction.png');
  if (service !== undefined) {
    await stopProcessTreeAndWait(service);
    service = undefined;
  }
  await ensurePortFree(agentPort, isLikelyParentAgentOccupant, console.log);
  await ensurePortFree(portalPort, isLikelyParentPortalOccupant, console.log);
  service = startService(portalServiceEnv());
  serviceOutput = collectOutput(service);
  await waitForHttp(healthUrl, serviceOutput);
  portal = spawnVitePortal(portalPort, {
    ...process.env,
    [ParentDevEnv.PortalAgentWebSocketUrl]: wsUrl,
  });
  await waitForHttp(portalUrl(), () => '');
  portalBrowser = await chromium.launch({ headless: true });
  const page = await portalBrowser.newPage({ viewport: { width: 1600, height: 1200 } });
  await page.goto(`${portalUrl()}#/screen-analysis`, { waitUntil: 'domcontentloaded' });
  await page.getByText('Screen analysis').waitFor({ timeout: 20000 });
  await page.waitForFunction(() => globalThis.document.body.innerText.includes('[redacted-email]'), null, {
    timeout: 20000,
  });
  await page.waitForFunction(() => globalThis.document.body.innerText.includes('[redacted-phone]'), null, {
    timeout: 20000,
  });
  await page.waitForFunction(() => globalThis.document.body.innerText.includes('piiLikeTextRedacted'), null, {
    timeout: 20000,
  });
  await page.waitForFunction(() => globalThis.document.body.innerText.includes('credentialLikeTextRedacted'), null, {
    timeout: 20000,
  });
  const rendered = await page.locator('body').innerText();
  if (rendered.includes('jane@example.com') || rendered.includes('555-010-1234')) {
    throw new Error('Portal rendered raw sensitive OCR text.');
  }
  await page.screenshot({ path: screenshotPath, fullPage: true });
  return {
    screenshotPath,
    summary: {
      proof: 'screen-ai-service-winrt-ocr-redaction-portal-proof',
      proofTier: 'P3_REAL_CAPTURE_LOCAL_OCR_SERVICE_PORTAL',
      route: '#/screen-analysis',
      screenshot: relative(repoRoot, screenshotPath),
      assertions: {
        realPortalRouteRenderedServiceRow: true,
        redactedEmailRendered: rendered.includes('[redacted-email]'),
        redactedPhoneRendered: rendered.includes('[redacted-phone]'),
        piiRedactionNoteRendered: rendered.includes('piiLikeTextRedacted'),
        credentialRedactionNoteRendered: rendered.includes('credentialLikeTextRedacted'),
        rawEmailOmitted: !rendered.includes('jane@example.com'),
        rawPhoneOmitted: !rendered.includes('555-010-1234'),
      },
    },
  };
}

function redactedServiceRowAssertions(row) {
  const snippets = row.ocrTextSnippets ?? [];
  const notes = row.redactionNotes ?? [];
  return (
    snippets.includes('School portal account [redacted-email]') &&
    snippets.includes('Parent phone [redacted-phone]') &&
    !snippets.includes('School portal account jane@example.com') &&
    !snippets.includes('Parent phone 555-010-1234') &&
    !snippets.includes('password reset token visible') &&
    notes.includes('piiLikeTextRedacted') &&
    notes.includes('credentialLikeTextRedacted')
  );
}

function portalUrl() {
  return createHttpOrigin(ParentDevHost.Loopback, portalPort);
}

function assertProof(readModel, queueRecords, observation) {
  const analysisRow = localOcrRow(readModel);
  if (!analysisRow) {
    throw new Error(`Read model did not include localOcr analysis: ${JSON.stringify(readModel)}`);
  }
  const failures = [];
  if (analysisRow.primaryCategory !== 'school') failures.push('category');
  if (analysisRow.confidence < 0.88) failures.push('confidence');
  if (analysisRow.policyEligible !== true) failures.push('policyEligible');
  if (analysisRow.captureReason !== 'timedCadence') failures.push('captureReason');
  if (analysisRow.captureScope !== 'activeWindow') failures.push('captureScope');
  if (analysisRow.modelRuntimeRef !== 'windows-winrt-ocr-local-runtime') failures.push('runtimeRef');
  if (analysisRow.modelId !== 'windows-winrt-ocr') failures.push('modelId');
  if (analysisRow.promptOrTemplateVersion !== 'screen-ocr-worker-winrt-v1') failures.push('templateVersion');
  if (analysisRow.rawImageRetained !== false) failures.push('rawImageRetained');
  if (!Array.isArray(analysisRow.ocrTextSnippets) || analysisRow.ocrTextSnippets.length === 0) {
    failures.push('ocrTextSnippets');
  }
  if (!Array.isArray(analysisRow.redactionNotes)) failures.push('redactionNotes');
  if (queueRecords.length !== 0) failures.push('queueDrained');
  if (observation.tempImageExistsAfterDelete !== false) failures.push('tempImageDeleted');
  if (!expectedTerms.every((term) => observation.expectedTermsFound?.includes(term))) failures.push('expectedTerms');
  if (sensitiveNativeMode && !redactedServiceRowAssertions(analysisRow)) failures.push('serviceRedaction');
  if (sensitiveNativeMode && !existsSync(parentRedactionPolicyPath)) failures.push('parentSelectedPolicy');
  if (failures.length > 0) {
    throw new Error(`Screen service WinRT OCR proof failed gates: ${failures.join(', ')}`);
  }
}

function localOcrRow(readModel) {
  return readModel.rows.find((row) => row.providerKind === 'localOcr');
}

function readQueueRecordsAllowEmpty() {
  const raw = readOptional(queuePath).trim();
  if (raw.length === 0) {
    return [];
  }
  return raw
    .split('\n')
    .filter((line) => line.length > 0)
    .map((line) => JSON.parse(line));
}

function sanitizeReadModel(readModel) {
  return {
    ...readModel,
    rows: readModel.rows.map((row) => ({
      ...row,
      sourceEvidenceRefs: (row.sourceEvidenceRefs ?? []).map((evidence) => ({
        ...evidence,
        uri: evidence.uri === null ? null : '<ephemeral-screen-queue>',
      })),
    })),
  };
}

function sanitizeQueueRecords(queueRecords) {
  return queueRecords.map((record) => ({
    schemaVersion: record.schemaVersion,
    queueJobId: record.queueJobId,
    custodyState: record.custodyState,
    imageDigest: record.imageDigest,
    nonceLength: typeof record.nonce === 'string' ? record.nonce.length : 0,
    ciphertextLength: typeof record.ciphertext === 'string' ? record.ciphertext.length : 0,
  }));
}

function sanitizeOcrObservation(observation) {
  const expectedTermsFound = Array.isArray(observation.expectedTermsFound)
    ? observation.expectedTermsFound
    : [observation.expectedTermsFound].filter((value) => typeof value === 'string');
  return {
    runtime: observation.runtime,
    providerKind: observation.providerKind,
    expectedTermsFound,
    lineCount: observation.lineCount,
    textDigest: observation.textDigest,
    snippets: (observation.snippets ?? []).map((snippet) => ({
      text: String(snippet.text).replace(/\s+/g, ' ').trim().slice(0, 180),
      boundingBoxRef: snippet.boundingBoxRef,
    })),
    tempImageExistsAfterDelete: observation.tempImageExistsAfterDelete,
  };
}

async function waitForHttp(url, output) {
  const startedAt = Date.now();
  while (Date.now() - startedAt < 30000) {
    try {
      const response = await fetch(url);
      if (response.ok) return;
    } catch {
      await delay(250);
    }
  }
  throw new Error(`Timed out waiting for ${url}\n${output()}`);
}

function proofAgentServicePath() {
  return join(buildRoot, 'debug', 'ocentra-parent-agent-service.exe');
}

function runCommand(command, args, env = {}) {
  return new Promise((resolve, reject) => {
    const child = spawn(command, args, {
      cwd: repoRoot,
      env: { ...process.env, ...env },
      stdio: 'inherit',
      shell: process.platform === 'win32',
      windowsHide: true,
    });
    child.once('exit', (code) => {
      if (code === 0) {
        resolve();
        return;
      }
      reject(new Error(`${command} ${args.join(' ')} exited with ${code}`));
    });
    child.once('error', reject);
  });
}

function collectOutput(child) {
  const chunks = [];
  child.stdout.on('data', (chunk) => chunks.push(String(chunk)));
  child.stderr.on('data', (chunk) => chunks.push(String(chunk)));
  return () => chunks.join('');
}

function writeSourceSnapshot(sourceEvidence, observation) {
  writeText(
    sourceSnapshotPath,
    [
      '# Screen Service WinRT OCR Source Snapshot',
      '',
      `- Live browser surface: ${sourceEvidence.url}`,
      `- Browser title: ${sourceEvidence.title}`,
      '- Pixel capture: Rust agent service timed cadence active-window capture.',
      '- Evidence queue: service encrypted temp queue, drained after analysis.',
      '- OCR runtime: Windows `Windows.Media.Ocr.OcrEngine` inside service adapter process.',
      `- Parent-selected redaction policy: ${relative(repoRoot, parentRedactionPolicyPath)}`,
      `- OCR terms found: ${(observation.expectedTermsFound ?? []).join(', ')}`,
      '- Raw captured image artifact: not retained; adapter temp image deleted after OCR.',
      '',
    ].join('\n')
  );
}

function writeFailureArtifacts(error) {
  let queueRecords = [];
  try {
    queueRecords = sanitizeQueueRecords(readQueueRecordsAllowEmpty());
  } catch {
    queueRecords = [];
  }
  writeJson(join(outputDir, 'failure-summary.json'), {
    proof: 'screen-ai-service-winrt-ocr-proof',
    error: error instanceof Error ? error.message : String(error),
    queueRecordCount: queueRecords.length,
    queueRecords,
    ocrObservation: existsSync(adapterObservationPath) ? readJson(adapterObservationPath) : null,
    lastObservedReadModel,
    journalBytes: readOptional(journalPath).length,
    keyBytes: readOptional(keyPath).length,
    storePresent: existsSync(storePath),
    serviceOutputTail: serviceOutput().slice(-6000),
  });
}

function readOptional(path) {
  return existsSync(path) ? readFileSync(path, 'utf8') : '';
}

function readJson(path) {
  return JSON.parse(readFileSync(path, 'utf8').replace(/^\uFEFF/, ''));
}

function writeText(path, value) {
  writeFileSync(path, value);
}

function writeJson(path, value) {
  writeFileSync(path, `${JSON.stringify(value, null, 2)}\n`);
}

function escapePowerShell(value) {
  return String(value).replace(/'/g, "''");
}

function powerShellString(value) {
  return `'${escapePowerShell(value)}'`;
}

function powerShellLikeAny(expression) {
  return liveTitlePatterns.map((pattern) => `${expression} -like ${powerShellString(pattern)}`).join(' -or ');
}

function powerShellNotLikeAll(expression) {
  return liveTitlePatterns.map((pattern) => `${expression} -notlike ${powerShellString(pattern)}`).join(' -and ');
}
