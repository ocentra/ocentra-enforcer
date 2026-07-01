import { execFileSync } from 'node:child_process';
import { existsSync } from 'node:fs';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';

const root = process.cwd();
const checklistPath = join(root, 'docs', 'plans', 'browser-plan', 'implementation-checklist.md');
const outputDirectory = join(root, 'output', 'browser-plan-proof', 'browser-plan-closure-audit');
const resultDirectory = join(root, 'test-results', 'browser-plan-closure-audit-proof');
const proofPath = join(resultDirectory, 'proof.json');
const manifestPath = join(outputDirectory, '01-browser-plan-closure-audit.md');

const expectedPartialRows = new Map([
  [
    '05',
    {
      reason: 'cross-platform-inventory-real-platform-proof-required',
      requiredEvidence: 'macOS desktop browser proof and iOS device/entitlement proof',
    },
  ],
  [
    'SOCIAL-17',
    {
      reason: 'ios-screentime-managedsettings-real-device-proof-required',
      requiredEvidence:
        'macOS/Xcode host, FamilyControls entitlement evidence, attached physical iOS device, token selection, DeviceActivity, and ManagedSettings proof',
    },
  ],
  [
    'SOCIAL-23',
    {
      reason: 'social-proof-artifact-gate-waits-on-social-17',
      requiredEvidence: 'SOCIAL-17 real iOS proof plus connector/native/provider runtime proof before product claims',
    },
  ],
  [
    'SOCIAL-24',
    {
      reason: 'social-rollout-gate-waits-on-social-17-and-social-23',
      requiredEvidence:
        'SOCIAL-17, SOCIAL-23, provider/report delivery, runtime custody mutation, broad/unmanaged policy execution boundaries, and enforcement proof',
    },
  ],
]);

const proofArtifacts = [
  {
    key: 'wp05',
    path: 'test-results/browser-platform-inventory-matrix-proof/proof.json',
    expectations: {
      productClaimed: false,
      failures: 0,
      manualRequiredRows: 2,
      unsupportedRows: 6,
      androidDeviceOwnerEnrollmentObserved: true,
      androidDeviceOwnerProofLimitedToProofLaunchedEmulator: true,
      androidDeviceOwnerPolicyMutationObserved: true,
      androidOwnedBrowserRoutingEnforcementObserved: true,
      androidEnforcementClaimed: false,
    },
  },
  {
    key: 'social17',
    path: 'test-results/social-ios-screen-time-host-proof/proof.json',
    expectations: {
      isDarwinHost: false,
      appleToolingAvailable: false,
      attachedDeviceCount: 0,
      resultState: 'host-tooling-unavailable',
    },
  },
  {
    key: 'social23',
    path: 'test-results/social-platform-account-feed-proof-artifacts/proof.json',
    expectations: {
      productClaimed: false,
      failures: 0,
      completeRows: 21,
      partialRows: 1,
    },
  },
  {
    key: 'social24',
    path: 'test-results/social-platform-account-feed-rollout-gate/proof.json',
    expectations: {
      productClaimed: false,
      failures: 0,
      completeRows: 21,
      partialRows: 2,
      rolloutState: 'partial/manual-required',
    },
  },
];

await main();

async function main() {
  const checklist = await readFile(checklistPath, 'utf8');
  const rows = parseChecklistRows(checklist);
  const partialRows = rows.filter((row) => row.status === '[~]');
  const openRows = rows.filter((row) => row.status === '[ ]');
  const unexpectedPartialRows = partialRows.filter((row) => !expectedPartialRows.has(row.id));
  const missingExpectedPartialRows = [...expectedPartialRows.keys()].filter(
    (id) => !partialRows.some((row) => row.id === id)
  );
  const artifactResults = await Promise.all(proofArtifacts.map(readProofArtifact));

  const failures = [
    ...openRows.map((row) => `${row.id} remains unchecked`),
    ...unexpectedPartialRows.map((row) => `${row.id} is partial but is not in the expected blocker list`),
    ...missingExpectedPartialRows.map((id) => `${id} is not marked partial/manual-required as expected`),
    ...artifactResults.flatMap((artifact) => artifact.failures),
  ];

  const proof = {
    schemaVersion: 1,
    proofMode: 'browser-plan-closure-audit-proof',
    generatedAt: new Date().toISOString(),
    branch: git(['rev-parse', '--abbrev-ref', 'HEAD']),
    sourceCommitAtGeneration: git(['rev-parse', 'HEAD']),
    baseCommit: git(['rev-parse', 'origin/main']),
    checklist: relativePath(checklistPath),
    summary: {
      checklistRows: rows.length,
      completeRows: rows.filter((row) => row.status === '[x]').length,
      partialRows: partialRows.length,
      uncheckedRows: openRows.length,
      expectedPartialRows: expectedPartialRows.size,
      failures: failures.length,
      planCompleteClaimed: false,
      prReadyClaimed: false,
    },
    partialRows: partialRows.map((row) => ({
      id: row.id,
      title: row.title,
      blocker: expectedPartialRows.get(row.id),
    })),
    proofArtifacts: artifactResults,
    noClaimLabels: [
      'browser-plan-completion-not-claimed',
      'product-checklist-upgrade-not-claimed',
      'macos-desktop-browser-proof-required',
      'android-implicit-routing-enforcement-proof-required',
      'android-broad-content-filter-enforcement-not-claimed',
      'ios-familycontrols-managedsettings-device-proof-required',
      'social-provider-report-delivery-not-claimed',
      'social-final-policy-execution-not-claimed',
      'social-enforcement-not-claimed',
    ],
    failures,
  };

  if (failures.length > 0) {
    throw new Error(`Browser plan closure audit failed:\n${failures.join('\n')}`);
  }

  await mkdir(outputDirectory, { recursive: true });
  await mkdir(resultDirectory, { recursive: true });
  await writeFile(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
  await writeFile(manifestPath, `${markdownFor(proof)}\n`);

  console.log('browser-plan-closure-audit-proof-ok=true');
  console.log(`proof=${relativePath(proofPath)}`);
  console.log(`manifest=${relativePath(manifestPath)}`);
  console.log(
    `complete=${proof.summary.completeRows} partial=${proof.summary.partialRows} unchecked=${proof.summary.uncheckedRows}`
  );
}

function parseChecklistRows(checklist) {
  return checklist
    .split(/\r?\n/)
    .filter((line) => /^\| (?:\d\d|AI-\d\d|SOCIAL-\d\d|GAME-\d\d) /.test(line))
    .map((line) => {
      const cells = line
        .slice(1, -1)
        .split('|')
        .map((cell) => cell.trim());
      return {
        id: cells[0],
        title: cells[1].replace(/\[([^\]]+)\]\([^)]+\)/, '$1'),
        status: cells[2],
      };
    });
}

async function readProofArtifact(artifact) {
  const absolutePath = join(root, artifact.path);
  if (!existsSync(absolutePath)) {
    return {
      key: artifact.key,
      path: artifact.path,
      exists: false,
      failures: [`${artifact.key} missing proof artifact ${artifact.path}`],
    };
  }

  const proof = JSON.parse(await readFile(absolutePath, 'utf8'));
  const observed = observedValues(artifact.key, proof);
  const failures = Object.entries(artifact.expectations)
    .filter(([key, expected]) => observed[key] !== expected)
    .map(([key, expected]) => `${artifact.key}.${key} expected ${expected} but saw ${observed[key]}`);

  return {
    key: artifact.key,
    path: artifact.path,
    exists: true,
    observed,
    expectations: artifact.expectations,
    failures,
  };
}

function observedValues(key, proof) {
  if (key === 'wp05') {
    return {
      productClaimed: proof.summary?.productClaimed,
      failures: proof.summary?.failures,
      manualRequiredRows: proof.summary?.manualRequiredRows,
      unsupportedRows: proof.summary?.unsupportedRows,
      androidDeviceOwnerEnrollmentObserved: proof.androidOwnedShellProof?.deviceOwnerEnrollmentObserved === true,
      androidDeviceOwnerProofLimitedToProofLaunchedEmulator:
        proof.androidOwnedShellProof?.deviceOwnerProofLimitedToProofLaunchedEmulator === true,
      androidDeviceOwnerPolicyMutationObserved:
        proof.androidOwnedShellProof?.deviceOwnerPolicyMutationObserved === true,
      androidOwnedBrowserRoutingEnforcementObserved:
        proof.androidOwnedShellProof?.androidOwnedBrowserRoutingEnforcementObserved === true,
      androidEnforcementClaimed: proof.androidOwnedShellProof?.enforcementClaimed === true,
    };
  }
  if (key === 'social17') {
    return {
      isDarwinHost: proof.hostProofSummary?.host?.isDarwinHost,
      appleToolingAvailable: proof.hostProofSummary?.appleToolingAvailable,
      attachedDeviceCount: proof.hostProofSummary?.attachedDeviceCount,
      resultState: proof.hostProofSummary?.resultState,
    };
  }
  if (key === 'social23') {
    return {
      productClaimed: proof.summary?.productClaimed,
      failures: proof.summary?.failures,
      completeRows: proof.summary?.completeRows,
      partialRows: proof.summary?.partialRows,
    };
  }
  if (key === 'social24') {
    return {
      productClaimed: proof.summary?.productClaimed,
      failures: proof.summary?.failures,
      completeRows: proof.summary?.completeRows,
      partialRows: proof.summary?.partialRows,
      rolloutState: proof.summary?.rolloutState,
    };
  }
  return {};
}

function markdownFor(proof) {
  const partialRows = proof.partialRows
    .map((row) => `| ${row.id} | ${row.title} | ${row.blocker.reason} | ${row.blocker.requiredEvidence} |`)
    .join('\n');
  const artifacts = proof.proofArtifacts
    .map((artifact) => `| ${artifact.key} | ${artifact.path} | ${artifact.failures.length === 0 ? 'ok' : 'failed'} |`)
    .join('\n');

  return [
    '# Browser Plan Closure Audit',
    '',
    `Generated: ${proof.generatedAt}`,
    `Branch: ${proof.branch}`,
    `Source commit at generation: ${proof.sourceCommitAtGeneration}`,
    `Base: ${proof.baseCommit}`,
    '',
    `Checklist rows: ${proof.summary.checklistRows}`,
    `Complete rows: ${proof.summary.completeRows}`,
    `Partial/manual-required rows: ${proof.summary.partialRows}`,
    `Unchecked rows: ${proof.summary.uncheckedRows}`,
    `Plan complete claimed: ${proof.summary.planCompleteClaimed}`,
    `PR-ready claimed: ${proof.summary.prReadyClaimed}`,
    '',
    '| Row | Title | Blocker | Required Evidence |',
    '| --- | --- | --- | --- |',
    partialRows,
    '',
    '| Proof | Path | State |',
    '| --- | --- | --- |',
    artifacts,
    '',
    'This audit is a blocker manifest, not a completion claim.',
    'The browser plan cannot be marked complete until the listed real-platform',
    'and runtime delivery proof exists. Product checklist upgrade and PR-ready',
    'state remain unclaimed.',
  ].join('\n');
}

function git(args) {
  return execFileSync('git', args, { cwd: root, encoding: 'utf8' }).trim();
}

function relativePath(path) {
  return relative(root, path).replaceAll('\\', '/');
}
