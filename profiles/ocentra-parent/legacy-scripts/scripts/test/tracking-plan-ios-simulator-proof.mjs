import { spawn, spawnSync } from 'node:child_process';
import { existsSync } from 'node:fs';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';

const repoRoot = process.cwd();
const proofMode = 'tracking-plan-ios-simulator-proof';
const resultDir = path.join(repoRoot, 'test-results', proofMode);
const proofPath = path.join(resultDir, 'proof.json');
const output11 = path.join(repoRoot, 'output', 'tracking-plan-proof', '11-ios-core-location-foreground-adapter');
const output12 = path.join(
  repoRoot,
  'output',
  'tracking-plan-proof',
  '12-ios-background-region-significant-change-adapter'
);
const output31 = path.join(
  repoRoot,
  'output',
  'tracking-plan-proof',
  '31-platform-extension-checklists-and-proof-routing'
);
const defaultAppPath = path.join(
  repoRoot,
  'target',
  'ios-derived-data',
  'Build',
  'Products',
  'Debug-iphonesimulator',
  'OcentraParentAgent.app'
);
const commands = [];
const options = parseArgs(process.argv.slice(2));

await main();

async function main() {
  await mkdir(resultDir, { recursive: true });
  await mkdir(output11, { recursive: true });
  await mkdir(output12, { recursive: true });
  await mkdir(output31, { recursive: true });

  const sourceProof = await assertSourceProof();
  const simulatorExecution = await runSimulatorProof();
  const proof = buildProof(sourceProof, simulatorExecution);
  await writeProofFiles(proof);

  console.log('tracking-plan-ios-simulator-proof-ok');
  console.log(`status=${simulatorExecution.currentStatus}`);
  console.log(`evidence=${relativePath(proofPath)}`);
}

async function assertSourceProof() {
  const project = await readRepoFile('platforms/ios/OcentraParentAgent.xcodeproj/project.pbxproj');
  const plist = await readRepoFile('platforms/ios/OcentraParentAgent/Info.plist');
  const statusView = await readRepoFile('platforms/ios/OcentraParentAgent/AgentStatusViewController.swift');
  const buildScript = await readRepoFile('scripts/release/ios/build-simulator-app.sh');
  const smokeScript = await readRepoFile('scripts/smoke/ios-simulator-smoke.sh');
  const workflow = await readRepoFile('.github/workflows/package-preview.yml');

  assertIncludes(project, 'OcentraParentAgent.app', 'iOS app product target');
  assertIncludes(project, 'PRODUCT_BUNDLE_IDENTIFIER = ca.ocentra.parent.agent', 'iOS bundle id');
  assertIncludes(plist, '<key>CFBundleIdentifier</key>', 'Info.plist bundle id');
  assertNotIncludes(plist, '<key>UIBackgroundModes</key>', 'no iOS background mode claim');
  assertIncludes(statusView, 'background-execution=manual-required', 'background manual-required label');
  assertIncludes(buildScript, 'xcodebuild', 'iOS simulator build command');
  assertIncludes(buildScript, 'CODE_SIGNING_ALLOWED=NO', 'unsigned simulator build');
  assertIncludes(smokeScript, 'xcrun simctl install', 'iOS simulator install smoke');
  assertIncludes(smokeScript, 'xcrun simctl launch', 'iOS simulator launch smoke');
  assertIncludes(workflow, 'iOS Simulator App Preview', 'iOS package-preview job');

  return {
    project: 'platforms/ios/OcentraParentAgent.xcodeproj/project.pbxproj',
    plist: 'platforms/ios/OcentraParentAgent/Info.plist',
    statusView: 'platforms/ios/OcentraParentAgent/AgentStatusViewController.swift',
    buildScript: 'scripts/release/ios/build-simulator-app.sh',
    smokeScript: 'scripts/smoke/ios-simulator-smoke.sh',
    packagePreviewWorkflow: '.github/workflows/package-preview.yml',
  };
}

async function runSimulatorProof() {
  const appPath = path.resolve(repoRoot, options.appPath ?? defaultAppPath);
  const host = {
    platform: process.platform,
    arch: process.arch,
    canRunXcodeSimulator: process.platform === 'darwin',
  };

  if (!host.canRunXcodeSimulator) {
    if (options.requireSimulator) {
      throw new Error('iOS simulator proof requires macOS with Xcode and simctl.');
    }
    return manualRequired(host, appPath, 'this worker host is not macOS');
  }

  if (!options.skipBuild) {
    await runCommand('bash', ['scripts/release/ios/build-simulator-app.sh'], '01-ios-simulator-build.log');
  }
  if (!existsSync(appPath)) {
    throw new Error(`iOS simulator app is missing: ${appPath}`);
  }

  await runCommand('bash', ['scripts/smoke/ios-simulator-smoke.sh', appPath], '02-ios-simulator-smoke.log');
  return {
    requiredProofTier: 'P3_LOCAL_DEV_MACHINE',
    currentProofTier: 'P3_LOCAL_DEV_MACHINE',
    currentStatus: 'proved',
    artifactPath: relativePath(proofPath),
    appPath: relativePath(appPath),
    host,
    manualRequiredReason: 'none for simulator build/install/launch on this macOS runner',
  };
}

function manualRequired(host, appPath, reason) {
  return {
    requiredProofTier: 'P3_LOCAL_DEV_MACHINE',
    currentProofTier: 'P2_HOSTED_CI',
    currentStatus: 'manual_required',
    artifactPath: relativePath(proofPath),
    appPath: relativePath(appPath),
    host,
    manualRequiredReason: reason,
  };
}

function buildProof(sourceProof, simulatorExecution) {
  return {
    schemaVersion: 1,
    checkedAt: new Date().toISOString(),
    commit: gitHead(),
    proofMode,
    commands,
    sourceProof,
    simulatorExecution,
    workpacks: {
      iosForeground: workpackProof('11-ios-core-location-foreground-adapter', simulatorExecution),
      iosBackground: {
        ...workpackProof('12-ios-background-region-significant-change-adapter', simulatorExecution),
        requiredProofTier: 'P4_PHYSICAL_DEVICE',
        currentStatus: 'manual_required',
        manualRequiredReason:
          'simulator package launch does not prove Always authorization, region monitoring, background delivery, significant-change, visits, low-power, or app-terminated behavior',
      },
      platformRouting: workpackProof('31-platform-extension-checklists-and-proof-routing', simulatorExecution),
    },
    productClaimReady: false,
    nonClaims: [
      'iOS Core Location authorization or foreground sample behavior',
      'iOS Always authorization, region monitoring, significant-change, visits, or background delivery',
      'notification authorization or delivery',
      'Apple signing, entitlement approval, TestFlight, App Store, or physical-device install',
      'Family Controls, DeviceActivity, Managed Settings, Network Extension, or child-agent parity',
    ],
  };
}

function workpackProof(workpackId, simulatorExecution) {
  return {
    workpackId,
    requiredProofTier: simulatorExecution.requiredProofTier,
    currentProofTier: simulatorExecution.currentProofTier,
    currentStatus: simulatorExecution.currentStatus,
    artifactPath: simulatorExecution.artifactPath,
    manualRequiredReason: simulatorExecution.manualRequiredReason,
  };
}

async function writeProofFiles(proof) {
  await writeJson(proofPath, proof);
  await writeJson(path.join(output11, '18-ios-simulator-proof.json'), proof.workpacks.iosForeground);
  await writeJson(path.join(output12, '18-ios-simulator-proof.json'), proof.workpacks.iosBackground);
  await writeJson(path.join(output31, '18-ios-simulator-proof.json'), proof.workpacks.platformRouting);
  const validationLog = `${commands.map((entry) => `${entry.command} exit=${entry.exitCode}`).join('\n')}\n`;
  await writeFile(path.join(output11, '18-ios-simulator-validation-commands.log'), validationLog);
  await writeFile(path.join(output12, '18-ios-simulator-validation-commands.log'), validationLog);
  await writeFile(path.join(output31, '18-ios-simulator-validation-commands.log'), validationLog);
}

async function runCommand(commandName, args, artifactName) {
  const result = await new Promise((resolve, reject) => {
    const child = spawn(commandName, args, { cwd: repoRoot, windowsHide: true });
    const output = [];
    child.stdout.on('data', (chunk) => output.push(String(chunk)));
    child.stderr.on('data', (chunk) => output.push(String(chunk)));
    child.once('error', reject);
    child.once('exit', (exitCode) => resolve({ exitCode, output: output.join('') }));
  });
  const artifact = path.join(resultDir, artifactName);
  await writeFile(artifact, result.output);
  commands.push({
    command: `${commandName} ${args.join(' ')}`,
    exitCode: result.exitCode,
    artifact: relativePath(artifact),
  });
  if (result.exitCode !== 0) {
    throw new Error(`${commandName} ${args.join(' ')} failed with exit ${result.exitCode}`);
  }
}

function parseArgs(args) {
  const parsed = { requireSimulator: false, skipBuild: false, appPath: null };
  for (let index = 0; index < args.length; index += 1) {
    const value = args[index];
    if (value === '--require-simulator') {
      parsed.requireSimulator = true;
    } else if (value === '--skip-build') {
      parsed.skipBuild = true;
    } else if (value === '--app-path') {
      parsed.appPath = args[index + 1];
      index += 1;
    }
  }
  return parsed;
}

async function readRepoFile(filePath) {
  return readFile(path.join(repoRoot, filePath), 'utf8');
}

function gitHead() {
  return process.env.GITHUB_SHA ?? runSync('git', ['rev-parse', 'HEAD']).trim();
}

function runSync(commandName, args) {
  const result = spawnSync(commandName, args, { cwd: repoRoot, encoding: 'utf8' });
  if ((result.status ?? 1) !== 0) {
    throw new Error(`${commandName} ${args.join(' ')} failed: ${result.stderr}`);
  }
  return result.stdout;
}

function assertIncludes(value, expected, label) {
  if (!value.includes(expected)) {
    throw new Error(`${label}: missing ${expected}`);
  }
}

function assertNotIncludes(value, expected, label) {
  if (value.includes(expected)) {
    throw new Error(`${label}: unexpectedly contains ${expected}`);
  }
}

async function writeJson(filePath, value) {
  await writeFile(filePath, `${JSON.stringify(value, null, 2)}\n`);
}

function relativePath(filePath) {
  return path.relative(repoRoot, filePath).replaceAll('\\', '/');
}
