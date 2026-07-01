import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';
import { pathToFileURL } from 'node:url';

const repoRoot = process.cwd();
const proofMode = 'billing-entitlement-contract-proof';
const outputDir = join(repoRoot, 'test-results', proofMode);
const proofPath = join(outputDir, 'proof.json');
const commands = [];

await main();

async function main() {
  await mkdir(outputDir, { recursive: true });
  await runCommand(...npmCommand(['run', 'build', '--workspace', '@ocentra-parent/schema-domain']));
  await runCommand(
    ...npmCommand([
      'run',
      'test',
      '--workspace',
      '@ocentra-parent/schema-domain',
      '--',
      'tests/unit/billing-parent-visible-summary.test.ts',
    ])
  );

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
      contract: 'packages/schema-domain/src/billing-entitlement.ts',
      proofModel: 'packages/schema-domain/src/billing-entitlement-proof.ts',
      contractTest: 'packages/schema-domain/tests/unit/billing-parent-visible-summary.test.ts',
      parentVisibleSummaryTest: 'packages/schema-domain/tests/unit/billing-parent-visible-summary.test.ts',
      packageExport,
      documentation,
      output: relativePath(proofPath),
    },
    planId: contract.planId,
    entitlementDecisions: contract.entitlementDecisions,
    subscriptionStatuses: contract.subscriptionStatuses,
    deviceLimitDecisions: contract.deviceLimitDecisions,
    failureStates: contract.failureStates,
    nonClaims: [
      'Stripe SDK',
      'billing provider backend',
      'provider token custody',
      'child activity custody',
      'billing-driven safety shutdown',
      'portal UI',
    ],
    knownGaps: [
      'billing provider integration',
      'account backend and entitlement signing runtime',
      'subscription sync delivery runtime',
      'portal billing UI',
      'child-device entitlement consumption',
    ],
  };

  await writeFile(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
  console.log(`billing-entitlement-contract-proof-ok:${relativePath(proofPath)}`);
}

async function assertBuiltContract() {
  const modulePath = pathToFileURL(join(repoRoot, 'packages', 'schema-domain', 'dist', 'billing-entitlement-proof.js'));
  const module = await import(modulePath.href);
  const proof = module.BillingEntitlementContractProofReadModel;

  assert.equal(proof.schemaVersion, 'billing-entitlement-contract-proof');
  assert.equal(proof.plan.planId, 'family-plus-monthly');
  assert.deepEqual(
    proof.nonClaims,
    [
      'no-stripe-sdk',
      'no-billing-provider-backend',
      'no-provider-token-custody',
      'no-child-activity-custody',
      'no-safety-shutdown',
      'no-portal-ui',
    ],
    'expected billing entitlement non-claims to remain explicit'
  );
  assert.deepEqual(summarizeValues(proof.failureStates.map((entry) => entry.failureKind)), {
    'provider-unavailable': 1,
    'network-unavailable': 1,
    'stale-snapshot': 1,
    'payment-required': 1,
    'account-mismatch': 1,
    'validation-failed': 1,
  });

  return {
    planId: proof.plan.planId,
    entitlementDecisions: proof.entitlementSnapshot.featureDecisions.map((entry) => entry.decision),
    subscriptionStatuses: proof.subscriptionStatusProofRows.map((entry) => entry.subscriptionStatus),
    deviceLimitDecisions: proof.deviceLimitDecisions.map((entry) => entry.decision),
    failureStates: proof.failureStates.map((entry) => entry.failureKind),
  };
}

async function assertPublicPackageExport() {
  const module = await import('@ocentra-parent/schema-domain/billing-entitlement');
  assert.equal(typeof module.decodeBillingEntitlementContractProof, 'function');
  assert.ok(module.BillingEntitlementContractProofSchema);
  return '@ocentra-parent/schema-domain/billing-entitlement';
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

function summarizeValues(values) {
  return Object.fromEntries(
    values.reduce((counts, value) => counts.set(value, (counts.get(value) ?? 0) + 1), new Map())
  );
}

function npmCommand(args) {
  const command = process.platform === 'win32' ? 'cmd' : 'npm';
  const commandArgs = process.platform === 'win32' ? ['/c', 'npm', ...args] : args;
  return [command, commandArgs];
}
