import { createHash } from 'node:crypto';
import { existsSync } from 'node:fs';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';
import { pathToFileURL } from 'node:url';
import { spawn } from 'node:child_process';
import { tsImport } from 'tsx/esm/api';

const repoRoot = process.cwd();
const proofMode = 'child-android-storage-protocol-capability-proof';
const outputDir = join(repoRoot, 'test-results', proofMode);
const proofPath = join(outputDir, 'proof.json');
const commands = [];
const proofLabels = [];

await main();

async function main() {
  await mkdir(outputDir, { recursive: true });

  await runNpm([
    'exec',
    '--workspace',
    '@ocentra-parent/schema-domain',
    '--',
    'vitest',
    'run',
    'tests/proof/child-android-storage-protocol-proof.test.ts',
  ]);
  await runNpm(['run', 'release:package:android']);

  const sourceProof = await assertAndroidSourceProof();
  const packageArtifacts = await assertPackageArtifacts();
  const runtimeReadModel = await parseRuntimeReadModel(buildRuntimeReadModel(packageArtifacts));
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
      packageArtifacts,
      contract: 'packages/schema-domain/src/child-android-storage-protocol-proof.ts',
      contractTest: 'packages/schema-domain/tests/proof/child-android-storage-protocol-proof.test.ts',
      matrix: 'docs/expectations/pre-ai-proof-matrix.json',
      checkpoint: 'docs/checkpoints/child-android-storage-protocol-capability-proof-2026-05-31.md',
      output: relativePath(proofPath),
    },
    runtimeReadModel,
    matrixProof,
    scriptWiring,
    androidStorageProtocolProved: {
      appPrivateFiles: 'package-local-scaffold: Android app-private storage target is named in the package proof',
      protocolStorageSnapshot: 'ci-mechanical-proof: native bridge exposes storage command/event constants',
      hostedStorageDefault: 'not-default: hosted child activity storage is explicitly not default or implemented',
    },
    androidStorageStillUnimplemented: [
      'encrypted evidence journal persistence',
      'SQLite query store persistence',
      'parent-owned export workflow',
      'external LAN/WebSocket child-agent storage transport',
      'emulator or physical-device storage runtime behavior',
    ],
    nonClaims: [
      'raw child activity upload to Ocentra-hosted storage',
      'durable encrypted evidence journal on Android',
      'SQLite query store on Android',
      'parent-owned export runtime behavior',
      'Android child enforcement parity',
      'physical-device or emulator persistence behavior',
    ],
  };

  await writeFile(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
  console.log(`child-android-storage-protocol-capability-proof-ok:${proofLabels.join(',')}`);
  console.log(`evidence=${proofPath}`);
}

async function assertAndroidSourceProof() {
  const activity = await readRepoFile(
    'platforms/android/agent/app/src/main/java/ca/ocentra/parent/agent/MainActivity.java'
  );
  const service = await readRepoFile(
    'platforms/android/agent/app/src/main/java/ca/ocentra/parent/agent/OcentraParentAgentService.java'
  );
  const storageBridge = await readRepoFile(
    'platforms/android/agent/app/src/main/java/ca/ocentra/parent/agent/ChildAndroidStorageProtocolProof.java'
  );

  assertIncludes(activity, 'ChildAndroidStorageProtocolProof.createStorageProtocolBundle()', 'activity storage proof');
  assertIncludes(service, 'ChildAndroidStorageProtocolProof.createStorageProtocolBundle()', 'service storage proof');
  assertIncludes(storageBridge, 'child.android.storage.snapshot.get', 'storage snapshot command');
  assertIncludes(storageBridge, 'child.android.storage.protocol.proof.reported', 'storage proof event');
  assertIncludes(storageBridge, 'app-private-files', 'app-private files surface');
  assertIncludes(storageBridge, 'encrypted-evidence-journal', 'encrypted journal surface');
  assertIncludes(storageBridge, 'sqlite-query-store', 'SQLite query store surface');
  assertIncludes(storageBridge, 'parent-owned-export', 'parent-owned export surface');
  assertIncludes(storageBridge, 'ocentra-hosted-child-activity-storage', 'hosted storage non-default surface');
  assertIncludes(storageBridge, 'notDefaultStorageSurfaces', 'hosted storage non-default boundary');
  assertIncludes(storageBridge, 'manualRequiredStorageSurfaces', 'manual storage boundary');

  proofLabels.push('android-wrapper.storage-protocol-source-proof');
  return {
    activity: 'platforms/android/agent/app/src/main/java/ca/ocentra/parent/agent/MainActivity.java',
    service: 'platforms/android/agent/app/src/main/java/ca/ocentra/parent/agent/OcentraParentAgentService.java',
    storageBridge:
      'platforms/android/agent/app/src/main/java/ca/ocentra/parent/agent/ChildAndroidStorageProtocolProof.java',
  };
}

async function assertPackageArtifacts() {
  const workspacePackage = JSON.parse(await readRepoFile('package.json'));
  const apkName = `ocentra-parent-agent-android-debug-v${workspacePackage.version}.apk`;
  const versionedApkPath = join(repoRoot, 'target', 'release-packages', 'android', apkName);
  const latestApkPath = join(
    repoRoot,
    'target',
    'release-packages',
    'android',
    'ocentra-parent-agent-android-debug-latest.apk'
  );

  await assertApkWithChecksum(versionedApkPath);
  await assertApkWithChecksum(latestApkPath);
  proofLabels.push('android-package.storage-proof-debug-apk-and-checksum');

  return {
    versionName: workspacePackage.version,
    versionedApk: relativePath(versionedApkPath),
    latestApk: relativePath(latestApkPath),
    checksumState: 'ci-mechanical-proof',
  };
}

function buildRuntimeReadModel(packageArtifacts) {
  return {
    schemaVersion: proofMode,
    protocolBridgeProof: {
      packageId: 'ca.ocentra.parent.agent',
      nativeBridgeClass: 'ca.ocentra.parent.agent.ChildAndroidStorageProtocolProof',
      bridgeState: 'package-local-scaffold',
      externalTransportState: 'not-implemented',
      commands: [
        'child.android.storage.snapshot.get',
        'child.android.storage.capability.proof.get',
        'child.android.storage.protocol.proof.get',
      ],
      events: [
        'child.android.storage.snapshot.reported',
        'child.android.storage.capability.proof.reported',
        'child.android.storage.protocol.proof.reported',
      ],
      runtimeOwner: 'android-native-wrapper',
      proofRequirement: `storage protocol bridge compiles into ${packageArtifacts.latestApk}`,
      claimBoundary: 'package-local storage bridge is not external child-agent transport',
    },
    storageSurfaces: storageSurfaces(),
    claimBoundaries: {
      appPrivateFiles: 'package-local app-private files are named, but device persistence is unproven',
      encryptedEvidenceJournal: 'encrypted evidence journal remains unimplemented',
      sqliteQueryStore: 'SQLite query store remains unimplemented',
      parentOwnedExport: 'parent-owned export remains planned and explicit',
      ocentraHostedStorage: 'hosted child activity storage is not default and is not implemented',
      protocolTransport: 'storage protocol proof is package-local only',
      childAndroidStoragePersistence: 'emulator or physical-device persistence proof is still required',
    },
    updatedAt: new Date().toISOString(),
  };
}

async function parseRuntimeReadModel(readModel) {
  const module = await importTsModule('packages/schema-domain/src/child-android-storage-protocol-proof.ts');
  const parsed = module.ChildAndroidStorageProtocolReadModelSchema.parse(readModel);
  proofLabels.push('schema-domain.child-android-storage-protocol-proof-parse');
  return parsed;
}

async function assertProofMatrix() {
  const matrix = JSON.parse(await readRepoFile('docs/expectations/pre-ai-proof-matrix.json'));
  const claim = matrix.claims.find((candidate) => candidate.id === proofMode);
  const scenario = matrix.checkpointScenarios.find((candidate) => candidate.id === proofMode);
  if (!claim || !scenario) {
    throw new Error('Proof matrix is missing child-android-storage-protocol-capability-proof claim or scenario.');
  }
  assertArrayIncludes(matrix.requiredCompletedClaimIds, proofMode, 'required completed claim');
  assertArrayIncludes(scenario.ciCommands, `node scripts/test/${proofMode}.mjs`, 'scenario command');
  assertArrayIncludes(claim.ciProof.commands, `node scripts/test/${proofMode}.mjs`, 'claim command');
  proofLabels.push('proof-matrix.child-android-storage-protocol-proof');
  return {
    claimId: claim.id,
    platformCoverage: claim.platformCoverage,
    runtimeSurfaceCoverage: claim.runtimeSurfaceCoverage,
  };
}

async function assertScriptWiring() {
  const packageJson = JSON.parse(await readRepoFile('package.json'));
  const schemaDomainPackage = JSON.parse(await readRepoFile('packages/schema-domain/package.json'));
  const script = packageJson.scripts['test:child-android-storage-protocol-capability-proof'];
  if (script !== `node scripts/test/${proofMode}.mjs`) {
    throw new Error('Missing root test:child-android-storage-protocol-capability-proof script.');
  }
  if (!schemaDomainPackage.exports['./child-android-storage-protocol-proof']) {
    throw new Error('Missing schema-domain export for ./child-android-storage-protocol-proof.');
  }
  proofLabels.push('package-scripts.child-android-storage-protocol-proof');
  return {
    rootScript: 'test:child-android-storage-protocol-capability-proof',
    schemaDomainExport: './child-android-storage-protocol-proof',
    sourceContract: 'packages/schema-domain/src/child-android-storage-protocol-proof.ts',
  };
}

function storageSurfaces() {
  return [
    storageSurface(
      'app-private-files',
      'local-storage',
      'scaffold',
      'package-local-scaffold',
      'android-app-private-storage',
      'child-device-local',
      'package-local-app-private',
      'not-collected'
    ),
    storageSurface(
      'encrypted-evidence-journal',
      'local-storage',
      'not-implemented',
      'not-implemented',
      'child-agent-runtime',
      'child-device-local',
      'disabled',
      'temporary-local-only'
    ),
    storageSurface(
      'sqlite-query-store',
      'local-storage',
      'not-implemented',
      'not-implemented',
      'child-agent-runtime',
      'child-device-local',
      'disabled',
      'not-collected'
    ),
    storageSurface(
      'parent-owned-export',
      'local-storage',
      'planned',
      'planned',
      'parent-owned-storage',
      'parent-owned-local',
      'parent-owned-export-only',
      'not-collected'
    ),
    storageSurface(
      'ocentra-hosted-child-activity-storage',
      'local-storage',
      'not-implemented',
      'not-implemented',
      'ocentra-hosted-service',
      'ocentra-hosted',
      'not-default',
      'not-default'
    ),
    storageSurface(
      'protocol-storage-snapshot',
      'typed-protocol-bridge',
      'scaffold',
      'ci-mechanical-proof',
      'agent-protocol',
      'none',
      'disabled',
      'not-collected'
    ),
  ];
}

function storageSurface(
  surface,
  parentCapability,
  parentCapabilityStatus,
  proofState,
  runtimeOwner,
  custody,
  defaultStorageMode,
  rawChildActivityStorage
) {
  const proofRequirement = `${surface} storage state is ${proofState}`;
  return {
    surface,
    parentCapability,
    parentCapabilityStatus,
    proofState,
    runtimeOwner,
    custody,
    defaultStorageMode,
    rawChildActivityStorage,
    proofRequirement,
    claimBoundary: proofRequirement,
  };
}

async function assertApkWithChecksum(apkPath) {
  if (!existsSync(apkPath)) {
    throw new Error(`Missing Android APK artifact: ${apkPath}`);
  }
  const checksumPath = `${apkPath}.sha256`;
  if (!existsSync(checksumPath)) {
    throw new Error(`Missing Android APK checksum artifact: ${checksumPath}`);
  }
  const apkBytes = await readFile(apkPath);
  const expected = createHash('sha256').update(apkBytes).digest('hex').toUpperCase();
  const checksum = await readFile(checksumPath, 'utf8');
  assertIncludes(checksum, expected, `${relativePath(checksumPath)} checksum`);
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
