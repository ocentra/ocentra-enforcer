import { execFileSync } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';

const root = process.cwd();
const proofName = 'browser-runtime-social-provider-receipt-ingestion-readiness-status-proof';
const resultDir = path.join(root, 'test-results', proofName);
const outputDir = path.join(
  root,
  'output',
  'browser-plan-proof',
  'browser-runtime-social-provider-receipt-ingestion-readiness-status'
);

const files = {
  portalStatus: path.join(
    root,
    'packages',
    'portal-domain',
    'src',
    'browser-social-provider-receipt-ingestion-readiness-status.ts'
  ),
  portalContracts: path.join(root, 'packages', 'portal-domain', 'src', 'contracts.ts'),
  portalTest: path.join(
    root,
    'packages',
    'portal-domain',
    'tests',
    'unit',
    'browser-social-provider-receipt-ingestion-readiness-status.test.ts'
  ),
  workpack: path.join(
    root,
    'docs',
    'plans',
    'browser-plan',
    'workpacks',
    '13-browser-read-models-and-service-events.md'
  ),
  checklist: path.join(root, 'docs', 'plans', 'browser-plan', 'implementation-checklist.md'),
  featureDoc: path.join(root, 'docs', 'features', 'browser-web-control.md'),
};

const commands = [
  {
    command: 'cmd',
    args: ['/c', 'npm', 'run', 'build', '--workspace', '@ocentra-parent/agent-protocol-domain'],
    label: 'cmd /c npm run build --workspace @ocentra-parent/agent-protocol-domain',
  },
  {
    command: 'cmd',
    args: [
      '/c',
      'npm',
      'run',
      'test',
      '--workspace',
      '@ocentra-parent/portal-domain',
      '--',
      'tests/unit/browser-social-provider-receipt-ingestion-readiness-status.test.ts',
    ],
    label:
      'cmd /c npm run test --workspace @ocentra-parent/portal-domain -- tests/unit/browser-social-provider-receipt-ingestion-readiness-status.test.ts',
  },
];

await main();

async function main() {
  await mkdir(resultDir, { recursive: true });
  await mkdir(outputDir, { recursive: true });

  const commandResults = commands.map(runCommand);
  const sourceChecks = await readSourceChecks();
  assertSourceChecks(sourceChecks);

  const proof = {
    proofName,
    branch: gitOutput(['rev-parse', '--abbrev-ref', 'HEAD']),
    commit: gitOutput(['rev-parse', '--short', 'HEAD']),
    commandResults,
    sourceChecks,
    verified: {
      portalDomainProjectsReceiptIngestionReadiness: true,
      providerDispatchRequiredRowsBecomeIngestionContractRequired: true,
      manualReceiptRequiredRowsStayManual: true,
      providerReceiptObservedCountRemainsZero: true,
      providerDeliveryClaimed: false,
      receiptIngestionRuntimeClaimed: false,
      webhookRuntimeClaimed: false,
      providerCredentialsClaimed: false,
      observedProviderReceiptsClaimed: false,
      parentNotificationDeliveryClaimed: false,
      reportDeliveryClaimed: false,
      finalPolicyExecutionClaimed: false,
      connectorNativeRuntimeClaimed: false,
      browserMutationClaimed: false,
      childInterventionClaimed: false,
      unmanagedExactUrlClaimed: false,
      enforcementClaimed: false,
    },
  };

  await writeFile(path.join(resultDir, 'proof.json'), `${JSON.stringify(proof, null, 2)}\n`);
  await writeFile(
    path.join(outputDir, '01-browser-runtime-social-provider-receipt-ingestion-readiness-status-proof.md'),
    markdownFor(proof)
  );

  console.log('browser-runtime-social-provider-receipt-ingestion-readiness-status-proof-ok=true');
  console.log(`proof=${relativePath(path.join(resultDir, 'proof.json'))}`);
}

async function readSourceChecks() {
  const [portalStatus, portalContracts, portalTest, workpack, checklist, featureDoc] = await Promise.all(
    Object.values(files).map((file) => readFile(file, 'utf8'))
  );
  return {
    portalStatusUsesDerivedProtocolStatus: portalStatus.includes(
      'deriveAgentBrowserRuntimeSocialProviderReceiptStatus'
    ),
    portalStatusMentionsIngestionContractRequired: portalStatus.includes('ingestion-contract-required'),
    portalStatusKeepsObservedProviderReceiptsUnclaimed: portalStatus.includes('0 provider receipts observed'),
    portalContractsExportsStatus: portalContracts.includes(
      'createBrowserSocialProviderReceiptIngestionReadinessStatusIntent'
    ),
    portalTestCoversDispatchRequiredReadiness: portalTest.includes('ingestion-contract-required'),
    portalTestCoversManualReceiptRequiredReadiness: portalTest.includes('manual-receipt-required'),
    workpackMentionsProof: workpack.includes('Social Provider Receipt Ingestion Readiness Stream Status Addendum'),
    checklistMentionsProof: checklist.includes(
      'browser-runtime-social-provider-receipt-ingestion-readiness-status-proof'
    ),
    featureDocMentionsReadinessProjection:
      featureDoc.includes('receipt ingestion readiness') && featureDoc.includes('observed provider receipts'),
  };
}

function assertSourceChecks(checks) {
  const missing = Object.entries(checks)
    .filter(([, ok]) => !ok)
    .map(([name]) => name);
  if (missing.length > 0) {
    throw new Error(`browser social provider receipt ingestion readiness status proof failed: ${missing.join(', ')}`);
  }
}

function runCommand(item) {
  execFileSync(item.command, item.args, {
    cwd: root,
    stdio: 'inherit',
    shell: process.platform === 'win32',
  });
  return {
    command: item.label,
    status: 'passed',
  };
}

function markdownFor(proof) {
  return (
    [
      '# Browser Runtime Social Provider Receipt Ingestion Readiness Status Proof',
      '',
      'This proof projects the already parsed browser runtime social provider receipt stream into a portal-domain receipt ingestion readiness status.',
      '',
      'Provider-dispatch-required receipt rows become ingestion-contract-required status because webhook contract, credential proof, durable receipt store proof, and observed provider receipt ingestion remain outside the current runtime. Manual receipt rows stay manual-required and carry no durable/provider refs.',
      '',
      'No-claim boundary: provider delivery, receipt ingestion runtime, webhook runtime, credentials, observed provider receipts, parent notification delivery, report delivery, final policy execution, connector/native runtime, browser mutation, child intervention, unmanaged exact URL support, and enforcement remain unclaimed.',
      '',
      'Validation:',
      ...proof.commandResults.map((result) => `- \`${result.command}\` (${result.status})`),
    ].join('\n') + '\n'
  );
}

function gitOutput(args) {
  return execFileSync('git', args, { cwd: root, encoding: 'utf8' }).trim();
}

function relativePath(targetPath) {
  return path.relative(root, targetPath).replaceAll('\\', '/');
}
