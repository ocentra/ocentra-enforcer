import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { mkdir, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';
import { pathToFileURL } from 'node:url';

const repoRoot = process.cwd();
const proofMode = 'payment-checkout-boundary-proof';
const outputDir = join(repoRoot, 'test-results', proofMode);
const proofPath = join(outputDir, 'proof.json');
const commands = [];

await main();

async function main() {
  await mkdir(outputDir, { recursive: true });
  await runCommand(...npmCommand(['run', 'build', '--workspace', '@ocentra-parent/schema-domain']));

  const contract = await assertBuiltContract();
  const proof = {
    schemaVersion: 1,
    checkedAt: new Date().toISOString(),
    proofMode,
    commands,
    evidence: {
      contract: 'packages/schema-domain/src/billing-checkout-portal-boundary.ts',
      values: 'packages/schema-domain/src/billing-checkout-portal-boundary-values.ts',
      unitTest: 'packages/schema-domain/tests/unit/billing-parent-visible-summary.test.ts',
      builtModule: 'packages/schema-domain/dist/billing-checkout-portal-boundary.js',
      output: relativePath(proofPath),
    },
    contract,
    noClaims: [
      'checkout redirect is not payment completion',
      'provider webhook confirmation remains required',
      'hosted portal is management-only and not entitlement truth',
      'browser and desktop surfaces never receive provider secrets',
    ],
  };

  await writeFile(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
  console.log(`payment-checkout-boundary-proof-ok:${relativePath(proofPath)}`);
}

async function assertBuiltContract() {
  const modulePath = pathToFileURL(
    join(repoRoot, 'packages', 'schema-domain', 'dist', 'billing-checkout-portal-boundary.js')
  );
  const module = await import(modulePath.href);

  assert.equal(module.BillingHostedReturnRoute.CheckoutSuccess.resolution, 'awaiting-provider-webhook');
  assert.equal(module.BillingHostedReturnRoute.CheckoutCancel.resolution, 'cancelled-before-provider-confirmation');
  assert.equal(module.BillingHostedReturnRoute.PortalReturn.resolution, 'portal-management-only');

  assert.equal(
    module.BillingCheckoutSessionRequestSchema.safeParse({
      schemaVersion: 'billing-checkout-portal-boundary',
      requestId: 'proof-bad-plan',
      kind: 'checkout-session-create',
      actor: { actorId: 'parent-actor-1', role: 'parent' },
      parentAccount: { parentAccountId: 'parent-account-1' },
      family: { familyId: 'family-1' },
      planId: 'family-safety-free',
      originGateState: 'same-origin-verified',
      csrfState: 'csrf-token-verified',
      surfaceSecretCustody: 'not-present',
      successRoute: module.BillingHostedReturnRoute.CheckoutSuccess,
      cancelRoute: module.BillingHostedReturnRoute.CheckoutCancel,
      abuseGateState: 'passed-turnstile',
    }).success,
    false
  );

  assert.equal(
    module.BillingCheckoutSessionRequestSchema.safeParse({
      schemaVersion: 'billing-checkout-portal-boundary',
      requestId: 'proof-bad-origin',
      kind: 'checkout-session-create',
      actor: { actorId: 'parent-actor-1', role: 'parent' },
      parentAccount: { parentAccountId: 'parent-account-1' },
      family: { familyId: 'family-1' },
      planId: 'family-monitor-core',
      originGateState: 'cross-origin',
      csrfState: 'csrf-token-verified',
      surfaceSecretCustody: 'not-present',
      successRoute: module.BillingHostedReturnRoute.CheckoutSuccess,
      cancelRoute: module.BillingHostedReturnRoute.CheckoutCancel,
      abuseGateState: 'passed-turnstile',
    }).success,
    false
  );

  assert.equal(
    module.BillingCheckoutSessionResponseSchema.safeParse({
      schemaVersion: 'billing-checkout-portal-boundary',
      requestId: 'proof-leaked-secret',
      kind: 'checkout-session-create',
      status: 'accepted',
      hostedSessionId: 'checkout-session-1',
      hostedUrl: 'https://checkout.stripe.com/c/pay/cs_test_a?client_secret=leak',
      expiresAt: '2026-06-13T09:00:00.000Z',
      rejectionReason: null,
    }).success,
    false
  );

  return {
    checkoutPlans: ['family-plus-monthly', 'family-monitor-core', 'family-monitor-plus'],
    returnResolutions: {
      checkoutSuccess: module.BillingHostedReturnRoute.CheckoutSuccess.resolution,
      checkoutCancel: module.BillingHostedReturnRoute.CheckoutCancel.resolution,
      portalReturn: module.BillingHostedReturnRoute.PortalReturn.resolution,
    },
  };
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

function relativePath(path) {
  return relative(repoRoot, path).replaceAll('\\', '/');
}

function npmCommand(args) {
  const command = process.platform === 'win32' ? 'cmd' : 'npm';
  const commandArgs = process.platform === 'win32' ? ['/c', 'npm', ...args] : args;
  return [command, commandArgs];
}
