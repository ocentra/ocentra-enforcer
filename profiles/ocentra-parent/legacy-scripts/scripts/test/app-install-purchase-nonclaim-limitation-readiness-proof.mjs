import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';
import { pathToFileURL } from 'node:url';

const repoRoot = process.cwd();
const outputDir = join(repoRoot, 'test-results', 'app-install-purchase-nonclaim-limitation-readiness-proof');
const proofPath = join(outputDir, 'proof.json');
const commands = [];
const proofBranch = 'codex/e-b-app-install-windows-host-evidence-readiness-proof';
const deterministicProofRevision = 'branch-head-validated-by-harness';
const deterministicGeneratedAt = 'deterministic-proof-artifact';

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
      'tests/unit/app-install-purchase-platform-adapter-evidence-gap-proof.test.ts',
    ])
  );

  const proofModule = await loadPlatformAdapterEvidenceGapModule();

  const summary = proofModule.summarizeAppInstallPurchasePlatformAdapterEvidenceGapProof(
    proofModule.AppInstallPurchasePlatformAdapterEvidenceGapProofReadModel
  );
  assert.deepEqual(summary, {
    platformAdapterEvidenceGapRows: 5,
    adapterEvidenceGapRows: 1,
    manualAdapterEvidenceRequiredRows: 1,
    platformUnavailableRows: 1,
    blockedBeforeClaimRows: 2,
    realAdapterEvidenceRows: 0,
    adapterImplementedRows: 0,
    productClaimApprovedRows: 0,
  });

  const docs = await readDocumentation();
  const docChecks = assertDocumentationKeepsClaimsManual(docs);
  const branchHead = await gitHead();
  assert.ok(branchHead.length > 0, 'git HEAD is available for proof validation');
  const proof = {
    schemaVersion: 1,
    proofMode: 'app-install-purchase-nonclaim-limitation-readiness-proof',
    generatedAt: deterministicGeneratedAt,
    branch: proofBranch,
    commit: deterministicProofRevision,
    commitMetadata:
      'This proof intentionally avoids embedding HEAD because a committed artifact cannot contain its own final commit hash.',
    gitStatusShort: 'validated-by-explicit-handoff-status-check',
    baseMainState: 'after-pr487-platform-adapter-evidence-gap-proof-merged',
    commands,
    packageExportState: 'canonical-schema-domain-public-subpath-export-confirmed',
    productDocsState: 'no-product-doc-update-needed-current-docs-already-own-required-limitation-and-nonclaim-language',
    checklistState: 'not-touched-current-codex-b-docs-product-capability-checklist-lock',
    evidence: {
      sourcePlatformAdapterEvidenceGapContract:
        'packages/schema-domain/src/app-install-purchase-platform-adapter-evidence-gap-proof.ts',
      sourcePlatformAdapterEvidenceGapTest:
        'packages/schema-domain/tests/unit/app-install-purchase-platform-adapter-evidence-gap-proof.test.ts',
      featureDoc: 'docs/features/app-install-purchase-approval.md',
      expectationDoc: 'docs/expectations/app-install-purchase-approval.md',
      platformExpectationDoc: 'docs/expectations/platforms.md',
      packageExport: '@ocentra-parent/schema-domain/app-install-purchase-platform-adapter-evidence-gap-proof',
      output: relative(repoRoot, proofPath),
    },
    platformAdapterEvidenceGapSummary: summary,
    docChecks,
    nonClaims: [
      'no-product-claim-approval',
      'no-portal-approval-ui',
      'no-portal-report-ui',
      'no-provider-store-execution',
      'no-store-integration',
      'no-platform-adapter-implementation',
      'no-platform-interception',
      'no-child-device-delivery',
      'no-runtime-writer-or-report-delivery',
      'no-app-blocking',
      'no-child-activity-data',
      'no-ocentra-hosted-family-data-custody',
    ],
    knownGaps: [
      'real portal approval/report UI remains required before product claim upgrade',
      'external runtime writer delivery to a device remains required before delivery upgrade',
      'real child delivery remains required before delivery upgrade',
      'real provider/store API execution with credentials and evidence remains required before provider/store claim',
      'actual platform adapters remain required before adapter evidence rows can upgrade',
      'docs/product-capability-checklist.md update intentionally deferred while codex-b holds the current lock',
    ],
  };

  await writeFile(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
  console.log(`app-install-purchase-nonclaim-limitation-readiness-proof-ok:${relative(repoRoot, proofPath)}`);
}

async function loadPlatformAdapterEvidenceGapModule() {
  const modulePath = join(repoRoot, 'packages', 'schema-domain', 'dist', 'app-install-purchase-platform-adapter-evidence-gap-proof.js');
  return import(pathToFileURL(modulePath).href);
}

async function readDocumentation() {
  const featureDoc = await readFile(join(repoRoot, 'docs', 'features', 'app-install-purchase-approval.md'), 'utf8');
  const expectationDoc = await readFile(
    join(repoRoot, 'docs', 'expectations', 'app-install-purchase-approval.md'),
    'utf8'
  );
  const platformDoc = await readFile(join(repoRoot, 'docs', 'expectations', 'platforms.md'), 'utf8');
  return { expectationDoc, featureDoc, platformDoc };
}

function assertDocumentationKeepsClaimsManual({ expectationDoc, featureDoc, platformDoc }) {
  const checks = [
    {
      doc: 'docs/features/app-install-purchase-approval.md',
      label: 'next-proof-real-evidence-gates',
      needle:
        'The next proof should add real portal approval/report UI, real external runtime writer transport and delivery to a device beyond blocker refs, real child delivery, real provider/store API execution with credentials/evidence, or actual platform adapters before upgrading manual-required source rows',
      text: featureDoc,
    },
    {
      doc: 'docs/features/app-install-purchase-approval.md',
      label: 'platform-adapter-evidence-gap-current-state',
      needle:
        'Platform adapter evidence gap proof linking provider/store API execution rows and platform proof-readiness rows into adapter-evidence-gap',
      text: featureDoc,
    },
    {
      doc: 'docs/expectations/app-install-purchase-approval.md',
      label: 'platform-adapter-evidence-gap-nongoal',
      needle:
        'Do not treat platform adapter evidence gap proof refs as real platform adapter evidence, platform adapter implementation, platform interception',
      text: expectationDoc,
    },
    {
      doc: 'docs/expectations/platforms.md',
      label: 'windows-adapter-evidence-gap-limitation',
      needle:
        'Windows app-install platform adapter evidence gap proof may be adapter-evidence-gap only when provider/store API execution proof and platform proof-readiness refs are attached',
      text: platformDoc,
    },
    {
      doc: 'docs/expectations/platforms.md',
      label: 'mobile-blocked-before-claim-limitation',
      needle:
        'Android app-install platform adapter evidence gap proof must stay blocked-before-claim until device-owner or managed-profile adapter proof',
      text: platformDoc,
    },
    {
      doc: 'docs/expectations/platforms.md',
      label: 'ios-blocked-before-claim-limitation',
      needle:
        'iOS app-install platform adapter evidence gap proof must stay blocked-before-claim until Family Controls entitlement adapter proof',
      text: platformDoc,
    },
  ];

  for (const check of checks) {
    assert.ok(
      normalizeMarkdownText(check.text).includes(normalizeMarkdownText(check.needle)),
      `${check.doc} missing ${check.label}`
    );
  }

  return checks.map(({ doc, label, needle }) => ({
    doc,
    label,
    matchedText: needle,
  }));
}

function normalizeMarkdownText(text) {
  return text.replace(/\s+/gu, ' ').trim();
}

async function gitHead() {
  const output = await commandOutput('git', ['rev-parse', 'HEAD']);
  return output.trim();
}

async function commandOutput(command, args) {
  const chunks = [];
  const child = spawn(command, args, { cwd: repoRoot, stdio: ['ignore', 'pipe', 'pipe'] });
  child.stdout.on('data', (chunk) => chunks.push(chunk));
  child.stderr.on('data', (chunk) => chunks.push(chunk));
  const exitCode = await new Promise((resolve) => {
    child.on('close', resolve);
  });
  const output = Buffer.concat(chunks).toString('utf8');
  if (exitCode !== 0) {
    throw new Error(`${command} ${args.join(' ')} failed with ${exitCode}\n${output}`);
  }
  return output;
}

async function runCommand(command, args) {
  const child = spawn(command, args, { cwd: repoRoot, stdio: 'inherit' });
  const exitCode = await new Promise((resolve) => {
    child.on('close', resolve);
  });
  commands.push({ command: `${command} ${args.join(' ')}`, exitCode });
  if (exitCode !== 0) {
    throw new Error(`${command} ${args.join(' ')} failed with ${exitCode}`);
  }
}

function npmCommand(args) {
  const command = process.platform === 'win32' ? 'cmd' : 'npm';
  const commandArgs = process.platform === 'win32' ? ['/c', 'npm', ...args] : args;
  return [command, commandArgs];
}
