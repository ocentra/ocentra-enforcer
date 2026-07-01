import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { spawn } from 'node:child_process';
import { join, relative } from 'node:path';
import { pathToFileURL } from 'node:url';
import { tsImport } from 'tsx/esm/api';

const repoRoot = process.cwd();
const proofMode = 'child-signing-store-device-owner-matrix';
const outputDir = join(repoRoot, 'test-results', proofMode);
const proofPath = join(outputDir, 'proof.json');
const commands = [];
const proofLabels = [];

await main();

async function main() {
  await mkdir(outputDir, { recursive: true });

  await runCommand('cargo', [
    'test',
    '-p',
    'ocentra-schema',
    '--test',
    'contract',
    'child_signing_store_device_owner_matrix',
  ]);
  await runNpm([
    'exec',
    '--workspace',
    '@ocentra-parent/schema-domain',
    '--',
    'vitest',
    'run',
    'tests/proof/child-signing-store-device-owner-matrix.test.ts',
  ]);

  const sourceProof = await assertSourceProof();
  const runtimeReadModel = await readRuntimeReadModel();
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
      rustContract: 'crates/schema/src/child_signing_store_device_owner_matrix.rs',
      rustContractTest: 'crates/schema/tests/contract/child_signing_store_device_owner_matrix.rs',
      generatedTypes:
        'packages/schema-domain/src/generated/child-signing-store-device-owner-matrix-contracts.ts',
      adapter: 'packages/schema-domain/src/child-signing-store-device-owner-matrix.ts',
      contractTest: 'packages/schema-domain/tests/proof/child-signing-store-device-owner-matrix.test.ts',
      output: relativePath(proofPath),
    },
    runtimeReadModel,
    scriptWiring,
    matrixStatesProved: {
      windows:
        'MSI/service packaging plus signed updater-manifest wiring are present; Authenticode signing and store publication are not claimed',
      macos:
        'launchd pkg packaging is present; codesign, notarization, and store publication are not claimed',
      linux:
        'systemd .deb packaging plus baseline metadata are present; package signing and repository publication are not claimed',
      android:
        'debug APK package output is present; Play Store, device-owner, and managed-profile remain explicit planned or manual-required states',
      ios:
        'unsigned simulator ZIP scaffold is present; Apple signing, provisioning, supervision, TestFlight, and App Store remain explicit device-proof-required or planned states',
    },
    manualRequiredStates: [
      'Windows Authenticode signing and any store publication',
      'macOS codesign, notarization, and any store publication',
      'Linux package signing or repository publication',
      'Android device-owner or managed-profile enrollment artifacts',
      'Android Play Store signing and release-track publication',
      'iOS Apple signing, provisioning, supervision, TestFlight, and App Store artifacts',
    ],
    nonClaims: [
      'generic matrix rows replace platform-specific proofs',
      'desktop rows imply mobile device-owner, managed-profile, or supervision states',
      'Windows updater-manifest signing implies signed child MSI artifacts',
      'Android debug APK implies Play Store, device-owner, or managed-profile parity',
      'iOS simulator packaging implies provisioning, hidden daemon authority, or parent-client parity',
    ],
  };

  await writeFile(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
  console.log(`child-signing-store-device-owner-matrix-ok:${proofLabels.join(',')}`);
  console.log(`evidence=${proofPath}`);
}

async function assertSourceProof() {
  const windowsScript = await readRepoFile('scripts/release/windows/build-agent-package.ps1');
  const macosScript = await readRepoFile('scripts/release/macos/build-agent-package.sh');
  const linuxScript = await readRepoFile('scripts/release/linux/build-agent-package.sh');
  const linuxProof = await readRepoFile('scripts/test/child-linux-service-package-proof.mjs');
  const androidScript = await readRepoFile('scripts/release/android/build-agent-package.mjs');
  const androidProof = await readRepoFile('scripts/test/child-android-device-proof-artifact-gate.mjs');
  const iosScript = await readRepoFile('scripts/release/ios/build-simulator-app.sh');
  const iosProof = await readRepoFile('scripts/test/child-ios-entitlement-capability-proof.mjs');

  assertIncludes(windowsScript, 'ocentra-parent-agent-windows-x64-latest.msi', 'Windows latest MSI artifact');
  assertIncludes(windowsScript, 'latest-windows.json', 'Windows signed manifest payload');
  assertIncludes(windowsScript, 'sign-manifest', 'Windows manifest signing step');
  assertNotIncludes(windowsScript, 'signtool', 'Windows Authenticode signing step');
  assertNotIncludes(windowsScript, 'Set-AuthenticodeSignature', 'Windows Authenticode signing cmdlet');

  assertIncludes(macosScript, 'ocentra-parent-agent-macos-latest.pkg', 'macOS latest pkg artifact');
  assertIncludes(macosScript, 'pkgbuild', 'macOS pkg build command');
  assertIncludes(macosScript, 'launchctl bootstrap system', 'macOS launchctl bootstrap');
  assertNotIncludes(macosScript, 'codesign', 'macOS codesign step');
  assertNotIncludes(macosScript, 'productsign', 'macOS package signing step');
  assertNotIncludes(macosScript, 'notarytool', 'macOS notarization step');

  assertIncludes(linuxScript, 'ocentra-parent-agent-linux-amd64-latest.deb', 'Linux latest deb artifact');
  assertIncludes(linuxScript, 'dpkg-deb --build', 'Linux deb packaging');
  assertIncludes(linuxScript, 'linux-baseline.json', 'Linux baseline metadata');
  assertIncludes(linuxProof, "packageSigningState: 'unsigned'", 'Linux unsigned package proof state');
  assertIncludes(linuxProof, "repositoryState: 'direct-deb-only'", 'Linux direct distribution proof state');
  assertNotIncludes(linuxScript, 'dpkg-sig', 'Linux package signing step');
  assertNotIncludes(linuxScript, 'gpg', 'Linux repository signing step');
  assertNotIncludes(linuxScript, 'snapcraft', 'Linux store publication step');

  assertIncludes(androidScript, 'ocentra-parent-agent-android-debug-latest.apk', 'Android latest APK artifact');
  assertIncludes(androidScript, 'assembleDebug', 'Android debug APK build');
  assertIncludes(androidScript, 'app-debug.apk', 'Android debug APK source');
  assertNotIncludes(androidScript, 'bundleRelease', 'Android release bundle build');
  assertNotIncludes(androidScript, 'upload', 'Android store upload');
  assertIncludes(
    androidProof,
    'Play Store signing and release-track proof remain planned and not collected',
    'Android Play Store manual/planned boundary'
  );
  assertIncludes(androidProof, "deviceOwnerAuthorityState: 'manual-required'", 'Android device-owner manual-required state');
  assertIncludes(
    androidProof,
    "managedProfileAuthorityState: 'manual-required'",
    'Android managed-profile manual-required state'
  );

  assertIncludes(iosScript, 'ocentra-parent-agent-ios-simulator-latest.zip', 'iOS latest simulator ZIP artifact');
  assertIncludes(iosScript, 'iphonesimulator', 'iOS simulator SDK');
  assertIncludes(iosScript, 'CODE_SIGNING_ALLOWED=NO', 'iOS code signing disabled');
  assertNotIncludes(iosScript, '-exportArchive', 'iOS App Store export step');
  assertIncludes(
    iosProof,
    'TestFlight and App Store distribution remain device-proof-required or planned',
    'iOS TestFlight/App Store manual/planned boundary'
  );
  assertIncludes(
    iosProof,
    'signing and entitlements remain signing-required; simulator script disables signing',
    'iOS signing-disabled boundary'
  );
  assertIncludes(
    iosProof,
    'supervision remains manual-required without supervised-device enrollment and device artifacts',
    'iOS supervision manual-required boundary'
  );

  proofLabels.push('child-artifact-matrix.platform-source-proof');

  return {
    windowsScript: 'scripts/release/windows/build-agent-package.ps1',
    macosScript: 'scripts/release/macos/build-agent-package.sh',
    linuxScript: 'scripts/release/linux/build-agent-package.sh',
    linuxProof: 'scripts/test/child-linux-service-package-proof.mjs',
    androidScript: 'scripts/release/android/build-agent-package.mjs',
    androidProof: 'scripts/test/child-android-device-proof-artifact-gate.mjs',
    iosScript: 'scripts/release/ios/build-simulator-app.sh',
    iosProof: 'scripts/test/child-ios-entitlement-capability-proof.mjs',
  };
}

async function readRuntimeReadModel() {
  const module = await importTsModule('packages/schema-domain/src/child-signing-store-device-owner-matrix.ts');
  proofLabels.push('schema-domain.child-signing-store-device-owner-matrix-parse');
  return module.ChildSigningStoreDeviceOwnerMatrixProofReadModel;
}

async function assertScriptWiring() {
  const packageJson = JSON.parse(await readRepoFile('package.json'));
  const schemaDomainPackage = JSON.parse(await readRepoFile('packages/schema-domain/package.json'));
  const script = packageJson.scripts['test:child-signing-store-device-owner-matrix'];
  if (script !== `node scripts/test/${proofMode}.mjs`) {
    throw new Error('Missing root test:child-signing-store-device-owner-matrix script.');
  }
  if (!schemaDomainPackage.exports['./child-signing-store-device-owner-matrix']) {
    throw new Error('Missing schema-domain export for ./child-signing-store-device-owner-matrix.');
  }
  proofLabels.push('package-scripts.child-signing-store-device-owner-matrix');
  return {
    rootScript: 'test:child-signing-store-device-owner-matrix',
    schemaDomainExport: './child-signing-store-device-owner-matrix',
    rustContract: 'crates/schema/src/child_signing_store_device_owner_matrix.rs',
    sourceAdapter: 'packages/schema-domain/src/child-signing-store-device-owner-matrix.ts',
  };
}

async function readRepoFile(path) {
  return readFile(join(repoRoot, path), 'utf8');
}

async function runNpm(args) {
  await runCommand(...npmCommand(args));
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

function assertNotIncludes(value, expected, label) {
  if (value.includes(expected)) {
    throw new Error(`${label}: unexpectedly contains ${expected}`);
  }
}

function npmCommand(args) {
  const command = process.platform === 'win32' ? 'cmd' : 'npm';
  const commandArgs = process.platform === 'win32' ? ['/c', 'npm', ...args] : args;
  return [command, commandArgs];
}
