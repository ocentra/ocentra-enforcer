import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';
import { pathToFileURL } from 'node:url';
import { spawn } from 'node:child_process';
import { tsImport } from 'tsx/esm/api';

const repoRoot = process.cwd();
const proofMode = 'child-ios-entitlement-capability-proof';
const outputDir = join(repoRoot, 'test-results', proofMode);
const proofPath = join(outputDir, 'proof.json');
const commands = [];
const proofLabels = [];
let cachedCanonicalReadModelModule;

await main();

async function main() {
  await mkdir(outputDir, { recursive: true });
  cachedCanonicalReadModelModule = await importTsModule(
    'packages/schema-domain/src/child-ios-entitlement-capability-proof.ts'
  );

  await runNpm([
    'exec',
    '--workspace',
    '@ocentra-parent/schema-domain',
    '--',
    'vitest',
    'run',
    'tests/proof/child-ios-entitlement-capability-proof.test.ts',
  ]);

  const sourceProof = await assertIosSourceProof();
  const runtimeReadModel = await parseRuntimeReadModel(buildRuntimeReadModel());
  const matrixProof = await assertProofMatrix();
  const scriptWiring = await assertScriptWiring();

  const proof = {
    schemaVersion: 1,
    checkedAt: new Date().toISOString(),
    commit: await gitHead(),
    proofMode,
    commands,
    proofLabels,
    evidence: {
      sourceProof,
      contract: 'crates/schema/src/child_ios_entitlement_capability_proof.rs',
      contractGenerator: 'crates/schema/src/child_ios_entitlement_capability_proof_ts.rs',
      generatedContract:
        'packages/schema-domain/src/generated/child-ios-entitlement-capability-proof-contracts.ts',
      adapter: 'packages/schema-domain/src/child-ios-entitlement-capability-proof.ts',
      contractTest: 'crates/schema/tests/contract/child_ios_entitlement_capability_proof.rs',
      adapterTest: 'packages/schema-domain/tests/proof/child-ios-entitlement-capability-proof.test.ts',
      matrix: 'docs/expectations/pre-ai-proof-matrix.json',
      checkpoint: 'docs/checkpoints/child-ios-entitlement-capability-proof-2026-05-31.md',
      output: relativePath(proofPath),
    },
    runtimeReadModel,
    matrixProof,
    scriptWiring,
    childIosEntitlementPackageProved: {
      simulatorTarget: 'ci-mechanical-proof: iOS app target exists in the Xcode project',
      bundleIdentifier: 'ci-mechanical-proof: bundle identifier remains ca.ocentra.parent.agent',
      infoPlist: 'ci-mechanical-proof: basic iOS app plist exists without entitlement or background claims',
      statusSurface:
        'simulator-scaffold: AgentStatusViewController exposes capability-only, launch, recovery, provisioning, supervision, and manual-required status labels',
      simulatorBuildScript: 'ci-mechanical-proof: simulator package script exists with code signing disabled',
    },
    childIosEntitlementStillManual: [
      'simulator launch availability from an Apple host',
      'physical-device launch availability and foreground behavior',
      'Family Controls entitlement approval and behavior',
      'DeviceActivity schedule and event behavior',
      'Screen Time API authorization and behavior',
      'Network Extension entitlement and filtering behavior',
      'notification authorization and delivery',
      'background execution mode and behavior',
      'Apple provisioning profile and install entitlement review',
      'supervised-device enrollment and supervision-only behavior',
      'Apple signing, provisioning, and entitlement files',
      'TestFlight install and App Store distribution',
      'physical-device install and runtime evidence',
    ],
    nonClaims: [
      'Family Controls, DeviceActivity, Screen Time, or Network Extension implementation',
      'notification permission grant or delivery',
      'background execution behavior',
      'simulator or physical-device launch proof from this Windows lane',
      'automatic relaunch or recovery behavior on iOS',
      'hidden daemon or persistent background service behavior',
      'Apple signing, provisioning, entitlement approval, TestFlight, or App Store proof',
      'simulator launch, physical-device install, or device behavior',
      'child-agent parity or external LAN/WebSocket iOS transport',
    ],
  };

  await writeFile(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
  console.log(`child-ios-entitlement-capability-proof-ok:${proofLabels.join(',')}`);
  console.log(`evidence=${proofPath}`);
}

async function assertIosSourceProof() {
  const project = await readRepoFile('platforms/ios/OcentraParentAgent.xcodeproj/project.pbxproj');
  const plist = await readRepoFile('platforms/ios/OcentraParentAgent/Info.plist');
  const statusView = await readRepoFile('platforms/ios/OcentraParentAgent/AgentStatusViewController.swift');
  const buildScript = await readRepoFile('scripts/release/ios/build-simulator-app.sh');

  assertIncludes(project, 'OcentraParentAgent.app', 'iOS app product target');
  assertIncludes(project, 'PRODUCT_BUNDLE_IDENTIFIER = ca.ocentra.parent.agent', 'iOS bundle identifier');
  assertIncludes(plist, '<key>CFBundleIdentifier</key>', 'Info.plist bundle identifier key');
  assertIncludes(plist, '<key>LSRequiresIPhoneOS</key>', 'Info.plist iPhone requirement');
  assertNotIncludes(plist, '<key>UIBackgroundModes</key>', 'background modes entitlement claim');
  assertNotIncludes(plist, 'FamilyControls', 'Family Controls framework claim');
  assertNotIncludes(plist, 'DeviceActivity', 'DeviceActivity framework claim');
  assertNotIncludes(plist, 'NetworkExtension', 'Network Extension framework claim');
  assertIncludes(statusView, 'child-ios-entitlement-capability-proof', 'iOS status schema label');
  assertIncludes(statusView, 'service-mode=capability-only', 'capability-only status label');
  assertIncludes(statusView, 'launch-availability=manual-required', 'launch availability manual label');
  assertIncludes(statusView, 'recovery=not-implemented', 'recovery non-implementation label');
  assertIncludes(statusView, 'family-controls=manual-required', 'Family Controls manual label');
  assertIncludes(statusView, 'device-activity=manual-required', 'DeviceActivity manual label');
  assertIncludes(statusView, 'screen-time=manual-required', 'Screen Time manual label');
  assertIncludes(statusView, 'network-extension=manual-required', 'Network Extension manual label');
  assertIncludes(statusView, 'notifications=manual-required', 'notifications manual label');
  assertIncludes(statusView, 'background-execution=manual-required', 'background execution manual label');
  assertIncludes(statusView, 'provisioning=manual-required', 'provisioning manual label');
  assertIncludes(statusView, 'supervision=manual-required', 'supervision manual label');
  assertIncludes(statusView, 'signing=manual-required', 'signing manual label');
  assertIncludes(statusView, 'testflight=manual-required', 'TestFlight manual label');
  assertIncludes(statusView, 'device-proof=manual-required', 'device proof manual label');
  assertIncludes(statusView, 'daemon=not-claimed', 'daemon non-claim label');
  assertIncludes(statusView, 'child-agent-parity=not-claimed', 'child-agent parity non-claim label');
  assertIncludes(buildScript, 'xcodebuild', 'iOS simulator build command');
  assertIncludes(buildScript, 'iphonesimulator', 'iOS simulator SDK');
  assertIncludes(buildScript, 'CODE_SIGNING_ALLOWED=NO', 'unsigned simulator package proof');
  proofLabels.push('ios-scaffold.entitlement-source-proof');

  return {
    project: 'platforms/ios/OcentraParentAgent.xcodeproj/project.pbxproj',
    infoPlist: 'platforms/ios/OcentraParentAgent/Info.plist',
    statusView: 'platforms/ios/OcentraParentAgent/AgentStatusViewController.swift',
    simulatorBuildScript: 'scripts/release/ios/build-simulator-app.sh',
  };
}

function buildRuntimeReadModel() {
  return structuredClone(canonicalReadModelModule().ChildIosEntitlementCapabilityReadModelProof);
}

async function parseRuntimeReadModel(readModel) {
  const module = canonicalReadModelModule();
  const parsed = module.ChildIosEntitlementCapabilityReadModelSchema.parse(readModel);
  proofLabels.push('schema-domain.child-ios-entitlement-capability-proof-thin-adapter-parse');
  return parsed;
}

async function assertProofMatrix() {
  const matrix = JSON.parse(await readRepoFile('docs/expectations/pre-ai-proof-matrix.json'));
  const claim = matrix.claims.find((candidate) => candidate.id === proofMode);
  const scenario = matrix.checkpointScenarios.find((candidate) => candidate.id === proofMode);
  if (!claim || !scenario) {
    throw new Error('Proof matrix is missing child-ios-entitlement-capability-proof claim or scenario.');
  }
  assertArrayIncludes(matrix.requiredCompletedClaimIds, proofMode, 'required completed claim');
  assertArrayIncludes(scenario.ciCommands, `node scripts/test/${proofMode}.mjs`, 'scenario command');
  assertArrayIncludes(claim.ciProof.commands, `node scripts/test/${proofMode}.mjs`, 'claim command');
  proofLabels.push('proof-matrix.child-ios-entitlement-capability-proof');
  return {
    claimId: claim.id,
    platformCoverage: claim.platformCoverage,
    runtimeSurfaceCoverage: claim.runtimeSurfaceCoverage,
  };
}

async function assertScriptWiring() {
  const packageJson = JSON.parse(await readRepoFile('package.json'));
  const schemaDomainPackage = JSON.parse(await readRepoFile('packages/schema-domain/package.json'));
  const script = packageJson.scripts['test:child-ios-entitlement-capability-proof'];
  if (script !== `node scripts/test/${proofMode}.mjs`) {
    throw new Error('Missing root test:child-ios-entitlement-capability-proof script.');
  }
  if (!schemaDomainPackage.exports['./child-ios-entitlement-capability-proof']) {
    throw new Error('Missing schema-domain export for ./child-ios-entitlement-capability-proof.');
  }
  proofLabels.push('package-scripts.child-ios-entitlement-capability-proof');
  return {
    rootScript: 'test:child-ios-entitlement-capability-proof',
    schemaDomainExport: './child-ios-entitlement-capability-proof',
    rustContract: 'crates/schema/src/child_ios_entitlement_capability_proof.rs',
    generatedContract:
      'packages/schema-domain/src/generated/child-ios-entitlement-capability-proof-contracts.ts',
    adapter: 'packages/schema-domain/src/child-ios-entitlement-capability-proof.ts',
  };
}

async function readRepoFile(path) {
  return readFile(join(repoRoot, path), 'utf8');
}

async function runNpm(args) {
  await runCommand(...npmCommand([...args]));
}

async function runCommand(commandName, args) {
  commands.push([commandName, ...args].join(' '));
  await new Promise((resolve, reject) => {
    const child = spawn(commandName, args, { cwd: repoRoot, stdio: 'inherit', windowsHide: true });
    child.once('exit', (code) =>
      code === 0 ? resolve() : reject(new Error(`${commandName} ${args.join(' ')} exited with ${code}`))
    );
    child.once('error', reject);
  });
}

async function importTsModule(relativePath) {
  return tsImport(pathToFileURL(join(repoRoot, relativePath)).href, import.meta.url);
}

function canonicalReadModelModule() {
  if (!cachedCanonicalReadModelModule) {
    throw new Error('canonical child iOS entitlement read model module has not been loaded');
  }
  return cachedCanonicalReadModelModule;
}

async function gitHead() {
  const chunks = [];
  await new Promise((resolve, reject) => {
    const child = spawn('git', ['rev-parse', 'HEAD'], { cwd: repoRoot, stdio: ['ignore', 'pipe', 'pipe'] });
    child.stdout.on('data', (chunk) => chunks.push(String(chunk)));
    child.once('exit', (code) => (code === 0 ? resolve() : reject(new Error('git rev-parse HEAD failed'))));
    child.once('error', reject);
  });
  return chunks.join('').trim();
}

function relativePath(path) {
  return relative(repoRoot, path).replaceAll('\\', '/');
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

function assertArrayIncludes(values, expected, label) {
  if (!Array.isArray(values) || !values.includes(expected)) {
    throw new Error(`${label}: missing ${expected}`);
  }
}

function npmCommand(args) {
  const command = process.platform === 'win32' ? 'cmd' : 'npm';
  const commandArgs = process.platform === 'win32' ? ['/c', 'npm', ...args] : args;
  return [command, commandArgs];
}
