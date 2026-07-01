import { spawn } from 'node:child_process';
import { existsSync } from 'node:fs';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';

const repoRoot = process.cwd();
const proofMode = 'app-game-android-child-runtime-local-receipt-physical-proof';
const serial = process.env.ANDROID_SERIAL || '192.168.2.45:5555';
const packageId = 'ca.ocentra.parent.agent';
const activityName = `${packageId}/.MainActivity`;
const outputDir = join(repoRoot, 'test-results', proofMode);
const appGameProofDir = join(
  repoRoot,
  'output',
  'app-game-plan-proof',
  '213-app-game-android-child-runtime-local-receipt-physical-proof'
);
const proofPath = join(outputDir, 'proof.json');
const uiDumpPath = join(outputDir, 'ui.xml');
const deviceUiDumpPath = '/sdcard/ocentra-app-game-wp213-ui.xml';
const apkPath = join(
  repoRoot,
  'target',
  'release-packages',
  'android',
  'ocentra-parent-agent-android-debug-latest.apk'
);
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
  await captureCommand('adb', ['-s', serial, 'shell', 'am', 'start', '-n', activityName]);
  await delay(1500);
  await captureCommand('adb', ['-s', serial, 'shell', 'cmd', 'statusbar', 'collapse']);
  const windowState = await captureCommand('adb', ['-s', serial, 'shell', 'dumpsys', 'window']);
  assertIncludes(windowState.stdout, activityName, 'launched Android activity');
  const uiDump = await captureCommand('adb', ['-s', serial, 'shell', 'uiautomator', 'dump', deviceUiDumpPath], {
    allowFailure: true,
  });
  let uiDumpObserved = false;
  if (uiDump.code === 0) {
    await captureCommand('adb', ['-s', serial, 'pull', deviceUiDumpPath, uiDumpPath]);
    const uiXml = await readFile(uiDumpPath, 'utf8');
    uiDumpObserved =
      uiXml.includes('local-receipt-append-recorded') && uiXml.includes('local-receipt-readback-observed');
  }
  const receiptRecord = await captureCommand('adb', [
    '-s',
    serial,
    'shell',
    'run-as',
    packageId,
    'cat',
    'files/app-game-child-runtime-receipts/receipt-proof-state.txt',
  ]);

  assertIncludes(receiptRecord.stdout, 'receiptId=android-child-runtime-local-receipt-ref', 'local receipt record');

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
      uiDump: uiDumpObserved
        ? 'test-results/app-game-android-child-runtime-local-receipt-physical-proof/ui.xml'
        : 'uiautomator-dump-unavailable',
      proof: 'test-results/app-game-android-child-runtime-local-receipt-physical-proof/proof.json',
      outputProof:
        'output/app-game-plan-proof/213-app-game-android-child-runtime-local-receipt-physical-proof/proof.json',
    },
    observedStates: {
      activityLaunched: true,
      uiDumpObserved,
      localReceiptRecord: 'receiptId=android-child-runtime-local-receipt-ref',
    },
    claimsProved: [
      'The Android debug package installs and launches on the physical Samsung Galaxy S9 target',
      'The package-local internal receipt marker is readable through run-as for the debug package proof target',
    ],
    claimsNotProved: [
      'Service receipt ingestion',
      'Provider delivery execution',
      'Platform delivery channel execution',
      'Adapter dispatch or platform enforcement',
      'Raw private source row storage',
    ],
  };

  await writeJson(proofPath, proof);
  await writeJson(join(appGameProofDir, 'proof.json'), proof);
  await writeFile(join(appGameProofDir, '00-physical-android-ui-snapshot.md'), sourceSnapshot(proof));
  await writeFile(
    join(appGameProofDir, '10-validation-commands.log'),
    `${commandResults.map((result) => result.command).join('\n')}\n`
  );

  console.log('app-game-android-child-runtime-local-receipt-physical-proof-ok');
  console.log(`evidence=${relativePath(proofPath)}`);
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
  if (result.code !== 0 && options.allowFailure !== true) {
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

async function writeJson(path, value) {
  await writeFile(path, `${JSON.stringify(value, null, 2)}\n`);
}

function sourceSnapshot(proof) {
  return [
    '# WP213 Android child runtime local receipt physical proof snapshot',
    '',
    `- Device: \`${proof.device.model}\``,
    `- Serial: \`${proof.serial}\``,
    `- Activity launched: \`${proof.observedStates.activityLaunched}\``,
    `- UI dump observed receipt states: \`${proof.observedStates.uiDumpObserved}\``,
    `- Local receipt record: \`${proof.observedStates.localReceiptRecord}\``,
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
