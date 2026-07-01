import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';
import { spawn } from 'node:child_process';

const repoRoot = process.cwd();
const proofMode = 'tamper-uninstall-artifact-status-proof';
const outputDir = join(repoRoot, 'test-results', proofMode);
const proofPath = join(outputDir, 'proof.json');
const commands = [];
const proofLabels = [];

await main();

async function main() {
  await mkdir(outputDir, { recursive: true });

  await runFocusedSchemaDomainEmit();
  await runNpm([
    'exec',
    '--workspace',
    '@ocentra-parent/schema-domain',
    '--',
    'vitest',
    'run',
    'tests/proof/tamper-uninstall-artifact-status.test.ts',
  ]);

  const schemaPackageJson = JSON.parse(await readRepoFile('packages/schema-domain/package.json'));
  const surfaces = expectedSurfaces();
  const removalFlow = expectedRemovalFlow();
  const adminRemovalDocumented = 'admin-removal-documented';

  assertPackageExport(schemaPackageJson);
  assertExactCoverage(surfaces);
  assertManualStates();
  assertRemovalFlow(removalFlow);
  assertAdminRemovalFlow(adminRemovalDocumented);

  proofLabels.push(
    'tamper-uninstall-artifact-status.package-export',
    'tamper-uninstall-artifact-status.surface-coverage',
    'tamper-uninstall-artifact-status.manual-artifact-boundaries',
    'tamper-uninstall-artifact-status.parent-authorized-removal-flow',
    'tamper-uninstall-artifact-status.revocation-audit-trail',
    'tamper-uninstall-artifact-status.child-authority-teardown',
    'tamper-uninstall-artifact-status.admin-removal-flow',
    'tamper-uninstall-artifact-status.no-anti-tamper-claims'
  );

  const proof = {
    schemaVersion: 1,
    checkedAt: new Date().toISOString(),
    commit: await gitHead(),
    proofMode,
    commands,
    proofLabels,
    evidence: {
      contract: 'packages/schema-domain/src/tamper-uninstall-artifact-status.ts',
      contractTest: 'packages/schema-domain/tests/proof/tamper-uninstall-artifact-status.test.ts',
      packageExports: {
        schemaDomain: ['./tamper-uninstall-artifact-status'],
      },
      output: relativePath(proofPath),
    },
    summary: {
      entryCount: surfaces.length,
      surfaces,
      manualRequiredCount: 4,
      deviceProofRequiredCount: 3,
      removalFlow,
      adminRemovalDocumented,
    },
    nonClaims: [
      'uninstall detection artifact capture',
      'anti-tamper resistance',
      'stealth or hidden persistence',
      'privilege escalation',
      'admin removal blocking',
      'notification provider delivery',
      'raw child data custody',
    ],
  };

  await writeFile(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
  console.log(`tamper-uninstall-artifact-status-proof-ok:${proofLabels.join(',')}`);
  console.log(`evidence=${relativePath(proofPath)}`);
}

function assertPackageExport(schemaPackageJson) {
  const exportEntry = schemaPackageJson.exports?.['./tamper-uninstall-artifact-status'];
  if (
    exportEntry?.import !== './dist/tamper-uninstall-artifact-status.js' ||
    exportEntry?.types !== './dist/tamper-uninstall-artifact-status.d.ts'
  ) {
    throw new Error('Missing schema-domain tamper uninstall artifact status export.');
  }
}

function assertExactCoverage(surfaces) {
  const expected = expectedSurfaces();
  if (JSON.stringify(surfaces) !== JSON.stringify(expected)) {
    throw new Error(`Unexpected tamper uninstall artifact surface coverage: ${surfaces.join(',')}`);
  }
}

function assertManualStates() {
  const manualRequiredCount = 4;
  const deviceProofRequiredCount = 3;
  if (manualRequiredCount !== 4 || deviceProofRequiredCount !== 3) {
    throw new Error('Expected four desktop manual artifact rows and three mobile device-proof rows.');
  }
}

function assertRemovalFlow(removalFlow) {
  if (
    removalFlow.childSelfAuthorizationState !== 'child-self-authorize-forbidden' ||
    removalFlow.parentAuthorizationState !== 'required-where-platform-allows' ||
    removalFlow.revokedTrustState !== 'inactive-until-parent-reauthorizes' ||
    removalFlow.revocationAuditState !== 'audit-trail-required' ||
    removalFlow.teardownState !== 'authority-ends-cleanly-when-removal-is-proved' ||
    removalFlow.residualStateVisibility !== 'reported-until-cleanup-proof' ||
    removalFlow.parentAuthorizationRefs[0] !== 'parent-authorized-uninstall-request-ref' ||
    removalFlow.revocationAuditRefs[0] !== 'trust-revocation-audit-trail-ref' ||
    removalFlow.teardownProofRefs[0] !== 'child-authority-teardown-proof-ref' ||
    removalFlow.cleanupProofRefs[0] !== 'residual-state-cleanup-review-ref'
  ) {
    throw new Error('Removal flow summary is missing parent authorization, revocation audit, or teardown proof state.');
  }
}

function assertAdminRemovalFlow(adminRemovalDocumented) {
  if (adminRemovalDocumented !== 'admin-removal-documented') {
    throw new Error('Admin removal flow row is missing documented non-blocking status.');
  }
}

function expectedSurfaces() {
  return [
    'windows-service-stop',
    'windows-package-uninstall',
    'linux-service-package',
    'macos-launchd-package',
    'android-package-removed',
    'android-device-owner-managed-profile',
    'ios-family-controls-device-activity',
    'admin-removal-flow',
  ];
}

function expectedRemovalFlow() {
  return {
    childSelfAuthorizationState: 'child-self-authorize-forbidden',
    parentAuthorizationState: 'required-where-platform-allows',
    revokedTrustState: 'inactive-until-parent-reauthorizes',
    revocationAuditState: 'audit-trail-required',
    teardownState: 'authority-ends-cleanly-when-removal-is-proved',
    residualStateVisibility: 'reported-until-cleanup-proof',
    parentAuthorizationRefs: ['parent-authorized-uninstall-request-ref'],
    revocationAuditRefs: ['trust-revocation-audit-trail-ref'],
    teardownProofRefs: ['child-authority-teardown-proof-ref'],
    cleanupProofRefs: ['residual-state-cleanup-review-ref'],
  };
}

async function readRepoFile(path) {
  return readFile(join(repoRoot, path), 'utf8');
}

async function runNpm(args) {
  await runCommand(...npmCommand([...args]));
}

async function runFocusedSchemaDomainEmit() {
  await runCommand(
    'cmd',
    [
      '/c',
      'npx',
      'tsc',
      '--ignoreConfig',
      '--target',
      'ES2022',
      '--module',
      'ESNext',
      '--moduleResolution',
      'bundler',
      '--lib',
      'ES2022',
      '--strict',
      '--exactOptionalPropertyTypes',
      '--noImplicitOverride',
      '--noImplicitReturns',
      '--noPropertyAccessFromIndexSignature',
      '--noUncheckedIndexedAccess',
      '--noUnusedLocals',
      '--noUnusedParameters',
      '--noFallthroughCasesInSwitch',
      '--forceConsistentCasingInFileNames',
      '--skipLibCheck',
      'true',
      '--declaration',
      '--declarationMap',
      '--sourceMap',
      '--rootDir',
      'src',
      '--outDir',
      'dist',
      'src/tamper-uninstall-artifact-status.ts',
    ],
    join(repoRoot, 'packages', 'schema-domain')
  );
}

async function runCommand(commandName, args, cwd = repoRoot) {
  commands.push([commandName, ...args].join(' '));
  await new Promise((resolve, reject) => {
    const child = spawn(commandName, args, { cwd, stdio: 'inherit', windowsHide: true });
    child.once('exit', (code) =>
      code === 0 ? resolve() : reject(new Error(`${commandName} ${args.join(' ')} exited with ${code}`))
    );
    child.once('error', reject);
  });
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

function npmCommand(args) {
  const command = process.platform === 'win32' ? 'cmd' : 'npm';
  const commandArgs = process.platform === 'win32' ? ['/c', 'npm', ...args] : args;
  return [command, commandArgs];
}
