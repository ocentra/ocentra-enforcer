import { spawn } from 'node:child_process';
import { existsSync } from 'node:fs';
import { mkdir, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';

const repoRoot = process.cwd();
const proofMode = 'app-game-android-child-runtime-local-delivery-queue-physical-proof';
const serial = process.env.ANDROID_SERIAL || '192.168.2.45:5555';
const packageId = 'ca.ocentra.parent.agent';
const activityName = `${packageId}/.MainActivity`;
const outputDir = join(repoRoot, 'test-results', proofMode);
const appGameProofDir = join(
  repoRoot,
  'output',
  'app-game-plan-proof',
  '217-app-game-android-child-runtime-local-delivery-queue-proof'
);
const proofPath = join(outputDir, 'proof.json');
const apkPath = join(
  repoRoot,
  'target',
  'release-packages',
  'android',
  'ocentra-parent-agent-android-debug-latest.apk'
);
const receiptDir = 'files/app-game-child-runtime-receipts';
const deliveryDir = 'files/app-game-child-runtime-deliveries';
const commandResults = [];

await main();

async function main() {
  await mkdir(outputDir, { recursive: true });
  await mkdir(appGameProofDir, { recursive: true });
  assertFileExists(apkPath, 'Android debug APK');

  await captureCommand('adb', ['connect', serial]);
  const devices = await captureCommand('adb', ['devices', '-l']);
  assertIncludes(devices.stdout, serial, 'ADB physical serial');
  assertIncludes(devices.stdout, 'model:SM_G965W', 'Samsung Galaxy S9 model');

  await captureCommand('adb', ['-s', serial, 'install', '-r', '-d', apkPath]);
  await captureCommand('adb', ['-s', serial, 'shell', 'input', 'keyevent', '224']);
  await captureCommand('adb', ['-s', serial, 'shell', 'wm', 'dismiss-keyguard']);
  await captureCommand('adb', ['-s', serial, 'shell', 'cmd', 'statusbar', 'collapse']);
  await captureCommand('adb', ['-s', serial, 'shell', 'am', 'force-stop', packageId]);
  await captureCommand('adb', [
    '-s',
    serial,
    'shell',
    'am',
    'start',
    '-n',
    activityName,
    '--ez',
    'ca.ocentra.parent.agent.RUN_APP_GAME_DELIVERY_INTAKE_PROOF',
    'true',
  ]);
  await delay(2000);
  const windowState = await captureCommand('adb', ['-s', serial, 'shell', 'dumpsys', 'window']);
  assertActivityVisible(windowState.stdout);

  const deliveryRecord = await readRunAsFileWithRetry(`${deliveryDir}/delivery-intake-proof-state.txt`, 5);
  const deliveryQueueRecord = await readRunAsFileWithRetry(`${deliveryDir}/delivery-queue-proof-state.txt`, 5);
  const deliveryDrainRecord = await readRunAsFileWithRetry(`${deliveryDir}/delivery-drain-proof-state.txt`, 5);
  const receiptChannelRecord = await readRunAsFileWithRetry(`${receiptDir}/receipt-channel-proof-state.txt`, 5);
  const receiptRecord = await readRunAsFileWithRetry(`${receiptDir}/receipt-proof-state.txt`, 3);
  const receiptAckRecord = await readRunAsFileWithRetry(`${receiptDir}/receipt-ack-proof-state.txt`, 3);

  assertIncludes(
    deliveryRecord,
    'deliveryId=android-child-runtime-package-local-delivery-intake-ref',
    'package-local delivery intake record'
  );
  assertIncludes(
    deliveryQueueRecord,
    'deliveryQueueId=android-child-runtime-package-local-delivery-queue-ref',
    'package-local delivery queue record'
  );
  assertIncludes(
    deliveryDrainRecord,
    'deliveryDrainId=android-child-runtime-package-local-delivery-drain-ref',
    'package-local delivery drain record'
  );
  assertIncludes(
    receiptChannelRecord,
    'receiptChannelId=android-child-runtime-package-local-receipt-channel-ref',
    'package-local receipt channel record'
  );
  assertIncludes(receiptRecord, 'receiptId=android-child-runtime-local-receipt-ref', 'local receipt record');
  assertIncludes(
    receiptAckRecord,
    'receiptAckId=android-child-runtime-local-receipt-ack-ref',
    'local receipt ack record'
  );

  const proof = {
    schemaVersion: 1,
    proofMode,
    checkedAt: 'deterministic-physical-android-proof',
    commit: await gitHead(),
    serial,
    device: {
      model: 'SM_G965W',
      product: 'star2qltecs',
      transport: 'wifi-adb',
    },
    commandResults,
    evidence: {
      apk: 'target/release-packages/android/ocentra-parent-agent-android-debug-latest.apk',
      proof: 'test-results/app-game-android-child-runtime-local-delivery-queue-physical-proof/proof.json',
      outputProof:
        'output/app-game-plan-proof/217-app-game-android-child-runtime-local-delivery-queue-proof/physical-proof.json',
    },
    observedStates: {
      activityLaunched: true,
      packageLocalDeliveryRecord: 'deliveryId=android-child-runtime-package-local-delivery-intake-ref',
      packageLocalDeliveryQueueRecord: 'deliveryQueueId=android-child-runtime-package-local-delivery-queue-ref',
      packageLocalDeliveryDrainRecord: 'deliveryDrainId=android-child-runtime-package-local-delivery-drain-ref',
      packageLocalReceiptChannelRecord: 'receiptChannelId=android-child-runtime-package-local-receipt-channel-ref',
      localReceiptRecord: 'receiptId=android-child-runtime-local-receipt-ref',
      localReceiptAckRecord: 'receiptAckId=android-child-runtime-local-receipt-ack-ref',
    },
    claimsProved: [
      'The Android debug package installs and launches on the physical Samsung Galaxy S9 target',
      'The activity-triggered in-package delivery path records package-local delivery intake, queue, and drain markers',
      'The same package-local delivery path records receipt channel, receipt, and receipt-ack markers',
    ],
    claimsNotProved: [
      'Service delivery or receipt ingestion',
      'Provider delivery execution',
      'Platform delivery channel execution outside the child package',
      'Adapter dispatch or platform enforcement',
      'Raw private source row storage',
    ],
  };

  await writeJson(proofPath, proof);
  await writeJson(join(appGameProofDir, 'physical-proof.json'), proof);
  await writeFile(join(appGameProofDir, '01-physical-android-delivery-queue-snapshot.md'), sourceSnapshot(proof));
  await writeFile(
    join(appGameProofDir, '11-physical-validation-commands.log'),
    `${commandResults.map((result) => result.command).join('\n')}\n`
  );

  console.log('app-game-android-child-runtime-local-delivery-queue-physical-proof-ok');
  console.log(`evidence=${relativePath(proofPath)}`);
}

async function readRunAsFileWithRetry(path, attempts) {
  let lastError;
  for (let attempt = 0; attempt < attempts; attempt += 1) {
    const result = await captureCommand('adb', ['-s', serial, 'shell', 'run-as', packageId, 'cat', path], {
      allowFailure: true,
    });
    if (result.code === 0 && result.stdout.trim().length > 0) {
      return result.stdout;
    }
    lastError = result.stderr || result.stdout || `empty ${path}`;
    await delay(1000);
  }
  throw new Error(`Could not read ${path}: ${lastError}`);
}

async function captureCommand(command, args, options = {}) {
  const commandLine = [command, ...args.map((arg) => (arg.includes(' ') ? `"${arg}"` : arg))].join(' ');
  const result = await new Promise((resolve, reject) => {
    const child = spawn(command, args, { cwd: repoRoot, stdio: ['ignore', 'pipe', 'pipe'], windowsHide: true });
    const stdout = [];
    const stderr = [];
    child.stdout.on('data', (chunk) => stdout.push(String(chunk)));
    child.stderr.on('data', (chunk) => stderr.push(String(chunk)));
    child.once('exit', (code) =>
      resolve({ command: commandLine, code, stdout: stdout.join(''), stderr: stderr.join('') })
    );
    child.once('error', reject);
  });
  commandResults.push({
    command: result.command,
    code: result.code,
    stdout: result.stdout.trim(),
    stderr: result.stderr.trim(),
  });
  if (result.code !== 0 && !options.allowFailure) {
    throw new Error(`${commandLine} exited with ${result.code}: ${result.stderr || result.stdout}`);
  }
  return result;
}

async function gitHead() {
  const result = await captureCommand('git', ['rev-parse', 'HEAD']);
  return result.stdout.trim();
}

function assertFileExists(path, label) {
  if (!existsSync(path)) {
    throw new Error(`${label} missing at ${relativePath(path)}`);
  }
}

function assertIncludes(value, expected, label) {
  if (!value.includes(expected)) {
    throw new Error(`${label} missing expected value: ${expected}`);
  }
}

function assertActivityVisible(value) {
  if (!value.includes(packageId) || !value.includes('MainActivity')) {
    throw new Error(`launched Android activity missing ${packageId} MainActivity evidence`);
  }
}

async function writeJson(path, value) {
  await writeFile(path, `${JSON.stringify(value, null, 2)}\n`);
}

function sourceSnapshot(proof) {
  return [
    '# WP217 Android child runtime local delivery queue physical proof snapshot',
    '',
    `- Device: \`${proof.device.model}\``,
    `- Serial: \`${proof.serial}\``,
    `- Activity launched: \`${proof.observedStates.activityLaunched}\``,
    `- Package-local delivery record: \`${proof.observedStates.packageLocalDeliveryRecord}\``,
    `- Package-local delivery queue record: \`${proof.observedStates.packageLocalDeliveryQueueRecord}\``,
    `- Package-local delivery drain record: \`${proof.observedStates.packageLocalDeliveryDrainRecord}\``,
    `- Package-local channel record: \`${proof.observedStates.packageLocalReceiptChannelRecord}\``,
    `- Local receipt record: \`${proof.observedStates.localReceiptRecord}\``,
    `- Local receipt ack record: \`${proof.observedStates.localReceiptAckRecord}\``,
    '',
  ].join('\n');
}

function relativePath(path) {
  return relative(repoRoot, path).replaceAll('\\', '/');
}

function delay(ms) {
  return new Promise((resolve) => {
    setTimeout(resolve, ms);
  });
}
