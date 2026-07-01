import { execFileSync } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';

const root = process.cwd();
const proofName = 'browser-runtime-social-provider-receipt-live-activity-state-proof';
const resultDir = path.join(root, 'test-results', proofName);
const outputDir = path.join(
  root,
  'output',
  'browser-plan-proof',
  'browser-runtime-social-provider-receipt-live-activity-state'
);

const files = {
  liveActivityState: path.join(root, 'apps', 'portal', 'src', 'live-activity-state.ts'),
  liveActivityTest: path.join(root, 'apps', 'portal', 'tests', 'live-activity-state.test.ts'),
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
    args: ['/c', 'npm', 'run', 'build', '--workspace', '@ocentra-parent/portal'],
    label: 'cmd /c npm run build --workspace @ocentra-parent/portal',
  },
  {
    command: 'cmd',
    args: [
      '/c',
      'npm',
      'run',
      'test',
      '--workspace',
      '@ocentra-parent/portal',
      '--',
      'tests/live-activity-state.test.ts',
    ],
    label: 'cmd /c npm run test --workspace @ocentra-parent/portal -- tests/live-activity-state.test.ts',
  },
  {
    command: 'cmd',
    args: ['/c', 'npm', 'run', 'type-check', '--workspace', '@ocentra-parent/portal'],
    label: 'cmd /c npm run type-check --workspace @ocentra-parent/portal',
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
      liveActivityStateDerivesReceiptStreamStatus: true,
      liveActivityStateDerivesReceiptIngestionReadinessStatus: true,
      liveActivityStateRejectsDishonestReceiptRowsBeforeProjection: true,
      sharedParserStillOwnsRawStreamParsing: true,
      portalVisualSurfaceChanged: false,
      providerDeliveryClaimed: false,
      receiptIngestionRuntimeClaimed: false,
      webhookRuntimeClaimed: false,
      providerCredentialsClaimed: false,
      observedProviderReceiptsClaimed: false,
      reportDeliveryClaimed: false,
      finalPolicyExecutionClaimed: false,
      browserMutationClaimed: false,
      childInterventionClaimed: false,
      unmanagedExactUrlClaimed: false,
      enforcementClaimed: false,
    },
  };

  await writeFile(path.join(resultDir, 'proof.json'), `${JSON.stringify(proof, null, 2)}\n`);
  await writeFile(
    path.join(outputDir, '01-browser-runtime-social-provider-receipt-live-activity-state-proof.md'),
    markdownFor(proof)
  );

  console.log('browser-runtime-social-provider-receipt-live-activity-state-proof-ok=true');
  console.log(`proof=${relativePath(path.join(resultDir, 'proof.json'))}`);
}

async function readSourceChecks() {
  const [liveActivityWrapper, liveActivityTest, workpack, checklist, featureDoc] = await Promise.all(
    Object.values(files).map((file) => readFile(file, 'utf8'))
  );
  return {
    appWrapperDelegatesToPortalDomainOwner: liveActivityWrapper.includes('resolvePortalDomainLiveActivityState'),
    appWrapperDelegatesReceiptProjection: liveActivityWrapper.includes(
      'return resolvePortalDomainLiveActivityState(events);'
    ),
    appWrapperExportsPortalDomainState: liveActivityWrapper.includes(
      'export type PortalLiveActivityState = PortalDomainPortalLiveActivityState;'
    ),
    appWrapperDoesNotParseReceiptFieldsDirectly: !liveActivityWrapper.includes(
      'BrowserRuntimeSocialProviderReceiptBoundaryRows'
    ),
    testCoversReceiptStreamProjection: liveActivityTest.includes('browserSocialProviderReceiptStreamStatusIntent'),
    testCoversReadinessProjection: liveActivityTest.includes(
      'browserSocialProviderReceiptIngestionReadinessStatusIntent'
    ),
    testRejectsDishonestReceiptRows: liveActivityTest.includes('dishonest social provider receipt stream rows'),
    workpackMentionsProof: workpack.includes('Social Provider Receipt Live Activity State Addendum'),
    checklistMentionsProof: checklist.includes('browser-runtime-social-provider-receipt-live-activity-state-proof'),
    featureDocMentionsLiveActivityState:
      featureDoc.includes('portal live activity') &&
      featureDoc.includes('app state') &&
      featureDoc.includes('receipt ingestion readiness'),
  };
}

function assertSourceChecks(checks) {
  const missing = Object.entries(checks)
    .filter(([, ok]) => !ok)
    .map(([name]) => name);
  if (missing.length > 0) {
    throw new Error(`browser social provider receipt live activity state proof failed: ${missing.join(', ')}`);
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
      '# Browser Runtime Social Provider Receipt Live Activity State Proof',
      '',
      'This proof carries parsed browser runtime social provider receipt stream status and receipt ingestion readiness status into the portal-domain live-activity owner through the app wrapper consumer.',
      '',
      'The current app live-activity wrapper delegates to the shared portal-domain resolver. It does not parse raw receipt stream fields directly and rejects dishonest receipt rows before either parent-visible status is populated.',
      '',
      'No-claim boundary: provider delivery, receipt ingestion runtime, webhook runtime, credentials, observed provider receipts, report delivery, final policy execution, browser mutation, child intervention, unmanaged exact URL support, and enforcement remain unclaimed.',
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
