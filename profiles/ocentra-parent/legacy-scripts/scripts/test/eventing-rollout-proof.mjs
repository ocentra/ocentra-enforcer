import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join, relative, resolve } from 'node:path';
import { spawnSync } from 'node:child_process';

const repoRoot = process.cwd();
const proofRoot = repoPath('output', 'eventing-plan-proof', 'rollout-proof');
const testRoot = repoPath('test-results', 'eventing-rollout-proof');
const logRoot = repoPath(proofRoot, 'command-logs');

mkdirSync(repoAbs(proofRoot), { recursive: true });
mkdirSync(repoAbs(testRoot), { recursive: true });
mkdirSync(repoAbs(logRoot), { recursive: true });

const checkedAt = new Date().toISOString();
const branch = runText('git', ['branch', '--show-current']).trim();
const commit = runText('git', ['rev-parse', 'HEAD']).trim();
const statusShort = runText('git', ['status', '--short']).trimEnd();

const docsUnderTest = [
  'docs/plans/eventing-plan/workpacks/12-rollout-proof-and-pr-gate.md',
  'docs/plans/eventing-plan/PLAN_EXECUTION_BLUEPRINT.md',
  'docs/plans/eventing-plan/PLAN_STATE.md',
  'docs/plans/eventing-plan/NEXT_ACTIONS.md',
  'docs/plans/eventing-plan/WORKPACK_INDEX.md',
  'docs/plans/eventing-plan/PLAN_HEALTH.md',
  'docs/plans/eventing-plan/PROOF_INDEX.md',
  'docs/proof/eventing-plan/PLAN_PROOF_MANIFEST.md',
  'docs/proof/eventing-plan/slice-01-envelope-version.md',
  'docs/proof/eventing-plan/slice-02-ordering-replay.md',
  'docs/proof/eventing-plan/slice-03-consumer-boundary.md',
];

const proofDocs = [
  'docs/proof/eventing-plan/slice-01-envelope-version.md',
  'docs/proof/eventing-plan/slice-02-ordering-replay.md',
  'docs/proof/eventing-plan/slice-03-consumer-boundary.md',
];

const checks = [];
const checkResults = [];
const commandResults = [];

commandResults.push(
  runCommand({
    name: 'git-diff-check-docs',
    command: 'git',
    args: ['diff', '--check', '--', 'docs/proof/eventing-plan', 'docs/plans/eventing-plan'],
  })
);

runCheck('eventing.rollout.markdown-link-check', () => {
  const resolvedLinks = [];

  for (const file of docsUnderTest) {
    const text = readText(file);
    for (const target of extractLocalMarkdownTargets(text)) {
      const resolved = resolveMarkdownTarget(file, target);
      if (!resolved) {
        continue;
      }
      if (!existsSync(resolved)) {
        throw new Error(`missing markdown target from ${file}: ${target}`);
      }
      resolvedLinks.push({ file, target, resolved: toRepoRelative(resolved) });
    }
  }

  return {
    filesChecked: docsUnderTest.length,
    linksChecked: resolvedLinks.length,
    sampleLinks: resolvedLinks.slice(0, 12),
  };
});

runCheck('eventing.rollout.route-state-check', () => {
  const workpackIndex = readText('docs/plans/eventing-plan/WORKPACK_INDEX.md');
  const planState = readText('docs/plans/eventing-plan/PLAN_STATE.md');
  const nextActions = readText('docs/plans/eventing-plan/NEXT_ACTIONS.md');
  const planHealth = readText('docs/plans/eventing-plan/PLAN_HEALTH.md');
  const workpack = readText('docs/plans/eventing-plan/workpacks/12-rollout-proof-and-pr-gate.md');

  assertContains(workpackIndex, '| done   | [12 Rollout Proof And PR Gate]', 'WP12 done route');
  assertContains(workpackIndex, '| done   | [13 Test Folder Layout Regression Audit]', 'WP13 done route');
  assertContains(workpackIndex, '| open   | [10 LAN Household Mesh Consumer]', 'WP10 open route');
  assertContains(workpackIndex, '| done   | [11 Type Safety And Ownership Hardening]', 'WP11 done route');
  assertContains(nextActions, 'WP11 is now locally proved:', 'WP11 completion note');
  assertContains(nextActions, 'WP12 is now locally proved', 'WP12 next action note');
  assertNotContains(nextActions, '[12 Rollout Proof And PR Gate] is open', 'stale WP12 open note');
  assertContains(
    planState,
    'Workpacks open in truth: WP10 consumer-boundary handoff only.',
    'plan-state open workpacks'
  );
  assertContains(
    planState,
    'A fresh WP12 route-proof bundle is restored locally at',
    'plan-state restored proof bundle'
  );
  assertContains(planHealth, 'WP11 source-boundary hardening is locally proved.', 'plan-health WP11 note');
  assertContains(planHealth, 'WP12 route-proof', 'plan-health rollout note');
  assertContains(
    workpack,
    'These paths are the required local route-proof bundle for WP12.',
    'workpack12 restored bundle note'
  );

  return {
    wp12Done: true,
    wp13Done: true,
    openWorkpacks: ['WP10'],
  };
});

runCheck('eventing.rollout.manifest-check', () => {
  const manifest = readText('docs/proof/eventing-plan/PLAN_PROOF_MANIFEST.md');
  const slice01 = readText('docs/proof/eventing-plan/slice-01-envelope-version.md');
  const slice02 = readText('docs/proof/eventing-plan/slice-02-ordering-replay.md');
  const slice03 = readText('docs/proof/eventing-plan/slice-03-consumer-boundary.md');

  assertContains(manifest, 'WP12 route-proof bundle is restored in this checkout.', 'manifest status');
  assertContains(manifest, 'Remaining open workpacks in truth are:', 'manifest open workpacks');
  assertContains(manifest, 'WP10 LAN Household Mesh Consumer', 'manifest WP10');
  assertContains(manifest, 'WP11 Type Safety And Ownership Hardening is now locally proved', 'manifest WP11');
  for (const proofDoc of proofDocs) {
    assertContains(manifest, proofDoc.split('/').at(-1), `manifest link ${proofDoc}`);
  }
  assertContains(slice01, 'WP10 remains open.', 'slice01 remaining gap');
  assertContains(slice02, 'does not restore the historical', 'slice02 negative note');
  assertContains(slice03, 'remaining open eventing-plan slice is WP10', 'slice03 open slices');
  assertContains(slice03, 'No `PR_READY` claim is made by this route-proof bundle.', 'slice03 no-pr-ready');

  return {
    manifest: 'docs/proof/eventing-plan/PLAN_PROOF_MANIFEST.md',
    slicesChecked: proofDocs.length,
  };
});

runCheck('eventing.rollout.consumer-claim-negative', () => {
  const slice03 = readText('docs/proof/eventing-plan/slice-03-consumer-boundary.md');
  const report = renderPrDoneReport();

  assertContains(slice03, 'broker-backed delivery', 'broker negative');
  assertContains(slice03, 'relay-hub delivery', 'relay negative');
  assertContains(slice03, 'portal-owned', 'portal negative');
  assertContains(slice03, 'business event publishing', 'portal negative detail');
  assertContains(slice03, 'No claim that WP10 LAN household mesh proof is complete.', 'WP10 negative');
  assertContains(report, 'Remaining open workpacks: WP10', 'report open workpacks');
  assertContains(report, 'No PR_READY claim.', 'report no-pr-ready');

  return {
    negativeClaimsChecked: 5,
    reportOpenWorkpacks: ['WP10'],
  };
});

const prDoneReport = renderPrDoneReport();
writeFileSync(repoAbs(proofRoot, 'pr-done-report.md'), `${prDoneReport}\n`);

runCheck('eventing.rollout.pr-done-report', () => {
  assertContains(prDoneReport, 'WP12 Rollout Proof And PR Gate', 'report workpack');
  assertContains(prDoneReport, 'docs/proof/eventing-plan/PLAN_PROOF_MANIFEST.md', 'report manifest');
  assertContains(prDoneReport, 'docs/proof/eventing-plan/slice-01-envelope-version.md', 'report slice 01');
  assertContains(prDoneReport, 'docs/proof/eventing-plan/slice-02-ordering-replay.md', 'report slice 02');
  assertContains(prDoneReport, 'docs/proof/eventing-plan/slice-03-consumer-boundary.md', 'report slice 03');
  assertContains(prDoneReport, 'node scripts/test/eventing-rollout-proof.mjs', 'report validation command');
  assertContains(prDoneReport, 'Skipped risks', 'report skipped risks');
  assertContains(prDoneReport, 'Remaining gaps', 'report remaining gaps');

  return {
    prDoneReport: repoPath(proofRoot, 'pr-done-report.md'),
  };
});

const proof = {
  proof: 'eventing-rollout',
  status: 'route-proof-restored',
  checkedAt,
  branch,
  commit,
  statusShort,
  proofRoot,
  testRoot,
  docsUnderTest,
  proofDocs,
  checksRun: checks,
  commands: [...commandResults, ...checkResults],
  artifacts: {
    proofSummary: repoPath(proofRoot, 'proof-summary.json'),
    testProof: repoPath(testRoot, 'proof.json'),
    prDoneReport: repoPath(proofRoot, 'pr-done-report.md'),
    commandLogs: repoPath(proofRoot, 'command-logs'),
    manifest: 'docs/proof/eventing-plan/PLAN_PROOF_MANIFEST.md',
  },
  selectedWorkpack: 'WP12 Rollout Proof And PR Gate',
  sourceRows: '05-implementation-workpacks main gates and merge-blocking failures',
  openWorkpacks: ['WP10 LAN Household Mesh Consumer'],
  validationCommands: [
    'node scripts/test/eventing-rollout-proof.mjs',
    'git diff --check -- docs/proof/eventing-plan docs/plans/eventing-plan',
  ],
  skippedRisks: [
    'full repo validation was intentionally not run',
    'consumer-owned LAN/remote-access proof gaps remain out of WP12 closure',
  ],
  remainingGaps: ['WP10 proof roots are still absent in this checkout'],
  notClaimed: [
    'full eventing plan DONE',
    'PR_READY',
    'broker-backed delivery',
    'relay-hub delivery',
    'portal-owned business event publishing',
  ],
};

writeFileSync(repoAbs(proofRoot, 'proof-summary.json'), `${JSON.stringify(proof, null, 2)}\n`);
writeFileSync(repoAbs(testRoot, 'proof.json'), `${JSON.stringify(proof, null, 2)}\n`);
console.log('eventing-rollout-proof-ok:route-proof-restored');
console.log(`proof=${repoPath(proofRoot, 'proof-summary.json')}`);

function runCheck(name, fn) {
  const logPath = repoAbs(logRoot, `${name.replace(/[^a-zA-Z0-9_.-]/g, '-')}.log`);
  try {
    const details = fn();
    writeFileSync(logPath, `${name}: pass\n${JSON.stringify(details, null, 2)}\n`);
    checkResults.push({
      name,
      command: `node scripts/test/eventing-rollout-proof.mjs :: ${name}`,
      status: 0,
      log: toRepoRelative(logPath),
    });
    checks.push(name);
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    writeFileSync(logPath, `${name}: fail\n${message}\n`);
    throw new Error(`${name} failed; log=${toRepoRelative(logPath)}\n${message}`);
  }
}

function runCommand(entry) {
  const result = spawnSync(entry.command, entry.args, { encoding: 'utf8', shell: false });
  const safeName = entry.name.replace(/[^a-zA-Z0-9_.-]/g, '-');
  const log = repoAbs(logRoot, `${safeName}.log`);
  writeFileSync(log, `${result.stdout ?? ''}${result.stderr ?? ''}`);
  if (result.status !== 0) {
    throw new Error(`${entry.name} failed with exit ${result.status}; log=${toRepoRelative(log)}`);
  }
  return {
    name: entry.name,
    command: [entry.command, ...entry.args].join(' '),
    status: result.status,
    log: toRepoRelative(log),
  };
}

function renderPrDoneReport() {
  return [
    '# Eventing WP12 Route Proof Report',
    '',
    '- selected workpack: WP12 Rollout Proof And PR Gate',
    '- source rows: 05-implementation-workpacks main gates and merge-blocking failures',
    '- proof artifacts:',
    `  - ${repoPath(proofRoot, 'proof-summary.json')}`,
    `  - ${repoPath(testRoot, 'proof.json')}`,
    `  - ${repoPath(proofRoot, 'pr-done-report.md')}`,
    `  - ${repoPath(proofRoot, 'command-logs')}`,
    '  - docs/proof/eventing-plan/PLAN_PROOF_MANIFEST.md',
    '  - docs/proof/eventing-plan/slice-01-envelope-version.md',
    '  - docs/proof/eventing-plan/slice-02-ordering-replay.md',
    '  - docs/proof/eventing-plan/slice-03-consumer-boundary.md',
    '- validation commands:',
    '  - node scripts/test/eventing-rollout-proof.mjs',
    '  - git diff --check -- docs/proof/eventing-plan docs/plans/eventing-plan',
    '- Remaining open workpacks: WP10',
    '- Skipped risks:',
    '  - full repo validation was intentionally not run',
    '  - consumer-owned LAN/remote-access proof gaps remain out of WP12 closure',
    '- Remaining gaps:',
    '  - WP10 proof roots are still absent in this checkout',
    '- No PR_READY claim.',
  ].join('\n');
}

function extractLocalMarkdownTargets(text) {
  const targets = [];
  const linkPattern = /\[[^\]]+\]\(([^)]+)\)/g;
  for (const match of text.matchAll(linkPattern)) {
    const rawTarget = match[1].trim();
    if (
      rawTarget.startsWith('http://') ||
      rawTarget.startsWith('https://') ||
      rawTarget.startsWith('mailto:') ||
      rawTarget.startsWith('#')
    ) {
      continue;
    }
    targets.push(rawTarget);
  }
  return targets;
}

function resolveMarkdownTarget(file, target) {
  const withoutAnchor = target.split('#', 1)[0].trim();
  const stripped = withoutAnchor.replace(/^<|>$/g, '');
  if (stripped.length === 0) {
    return null;
  }
  return resolve(dirname(repoAbs(file)), stripped);
}

function assertContains(text, needle, label) {
  if (!text.includes(needle)) {
    throw new Error(`missing ${label}: ${needle}`);
  }
}

function assertNotContains(text, needle, label) {
  if (text.includes(needle)) {
    throw new Error(`unexpected ${label}: ${needle}`);
  }
}

function readText(relativePath) {
  return readFileSync(repoAbs(relativePath), 'utf8');
}

function runText(command, args) {
  const result = spawnSync(command, args, { encoding: 'utf8', shell: false });
  if (result.status !== 0) {
    const stderr = result.stderr ? `\n${result.stderr}` : '';
    const stdout = result.stdout ? `\n${result.stdout}` : '';
    throw new Error(`${command} ${args.join(' ')} failed with exit ${result.status}${stdout}${stderr}`);
  }
  return `${result.stdout ?? ''}${result.stderr ?? ''}`;
}

function repoAbs(...parts) {
  return resolve(repoRoot, ...parts);
}

function repoPath(...parts) {
  return join(...parts).replace(/\\/gu, '/');
}

function toRepoRelative(target) {
  return relative(repoRoot, target).replace(/\\/gu, '/');
}
