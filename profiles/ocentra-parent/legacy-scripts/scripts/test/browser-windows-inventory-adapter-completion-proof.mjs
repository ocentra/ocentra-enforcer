import { existsSync } from 'node:fs';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';

const root = process.cwd();
const outputDirectory = join(root, 'output', 'browser-plan-proof', '04-windows-browser-inventory-adapter');
const resultDirectory = join(root, 'test-results', 'browser-windows-inventory-adapter-completion-proof');
const proofPath = join(resultDirectory, 'proof.json');
const manifestPath = join(outputDirectory, '11-completion-proof-gate.md');

await main();

async function main() {
  const liveInventory = await readJson(
    join(root, 'test-results', 'browser-windows-live-inventory-proof', 'proof.json')
  );
  const portalInventory = await readJson(
    join(
      root,
      'output',
      'browser-plan-proof',
      '14-portal-browser-status-surfaces',
      '06-ui-snapshots',
      'browser-route-inventory-status.json'
    )
  );
  const appControlProof = await readJson(join(root, 'test-results', 'v0-8-browser-domain-adapter-proof', 'proof.json'));
  const workpack = await readText(
    join(root, 'docs', 'plans', 'browser-plan', 'workpacks', '04-windows-browser-inventory-adapter.md')
  );

  const proofFiles = requiredProofFiles().map((path) => ({
    path,
    exists: existsSync(join(root, path)),
  }));
  const checks = [
    checkProofFiles(proofFiles),
    checkLiveInventory(liveInventory),
    checkPortalConsumption(portalInventory),
    checkAppControlNoClaim(appControlProof),
    checkWorkpackBoundary(workpack),
  ];
  const failures = checks.flatMap((check) => check.failures);
  const proof = {
    schemaVersion: 1,
    proofMode: 'browser-windows-inventory-adapter-completion-proof',
    generatedAt: new Date().toISOString(),
    sourceWorkpack: 'docs/plans/browser-plan/workpacks/04-windows-browser-inventory-adapter.md',
    evidenceWorkpacks: [
      'docs/plans/browser-plan/workpacks/14-portal-browser-status-surfaces.md',
      'docs/plans/browser-plan/workpacks/20-windows-applocker-app-control-proof.md',
    ],
    summary: {
      status: failures.length === 0 ? 'complete-with-no-claim-boundaries' : 'failed',
      proofFilesChecked: proofFiles.length,
      checksPassed: checks.filter((check) => check.failures.length === 0).length,
      failures: failures.length,
      productChecklistUpgradeClaimed: false,
      exactUrlClaimed: false,
      appControlPreventionClaimed: false,
      appControlPolicyMutationClaimed: false,
    },
    checks,
    proofFiles,
    noClaimBoundaries: [
      'windows-inventory-detects-browsers-without-url-or-tab-capture',
      'portal-consumes-inventory-read-model-with-not-claimed-exact-url-and-active-tab-labels',
      'app-control-state-artifacts-remain-representation-only',
      'real-applocker-wdac-apply-rollback-and-launch-prevention-remain-unclaimed',
      'cross-platform-non-windows-adapters-remain-in-wp05',
    ],
    failures,
  };

  if (failures.length > 0) {
    throw new Error(`Windows browser inventory adapter completion proof failed:\n${failures.join('\n')}`);
  }

  await mkdir(outputDirectory, { recursive: true });
  await mkdir(resultDirectory, { recursive: true });
  await writeFile(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
  await writeFile(manifestPath, `${markdownFor(proof)}\n`);

  console.log('browser-windows-inventory-adapter-completion-proof-ok=true');
  console.log(`proof=${relativePath(proofPath)}`);
  console.log(`manifest=${relativePath(manifestPath)}`);
  console.log(`checks=${proof.summary.checksPassed} proofFiles=${proof.summary.proofFilesChecked}`);
}

function requiredProofFiles() {
  return [
    'output/browser-plan-proof/04-windows-browser-inventory-adapter/00-source-snapshot.md',
    'output/browser-plan-proof/04-windows-browser-inventory-adapter/01-contract-proof.log',
    'output/browser-plan-proof/04-windows-browser-inventory-adapter/02-rust-protocol-proof.log',
    'output/browser-plan-proof/04-windows-browser-inventory-adapter/03-runtime-evidence.json',
    'output/browser-plan-proof/04-windows-browser-inventory-adapter/08-security-negative-proof.log',
    'output/browser-plan-proof/04-windows-browser-inventory-adapter/09-manual-platform-proof.md',
    'output/browser-plan-proof/04-windows-browser-inventory-adapter/10-validation-commands.log',
    'test-results/browser-windows-live-inventory-proof/proof.json',
    'output/browser-plan-proof/14-portal-browser-status-surfaces/06-ui-snapshots/browser-route-inventory-status.json',
    'output/browser-plan-proof/14-portal-browser-status-surfaces/06-ui-snapshots/browser-route-inventory-status.png',
    'test-results/v0-8-browser-domain-adapter-proof/proof.json',
  ];
}

function checkProofFiles(proofFiles) {
  return {
    id: 'required-proof-files-exist',
    status: proofFiles.every((file) => file.exists) ? 'pass' : 'fail',
    failures: proofFiles.filter((file) => !file.exists).map((file) => `missing proof artifact: ${file.path}`),
  };
}

function checkLiveInventory(proof) {
  const rows = proof.rows ?? [];
  const summary = proof.summary ?? {};
  const sourceKinds = new Set(rows.flatMap((row) => row.sourceKinds ?? []));
  const failures = [];
  for (const sourceKind of [
    'known-path',
    'registry-uninstall',
    'start-menu-shortcut',
    'store-package',
    'running-process',
  ]) {
    if (!sourceKinds.has(sourceKind)) {
      failures.push(`live Windows inventory proof missing source kind: ${sourceKind}`);
    }
  }
  if ((summary.totalRows ?? rows.length) < 3) {
    failures.push('live Windows inventory proof has fewer than three rows');
  }
  if ((summary.exactUrlClaimedRows ?? 0) !== 0) {
    failures.push('live Windows inventory proof unexpectedly claims exact URL rows');
  }
  if (JSON.stringify(proof).includes('block-enforcement') === false) {
    failures.push('live Windows inventory proof missing explicit block-enforcement no-claim labels');
  }
  return {
    id: 'live-windows-inventory-evidence',
    status: failures.length === 0 ? 'pass' : 'fail',
    failures,
  };
}

function checkPortalConsumption(artifact) {
  const text = JSON.stringify(artifact);
  const requiredTokens = ['Browser inventory', 'Exact URL capability', 'Active tab proof', 'not-claimed'];
  return {
    id: 'portal-inventory-read-model-consumption',
    status: requiredTokens.every((token) => text.includes(token)) ? 'pass' : 'fail',
    failures: requiredTokens
      .filter((token) => !text.includes(token))
      .map((token) => `portal inventory artifact missing token: ${token}`),
  };
}

function checkAppControlNoClaim(proof) {
  const counts = proof.counts ?? {};
  const labels = new Set(proof.proofLabels ?? []);
  const failures = [];
  for (const label of [
    'v0.8.windows-app-control.state-representation',
    'v0.8.browser-domain-adapter.no-claim-upgrade',
  ]) {
    if (!labels.has(label)) {
      failures.push(`app-control proof missing label: ${label}`);
    }
  }
  for (const [key, value] of Object.entries({
    appControlPreventionClaimed: 0,
    appControlPolicyCreationClaimed: 0,
    appControlPolicyUpdateClaimed: 0,
    appControlRollbackClaimed: 0,
    managedExactUrlClaimed: 0,
    unmanagedExactUrlClaimed: 0,
    broadBrowserControlClaimed: 0,
  })) {
    if (counts[key] !== value) {
      failures.push(`app-control proof has unexpected ${key}: ${counts[key]}`);
    }
  }
  return {
    id: 'app-control-state-artifacts-keep-no-claim-boundary',
    status: failures.length === 0 ? 'pass' : 'fail',
    failures,
  };
}

function checkWorkpackBoundary(workpack) {
  const requiredTokens = [
    'exact URL/tab',
    'browser content',
    'blocking',
    'rollback',
    'enforcement',
    'complete with no-claim boundaries',
  ];
  return {
    id: 'workpack-records-no-claim-boundary',
    status: requiredTokens.every((token) => workpack.includes(token)) ? 'pass' : 'fail',
    failures: requiredTokens
      .filter((token) => !workpack.includes(token))
      .map((token) => `workpack missing boundary token: ${token}`),
  };
}

function markdownFor(proof) {
  const checkRows = proof.checks
    .map((check) => `| ${check.id} | ${check.status} | ${check.failures.length} |`)
    .join('\n');
  const fileRows = proof.proofFiles.map((file) => `| ${file.path} | ${file.exists ? 'yes' : 'no'} |`).join('\n');
  return [
    '# WP04 Windows Browser Inventory Adapter Completion Proof Gate',
    '',
    `Generated: ${proof.generatedAt}`,
    '',
    `Status: ${proof.summary.status}`,
    `Product checklist upgrade claimed: ${proof.summary.productChecklistUpgradeClaimed}`,
    '',
    'This gate completes the Windows inventory adapter row by verifying live Windows inventory evidence, Browser-route read-model consumption, and AppLocker/App Control state artifacts while preserving no-claim boundaries. It does not claim real AppLocker/WDAC policy creation, apply, rollback execution, launch prevention, exact URL capture, active-tab capture, browser content capture, or enforcement.',
    '',
    '## Checks',
    '',
    '| Check | Status | Failures |',
    '| --- | --- | --- |',
    checkRows,
    '',
    '## Proof Files',
    '',
    '| File | Exists |',
    '| --- | --- |',
    fileRows,
  ].join('\n');
}

async function readJson(path) {
  return JSON.parse(await readText(path));
}

async function readText(path) {
  return readFile(path, 'utf8');
}

function relativePath(path) {
  return relative(root, path).replaceAll('\\', '/');
}
