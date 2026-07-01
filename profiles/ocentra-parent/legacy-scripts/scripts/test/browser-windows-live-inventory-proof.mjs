import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { existsSync, statSync } from 'node:fs';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { basename, join, relative } from 'node:path';

const root = process.cwd();
const outputDirectory = join(root, 'output', 'browser-plan-proof', '04-windows-browser-inventory-adapter');
const resultDirectory = join(root, 'test-results', 'browser-windows-live-inventory-proof');
const proofPath = join(resultDirectory, 'proof.json');
const manualProofPath = join(outputDirectory, '09-manual-platform-proof.md');

const knownExecutables = new Map([
  ['msedge.exe', managed('Microsoft Edge', 'edge')],
  ['chrome.exe', managed('Google Chrome', 'chrome')],
  ['brave.exe', manual('Brave Browser', 'brave')],
  ['vivaldi.exe', manual('Vivaldi Browser', 'unknown-chromium')],
  ['opera.exe', manual('Opera Browser', 'opera')],
  ['opera_gx.exe', manual('Opera GX Browser', 'opera')],
  ['chromium.exe', manual('Chromium', 'unknown-chromium')],
  ['firefox.exe', unsupported('Mozilla Firefox', 'firefox')],
  ['tor.exe', unsupported('Tor Browser', 'unknown')],
  ['duckduckgo.exe', unsupported('DuckDuckGo Browser', 'unknown')],
  ['arc.exe', unsupported('Arc Browser', 'unknown-chromium')],
]);

await main();

async function main() {
  const host = hostSnapshot();
  const discovered = uniqueRows([
    ...knownPathCandidates(host.environment),
    ...registryCandidates(),
    ...shortcutCandidates(),
    ...storePackageCandidates(),
    ...runningProcessCandidates(),
  ]);
  const rows = await Promise.all(discovered.map(toProofRow));
  const proof = {
    schemaVersion: 1,
    proofMode: 'browser-windows-live-inventory-proof',
    generatedAt: new Date().toISOString(),
    sourceWorkpack: 'docs/plans/browser-plan/workpacks/04-windows-browser-inventory-adapter.md',
    host,
    rows,
    summary: {
      totalRows: rows.length,
      sourceCounts: countBy(rows.flatMap((row) => row.sourceKinds)),
      managementTierCounts: countBy(rows.map((row) => row.managementTier)),
      supportTierCounts: countBy(rows.map((row) => row.supportTier)),
      signatureStatusCounts: countBy(rows.map((row) => row.signatureStatus)),
      packageRows: rows.filter((row) => row.installState === 'packaged').length,
      productClaimed: false,
      exactUrlClaimedRows: rows.filter((row) => row.exactUrlCapability === 'managed-exact-url-available').length,
      rawPathStored: false,
      rawUrlStored: false,
    },
    noClaimLabels: [
      'live-windows-inventory-only',
      'raw-paths-redacted',
      'file-hashes-are-evidence-refs-not-content',
      'signature-subjects-hashed',
      'registry-and-shortcut-inputs-redacted',
      'store-package-inputs-redacted',
      'running-processes-are-process-only-when-not-managed',
      'exact-url-tab-content-not-captured',
      'browser-blocking-and-enforcement-not-claimed',
    ],
  };

  proof.failures = validateProof(proof);
  if (proof.failures.length > 0) {
    throw new Error(`browser Windows live inventory proof failed:\n${proof.failures.join('\n')}`);
  }

  await mkdir(outputDirectory, { recursive: true });
  await mkdir(resultDirectory, { recursive: true });
  await writeFile(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
  await writeFile(manualProofPath, `${manualMarkdown(proof)}\n`);

  console.log('browser-windows-live-inventory-proof-ok=true');
  console.log(`proof=${relativePath(proofPath)}`);
  console.log(`manualProof=${relativePath(manualProofPath)}`);
  console.log(
    `rows=${proof.summary.totalRows} managed=${proof.summary.managementTierCounts.managed ?? 0} manualRequired=${
      proof.summary.managementTierCounts['manual-required'] ?? 0
    } unsupported=${proof.summary.managementTierCounts.unsupported ?? 0}`
  );
}

function hostSnapshot() {
  return {
    platform: process.platform,
    arch: process.arch,
    release: runPowerShell('(Get-CimInstance Win32_OperatingSystem).Version', '').trim(),
    environment: {
      programFilesRef: optionalPathRef(process.env.ProgramFiles),
      programFilesX86Ref: optionalPathRef(process.env['ProgramFiles(x86)']),
      localAppDataRef: optionalPathRef(process.env.LOCALAPPDATA),
    },
  };
}

function knownPathCandidates(environment) {
  const roots = [process.env.ProgramFiles, process.env['ProgramFiles(x86)'], process.env.LOCALAPPDATA].filter(Boolean);
  const templates = [
    ['Microsoft', 'Edge', 'Application', 'msedge.exe'],
    ['Microsoft', 'Edge Beta', 'Application', 'msedge.exe'],
    ['Microsoft', 'Edge Dev', 'Application', 'msedge.exe'],
    ['Microsoft', 'Edge SxS', 'Application', 'msedge.exe'],
    ['Google', 'Chrome', 'Application', 'chrome.exe'],
    ['Google', 'Chrome for Testing', 'Application', 'chrome.exe'],
    ['BraveSoftware', 'Brave-Browser', 'Application', 'brave.exe'],
    ['Vivaldi', 'Application', 'vivaldi.exe'],
    ['Opera Software', 'Opera Stable', 'opera.exe'],
    ['Opera Software', 'Opera GX Stable', 'opera.exe'],
    ['Chromium', 'Application', 'chromium.exe'],
    ['Mozilla Firefox', 'firefox.exe'],
    ['Firefox Developer Edition', 'firefox.exe'],
    ['Firefox Nightly', 'firefox.exe'],
    ['Tor Browser', 'Browser', 'firefox.exe'],
    ['DuckDuckGo', 'duckduckgo.exe'],
    ['Arc', 'arc.exe'],
  ];
  return roots.flatMap((rootPath) =>
    templates.map((parts) => candidate(join(rootPath, ...parts), 'known-path')).filter((row) => row.exists)
  );
}

function registryCandidates() {
  const script = String.raw`
$roots = @(
  'HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall\*',
  'HKLM:\Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\*',
  'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\*'
)
$rows = foreach ($root in $roots) {
  Get-ItemProperty -Path $root -ErrorAction SilentlyContinue |
    Where-Object { $_.DisplayName -match 'Edge|Chrome|Brave|Vivaldi|Opera|Firefox|Tor|DuckDuckGo|Arc|Chromium' } |
    Select-Object DisplayName, DisplayIcon, InstallLocation, Publisher
}
$rows | ConvertTo-Json -Depth 4
`;
  const rows = parseJson(runPowerShell(script, '[]'));
  return asArray(rows).flatMap((row) => {
    const candidates = [];
    const iconPath = executableTargetPath(row.DisplayIcon);
    if (iconPath !== undefined) {
      candidates.push(candidate(iconPath, 'registry-uninstall'));
    }
    if (typeof row.InstallLocation === 'string' && row.InstallLocation.trim() !== '') {
      for (const path of installLocationCandidates(row.InstallLocation.trim())) {
        candidates.push(candidate(path, 'registry-uninstall'));
      }
    }
    return candidates.filter((item) => item.exists);
  });
}

function shortcutCandidates() {
  const script = String.raw`
$shell = New-Object -ComObject WScript.Shell
$roots = @(
  "$env:ProgramData\Microsoft\Windows\Start Menu\Programs",
  "$env:APPDATA\Microsoft\Windows\Start Menu\Programs"
)
$rows = foreach ($root in $roots) {
  if (Test-Path $root) {
    Get-ChildItem -Path $root -Filter *.lnk -Recurse -ErrorAction SilentlyContinue |
      ForEach-Object {
        $shortcut = $shell.CreateShortcut($_.FullName)
        [PSCustomObject]@{ Name = $_.Name; TargetPath = $shortcut.TargetPath }
      }
  }
}
$rows | Where-Object { $_.TargetPath -match 'msedge|chrome|brave|vivaldi|opera|firefox|tor|duckduckgo|arc|chromium' } |
  ConvertTo-Json -Depth 4
`;
  const rows = parseJson(runPowerShell(script, '[]'));
  return asArray(rows)
    .map((row) => candidate(executableTargetPath(row.TargetPath), 'start-menu-shortcut'))
    .filter((row) => row.exists);
}

function runningProcessCandidates() {
  const names = [...knownExecutables.keys()].map((name) => `'${name}'`).join(',');
  const script = `
$names = @(${names})
Get-CimInstance Win32_Process | Where-Object { $names -contains $_.Name.ToLowerInvariant() } |
  Select-Object Name, ProcessId, ExecutablePath | ConvertTo-Json -Depth 4
`;
  const rows = parseJson(runPowerShell(script, '[]'));
  return asArray(rows).map((row) =>
    candidate(row.ExecutablePath || row.Name, 'running-process', {
      processIdHash: stableHash(String(row.ProcessId ?? 'unknown')),
      processName: String(row.Name ?? '').toLowerCase(),
    })
  );
}

function storePackageCandidates() {
  const script = String.raw`
Get-AppxPackage -ErrorAction SilentlyContinue |
  Where-Object { "$($_.Name) $($_.PackageFullName)" -match 'Edge|Chrome|Brave|Vivaldi|Opera|Firefox|Tor|DuckDuckGo|Arc|Chromium' } |
  Select-Object Name, PackageFullName, PackageFamilyName, InstallLocation |
  ConvertTo-Json -Depth 4
`;
  const rows = parseJson(runPowerShell(script, '[]'));
  return asArray(rows).map((row) =>
    packageCandidate(String(row.Name ?? row.PackageFullName ?? ''), {
      packageFullNameHash:
        typeof row.PackageFullName === 'string' && row.PackageFullName !== ''
          ? stableHash(row.PackageFullName.toLowerCase())
          : null,
      packageFamilyNameHash:
        typeof row.PackageFamilyName === 'string' && row.PackageFamilyName !== ''
          ? stableHash(row.PackageFamilyName.toLowerCase())
          : null,
      installLocationRef:
        typeof row.InstallLocation === 'string' && row.InstallLocation !== ''
          ? stableHash(row.InstallLocation.toLowerCase())
          : null,
    })
  );
}

async function toProofRow(row) {
  if (row.sourceKinds.has('store-package')) {
    const classification = packagedClassification(row.packageName);
    return {
      sourceKinds: [...row.sourceKinds].sort(),
      productName: classification.productName,
      browserFamily: classification.browserFamily,
      executableName: null,
      pathRef: stableHash(row.packageName.toLowerCase()),
      packageFullNameHash: row.packageFullNameHash,
      packageFamilyNameHash: row.packageFamilyNameHash,
      installLocationRef: row.installLocationRef,
      fileSha256: null,
      fileSizeBytes: null,
      signatureStatus: 'not-checked-package',
      signerSubjectHash: null,
      installState: 'packaged',
      runningState: 'not-running',
      managementTier: classification.managementTier,
      supportTier: classification.supportTier,
      exactUrlCapability: classification.exactUrlCapability,
      activeTabCapability: classification.activeTabCapability,
      capabilityStatus: classification.capabilityStatus,
      processIdHash: null,
      noClaims: ['raw-path', 'raw-url', 'page-title', 'page-body', 'active-tab', 'block-enforcement'],
    };
  }
  const executable = basename(row.path).toLowerCase();
  const classification = knownExecutables.get(executable) ?? unknown();
  const stat = row.exists ? statSync(row.path) : null;
  const signature = row.exists ? authenticodeSignature(row.path) : { status: 'not-checked', signerSubjectHash: null };
  return {
    sourceKinds: [...row.sourceKinds].sort(),
    productName: classification.productName,
    browserFamily: classification.browserFamily,
    executableName: executable,
    pathRef: stableHash(row.path.toLowerCase()),
    fileSha256: row.exists ? await sha256File(row.path) : null,
    fileSizeBytes: stat?.size ?? null,
    signatureStatus: signature.status,
    signerSubjectHash: signature.signerSubjectHash,
    installState: row.exists ? installState(row.path) : 'candidate-running',
    runningState: row.sourceKinds.has('running-process') ? 'running-unmanaged' : 'not-running',
    managementTier: classification.managementTier,
    supportTier: classification.supportTier,
    exactUrlCapability: classification.exactUrlCapability,
    activeTabCapability: classification.activeTabCapability,
    capabilityStatus: row.sourceKinds.has('running-process')
      ? classification.runningCapabilityStatus
      : classification.capabilityStatus,
    processIdHash: row.processIdHash ?? null,
    noClaims: ['raw-path', 'raw-url', 'page-title', 'page-body', 'active-tab', 'block-enforcement'],
  };
}

function candidate(path, sourceKind, extra = {}) {
  const normalizedPath = path === null ? '' : String(path);
  const exists = normalizedPath !== '' && existsSync(normalizedPath) && statSync(normalizedPath).isFile();
  return {
    path: normalizedPath,
    exists,
    sourceKinds: new Set([sourceKind]),
    ...extra,
  };
}

function packageCandidate(packageName, extra = {}) {
  return {
    path: `package:${stableHash(packageName.toLowerCase())}`,
    packageName,
    exists: false,
    sourceKinds: new Set(['store-package']),
    ...extra,
  };
}

function installLocationCandidates(path) {
  return [...knownExecutables.keys()].flatMap((executable) => [
    join(path, executable),
    join(path, 'Application', executable),
  ]);
}

function uniqueRows(rows) {
  const byPath = new Map();
  for (const row of rows.filter((item) => item.path !== '')) {
    const key = row.path.toLowerCase();
    const existing = byPath.get(key);
    if (existing) {
      for (const sourceKind of row.sourceKinds) existing.sourceKinds.add(sourceKind);
      existing.processIdHash ??= row.processIdHash;
      continue;
    }
    byPath.set(key, row);
  }
  return [...byPath.values()];
}

function executableTargetPath(value) {
  if (typeof value !== 'string' || value.trim() === '') return null;
  const trimmed = value.trim();
  if (trimmed.startsWith('"')) {
    const quoted = trimmed.slice(1).split('"')[0] || null;
    return quoted !== undefined && knownExecutables.has(basename(quoted).toLowerCase()) ? quoted : null;
  }
  const lower = trimmed.toLowerCase();
  const executable = [...knownExecutables.keys()]
    .map((name) => ({ name, index: executableNameIndex(lower, name) }))
    .filter((match) => match.index !== undefined)
    .sort((left, right) => left.index - right.index)[0];
  if (executable) return trimmed.slice(0, executable.index + executable.name.length);
  return null;
}

function executableNameIndex(target, executableName) {
  let start = 0;
  while (start < target.length) {
    const index = target.indexOf(executableName, start);
    if (index < 0) return null;
    const before = index === 0 ? '' : target[index - 1];
    const afterIndex = index + executableName.length;
    const after = afterIndex >= target.length ? '' : target[afterIndex];
    const beforeOk = before === '' || before === '\\' || before === '/' || before === '"' || before === ' ';
    const afterOk = after === '' || after === '"' || after === ',' || after === ' ';
    if (beforeOk && afterOk) return index;
    start = index + executableName.length;
  }
  return null;
}

function managed(productName, browserFamily) {
  return {
    productName,
    browserFamily,
    managementTier: 'managed',
    supportTier: 'candidate',
    exactUrlCapability: 'unavailable',
    activeTabCapability: 'unavailable',
    capabilityStatus: 'managed-profile-missing',
    runningCapabilityStatus: 'unmanaged-browser',
  };
}

function manual(productName, browserFamily) {
  return {
    productName,
    browserFamily,
    managementTier: 'manual-required',
    supportTier: 'candidate',
    exactUrlCapability: 'manual-required',
    activeTabCapability: 'manual-required',
    capabilityStatus: 'permission-limited',
    runningCapabilityStatus: 'unmanaged-browser',
  };
}

function packagedManual(productName, browserFamily) {
  return {
    productName,
    browserFamily,
    managementTier: 'manual-required',
    supportTier: 'manual-required',
    exactUrlCapability: 'manual-required',
    activeTabCapability: 'manual-required',
    capabilityStatus: 'permission-limited',
    runningCapabilityStatus: 'unmanaged-browser',
  };
}

function unsupported(productName, browserFamily) {
  return {
    productName,
    browserFamily,
    managementTier: 'unsupported',
    supportTier: 'unsupported',
    exactUrlCapability: 'unsupported',
    activeTabCapability: 'unsupported',
    capabilityStatus: 'unsupported-browser',
    runningCapabilityStatus: 'unsupported-browser',
  };
}

function unknown() {
  return unsupported('Unknown Browser', 'unknown');
}

function packagedClassification(packageName) {
  const normalized = String(packageName).toLowerCase();
  if (normalized.includes('microsoftedge') || normalized.includes('edge')) {
    return packagedManual('Microsoft Edge', 'edge');
  }
  if (normalized.includes('chrome')) return packagedManual('Google Chrome', 'chrome');
  if (normalized.includes('brave')) return packagedManual('Brave Browser', 'brave');
  if (normalized.includes('vivaldi')) return packagedManual('Vivaldi Browser', 'unknown-chromium');
  if (normalized.includes('opera')) return packagedManual('Opera Browser', 'opera');
  if (normalized.includes('chromium')) return packagedManual('Chromium', 'unknown-chromium');
  if (normalized.includes('firefox')) return unsupported('Mozilla Firefox', 'firefox');
  if (normalized.includes('tor')) return unsupported('Tor Browser', 'unknown');
  if (normalized.includes('duckduckgo')) return unsupported('DuckDuckGo Browser', 'unknown');
  if (normalized.includes('arc')) return unsupported('Arc Browser', 'unknown-chromium');
  return unknown();
}

function installState(path) {
  const lower = path.toLowerCase();
  if (lower.includes('\\windowsapps\\')) return 'packaged';
  if (lower.includes('portable')) return 'portable';
  return 'installed';
}

async function sha256File(path) {
  const data = await readFile(path);
  return createHash('sha256').update(data).digest('hex');
}

function authenticodeSignature(path) {
  const script = `
$sig = Get-AuthenticodeSignature -LiteralPath ${JSON.stringify(path)}
[PSCustomObject]@{ Status = [string]$sig.Status; Subject = [string]$sig.SignerCertificate.Subject } | ConvertTo-Json -Depth 3
`;
  const row = parseJson(runPowerShell(script, '{}'));
  return {
    status: typeof row.Status === 'string' && row.Status !== '' ? row.Status : 'unknown',
    signerSubjectHash: typeof row.Subject === 'string' && row.Subject !== '' ? stableHash(row.Subject) : null,
  };
}

function validateProof(proof) {
  const failures = [];
  if (process.platform !== 'win32') failures.push('live Windows proof must run on Windows');
  if (proof.rows.length === 0) failures.push('no live browser inventory rows were captured');
  if (proof.summary.exactUrlClaimedRows !== 0) failures.push('live inventory proof must not claim exact URL support');
  const serialized = JSON.stringify(proof);
  if (/[A-Z]:\\\\/i.test(serialized) || /\\\\Users\\\\/i.test(serialized))
    failures.push('proof leaked a raw Windows path');
  if (/https?:\/\//i.test(serialized)) failures.push('proof leaked a raw URL');
  for (const row of proof.rows) {
    if (row.installState !== 'packaged' && (!row.fileSha256 || row.fileSha256.length !== 64))
      failures.push(`${row.productName} missing file hash ref`);
    if (row.installState === 'packaged' && row.executableName !== undefined)
      failures.push(`${row.productName} packaged row must not claim an executable`);
    if (row.exactUrlCapability === 'managed-exact-url-available') failures.push(`${row.productName} claimed exact URL`);
    if (!row.noClaims.includes('page-body')) failures.push(`${row.productName} missing no-content no-claim`);
  }
  return failures;
}

function manualMarkdown(proof) {
  const rows = proof.rows
    .map(
      (row) =>
        `| ${row.productName} | ${row.executableName ?? 'package-ref-only'} | ${row.sourceKinds.join(', ')} | ${row.managementTier} | ${row.supportTier} | ${row.exactUrlCapability} | ${row.signatureStatus} | ${row.pathRef.slice(0, 12)} |`
    )
    .join('\n');
  return [
    '# WP04 Live Windows Browser Inventory Proof',
    '',
    `Generated: ${proof.generatedAt}`,
    '',
    'This proof ran against the local Windows host and stores only redacted refs, hashes, counts, executable basenames, and capability labels.',
    'It does not store raw paths, raw URLs, page titles, page bodies, browser profile data, cookies, tokens, tabs, or browsing content.',
    '',
    `Rows captured: ${proof.summary.totalRows}`,
    `Source counts: ${JSON.stringify(proof.summary.sourceCounts)}`,
    '',
    '| Product | Executable | Sources | Management | Support | Exact URL | Signature | Path Ref |',
    '| --- | --- | --- | --- | --- | --- | --- | --- |',
    rows,
    '',
    'No product checklist upgrade is claimed. Live registry, shortcut, known-path, store-package, and process evidence improves the WP04 manual platform proof, but exact URL/tab evidence, active-tab certainty, browser content capture, AppLocker/App Control application, blocking, rollback, and enforcement remain unclaimed.',
  ].join('\n');
}

function runPowerShell(script, fallback) {
  try {
    return execFileSync('powershell', ['-NoProfile', '-Command', script], {
      cwd: root,
      encoding: 'utf8',
      stdio: ['ignore', 'pipe', 'pipe'],
      windowsHide: true,
      timeout: 120000,
    });
  } catch {
    return fallback;
  }
}

function parseJson(text) {
  const trimmed = String(text).trim();
  if (trimmed === '') return null;
  return JSON.parse(trimmed);
}

function asArray(value) {
  if (Array.isArray(value)) return value;
  if (value === null || value === undefined) return [];
  return [value];
}

function stableHash(value) {
  return createHash('sha256').update(value).digest('hex');
}

function optionalPathRef(value) {
  return typeof value === 'string' && value !== '' ? stableHash(value.toLowerCase()) : null;
}

function countBy(values) {
  return values.reduce((counts, value) => {
    counts[value] = (counts[value] ?? 0) + 1;
    return counts;
  }, {});
}

function relativePath(path) {
  return relative(root, path).replaceAll('\\', '/');
}
