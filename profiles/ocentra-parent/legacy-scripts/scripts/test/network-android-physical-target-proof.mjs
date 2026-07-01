import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const proofRoot = join('output', 'network-plan-proof', 'android-physical-target-proof');
const testRoot = join('test-results', 'network-android-physical-target-proof');
const commandLogRoot = join(proofRoot, 'command-logs');
mkdirSync(proofRoot, { recursive: true });
mkdirSync(testRoot, { recursive: true });
mkdirSync(commandLogRoot, { recursive: true });

const expectedTarget = {
  targetRef: 'android-physical-target-sm-g965w-row40a',
  serial: process.env.NETWORK_ANDROID_TARGET_SERIAL ?? '192.168.2.45:5555',
  product: process.env.NETWORK_ANDROID_TARGET_PRODUCT ?? 'star2qltecs',
  model: process.env.NETWORK_ANDROID_TARGET_MODEL ?? 'SM_G965W',
  device: process.env.NETWORK_ANDROID_TARGET_DEVICE ?? 'star2qltecs',
  androidRelease: process.env.NETWORK_ANDROID_TARGET_RELEASE ?? '10',
  abi: process.env.NETWORK_ANDROID_TARGET_ABI ?? 'arm64-v8a',
};
const adbPath = findAdbPath();

const sourceRefs = [
  'crates/ocentra-network-evidence/src/android_physical_target/mod.rs',
  'crates/ocentra-network-evidence/src/android_physical_target/types.rs',
  'crates/ocentra-network-evidence/src/android_physical_target/validation.rs',
  'crates/ocentra-network-evidence/tests/unit/android_physical_target.rs',
  'crates/ocentra-network-evidence/tests/unit.rs',
  'crates/ocentra-network-evidence/src/lib.rs',
  'crates/ocentra-network-evidence/README.md',
  'docs/features/network-domain-control.md',
  'docs/plans/network-plan/implementation-checklist.md',
  'docs/plans/network-plan/current-network-snapshot.md',
  'scripts/test/network-android-physical-target-proof.mjs',
];

const boundary = {
  reportRef: 'network-android-physical-target-row40a',
  sourceRows: [
    '40 Android VpnService proof gate',
    '52 Platform claim manifest proof',
    '13b Live-capture bounded execution/status proof context',
  ],
  expectedTarget,
  proofBoundary:
    'Read-only ADB identity observation proves only that the named physical Android target is reachable and matches product/model/device/release/ABI expectations.',
  unsupportedClaimsRejected: [
    'production Android support',
    'emulator-only product support',
    'live VpnService tunnel execution',
    'packet capture',
    'packet blocking',
    'app package correlation',
    'adapter authority',
    'policy authority',
    'host filtering',
    'enforcement command',
    'exact URL',
    'decrypted payload',
    'page content',
  ],
};
writeJson(join(proofRoot, 'android-physical-target-boundary.json'), boundary);

const observation = collectAndroidObservation();
writeJson(join(proofRoot, 'android-physical-target-observation.json'), observation);

const commands = [
  {
    name: 'network-android-physical-target-tests',
    command: 'cargo',
    args: ['test', '-p', 'ocentra-network-evidence', '--test', 'unit'],
  },
  {
    name: 'network-evidence-clippy',
    command: 'cargo',
    args: ['clippy', '-p', 'ocentra-network-evidence', '--all-targets', '--', '-D', 'warnings'],
  },
];
const commandResults = commands.map(runCommand);
writeFileSync(join(proofRoot, 'validation-commands.log'), validationCommandsLog(commandResults));

if (observation.state === 'mismatch') {
  throw new Error(
    `Android physical target identity proof failed: ${observation.state}; details=${join(
      proofRoot,
      'android-physical-target-observation.json'
    )}`
  );
}

const proof = {
  schemaVersion: 1,
  proof: 'network-android-physical-target',
  checkedAt: new Date().toISOString(),
  branch: runText('git', ['branch', '--show-current']).trim(),
  commit: runText('git', ['rev-parse', 'HEAD']).trim(),
  statusShort: runText('git', ['status', '--short']),
  sourceRefs,
  sourceFingerprint: `source-tree:${sourceFingerprint(sourceRefs)}`,
  proofRoot,
  testRoot,
  commands: commandResults,
  expectedTarget,
  physicalDeviceIdentityProved: observation.physicalDeviceIdentityProved,
  artifacts: {
    boundary: join(proofRoot, 'android-physical-target-boundary.json'),
    observation: join(proofRoot, 'android-physical-target-observation.json'),
    commandLog: join(proofRoot, 'validation-commands.log'),
    proofSummary: join(proofRoot, 'proof-summary.json'),
    testProof: join(testRoot, 'proof.json'),
  },
  provenRows: ['40a Android physical target identity proof'],
  provenBoundaries: [
    'physical Android target identity was observed through explicit adb connect, adb devices -l, and getprop probes',
    'emulator-only evidence cannot satisfy the physical target proof',
    'the proof does not execute VpnService, packet capture, packet blocking, adapter apply, or enforcement commands',
    'network-only content fields remain unavailable',
  ],
  notClaimed: [
    'production Android support',
    'live VpnService tunnel execution',
    'packet capture',
    'packet blocking',
    'app package traffic correlation',
    'Device Owner or authority-enrolled state',
    'policy execution',
    'adapter authority',
    'host filtering',
    'enforcement command publication',
    'exact URL, decrypted payload, page content, private message, or search query visibility',
  ],
};
writeJson(join(proofRoot, 'proof-summary.json'), proof);
writeJson(join(testRoot, 'proof.json'), proof);
console.log('network-android-physical-target-proof-ok:adb-identity,cargo-tests,clippy');
console.log(`proof=${join(proofRoot, 'proof-summary.json')}`);

function collectAndroidObservation() {
  const connect = runProbe('adb-connect', adbPath, ['connect', expectedTarget.serial]);
  const devices = runProbe('adb-devices', adbPath, ['devices', '-l']);
  const model = runTargetGetprop('ro.product.model');
  const product = runTargetGetprop('ro.product.name');
  const device = runTargetGetprop('ro.product.device');
  const release = runTargetGetprop('ro.build.version.release');
  const abi = runTargetGetprop('ro.product.cpu.abi');
  const deviceRow = parseTargetRow(devices.stdout, expectedTarget.serial);
  const observed = {
    serial: expectedTarget.serial,
    product: deviceRow?.product ?? product.value,
    model: deviceRow?.model ?? normalizeModelForDevices(model.value),
    device: deviceRow?.device ?? device.value,
    androidRelease: release.value,
    abi: abi.value,
    getpropModel: model.value,
  };
  const adbAvailable = connect.summary.status === 0 || devices.summary.status === 0;
  const targetConnected = deviceRow?.state === 'device';
  const mismatches = targetConnected ? identityMismatches(observed) : [];
  const state = !adbAvailable
    ? 'unavailable'
    : !targetConnected
      ? 'manual-required-target-not-connected'
      : mismatches.length > 0
        ? 'mismatch'
        : 'physical-device-observed';

  return {
    checkedAt: new Date().toISOString(),
    state,
    expectedTarget,
    observed,
    adbAvailable,
    targetConnected,
    physicalDeviceIdentityProved: state === 'physical-device-observed',
    mismatches,
    probes: [
      connect.summary,
      devices.summary,
      model.summary,
      product.summary,
      device.summary,
      release.summary,
      abi.summary,
    ],
    evidenceRefs: ['adb-connect-proof-ref-row40a', 'adb-devices-proof-ref-row40a', 'adb-getprop-proof-ref-row40a'],
    unsupportedClaims: {
      emulatorOnlyProductSupportClaimed: false,
      liveVpnServiceExecutionClaimed: false,
      packetCaptureClaimed: false,
      packetBlockClaimed: false,
      appPackageCorrelationClaimed: false,
      adapterAuthorityClaimed: false,
      policyAuthorityClaimed: false,
      hostFilteringClaimed: false,
      enforcementCommandClaimed: false,
      productionAndroidSupportClaimed: false,
      exactUrlClaimed: false,
      decryptedPayloadClaimed: false,
      pageContentClaimed: false,
    },
  };
}

function runTargetGetprop(propertyName) {
  const probe = runProbe(`adb-getprop-${propertyName}`, adbPath, [
    '-s',
    expectedTarget.serial,
    'shell',
    'getprop',
    propertyName,
  ]);
  return {
    value: firstLine(probe.stdout),
    summary: probe.summary,
  };
}

function identityMismatches(observed) {
  const rows = [];
  for (const [field, expectedValue, observedValue] of [
    ['serial', expectedTarget.serial, observed.serial],
    ['product', expectedTarget.product, observed.product],
    ['model', expectedTarget.model, observed.model],
    ['device', expectedTarget.device, observed.device],
    ['androidRelease', expectedTarget.androidRelease, observed.androidRelease],
    ['abi', expectedTarget.abi, observed.abi],
  ]) {
    if (expectedValue !== observedValue) {
      rows.push({ field, expected: expectedValue, observed: observedValue });
    }
  }
  return rows;
}

function parseTargetRow(output, serial) {
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

function findAdbPath() {
  const sdkRoot =
    process.env.ANDROID_HOME ?? process.env.ANDROID_SDK_ROOT ?? join(process.env.LOCALAPPDATA ?? '', 'Android', 'Sdk');
  const candidate = join(sdkRoot, 'platform-tools', process.platform === 'win32' ? 'adb.exe' : 'adb');
  return existsSync(candidate) ? candidate : 'adb';
}

function runCommand(entry) {
  const result = spawnSync(entry.command, entry.args, { encoding: 'utf8', shell: false });
  const log = join(commandLogRoot, `${safeName(entry.name)}.log`);
  writeFileSync(log, normalizeCommandOutput(`${result.stdout ?? ''}${result.stderr ?? ''}`));
  if (result.status !== 0) {
    throw new Error(`${entry.name} failed with exit ${result.status}; log=${log}`);
  }
  return {
    name: entry.name,
    command: [entry.command, ...entry.args].join(' '),
    status: result.status,
    log,
  };
}

function runProbe(name, command, args) {
  const result = spawnSync(command, args, { encoding: 'utf8', shell: false });
  const stdout = normalizeProbeOutput(result.stdout ?? '');
  const stderr = normalizeProbeOutput(result.stderr ?? '');
  return {
    stdout,
    stderr,
    summary: {
      name,
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

function runText(command, args) {
  const result = spawnSync(command, args, { encoding: 'utf8', shell: false });
  if (result.status !== 0) {
    throw new Error(`${command} ${args.join(' ')} failed with exit ${result.status}`);
  }
  return `${result.stdout ?? ''}${result.stderr ?? ''}`;
}

function sourceFingerprint(paths) {
  const hash = createHash('sha256');
  for (const path of paths) {
    hash.update(path);
    hash.update('\0');
    hash.update(readFileSync(path, 'utf8').replace(/\r\n/gu, '\n'));
    hash.update('\0');
  }
  return hash.digest('hex');
}

function writeJson(path, value) {
  writeFileSync(path, `${JSON.stringify(value, null, 2)}\n`);
}

function validationCommandsLog(results) {
  return `${results
    .map((result) => `${result.name}: ${result.command} -> exit ${result.status}; log=${result.log}`)
    .join('\n')}\n`;
}

function normalizeModelForDevices(value) {
  return value.replace(/-/gu, '_');
}

function firstLine(value) {
  return value.split('\n')[0]?.trim() ?? '';
}

function safeName(value) {
  return value.replace(/[^a-zA-Z0-9_.-]/gu, '-');
}

function normalizeProbeOutput(value) {
  return value.replace(/\r\n/gu, '\n').trim();
}

function lineCount(value) {
  return value.length === 0 ? 0 : value.split('\n').length;
}

function hashText(value) {
  return createHash('sha256').update(value).digest('hex');
}

function redactCommand(parts) {
  return parts.map((part) => part.replace(expectedTarget.serial, '<android-target>')).join(' ');
}

function normalizeCommandOutput(value) {
  return `${value
    .replace(/\r\n/gu, '\n')
    .replace(/\\/gu, '/')
    .replace(/target\/debug\/deps\/[^\s)]+/gu, 'target/debug/deps/<test-binary>')
    .replace(/\b\d+\.\d+s\b/gu, '<duration>s')
    .replace(/\b\d+\.\d{2}ms\b/gu, '<duration>ms')
    .replace(/target\(s\) in [^\n]+/gu, 'target(s) in <duration>')
    .replace(/finished in [^\n]+/giu, 'finished in <duration>')
    .trim()}\n`;
}
