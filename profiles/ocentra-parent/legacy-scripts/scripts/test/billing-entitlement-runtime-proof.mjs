import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';
import { pathToFileURL } from 'node:url';

const repoRoot = process.cwd();
const proofMode = 'billing-entitlement-runtime-proof';
const outputDir = join(repoRoot, 'test-results', proofMode);
const proofPath = join(outputDir, 'proof.json');
const commands = [];

await main();

async function main() {
  await mkdir(outputDir, { recursive: true });
  await runCommand(...npmCommand(['run', 'build', '--workspace', '@ocentra-parent/schema-domain']));

  const contract = await assertBuiltContract();
  const packageExport = await assertPublicPackageExport();
  const documentation = await assertDocumentationProof();
  const commit = await gitHead();
  const proof = {
    schemaVersion: 1,
    checkedAt: new Date().toISOString(),
    commit,
    proofMode,
    commands,
    evidence: {
      contract: 'packages/schema-domain/src/billing-entitlement-runtime-proof.ts',
      contractTest: 'packages/schema-domain/tests/unit/billing-parent-visible-summary.test.ts',
      packageExport,
      documentation,
      output: relativePath(proofPath),
    },
    runtimeStates: contract.runtimeStates,
    deviceLimitConsumptionStates: contract.deviceLimitConsumptionStates,
    failureConsumptions: contract.failureConsumptions,
    nonClaims: [
      'Stripe SDK',
      'live provider execution',
      'provider contact',
      'refund or credit runtime',
      'child activity custody',
      'production billing claim',
      'portal UI',
    ],
    knownGaps: [
      'Stripe/live provider execution',
      'provider contact execution',
      'refund and credit execution',
      'entitlement signing/delivery runtime',
      'child-device entitlement consumption',
      'production subscription support claim',
    ],
  };

  await writeFile(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
  console.log(`billing-entitlement-runtime-proof-ok:${relativePath(proofPath)}`);
}

async function assertBuiltContract() {
  const modulePath = pathToFileURL(
    join(repoRoot, 'packages', 'schema-domain', 'dist', 'billing-entitlement-runtime-proof.js')
  );
  const module = await import(modulePath.href);
  const proof = module.BillingEntitlementRuntimeProofReadModel;

  assert.equal(proof.schemaVersion, proofMode);
  assert.deepEqual(module.summarizeBillingEntitlementRuntimeSnapshotStates(proof.snapshotConsumptions), {
    'snapshot-active': 1,
    'snapshot-stale': 1,
    'payment-required': 1,
    'provider-unavailable': 1,
    'manual-review': 1,
  });
  assert.deepEqual(module.summarizeBillingEntitlementRuntimeConsumptionStates(proof.deviceLimitConsumptions), {
    'accepted-local': 1,
    'accepted-grace': 1,
    'blocked-new-device': 1,
    'manual-required': 1,
    'unavailable-local-safety': 0,
  });
  assert.deepEqual(
    proof.nonClaims,
    [
      'no-stripe-sdk',
      'no-live-provider-execution',
      'no-provider-contact',
      'no-refund-credit-runtime',
      'no-child-activity-custody',
      'no-production-billing-claim',
      'no-portal-ui',
    ],
    'expected billing entitlement runtime non-claims to remain explicit'
  );

  return {
    runtimeStates: proof.snapshotConsumptions.map((entry) => entry.runtimeState),
    deviceLimitConsumptionStates: proof.deviceLimitConsumptions.map((entry) => entry.consumptionState),
    failureConsumptions: proof.failureConsumptions.map((entry) => entry.failureState.failureKind),
  };
}

async function assertPublicPackageExport() {
  const module = await import('@ocentra-parent/schema-domain/billing-entitlement-runtime-proof');
  assert.equal(typeof module.decodeBillingEntitlementRuntimeProof, 'function');
  assert.ok(module.BillingEntitlementRuntimeProofSchema);
  return '@ocentra-parent/schema-domain/billing-entitlement-runtime-proof';
}

async function assertDocumentationProof() {
  const productionDistribution = await readRepoFile('docs/features/production-distribution-support.md');
  const billing = await readRepoFile('docs/expectations/billing.md');
  assertIncludes(productionDistribution, proofMode, 'production distribution feature proof note');
  assertIncludes(billing, proofMode, 'billing expectation proof note');
  return ['docs/features/production-distribution-support.md', 'docs/expectations/billing.md'];
}

async function readRepoFile(path) {
  return readFile(join(repoRoot, path), 'utf8');
}

async function runCommand(commandName, args, env = {}) {
  commands.push([commandName, ...args].join(' '));
  await new Promise((resolve, reject) => {
    const child = spawn(commandName, args, {
      cwd: repoRoot,
      env: { ...process.env, ...env },
      stdio: 'inherit',
      windowsHide: true,
    });
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

function assertIncludes(value, expected, label) {
  if (!value.includes(expected)) {
    throw new Error(`${label}: missing ${expected}`);
  }
}

function relativePath(path) {
  return relative(repoRoot, path).replaceAll('\\', '/');
}

function npmCommand(args) {
  const command = process.platform === 'win32' ? 'cmd' : 'npm';
  const commandArgs = process.platform === 'win32' ? ['/c', 'npm', ...args] : args;
  return [command, commandArgs];
}
