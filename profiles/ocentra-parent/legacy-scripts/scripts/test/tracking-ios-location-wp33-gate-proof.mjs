import { spawnSync } from 'node:child_process';
import { mkdir, readFile, rm, writeFile } from 'node:fs/promises';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const repoRoot = dirname(dirname(dirname(fileURLToPath(import.meta.url))));
const sourceProofPath = join(repoRoot, 'test-results', 'tracking-ios-location-manual-required-proof', 'proof.json');
const outputDir = join(repoRoot, 'test-results', 'tracking-ios-location-wp33-gate-proof');
const proofDir = join(repoRoot, 'output', 'tracking-plan-proof', 'tracking-ios-location-wp33-gate-proof');
const wp33ProofDir = join(repoRoot, 'output', 'tracking-plan-proof', '33-proof-gates-fixtures-rollout-and-pr-gate');
const generatedAt = '2026-06-06T14:25:00.000Z';
const commands = [];

await rm(outputDir, { recursive: true, force: true });
await rm(proofDir, { recursive: true, force: true });
await mkdir(outputDir, { recursive: true });
await mkdir(proofDir, { recursive: true });
await mkdir(wp33ProofDir, { recursive: true });

run('node', ['scripts/test/tracking-ios-location-manual-required-proof.mjs']);

const sourceProof = JSON.parse(await readFile(sourceProofPath, 'utf8'));
const proof = buildProof(sourceProof);

assertProof(proof);
await writeJson(join(outputDir, 'proof.json'), proof);
await writeJson(join(proofDir, 'proof.json'), proof);
await writeFile(join(proofDir, '00-source-snapshot.md'), sourceSnapshot(proof), 'utf8');
await writeFile(join(proofDir, '16-validation-commands.log'), validationLog(), 'utf8');
await writeJson(join(wp33ProofDir, '27-ios-location-manual-required-proof.json'), proof);

console.log('tracking-ios-location-wp33-gate-proof-ok');
console.log(`evidence=${join('test-results', 'tracking-ios-location-wp33-gate-proof', 'proof.json')}`);

function buildProof(sourceProof) {
  return {
    proofMode: 'tracking-ios-location-wp33-gate-proof',
    generatedAt,
    branch: gitOutput(['rev-parse', '--abbrev-ref', 'HEAD']),
    baseCommitAtGeneration: gitOutput(['rev-parse', 'HEAD']),
    commands,
    sourceProofPath: 'test-results/tracking-ios-location-manual-required-proof/proof.json',
    sourceProofSummary: sourceProof.summary,
    sourceNonClaims: sourceProof.nonClaims,
    iosManualRequiredRows: sourceProof.readModel.rows.map((row) => ({
      rowId: row.rowId,
      caseKind: row.caseKind,
      claimState: row.claimState,
      parentVisibleStatusToken: row.parentVisibleStatusToken,
      missingProofReasonRefs: row.missingProofReasonRefs,
    })),
    proofPaths: {
      sourceHarness: 'scripts/test/tracking-ios-location-manual-required-proof.mjs',
      companionHarness: 'scripts/test/tracking-ios-location-wp33-gate-proof.mjs',
      wp11Proof:
        'output/tracking-plan-proof/11-ios-core-location-foreground-adapter/19-ios-location-manual-required-proof.json',
      wp12Proof:
        'output/tracking-plan-proof/12-ios-background-region-significant-change-adapter/19-ios-location-manual-required-proof.json',
      wp33Proof:
        'output/tracking-plan-proof/33-proof-gates-fixtures-rollout-and-pr-gate/27-ios-location-manual-required-proof.json',
      evidence: 'test-results/tracking-ios-location-wp33-gate-proof/proof.json',
    },
  };
}

function assertProof(proof) {
  const expectedCaseKinds = [
    'when-in-use-authorization-manual-required',
    'foreground-sample-manual-required',
    'denied-restricted-services-disabled-manual-required',
    'always-authorization-manual-required',
    'region-transition-manual-required',
    'significant-change-visit-manual-required',
    'background-terminated-relaunch-manual-required',
  ];
  const actualCaseKinds = proof.iosManualRequiredRows.map((row) => row.caseKind);

  if (JSON.stringify(actualCaseKinds) !== JSON.stringify(expectedCaseKinds)) {
    throw new Error(`Unexpected iOS manual-required rows: ${JSON.stringify(actualCaseKinds)}`);
  }
  if (
    proof.sourceProofSummary.whenInUseAuthorizationManualRequiredCount !== 1 ||
    proof.sourceProofSummary.foregroundSampleManualRequiredCount !== 1 ||
    proof.sourceProofSummary.degradedStateManualRequiredCount !== 1 ||
    proof.sourceProofSummary.alwaysAuthorizationManualRequiredCount !== 1 ||
    proof.sourceProofSummary.regionTransitionManualRequiredCount !== 1 ||
    proof.sourceProofSummary.significantChangeVisitManualRequiredCount !== 1 ||
    proof.sourceProofSummary.backgroundTerminatedRelaunchManualRequiredCount !== 1
  ) {
    throw new Error(`Unexpected iOS manual-required summary: ${JSON.stringify(proof.sourceProofSummary)}`);
  }
  if (Object.values(proof.sourceNonClaims).some((value) => value !== false)) {
    throw new Error(`iOS WP33 gate proof overclaimed behavior: ${JSON.stringify(proof.sourceNonClaims)}`);
  }
}

function sourceSnapshot(proof) {
  return [
    '# Tracking iOS Location WP33 Gate Proof Source Snapshot',
    '',
    `- Branch: ${proof.branch}`,
    `- Base commit at generation: ${proof.baseCommitAtGeneration}`,
    '- Source proof: `node scripts/test/tracking-ios-location-manual-required-proof.mjs`.',
    '- Scope: WP11/WP12 iOS manual-required rows for When In Use authorization, foreground sample, denied/restricted/services-disabled, Always authorization, region transition, significant-change/visit, and background terminated/relaunch gaps.',
    '- Boundary: companion WP33 gate only; Core Location runtime, entitlement, notification delivery, physical-device behavior, authority, and product-ready iOS tracking remain unclaimed.',
    '',
  ].join('\n');
}

function validationLog() {
  return commands
    .map((command) =>
      [`$ ${command.command}`, command.stdout.trim(), command.stderr.trim()]
        .filter((line) => line.length > 0)
        .join('\n')
    )
    .join('\n\n');
}

function run(command, args) {
  const result = spawnSync(command, args, { cwd: repoRoot, encoding: 'utf8', shell: false });
  commands.push({
    command: [command, ...args].join(' '),
    status: result.status,
    stdout: result.stdout,
    stderr: result.stderr,
  });
  if (result.status !== 0) {
    throw new Error(
      `Command failed: ${[command, ...args].join(' ')}\nstdout:\n${result.stdout}\nstderr:\n${result.stderr}`
    );
  }
}

function gitOutput(args) {
  const result = spawnSync('git', args, { cwd: repoRoot, encoding: 'utf8', shell: false });
  if (result.status !== 0) {
    throw new Error(`git ${args.join(' ')} failed: ${result.stderr}`);
  }
  return result.stdout.trim();
}

async function writeJson(path, value) {
  await mkdir(dirname(path), { recursive: true });
  await writeFile(path, `${JSON.stringify(value, null, 2)}\n`, 'utf8');
}
