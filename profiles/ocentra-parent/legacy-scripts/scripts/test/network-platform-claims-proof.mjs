import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { existsSync, mkdirSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const proofRoot = join('output', 'network-plan-proof', '52-platform-claims-proof');
const testRoot = join('test-results', 'network-platform-claims-proof');
mkdirSync(proofRoot, { recursive: true });
mkdirSync(testRoot, { recursive: true });

const sourceBranch = runText('git', ['branch', '--show-current']).trim();
const sourceCommit = runText('git', ['rev-parse', 'HEAD']).trim();
const sourceOriginMain = runText('git', ['rev-parse', 'origin/main']).trim();
const sourceMergeBase = runText('git', ['merge-base', 'HEAD', 'origin/main']).trim();
const sourceStatusShort = readSourceStatusShort();
const expectedAndroidTarget = {
  targetRef: 'android-physical-target-sm-g965w-row40a',
  serial: process.env.NETWORK_ANDROID_TARGET_SERIAL ?? '192.168.2.45:5555',
  product: process.env.NETWORK_ANDROID_TARGET_PRODUCT ?? 'star2qltecs',
  model: process.env.NETWORK_ANDROID_TARGET_MODEL ?? 'SM_G965W',
  device: process.env.NETWORK_ANDROID_TARGET_DEVICE ?? 'star2qltecs',
  androidRelease: process.env.NETWORK_ANDROID_TARGET_RELEASE ?? '10',
  abi: process.env.NETWORK_ANDROID_TARGET_ABI ?? 'arm64-v8a',
};

writeFileSync(
  join(proofRoot, 'expected-platform-claims.json'),
  `${JSON.stringify(
    {
      row: 52,
      requiredTargets: [
        'Windows Firewall',
        'Windows WFP',
        'Android VpnService',
        'Apple Network Extension macOS',
        'Apple Network Extension iOS',
        'Linux nftables',
        'Linux eBPF',
        'Linux TUN',
      ],
      platformClaimInvariants: [
        'every platform claim names fixture platform scope, permission/entitlement, capability, and audit refs',
        'every ready claim names permission, entitlement, capability, or manual follow-up refs',
        'every platform claim names an audit ref',
        'local Windows, Android SDK, Linux WSL, and Apple CI-unavailable probe observations align with adapter status rows',
        'named Android physical target identity is attached as read-only row40a proof context without VpnService execution',
        'manual-required and unavailable states remain reportable without live execution',
        'adapter authorization is accepted only on ready platform claim rows',
        'UI has no policy authority',
        'no proof source publishes enforcement commands',
      ],
      notClaimed: [
        'generic platform support',
        'live adapter execution',
        'host packet blocking',
        'exact URL from network-only evidence',
        'decrypted payload or page content',
        'UI policy authority',
      ],
    },
    null,
    2
  )}\n`
);
writeFileSync(join(proofRoot, '11-manual-platform-proof.md'), manualPlatformProof());
const localPlatformObservations = collectLocalPlatformObservations();
writeFileSync(
  join(proofRoot, 'local-platform-observations.json'),
  `${JSON.stringify(localPlatformObservations, null, 2)}\n`
);

const commands = [
  {
    name: 'network-platform-claim-manifest-tests',
    command: 'cargo',
    args: ['test', '-p', 'ocentra-network-evidence', 'platform_claims'],
    log: join(proofRoot, 'platform-claims-tests.log'),
  },
  {
    name: 'network-local-platform-probe-tests',
    command: 'cargo',
    args: ['test', '-p', 'ocentra-network-evidence', 'local_platform_probe'],
    log: join(proofRoot, 'local-platform-probe-tests.log'),
  },
  {
    name: 'network-evidence-clippy',
    command: 'cargo',
    args: ['clippy', '-p', 'ocentra-network-evidence', '--all-targets', '--', '-D', 'warnings'],
    log: join(proofRoot, 'clippy.log'),
  },
  {
    name: 'source-shape',
    command: 'node',
    args: ['scripts/check-source-shape.mjs'],
    log: join(proofRoot, 'source-shape.log'),
  },
  {
    name: 'diff-check',
    command: 'git',
    args: ['diff', '--check'],
    log: join(proofRoot, 'diff-check.log'),
  },
];
const commandResults = commands.map(runCommand);
writeFileSync(join(proofRoot, '12-validation-commands.log'), validationCommandsLog(commandResults));

const proof = {
  proof: 'network-platform-claims',
  checkedAt: new Date().toISOString(),
  branch: sourceBranch,
  commit: sourceCommit,
  originMain: sourceOriginMain,
  mergeBase: sourceMergeBase,
  sourceStatusShort,
  proofRoot,
  testRoot,
  commands: commandResults,
  artifacts: {
    expectedPlatformClaims: join(proofRoot, 'expected-platform-claims.json'),
    manualPlatformProof: join(proofRoot, '11-manual-platform-proof.md'),
    localPlatformObservations: join(proofRoot, 'local-platform-observations.json'),
    validationCommands: join(proofRoot, '12-validation-commands.log'),
    proofSummary: join(proofRoot, 'proof-summary.json'),
    testProof: join(testRoot, 'proof.json'),
  },
  provenRows: ['52 Platform claim manifest proof'],
  provenRootGates: [
    'platform claims name fixture platform scope, permission/entitlement, capability, and audit refs',
    'manual-required missing artifacts have explicit follow-up entries',
    'unavailable platform rows remain visible without adapter authorization',
    'local Windows, Android SDK, Linux WSL, and Apple CI-unavailable probe observations stay read-only/manual/unavailable and match adapter status',
    'named Android physical target identity proof context is recorded without upgrading Android VpnService execution',
    'proof sources cannot authorize adapters unless the platform claim row is ready',
    'generic platform support and live adapter execution claims are rejected',
    'proof sources cannot publish enforcement commands',
    'UI policy authority remains rejected',
  ],
  notClaimed: [
    'generic platform support',
    'live host adapter mutation or packet blocking',
    'production platform support',
    'Device Owner or authority-enrolled Android support',
    'exact URL, page content, or decrypted payload from network-only evidence',
    'policy engine execution',
    'enforcement command publication',
  ],
};
writeFileSync(join(proofRoot, 'proof-summary.json'), `${JSON.stringify(proof, null, 2)}\n`);
writeFileSync(join(testRoot, 'proof.json'), `${JSON.stringify(proof, null, 2)}\n`);
console.log('network-platform-claims-proof-ok:manifest-tests,local-platform-probe,clippy,source-shape');
console.log(`proof=${join(proofRoot, 'proof-summary.json')}`);

function runCommand(entry) {
  const result = spawnSync(entry.command, entry.args, { encoding: 'utf8', shell: false });
  writeFileSync(entry.log, `${result.stdout ?? ''}${result.stderr ?? ''}`);
  if (result.status !== 0) {
    throw new Error(`${entry.name} failed with exit ${result.status}`);
  }
  return {
    name: entry.name,
    command: [entry.command, ...entry.args].join(' '),
    status: result.status,
    log: entry.log,
  };
}

function manualPlatformProof() {
  return `# Network Platform Claim Manual Proof

Row: 52 Platform claim manifest proof

Fixture target rows:

- Windows Firewall: fixture Windows OS scope ref, adapter authorization ref, capability proof ref, target/rule capability refs, audit ref.
- Windows WFP: fixture target/provider/layer refs, administrator permission, driver signing/package, provider registration, layer capability, lab result, audit ref.
- Android VpnService: fixture package/service/device refs, VpnService declaration, user consent, package identity, virtual interface, traffic observation, Device Owner proof when claimed, audit ref.
- Apple Network Extension macOS: fixture bundle/extension/device refs, developer team, entitlement approval, provisioning, signing, declaration, configuration, supervision/MDM proof when claimed, audit ref.
- Apple Network Extension iOS: fixture bundle/extension/device refs, developer team, entitlement approval, provisioning, signing, declaration, configuration, supervision/MDM proof when claimed, audit ref.
- Linux nftables: fixture distro/kernel refs, permission, adapter API, adapter plan, service-manager scope, lab result, audit ref.
- Linux eBPF: fixture distro/kernel refs, permission, adapter API, adapter plan, service-manager scope, lab result, audit ref.
- Linux TUN: fixture distro/kernel refs, permission, adapter API, adapter plan, service-manager scope, lab result, audit ref.

Manual-required and unavailable labels:

- Missing WFP administrator permission records manual follow-up label \`windows-wfp.administrator-permission\`.
- Unavailable Linux TUN rows remain visible, do not authorize adapter apply, and record follow-up label \`linux-adapter.permission\` when permission proof is absent.
- Non-ready platform rows cannot carry adapter authorization, so dry-run, research-only, manual-required, and unavailable states remain non-executable.
- Local host probe observations are written to \`local-platform-observations.json\`: Windows read-only command summaries, Android SDK/emulator availability, row40a named physical-target identity summaries, WSL/Linux tool availability, and macOS/iOS CI/manual-unavailable boundaries.

Screenshots/logs:

- UI screenshots are N/A for this row because the proof is a Rust manifest/harness boundary, not a portal rendering change.
- Command logs are written by this harness under \`output/network-plan-proof/52-platform-claims-proof/\`.

Known follow-up owner:

- Platform adapter implementation owners must replace fixture refs with real OS/device/permission artifacts before any production platform support claim.

No-claim boundary:

- No generic platform support.
- No live adapter execution.
- No host packet blocking.
- No exact URL from network-only evidence.
- No decrypted payload or page content.
- No UI policy authority.
- No enforcement command publication.
`;
}

function collectLocalPlatformObservations() {
  const androidSdkRoot =
    process.env.ANDROID_HOME ?? process.env.ANDROID_SDK_ROOT ?? join(process.env.LOCALAPPDATA ?? '', 'Android', 'Sdk');
  const adbPath = existingCommand(join(androidSdkRoot, 'platform-tools', 'adb.exe'), 'adb');
  const emulatorPath = existingCommand(join(androidSdkRoot, 'emulator', 'emulator.exe'), 'emulator');
  const windowsNetsh = runProbe('netsh', ['advfirewall', 'show', 'allprofiles', 'state']);
  const windowsPktmon = runProbe('pktmon', ['status']);
  const androidConnect = runProbe(adbPath, ['connect', expectedAndroidTarget.serial]);
  const androidDevices = runProbe(adbPath, ['devices', '-l']);
  const androidAvds = runProbe(emulatorPath, ['-list-avds']);
  const androidModel = runProbe(adbPath, ['-s', expectedAndroidTarget.serial, 'shell', 'getprop', 'ro.product.model']);
  const androidProduct = runProbe(adbPath, ['-s', expectedAndroidTarget.serial, 'shell', 'getprop', 'ro.product.name']);
  const androidDevice = runProbe(adbPath, [
    '-s',
    expectedAndroidTarget.serial,
    'shell',
    'getprop',
    'ro.product.device',
  ]);
  const androidRelease = runProbe(adbPath, [
    '-s',
    expectedAndroidTarget.serial,
    'shell',
    'getprop',
    'ro.build.version.release',
  ]);
  const androidAbi = runProbe(adbPath, ['-s', expectedAndroidTarget.serial, 'shell', 'getprop', 'ro.product.cpu.abi']);
  const wslTools = runProbe('wsl', [
    '-d',
    'Ubuntu-22.04',
    'sh',
    '-lc',
    'command -v nft; command -v ip; command -v tcpdump; command -v bpftool || true; command -v tshark || true',
  ]);
  const androidTargetRow = parseAndroidTargetRow(androidDevices.stdout, expectedAndroidTarget.serial);
  const androidPhysicalDeviceDetected = androidTargetRow?.state === 'device';
  const androidAvdDetected = androidAvds.stdout.trim().length > 0;
  const linuxToolSetReady = ['nft', 'ip', 'tcpdump'].every((tool) => wslTools.stdout.includes(tool));
  const androidObserved = {
    serial: expectedAndroidTarget.serial,
    product: androidTargetRow?.product ?? firstLine(androidProduct.stdout),
    model: androidTargetRow?.model ?? firstLine(androidModel.stdout).replace(/-/gu, '_'),
    device: androidTargetRow?.device ?? firstLine(androidDevice.stdout),
    androidRelease: firstLine(androidRelease.stdout),
    abi: firstLine(androidAbi.stdout),
    getpropModelSha256: hashText(firstLine(androidModel.stdout)),
  };
  const androidIdentityMismatches = androidPhysicalDeviceDetected ? androidMismatches(androidObserved) : [];

  return {
    checkedAt: new Date().toISOString(),
    hostPlatform: process.platform,
    windows: {
      firewallProbeState: windowsNetsh.status === 0 ? 'read-only-observed' : 'manual-required',
      wfpProbeState: windowsPktmon.status === 0 ? 'read-only-observed' : 'manual-required',
      probes: [windowsNetsh.summary, windowsPktmon.summary],
    },
    android: {
      sdkProbeState: androidDevices.status === 0 || androidAvds.status === 0 ? 'manual-required' : 'unavailable',
      physicalDeviceDetected: androidPhysicalDeviceDetected,
      avdDetected: androidAvdDetected,
      physicalTargetProof: {
        proofRef: 'android-physical-target-proof-row40a',
        expectedTarget: expectedAndroidTarget,
        observed: androidObserved,
        state:
          !androidConnect.summary.status && !androidDevices.summary.status && androidPhysicalDeviceDetected
            ? androidIdentityMismatches.length === 0
              ? 'physical-device-observed'
              : 'mismatch'
            : 'manual-required',
        identityMatched: androidPhysicalDeviceDetected && androidIdentityMismatches.length === 0,
        mismatches: androidIdentityMismatches,
        liveVpnServiceExecutionClaimed: false,
        packetCaptureClaimed: false,
        packetBlockClaimed: false,
        productionAndroidSupportClaimed: false,
      },
      probes: [
        androidConnect.summary,
        androidDevices.summary,
        androidAvds.summary,
        androidModel.summary,
        androidProduct.summary,
        androidDevice.summary,
        androidRelease.summary,
        androidAbi.summary,
      ],
    },
    linux: {
      wslProbeState: linuxToolSetReady ? 'lab-ready' : 'manual-required',
      requiredToolsObserved: linuxToolSetReady,
      probes: [wslTools.summary],
    },
    apple: {
      macOsProbeState: process.platform === 'darwin' ? 'ci-only' : 'unavailable-on-windows-host',
      iosProbeState: 'ci-only-manual-device-required',
      localWindowsHostCanProveAppleNetworkExtension: false,
    },
    unsupportedClaims: {
      exactUrlClaimed: false,
      decryptedPayloadClaimed: false,
      pageContentClaimed: false,
      liveAdapterExecutionClaimed: false,
      enforcementCommandClaimed: false,
      uiPolicyAuthorityClaimed: false,
      productionPlatformSupportClaimed: false,
    },
  };
}

function existingCommand(path, fallback) {
  return path && existsSync(path) ? path : fallback;
}

function runProbe(command, args) {
  const result = spawnSync(command, args, { encoding: 'utf8', shell: false });
  const stdout = normalizeProbeOutput(result.stdout ?? '');
  const stderr = normalizeProbeOutput(result.stderr ?? '');
  return {
    command: redactCommand([command, ...args]),
    status: result.status,
    stdout,
    stderr,
    error: result.error?.message,
    summary: {
      command: redactCommand([command, ...args]),
      status: result.status,
      stdoutLineCount: lineCount(stdout),
      stderrLineCount: lineCount(stderr),
      stdoutSha256: hashText(stdout),
      stderrSha256: hashText(stderr),
      error: result.error?.message,
    },
  };
}

function parseAndroidTargetRow(output, serial) {
  const row = output
    .split('\n')
    .map((line) => line.trim())
    .find((line) => line.startsWith(`${serial} `));
  if (!row) {
    return undefined;
  }
  return {
    state: row.split(/\s+/u)[1],
    product: namedValue(row, 'product'),
    model: namedValue(row, 'model'),
    device: namedValue(row, 'device'),
    rowSha256: hashText(row),
  };
}

function namedValue(row, name) {
  return row.match(new RegExp(`(?:^|\\s)${name}:([^\\s]+)`, 'u'))?.[1] ?? '';
}

function androidMismatches(observed) {
  const rows = [];
  for (const [field, expectedValue, observedValue] of [
    ['serial', expectedAndroidTarget.serial, observed.serial],
    ['product', expectedAndroidTarget.product, observed.product],
    ['model', expectedAndroidTarget.model, observed.model],
    ['device', expectedAndroidTarget.device, observed.device],
    ['androidRelease', expectedAndroidTarget.androidRelease, observed.androidRelease],
    ['abi', expectedAndroidTarget.abi, observed.abi],
  ]) {
    if (expectedValue !== observedValue) {
      rows.push({ field, expected: expectedValue, observed: observedValue });
    }
  }
  return rows;
}

function firstLine(value) {
  return value.split('\n')[0]?.trim() ?? '';
}

function lineCount(value) {
  return value.length === 0 ? 0 : value.split('\n').length;
}

function hashText(value) {
  return createHash('sha256').update(value).digest('hex');
}

function redactCommand(parts) {
  return parts.map((part) => part.replace(expectedAndroidTarget.serial, '<android-target>')).join(' ');
}

function normalizeProbeOutput(value) {
  return value.replace(/\r\n/gu, '\n').trim();
}

function validationCommandsLog(results) {
  const lines = [
    'network-platform-claims validation commands',
    '',
    ...results.map((result) => `${result.name}: ${result.command} -> exit ${result.status}; log=${result.log}`),
  ];
  return `${lines.join('\n')}\n`;
}

function runText(command, args) {
  const result = spawnSync(command, args, { encoding: 'utf8', shell: false });
  if (result.status !== 0) {
    throw new Error(`${command} ${args.join(' ')} failed with exit ${result.status}`);
  }
  return `${result.stdout ?? ''}${result.stderr ?? ''}`;
}

function readSourceStatusShort() {
  return runText('git', [
    'status',
    '--short',
    '--',
    '.',
    ':(exclude)output/network-plan-proof/52-platform-claims-proof',
    ':(exclude)test-results/network-platform-claims-proof',
  ]);
}
