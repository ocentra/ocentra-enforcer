import { existsSync } from 'node:fs';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';

const root = process.cwd();
const outputDirectory = join(root, 'output', 'browser-plan-proof', 'social-24-rollout-manual-required-labels');
const resultDirectory = join(root, 'test-results', 'social-platform-account-feed-rollout-gate');
const checklistPath = 'docs/plans/browser-plan/implementation-checklist.md';
const requiredRolloutProofFiles = [
  'test-results/social-alert-report-intent-ui-proof/proof.json',
  'output/browser-plan-proof/social-alert-report-intent-ui-proof/07-rendered-alert-report-ui-proof.json',
  'output/browser-plan-proof/social-alert-report-intent-ui-proof/06-ui-snapshots/social-alert-report-browser-route.png',
  'output/browser-plan-proof/social-alert-report-intent-ui-proof/06-ui-snapshots/social-alert-report-browser-route-mobile.png',
  'test-results/social-alert-report-provider-preflight-proof/proof.json',
  'output/browser-plan-proof/social-alert-report-provider-preflight-proof/01-social-alert-report-provider-preflight-proof.md',
  'test-results/social-alert-report-provider-status-handoff-proof/proof.json',
  'output/browser-plan-proof/social-alert-report-provider-status-handoff-proof/01-social-alert-report-provider-status-handoff-proof.md',
  'test-results/social-alert-report-local-outbox-bridge-proof/proof.json',
  'test-results/social-alert-report-local-outbox-bridge-proof/local-outbox-records.jsonl',
  'output/browser-plan-proof/social-alert-report-local-outbox-bridge-proof/01-social-alert-report-local-outbox-bridge-proof.md',
  'test-results/social-alert-report-parent-surface-service-ui-proof/proof.json',
  'test-results/social-alert-report-parent-surface-service-ui-proof/accessibility-summary.json',
  'output/browser-plan-proof/social-alert-report-parent-surface-service-ui-proof/01-social-alert-report-parent-surface-service-ui-proof.md',
  'output/browser-plan-proof/social-alert-report-parent-surface-service-ui-proof/06-ui-snapshots/social-alert-report-browser-route.png',
  'output/browser-plan-proof/social-alert-report-parent-surface-service-ui-proof/06-ui-snapshots/social-alert-report-browser-route-mobile.png',
  'test-results/social-alert-report-scheduler-bridge-proof/proof.json',
  'test-results/social-alert-report-scheduler-bridge-proof/scheduler-records.jsonl',
  'output/browser-plan-proof/social-alert-report-scheduler-bridge-proof/01-social-alert-report-scheduler-bridge-proof.md',
  'test-results/social-alert-report-preference-preflight-proof/proof.json',
  'test-results/social-alert-report-preference-preflight-proof/preference-preflight-read-model.json',
  'output/browser-plan-proof/social-alert-report-preference-preflight-proof/01-social-alert-report-preference-preflight-proof.md',
  'test-results/social-alert-report-preference-status-handoff-proof/proof.json',
  'test-results/social-alert-report-preference-status-handoff-proof/preference-status-handoff-read-model.json',
  'output/browser-plan-proof/social-alert-report-preference-status-handoff-proof/01-social-alert-report-preference-status-handoff-proof.md',
  'test-results/social-alert-report-audit-history-bridge-proof/proof.json',
  'test-results/social-alert-report-audit-history-bridge-proof/audit-history-handoff.json',
  'output/browser-plan-proof/social-alert-report-audit-history-bridge-proof/01-social-alert-report-audit-history-bridge-proof.md',
  'test-results/social-parent-notification-delivery-readiness-proof/proof.json',
  'output/browser-plan-proof/social-parent-notification-delivery-readiness-proof/01-social-parent-notification-delivery-readiness-proof.md',
  'test-results/social-managed-browser-policy-execution-proof/proof.json',
  'output/browser-plan-proof/social-managed-browser-policy-execution-proof/01-social-managed-browser-policy-execution-proof.md',
  'test-results/social-alert-report-provider-dispatch-execution-proof/proof.json',
  'output/browser-plan-proof/social-alert-report-provider-dispatch-execution-proof/01-social-alert-report-provider-dispatch-execution-proof.md',
];

const rolloutGuards = [
  {
    docKey: 'plan',
    text: 'SOCIAL rollout state: partial/manual-required.',
  },
  {
    docKey: 'readme',
    text: 'SOCIAL rollout state: partial/manual-required.',
  },
  {
    docKey: 'feature',
    text: 'Product completion remains unclaimed;',
  },
  {
    docKey: 'browserFeature',
    text: 'Product checklist upgrade is not claimed.',
  },
  {
    docKey: 'expectation',
    text: 'Rollout/manual-required gates may label rows as partial/manual-required only.',
  },
];

if (!existsSync(join(root, checklistPath))) {
  throw new Error(`Missing checklist: ${checklistPath}`);
}

await main();

async function main() {
  await mkdir(outputDirectory, { recursive: true });
  await mkdir(resultDirectory, { recursive: true });

  const docs = await loadDocs();
  const rows = expectedRows().map((row) => validateChecklistRow(row, docs.checklist));
  const guardFailures = validateRolloutGuards(docs);
  const proofFailures = validateRolloutProofFiles();
  const failures = [...rows.flatMap((row) => row.failures), ...guardFailures, ...proofFailures];
  const manifest = manifestFor(rows, failures);

  if (manifest.failures.length > 0) {
    throw new Error(`Social platform account/feed rollout gate failed:\n${manifest.failures.join('\n')}`);
  }

  const proofPath = join(resultDirectory, 'proof.json');
  const markdownPath = join(outputDirectory, '01-rollout-manual-required-labels.md');
  await writeFile(proofPath, `${JSON.stringify(manifest, null, 2)}\n`);
  await writeFile(markdownPath, `${markdownFor(manifest)}\n`);

  console.log('social-platform-account-feed-rollout-gate-ok=true');
  console.log(`proof=${relativePath(proofPath)}`);
  console.log(`manifest=${relativePath(markdownPath)}`);
  console.log(`complete=${manifest.summary.completeRows} partial=${manifest.summary.partialRows}`);
}

async function loadDocs() {
  return {
    checklist: await readText(checklistPath),
    plan: await readText('docs/plans/browser-plan/v0-5-social-platform-account-feed-gating-plan.md'),
    readme: await readText('docs/plans/browser-plan/social-platform-account-feed/readme.md'),
    feature: await readText('docs/features/social-video-control.md'),
    browserFeature: await readText('docs/features/browser-web-control.md'),
    expectation: await readText('docs/expectations/social-video-control.md'),
  };
}

function expectedRows() {
  const completeRows = new Set([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 18, 19, 20, 21, 22]);
  return Array.from({ length: 23 }, (_, index) => {
    const rowNumber = index + 1;
    const complete = completeRows.has(rowNumber);
    return {
      rowNumber,
      rowId: `SOCIAL-${String(rowNumber).padStart(2, '0')}`,
      expectedStatus: complete ? '[x]' : '[~]',
      expectedState: complete ? 'proof-present' : 'partial-manual-required',
    };
  });
}

function validateChecklistRow(row, checklist) {
  const rowText = checklistRowText(row.rowId, checklist);
  const failures = [];
  if (!rowText) {
    failures.push(`${row.rowId} missing checklist row`);
    return { ...row, failures };
  }
  if (!rowText.includes(row.expectedStatus)) {
    failures.push(`${row.rowId} status is not ${row.expectedStatus}`);
  }
  if (!rowText.includes('codex-d')) {
    failures.push(`${row.rowId} owner is not codex-d`);
  }
  return {
    ...row,
    state: row.expectedState,
    failures,
  };
}

function validateRolloutGuards(docs) {
  return rolloutGuards
    .filter((guard) => !docs[guard.docKey].includes(guard.text))
    .map((guard) => `${guard.docKey} missing rollout guard: ${guard.text}`);
}

function manifestFor(rows, failures) {
  return {
    schemaVersion: 1,
    proofMode: 'social-platform-account-feed-rollout-gate',
    generatedAt: new Date().toISOString(),
    rows,
    summary: {
      totalRows: rows.length,
      completeRows: rows.filter((row) => row.expectedState === 'proof-present').length,
      partialRows: rows.filter((row) => row.expectedState === 'partial-manual-required').length,
      failures: failures.length,
      rolloutState: 'partial/manual-required',
      productClaimed: false,
    },
    guardTexts: rolloutGuards.map((guard) => guard.text),
    noClaimLabels: [
      'rendered-proof-bundle-social-ui-present',
      'service-backed-dashboard-and-explanation-read-model-proof-present',
      'social-live-platform-route-boundary-proof-present',
      'social-live-url-pattern-boundary-proof-present',
      'social-live-account-flow-boundary-proof-present',
      'social-live-form-shape-boundary-proof-present',
      'social-live-identity-registry-boundary-proof-present',
      'social-live-parent-approval-boundary-proof-present',
      'social-live-route-classification-proof-present',
      'social-live-metadata-extraction-proof-present',
      'social-live-evidence-ai-boundary-proof-present',
      'social-live-evidence-risk-benefit-boundary-proof-present',
      'social-live-evidence-policy-compiler-proof-present',
      'social-live-public-connector-boundary-proof-present',
      'social-live-evidence-decision-memory-proof-present',
      'social-alert-report-intent-proof-present',
      'social-alert-report-local-outbox-bridge-proof-present',
      'social-alert-report-parent-surface-service-ui-proof-present',
      'social-alert-report-scheduler-bridge-proof-present',
      'social-alert-report-preference-preflight-proof-present',
      'social-alert-report-preference-status-handoff-proof-present',
      'social-alert-report-audit-history-bridge-proof-present',
      'social-alert-report-intent-ui-proof-present',
      'social-alert-report-provider-preflight-proof-present',
      'social-alert-report-provider-status-handoff-proof-present',
      'social-report-writer-delivery-proof-present',
      'social-parent-notification-delivery-readiness-proof-present',
      'social-parent-local-delivery-result-proof-present',
      'social-applied-schedule-time-budget-proof-present',
      'social-schedule-time-budget-compiler-proof-present',
      'social-parent-sensitivity-settings-proof-present',
      'social-source-custody-settings-proof-present',
      'social-source-custody-mutation-proof-present',
      'external-runtime-report-delivery-not-claimed',
      'parent-notification-ui-delivery-not-claimed',
      'provider-delivery-not-claimed',
      'runtime-applied-schedule-time-budget-not-claimed',
      'connector-native-runtime-not-claimed',
      'final-policy-execution-not-claimed',
      'enforcement-not-claimed',
      'product-checklist-upgrade-not-claimed',
    ],
    requiredRolloutProofFiles: requiredRolloutProofFiles.map((file) => ({
      file,
      present: existsSync(join(root, file)),
    })),
    failures,
  };
}

function markdownFor(manifest) {
  const rows = manifest.rows.map((row) => `| ${row.rowId} | ${row.state} | ${row.expectedStatus} |`).join('\n');
  return [
    '# SOCIAL-24 Rollout Manual-Required Labels',
    '',
    `Generated: ${manifest.generatedAt}`,
    '',
    `Rows checked: ${manifest.summary.totalRows}`,
    `Proof-present rows: ${manifest.summary.completeRows}`,
    `Partial/manual-required rows: ${manifest.summary.partialRows}`,
    `Rollout state: ${manifest.summary.rolloutState}`,
    `Product claimed: ${manifest.summary.productClaimed}`,
    '',
    '| Row | State | Checklist Status |',
    '| --- | --- | --- |',
    rows,
    '',
    'SOCIAL rollout state: partial/manual-required.',
    'Product checklist upgrade is not claimed.',
    'Rendered proof-bundle social UI exists for dashboard, child intervention,',
    'and parent explanation states. Service-backed dashboard and explanation',
    'read-model delivery is present. Live SOCIAL-02 platform route boundary',
    'proof is present. Live SOCIAL-03 URL pattern proof is present.',
    'Live SOCIAL-04 account-flow proof is present.',
    'Live SOCIAL-05 form-shape proof is present.',
    'Live SOCIAL-06 identity registry proof is present.',
    'Live SOCIAL-07 parent approval contract proof is present.',
    'Live SOCIAL-08 route classification proof is present.',
    'Live SOCIAL-09 metadata extraction proof is present.',
    'Live SOCIAL-10 evidence-bound AI degradation proof is present.',
    'Live SOCIAL-11 evidence-bound risk/benefit degradation proof is present.',
    'Live SOCIAL-12 evidence-bound policy compiler proof is present.',
    'Live SOCIAL-18 public connector boundary proof is present.',
    'Live SOCIAL-19 evidence-bound decision memory proof is present.',
    'Ref-only social alert/report intent proof is present.',
    'Parent-owned social alert/report local outbox JSONL bridge proof is present.',
    'Service-backed social alert/report parent-surface UI proof is present for the real Browser route.',
    'Parent-owned social alert/report scheduler JSONL bridge proof is present.',
    'Social alert/report audit-history bridge proof is present through the logging-domain handoff.',
    'Service-backed social alert/report intent UI proof is present for the real Browser route.',
    'Social alert/report provider preflight proof is present and requires provider adapter setup before delivery.',
    'Social alert/report provider status handoff proof is present and maps preflight rows to manual-required or unavailable provider status boundary rows.',
    'Parent-owned report writer delivery-readiness proof is present.',
    'Parent notification/report delivery readiness proof is present.',
    'Parent-owned local delivery result proof is present for report-ready rows.',
    'Managed-browser social policy execution proof is present for a real YouTube block intervention.',
    'Social alert/report provider dispatch execution proof prepares local dispatch packets without claiming provider delivery.',
    'Parent-owned schedule/time-budget application-readiness proof is present.',
    'Schedule/time-budget compiler proof and parent sensitivity',
    'settings proof are present. Source custody settings proof is present over',
    'source/privacy refs, and service-backed source custody mutation proof is',
    'present. Connector/native runtime, external provider delivery, parent',
    'notification UI delivery, runtime-applied schedules/budgets, broad or',
    'unmanaged policy execution, and enforcement remain unclaimed.',
  ].join('\n');
}

function validateRolloutProofFiles() {
  return requiredRolloutProofFiles
    .filter((file) => !existsSync(join(root, file)))
    .map((file) => `SOCIAL-24 missing rollout proof file ${file}`);
}

function checklistRowText(rowId, checklist) {
  return checklist.split(/\r?\n/).find((line) => line.startsWith(`| ${rowId} |`));
}

async function readText(path) {
  return readFile(join(root, path), 'utf8');
}

function relativePath(path) {
  return relative(root, path).replaceAll('\\', '/');
}
