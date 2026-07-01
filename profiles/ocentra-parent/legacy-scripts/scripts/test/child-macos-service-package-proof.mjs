import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';
import { pathToFileURL } from 'node:url';
import { spawn } from 'node:child_process';
import { tsImport } from 'tsx/esm/api';

const repoRoot = process.cwd();
const proofMode = 'child-macos-service-package-proof';
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
    'tests/proof/child-macos-service-package-proof.test.ts',
  ]);

  const sourceProof = await assertMacosSourceProof();
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
      contract: 'packages/schema-domain/src/child-macos-service-package-proof.ts',
      contractTest: 'packages/schema-domain/tests/proof/child-macos-service-package-proof.test.ts',
      matrix: 'docs/expectations/pre-ai-proof-matrix.json',
      output: relativePath(proofPath),
    },
    runtimeReadModel,
    matrixProof,
    scriptWiring,
    childMacosServicePackageProved: {
      distributionMode: 'launchd-pkg-script: the child artifact is packaged through a macOS pkg + LaunchDaemon script boundary',
      artifactState: 'pkg-script-defined: the release script stages the child agent binary and LaunchDaemon plist into a macOS package layout',
      launchdBoundaryState:
        'launchd-boundary-scripted: preinstall/postinstall scripts bootout, bootstrap, and enable the ca.ocentra.parent.agent launchd label',
      signingState: 'unsigned: no codesign or productsign step is present in the macOS child package script',
    },
    childMacosServiceStillManual: [
      'package install on a real macOS host',
      'launchd service start and health on a real macOS host',
      'restart or crash recovery beyond the KeepAlive declaration',
      'Apple codesign identity and signed entitlements',
      'Apple notarization and stapled ticket artifacts',
      'disable, uninstall, removal, and cleanup artifacts',
      'parent-client parity or hidden background-service authority',
    ],
    nonClaims: [
      'real macOS install success',
      'runtime launch success or steady-state service health',
      'KeepAlive as restart or recovery proof',
      'signed, notarized, or entitlement-approved macOS artifacts',
      'uninstall, disable, removal, or cleanup behavior',
      'parent-client parity or hidden persistence',
    ],
  };

  await writeFile(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
  console.log(`child-macos-service-package-proof-ok:${proofLabels.join(',')}`);
  console.log(`evidence=${proofPath}`);
}

async function assertMacosSourceProof() {
  const buildScript = await readRepoFile('scripts/release/macos/build-agent-package.sh');
  const plist = await readRepoFile('scripts/release/macos/ca.ocentra.parent.agent.plist');

  assertIncludes(buildScript, 'pkgbuild', 'macOS pkgbuild command');
  assertIncludes(buildScript, 'cargo build --release -p ocentra-parent-agent-service', 'macOS child binary build');
  assertIncludes(
    buildScript,
    '/Library/Ocentra/Ocentra Parent Agent/bin/ocentra-parent-agent-service',
    'macOS child binary install path'
  );
  assertIncludes(
    buildScript,
    '/Library/LaunchDaemons/ca.ocentra.parent.agent.plist',
    'macOS launchd plist install path'
  );
  assertIncludes(buildScript, 'launchctl bootstrap system', 'macOS launchctl bootstrap');
  assertIncludes(buildScript, 'launchctl enable system/ca.ocentra.parent.agent', 'macOS launchctl enable');
  assertIncludes(buildScript, 'launchctl bootout system', 'macOS launchctl bootout');
  assertNotIncludes(buildScript, 'productsign', 'macOS package signing');
  assertNotIncludes(buildScript, 'codesign', 'macOS binary signing');
  assertNotIncludes(buildScript, 'notarytool', 'macOS notarization');
  assertNotIncludes(buildScript, 'stapler', 'macOS stapled notarization ticket');
  assertNotIncludes(buildScript, 'pkgutil --forget', 'macOS removal cleanup');

  assertIncludes(plist, '<string>ca.ocentra.parent.agent</string>', 'macOS launchd label');
  assertIncludes(
    plist,
    '<string>/Library/Ocentra/Ocentra Parent Agent/bin/ocentra-parent-agent-service</string>',
    'macOS launchd binary path'
  );
  assertIncludes(plist, '<key>RunAtLoad</key>', 'macOS RunAtLoad declaration');
  assertIncludes(plist, '<key>KeepAlive</key>', 'macOS KeepAlive declaration');
  proofLabels.push('macos-launchd.service-package-source-proof');

  return {
    buildScript: 'scripts/release/macos/build-agent-package.sh',
    launchdPlist: 'scripts/release/macos/ca.ocentra.parent.agent.plist',
  };
}

function buildRuntimeReadModel() {
  return {
    schemaVersion: proofMode,
    serviceLabel: 'ca.ocentra.parent.agent',
    plistPath: '/Library/LaunchDaemons/ca.ocentra.parent.agent.plist',
    binaryPath: '/Library/Ocentra/Ocentra Parent Agent/bin/ocentra-parent-agent-service',
    distributionMode: 'launchd-pkg-script',
    artifactState: 'pkg-script-defined',
    launchdBoundaryState: 'launchd-boundary-scripted',
    installState: 'manual-install-proof-required',
    runtimeState: 'manual-service-proof-required',
    restartState: 'keepalive-declared-manual-recovery-proof',
    signingState: 'unsigned',
    notarizationState: 'manual-required',
    entitlementState: 'manual-required',
    uninstallState: 'manual-uninstall-proof-required',
    removalState: 'manual-removal-proof-required',
    protocolBridgeProof: {
      serviceLabel: 'ca.ocentra.parent.agent',
      plistPath: '/Library/LaunchDaemons/ca.ocentra.parent.agent.plist',
      binaryPath: '/Library/Ocentra/Ocentra Parent Agent/bin/ocentra-parent-agent-service',
      commands: [
        'child.macos.service.package.proof.get',
        'child.macos.service.lifecycle.proof.get',
        'child.macos.service.manual-proof.get',
      ],
      events: [
        'child.macos.service.package.proof.reported',
        'child.macos.service.lifecycle.proof.reported',
        'child.macos.service.manual-proof.reported',
      ],
      runtimeOwner: 'macos-launchctl-script',
      proofRequirement:
        'macOS child package proof names the launchd service boundary, install script bootstrap path, and manual-required runtime gaps',
      claimBoundary:
        'launchd plist and install scripts prove only the macOS service-manager boundary; they do not prove installed runtime health, restart recovery, notarization, or parent-client parity',
    },
    surfaceProofs: surfaceProofs(),
    lifecycleProofs: lifecycleProofs(),
    claimBoundaries: {
      packageArtifact:
        'pkgbuild script and staged payload prove only the child macOS artifact layout and install script boundary',
      launchdBoundary: 'launchd plist plus bootstrap/enable commands prove only the macOS service-manager boundary',
      runtimeBoundary:
        'launchd source proof does not prove installed service health, launch success, or crash-free runtime behavior',
      restartBoundary: 'KeepAlive declaration is not runtime restart or recovery proof without macOS service artifacts',
      signingBoundary:
        'the child macOS package is unsigned in this proof surface because no codesign or productsign artifact is attached',
      notarizationBoundary:
        'notarization remains manual-required because no notarytool or stapled ticket artifact is attached',
      entitlementBoundary:
        'entitlement or hardened-runtime claims remain manual-required without signed entitlement artifacts',
      uninstallBoundary:
        'disable and uninstall remain manual-required because no uninstall script or launchctl disable artifact is attached',
      removalBoundary:
        'removal and cleanup remain manual-required because no package removal, plist cleanup, or post-remove heartbeat artifact is attached',
      parentParityBoundary:
        'child macOS launchd proof does not imply parent-client parity or hidden background-service authority',
    },
    updatedAt: new Date().toISOString(),
  };
}

async function parseRuntimeReadModel(readModel) {
  const module = await importTsModule('packages/schema-domain/src/child-macos-service-package-proof.ts');
  const parsed = module.ChildMacosServicePackageProofReadModelSchema.parse(readModel);
  proofLabels.push('schema-domain.child-macos-service-package-proof-parse');
  return parsed;
}

async function assertProofMatrix() {
  const matrix = JSON.parse(await readRepoFile('docs/expectations/pre-ai-proof-matrix.json'));
  const claim = matrix.claims.find((candidate) => candidate.id === proofMode);
  const scenario = matrix.checkpointScenarios.find((candidate) => candidate.id === proofMode);
  if (!claim || !scenario) {
    throw new Error('Proof matrix is missing child-macos-service-package-proof claim or scenario.');
  }
  assertArrayIncludes(matrix.requiredCompletedClaimIds, proofMode, 'required completed claim');
  assertArrayIncludes(scenario.ciCommands, `node scripts/test/${proofMode}.mjs`, 'scenario command');
  assertArrayIncludes(claim.ciProof.commands, `node scripts/test/${proofMode}.mjs`, 'claim command');
  proofLabels.push('proof-matrix.child-macos-service-package-proof');
  return {
    claimId: claim.id,
    platformCoverage: claim.platformCoverage,
    runtimeSurfaceCoverage: claim.runtimeSurfaceCoverage,
  };
}

async function assertScriptWiring() {
  const packageJson = JSON.parse(await readRepoFile('package.json'));
  const schemaDomainPackage = JSON.parse(await readRepoFile('packages/schema-domain/package.json'));
  const script = packageJson.scripts['test:child-macos-service-package-proof'];
  if (script !== `node scripts/test/${proofMode}.mjs`) {
    throw new Error('Missing root test:child-macos-service-package-proof script.');
  }
  if (!schemaDomainPackage.exports['./child-macos-service-package-proof']) {
    throw new Error('Missing schema-domain export for ./child-macos-service-package-proof.');
  }
  proofLabels.push('package-scripts.child-macos-service-package-proof');
  return {
    rootScript: 'test:child-macos-service-package-proof',
    schemaDomainExport: './child-macos-service-package-proof',
    sourceContract: 'packages/schema-domain/src/child-macos-service-package-proof.ts',
  };
}

function surfaceProofs() {
  return [
    surfaceProof('pkgbuild-script', 'package-lifecycle', 'manual-required', 'ci-mechanical-proof', 'macos-pkgbuild-script'),
    surfaceProof(
      'service-binary-path',
      'headless-agent-service',
      'manual-required',
      'ci-mechanical-proof',
      'macos-release-binary'
    ),
    surfaceProof('launchd-plist', 'headless-agent-service', 'manual-required', 'ci-mechanical-proof', 'macos-launchd-plist'),
    surfaceProof(
      'launchctl-bootstrap',
      'headless-agent-service',
      'manual-required',
      'ci-mechanical-proof',
      'macos-launchctl-script'
    ),
    surfaceProof(
      'launchctl-enable',
      'headless-agent-service',
      'manual-required',
      'ci-mechanical-proof',
      'macos-launchctl-script'
    ),
    surfaceProof('run-at-load', 'headless-agent-service', 'manual-required', 'ci-mechanical-proof', 'macos-launchd-plist'),
    surfaceProof(
      'keepalive-declaration',
      'headless-agent-service',
      'manual-required',
      'ci-mechanical-proof',
      'macos-launchd-plist'
    ),
    surfaceProof('signing-review', 'signing-entitlements', 'manual-required', 'unsigned', 'apple-codesign'),
    surfaceProof('notarization-review', 'store-distribution', 'manual-required', 'manual-required', 'apple-notarytool'),
    surfaceProof('entitlement-review', 'signing-entitlements', 'manual-required', 'manual-required', 'apple-codesign'),
    surfaceProof(
      'uninstall-disable-review',
      'package-lifecycle',
      'manual-required',
      'manual-required',
      'macos-manual-proof'
    ),
    surfaceProof('removal-review', 'package-lifecycle', 'manual-required', 'manual-required', 'macos-manual-proof'),
  ];
}

function lifecycleProofs() {
  return [
    lifecycleProof('release-script', 'ci-mechanical-proof', 'macos-pkgbuild-script'),
    lifecycleProof('binary-stage', 'ci-mechanical-proof', 'macos-release-binary'),
    lifecycleProof('launchd-plist', 'ci-mechanical-proof', 'macos-launchd-plist'),
    lifecycleProof('package-build', 'ci-mechanical-proof', 'macos-pkgbuild-script'),
    lifecycleProof('install-bootstrap', 'ci-mechanical-proof', 'macos-launchctl-script'),
    lifecycleProof('install-enable', 'ci-mechanical-proof', 'macos-launchctl-script'),
    lifecycleProof('service-start', 'manual-required', 'macos-manual-proof'),
    lifecycleProof('restart-recovery', 'manual-required', 'macos-manual-proof'),
    lifecycleProof('signing-review', 'unsigned', 'apple-codesign'),
    lifecycleProof('notarization-review', 'manual-required', 'apple-notarytool'),
    lifecycleProof('uninstall-disable', 'manual-required', 'macos-manual-proof'),
    lifecycleProof('removal-cleanup', 'manual-required', 'macos-manual-proof'),
  ];
}

function surfaceProof(surface, parentCapability, parentCapabilityStatus, proofState, runtimeOwner) {
  const proofRequirement = `${surface} remains ${proofState} until real macOS artifacts change it`;
  return {
    surface,
    parentCapability,
    parentCapabilityStatus,
    proofState,
    runtimeOwner,
    proofRequirement,
    claimBoundary: proofRequirement,
  };
}

function lifecycleProof(phase, proofState, runtimeOwner) {
  return {
    phase,
    proofState,
    runtimeOwner,
    proofRequirement: `${phase} proof state is ${proofState}`,
    claimBoundary: `${phase} does not upgrade macOS service claims without platform artifacts`,
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
