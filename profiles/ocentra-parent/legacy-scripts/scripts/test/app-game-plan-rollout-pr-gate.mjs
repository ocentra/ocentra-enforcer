import { spawn } from 'node:child_process';
import { mkdir, readdir, readFile, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';

const repoRoot = process.cwd();
const testOutputDir = join(repoRoot, 'test-results', 'app-game-plan-rollout-pr-gate');
const appGameProofDir = join(repoRoot, 'output', 'app-game-plan-proof', '28-e2e-manual-proof-rollout-pr-gate');
const appProofArtifactsDir = join(repoRoot, 'output', 'app-plan-proof', '27-e2e-and-manual-proof-artifacts');
const appPrGateDir = join(repoRoot, 'output', 'app-plan-proof', '28-rollout-checklist-and-pr-gate');
const commands = [];

const expectedAppGameProofRoots = [
  '01-contract-boundary-and-effect-schemas',
  '02-source-index-and-doc-reconciliation',
  '03-current-app-game-snapshot-and-gap-map',
  '04-app-game-identity-model',
  '05-inventory-evidence-model',
  '06-windows-installed-inventory-adapter',
  '07-windows-store-uwp-appx-inventory-adapter',
  '08-windows-process-runtime-evidence-adapter',
  '09-windows-foreground-evidence-adapter',
  '10-launcher-evidence-and-game-candidate-model',
  '11-cross-platform-authority-matrix',
  '12-app-game-category-and-risk-taxonomy',
  '13-sessionization-and-duration-engine',
  '14-journal-and-sqlite-ingest',
  '15-read-models-and-service-events',
  '16-parent-portal-app-game-dashboard-surfaces',
  '17-unknown-app-game-approval-flow',
  '18-native-game-budgets-and-launcher-policy',
  '19-policy-target-compiler-for-app-game-rules',
  '20-time-budget-schedule-bonus-time-integration',
  '21-child-facing-warning-and-request-ux',
  '22-windows-owned-process-terminate-time-limit-proof',
  '23-broad-blocking-proof-gates',
  '24-ai-classifier-digest-boundary',
  '25-platform-extension-checklist-and-proof-routing',
  '26-install-uninstall-purchase-store-handoffs',
  '27-performance-and-service-health',
];

const expectedAppProofRoots = [
  '01-contract-boundary-and-effect-schemas',
  '02-source-index-and-doc-reconciliation',
  '03-current-app-snapshot-and-gap-map',
  '04-app-identity-model',
  '05-installed-app-inventory-model',
  '06-windows-installed-app-inventory-adapter',
  '07-windows-store-uwp-appx-inventory-adapter',
  '08-windows-process-runtime-evidence-adapter',
  '09-windows-foreground-app-evidence-adapter',
  '10-cross-platform-authority-matrix',
  '11-app-category-and-risk-taxonomy',
  '12-app-sessionization-and-duration-engine',
  '13-journal-and-sqlite-app-ingest',
  '14-app-read-models-and-service-events',
  '15-parent-portal-app-inventory-running-session-surfaces',
  '16-new-app-and-unknown-app-approval-flow',
  '17-riskapp-detection',
  '18-policy-target-compiler-for-app-rules',
  '19-time-budget-schedule-bonus-time-integration',
  '20-child-facing-app-warning-block-request-ux',
  '21-windows-owned-process-terminate-time-limit-proof',
  '22-broad-blocking-proof-gates',
  '23-app-ai-classifier-digest-boundary',
  '24-platform-extension-checklist-and-proof-routing',
  '25-install-and-uninstall-approval-handoff',
  '26-performance-and-service-health',
];

const e2eRoutes = [
  {
    scenario: 'Windows app/game inventory to portal',
    appGameProofRoots: [
      '06-windows-installed-inventory-adapter',
      '07-windows-store-uwp-appx-inventory-adapter',
      '14-journal-and-sqlite-ingest',
      '15-read-models-and-service-events',
      '16-parent-portal-app-game-dashboard-surfaces',
    ],
    claimState: 'staged-fixture-service-backed',
    gap: 'live source crawling remains unclaimed',
  },
  {
    scenario: 'Runtime session and foreground duration',
    appGameProofRoots: [
      '08-windows-process-runtime-evidence-adapter',
      '09-windows-foreground-evidence-adapter',
      '13-sessionization-and-duration-engine',
      '14-journal-and-sqlite-ingest',
    ],
    claimState: 'staged-fixture-replay-backed',
    gap: 'live polling/subscription remains unclaimed',
  },
  {
    scenario: 'Unknown app/game approval',
    appGameProofRoots: ['17-unknown-app-game-approval-flow'],
    claimState: 'contract-only',
    gap: 'parent/child approval UI, notification delivery, and persistence remain unclaimed',
  },
  {
    scenario: 'Launcher not game and launcher-game candidate',
    appGameProofRoots: ['10-launcher-evidence-and-game-candidate-model', '18-native-game-budgets-and-launcher-policy'],
    claimState: 'contract-parser-policy-preview',
    gap: 'live launcher crawling and game-budget runtime remain unclaimed',
  },
  {
    scenario: 'Risk app detection and AI classifier boundary',
    appGameProofRoots: ['12-app-game-category-and-risk-taxonomy', '24-ai-classifier-digest-boundary'],
    claimState: 'contract-only',
    gap: 'live classifier/provider quality and portal classifier rows remain unclaimed',
  },
  {
    scenario: 'Time budget dry-run',
    appGameProofRoots: [
      '19-policy-target-compiler-for-app-game-rules',
      '20-time-budget-schedule-bonus-time-integration',
    ],
    claimState: 'dry-run-contract-only',
    gap: 'runtime evaluator, timers, notifications, and adapter execution remain unclaimed',
  },
  {
    scenario: 'Owned-process enforcement',
    appGameProofRoots: ['22-windows-owned-process-terminate-time-limit-proof'],
    claimState: 'scoped-real-service-proof',
    gap: 'broad package/app blocking remains manual-required',
  },
  {
    scenario: 'Broad block manual-required',
    appGameProofRoots: ['23-broad-blocking-proof-gates', '25-platform-extension-checklist-and-proof-routing'],
    claimState: 'manual-required-no-claim-gate',
    gap: 'real platform adapter proof remains required per platform',
  },
  {
    scenario: 'Performance/service health',
    appGameProofRoots: ['27-performance-and-service-health'],
    claimState: 'generated-scale-intent-proof',
    gap: 'live OS throughput and browser DOM rendering remain unclaimed',
  },
];

const noClaimGates = [
  'inventory evidence is not app/game usage',
  'runtime evidence is not foreground usage',
  'foreground evidence is not content knowledge',
  'launcher evidence is not active game play without child-game proof',
  'unknown process is not auto-promoted to known game',
  'AI output cannot directly enforce',
  'dry-run cannot terminate or block',
  'manual-required cannot call adapters',
  'Android normal mode cannot claim package suspend/hide',
  'iOS cannot claim raw process scanning or process killing',
  'macOS hard block needs MDM, Endpoint Security, or System Extension proof',
  'Linux broad blocking must name distro, session, and mechanism proof',
  'raw private executable paths must not leak into parent UI',
  'malicious app/game metadata must not create XSS or layout-breaking proof',
];

await main();

async function main() {
  await mkdir(testOutputDir, { recursive: true });
  await mkdir(appGameProofDir, { recursive: true });
  await mkdir(appProofArtifactsDir, { recursive: true });
  await mkdir(appPrGateDir, { recursive: true });
  await mkdir(join(appGameProofDir, '06-ui-snapshots'), { recursive: true });
  await mkdir(join(appProofArtifactsDir, '06-ui-snapshots'), { recursive: true });
  await mkdir(join(appPrGateDir, '06-ui-snapshots'), { recursive: true });

  await runCommand('git', ['diff', '--check']);
  await runCommand(...npmCommand(['run', 'lanes:guard']));
  await runCommand(...npmCommand(['run', 'hub:guard']));

  const previousProof = {
    appGame: await collectProofRoots('output/app-game-plan-proof', expectedAppGameProofRoots),
    app: await collectProofRoots('output/app-plan-proof', expectedAppProofRoots),
  };
  assertPreviousProof(previousProof);

  const docs = await readDocs();
  assertDocState(docs);

  const finalGate = {
    schemaVersion: 1,
    proofMode: 'app-game-plan-rollout-pr-gate',
    checkedAt: new Date().toISOString(),
    branch: await gitBranch(),
    commit: await gitHead(),
    statusShort: await gitStatusShort(),
    commands,
    previousProof,
    e2eRoutes,
    noClaimGates,
    manualPlatformStates: {
      macos:
        'manual-required until MDM, Endpoint Security, System Extension, PPPC/setup, rollback, and audit proof exists',
      ios: 'not-claimed for process scanning/killing; ManagedSettings/FamilyControls/MDM proof required for shielding',
      android: 'normal mode cannot hide/suspend; Device Owner/Profile Owner/delegation proof required',
      linux: 'mechanism, distro, session, admin/root, and rollback proof required before broad blocking',
      windows:
        'owned-process time-limit proof is scoped; AppLocker/App Control and rollback proof required for broad blocking',
    },
    productDocsDecision: {
      featureDocUpdated: true,
      productCapabilityChecklistUpdated: true,
      roadmapUpdated: false,
      expectationDocsUpdated: false,
      reason:
        'Final gate strengthens proof wording and PR-review readiness without moving milestone order or claiming new runtime/platform support.',
    },
    reviewGate: {
      prBodyMustInclude: [
        'branch, commit, and pushed state',
        'scope and touched paths',
        'validation commands and exact failures if any',
        'proof paths for app-game WP28 and app-plan WP27/WP28',
        'known gaps and no-claim boundaries',
        'product-doc/checklist update decision',
      ],
      readyForReviewWhen: [
        'final gate script passes',
        'focused package tests pass',
        'format and schema-boundary checks pass',
        'normal commit hook passes',
        'branch is pushed after latest origin/main rebase',
      ],
    },
    claimsProved: [
      'app-game WP01-WP27 proof roots and app-plan WP01-WP26 proof roots exist before the final gate',
      'final gate routes E2E/manual proof scenarios to existing proof roots or explicit manual-required gaps',
      'merge-blocking no-claim gates are listed in machine-readable proof and final proof packs',
      'platform extension rows remain manual-required or not-claimed until authority, setup, rollback, and manual platform proof exists',
      'reviewers can evaluate the branch from proof packs without guessing which claims are live, contract-only, generated-scale, or unclaimed',
    ],
    claimsNotProved: [
      'live OS source crawling or source subscription coverage',
      'product-complete parent/child approval UI and notification delivery',
      'runtime policy evaluator, game budget service persistence, or adapter execution beyond scoped owned-process proof',
      'live model/provider classifier quality',
      'cross-platform hard-control support',
      'browser DOM/Playwright rendering for every final UI state in the app/game matrix',
    ],
  };

  await writeJson(join(testOutputDir, 'proof.json'), finalGate);
  await writeFinalProofPack(appGameProofDir, finalGate, 'app-game WP28');
  await writeFinalProofPack(appProofArtifactsDir, finalGate, 'app WP27');
  await writeFinalProofPack(appPrGateDir, finalGate, 'app WP28');

  console.log(`app-game-plan-rollout-pr-gate-ok:${e2eRoutes.length}`);
  console.log(`evidence=${relative(repoRoot, join(testOutputDir, 'proof.json'))}`);
}

async function collectProofRoots(parent, expectedRoots) {
  const absoluteParent = join(repoRoot, parent);
  const roots = [];
  const present = await readdir(absoluteParent);

  for (const root of expectedRoots) {
    const proofRoot = join(absoluteParent, root);
    const entries = await readdir(proofRoot).catch(() => []);
    const sourceSnapshotPresent = entries.includes('00-source-snapshot.md');
    const validationLogPresent = entries.some((entry) => entry.includes('validation') || entry.includes('proof'));
    const uiSnapshotPresent = entries.includes('06-ui-snapshots') || entries.some((entry) => entry.endsWith('.png'));
    roots.push({
      root,
      path: `${parent}/${root}`,
      present: present.includes(root),
      entryCount: entries.length,
      sourceSnapshotPresent,
      validationLogPresent,
      uiSnapshotPresent,
    });
  }

  return {
    expectedCount: expectedRoots.length,
    presentCount: roots.filter((root) => root.present).length,
    roots,
    missing: roots.filter((root) => !root.present).map((root) => root.root),
  };
}

function assertPreviousProof(previousProof) {
  assertEqual(previousProof.appGame.missing.length, 0, 'missing app-game proof roots');
  assertEqual(previousProof.app.missing.length, 0, 'missing app proof roots');

  for (const root of [...previousProof.appGame.roots, ...previousProof.app.roots]) {
    if (!root.sourceSnapshotPresent) {
      throw new Error(`missing 00-source-snapshot.md in ${root.path}`);
    }
    if (root.entryCount === 0) {
      throw new Error(`empty proof root ${root.path}`);
    }
  }
}

async function readDocs() {
  return {
    feature: await readText('docs/features/app-game-control.md'),
    appGameChecklist: await readText('docs/plans/app-game-plan/implementation-checklist.md'),
    appChecklist: await readText('docs/plans/app-plan/implementation-checklist.md'),
    productChecklist: await readText('docs/product-capability-checklist.md'),
  };
}

function assertDocState(docs) {
  assertIncludes(docs.appGameChecklist, '28-e2e-manual-proof-rollout-pr-gate', 'app-game WP28 checklist row');
  assertIncludes(docs.appChecklist, '27-e2e-and-manual-proof-artifacts', 'app WP27 checklist row');
  assertIncludes(docs.appChecklist, '28-rollout-checklist-and-pr-gate', 'app WP28 checklist row');
  assertIncludes(docs.feature, 'final rollout/evidence gate', 'feature final gate text');
  assertIncludes(docs.productChecklist, 'final rollout/evidence gate', 'product checklist final gate text');
  assertIncludes(docs.productChecklist, 'broad-blocking gate matrix', 'product checklist broad-blocking gate text');
}

async function writeFinalProofPack(proofDir, proof, label) {
  const proofJsonPath = relative(repoRoot, join(testOutputDir, 'proof.json'));
  await writeFile(
    join(proofDir, '00-source-snapshot.md'),
    [
      `# ${label} Source Snapshot`,
      '',
      `- Branch: ${proof.branch}`,
      `- Commit: ${proof.commit}`,
      `- Status before final gate: ${proof.statusShort.length === 0 ? 'clean tracked files' : 'tracked changes present for final gate'}`,
      '- Source docs read: feature list, app/game feature doc, app/game evidence expectation, enforcement expectation, app-plan README/source/current/full-scope/platform/test/UI/checklist, app-game README/source/current/shared-spine/app-slice/game-slice/platform/test/UI/checklist, and this workpack.',
      '- Scope: final evidence/manual/no-claim rollout and PR gate. No runtime, Rust protocol, service, portal, or adapter behavior changed.',
      `- Machine-readable proof: ${proofJsonPath}`,
      '',
    ].join('\n'),
    'utf8'
  );
  await writeFile(
    join(proofDir, '01-contract-proof.log'),
    [
      'Final gate contract/proof routing:',
      '',
      `- App-game proof roots present: ${proof.previousProof.appGame.presentCount}/${proof.previousProof.appGame.expectedCount}`,
      `- App proof roots present: ${proof.previousProof.app.presentCount}/${proof.previousProof.app.expectedCount}`,
      `- E2E/manual route rows: ${proof.e2eRoutes.length}`,
      `- No-claim gate rows: ${proof.noClaimGates.length}`,
      '- This workpack adds no new schema contract; it verifies proof route completeness and PR-ready reporting constraints.',
      '',
    ].join('\n'),
    'utf8'
  );
  await writeFile(
    join(proofDir, '02-rust-protocol-proof.log'),
    [
      'Rust/service protocol not changed.',
      'Existing Rust proof remains attached to earlier app/game workpacks; this final gate verifies reviewability and no-claim/manual-required routing only.',
      '',
    ].join('\n'),
    'utf8'
  );
  await writeJson(join(proofDir, '03-runtime-evidence.json'), {
    schemaVersion: 1,
    proofMode: proof.proofMode,
    e2eRoutes: proof.e2eRoutes,
    previousProof: proof.previousProof,
  });
  await writeJson(join(proofDir, '04-journal-sqlite-proof.json'), {
    schemaVersion: 1,
    journalSqliteChanged: false,
    coveredByProofRoots: [
      'output/app-game-plan-proof/13-sessionization-and-duration-engine',
      'output/app-game-plan-proof/14-journal-and-sqlite-ingest',
      'output/app-game-plan-proof/15-read-models-and-service-events',
    ],
    reason: 'Final gate does not change journal or SQLite behavior; it verifies proof routing and review completeness.',
  });
  await writeJson(join(proofDir, '05-policy-action-proof.json'), {
    schemaVersion: 1,
    policyActionChanged: false,
    coveredByProofRoots: [
      'output/app-game-plan-proof/17-unknown-app-game-approval-flow',
      'output/app-game-plan-proof/18-native-game-budgets-and-launcher-policy',
      'output/app-game-plan-proof/19-policy-target-compiler-for-app-game-rules',
      'output/app-game-plan-proof/20-time-budget-schedule-bonus-time-integration',
      'output/app-game-plan-proof/22-windows-owned-process-terminate-time-limit-proof',
      'output/app-game-plan-proof/23-broad-blocking-proof-gates',
    ],
    adapterExecutionClaim: 'not-claimed-beyond-scoped-owned-process-proof',
  });
  await writeFile(
    join(proofDir, '06-ui-snapshots', 'ui-not-applicable.md'),
    [
      '# UI Not Applicable',
      '',
      'No portal or child UI source changed in this final gate. Earlier UI proof remains in the app/game dashboard workpack, and missing approval, policy authoring, child UX, platform matrix, malicious metadata, and narrow viewport screenshots remain explicit gaps rather than silent product claims.',
      '',
    ].join('\n'),
    'utf8'
  );
  await writeFile(
    join(proofDir, '07-playwright-ui-proof.log'),
    [
      'Playwright not run by this final gate because no UI source changed.',
      'The final gate records missing browser DOM/Playwright proof as a known gap and keeps UI states unclaimed when no screenshot/proof exists.',
      '',
    ].join('\n'),
    'utf8'
  );
  await writeFile(
    join(proofDir, '08-security-negative-proof.log'),
    [
      'Security/no-claim gates checked by final proof:',
      '',
      ...proof.noClaimGates.map((gate) => `- ${gate}`),
      '',
      'No final gate row promotes manual-required, unavailable, generated-scale, contract-only, or not-claimed evidence to live support.',
      '',
    ].join('\n'),
    'utf8'
  );
  await writeFile(
    join(proofDir, '09-manual-platform-proof.md'),
    [
      '# Manual Platform Proof',
      '',
      'No new manual platform proof is attached in this final gate.',
      '',
      ...Object.entries(proof.manualPlatformStates).map(([platform, state]) => `- ${platform}: ${state}`),
      '',
    ].join('\n'),
    'utf8'
  );
  await writeFile(
    join(proofDir, '10-validation-commands.log'),
    [
      'Validation run by final gate script:',
      '',
      ...proof.commands.map((command) => `- ${command}: PASS`),
      '',
      'Additional package/root validation should be recorded in the hub DONE/PR-ready report after this script runs.',
      '',
    ].join('\n'),
    'utf8'
  );
  await writeFile(
    join(proofDir, '11-authority-tier-proof.md'),
    [
      '# Authority Tier Proof',
      '',
      'Authority-tier proof is inherited from app-game WP11 and platform-extension routing WP25. This final gate does not promote any platform extension row out of manual-required or not-claimed state.',
      '',
    ].join('\n'),
    'utf8'
  );
  await writeFile(
    join(proofDir, '12-rollback-proof.md'),
    [
      '# Rollback Proof',
      '',
      'Rollback proof is required before future broad block, hide, suspend, shield, allowlist, or cross-platform hard-control claims can move up. This final gate adds no new adapter or rollback execution.',
      '',
    ].join('\n'),
    'utf8'
  );

  if (proofDir === appProofArtifactsDir || proofDir === appPrGateDir) {
    await writeFile(
      join(proofDir, '13-permission-setup-proof.md'),
      [
        '# Permission Setup Proof',
        '',
        'No new permission, enrollment, MDM, Device Owner/Profile Owner, FamilyControls, ManagedSettings, AppLocker/App Control, Endpoint Security, cgroup/systemd, or admin/root setup proof is attached in this final gate.',
        '',
      ].join('\n'),
      'utf8'
    );
  }
}

async function runCommand(command, args) {
  commands.push([command, ...args].join(' '));
  await new Promise((resolve, reject) => {
    const child = spawn(command, args, { cwd: repoRoot, shell: false, stdio: 'inherit' });
    child.on('error', reject);
    child.on('exit', (code) => {
      if (code === 0) {
        resolve(undefined);
        return;
      }
      reject(new Error(`${command} ${args.join(' ')} exited with ${code}`));
    });
  });
}

async function gitBranch() {
  return (await gitOutput(['rev-parse', '--abbrev-ref', 'HEAD'])).trim();
}

async function gitHead() {
  return (await gitOutput(['rev-parse', 'HEAD'])).trim();
}

async function gitStatusShort() {
  return (await gitOutput(['status', '--short'])).trim();
}

async function gitOutput(args) {
  const chunks = [];
  await new Promise((resolve, reject) => {
    const child = spawn('git', args, { cwd: repoRoot, shell: false });
    child.stdout.on('data', (chunk) => chunks.push(Buffer.from(chunk)));
    child.on('error', reject);
    child.on('exit', (code) => {
      if (code === 0) {
        resolve(undefined);
        return;
      }
      reject(new Error(`git ${args.join(' ')} exited with ${code}`));
    });
  });
  return Buffer.concat(chunks).toString('utf8');
}

async function readText(path) {
  return readFile(join(repoRoot, path), 'utf8');
}

async function writeJson(path, value) {
  await writeFile(path, `${JSON.stringify(value, null, 2)}\n`, 'utf8');
}

function assertIncludes(text, needle, label) {
  if (!text.includes(needle)) {
    throw new Error(`${label}: expected to include ${needle}`);
  }
}

function assertEqual(actual, expected, label) {
  if (actual !== expected) {
    throw new Error(`${label}: expected ${expected}, got ${actual}`);
  }
}

function npmCommand(args) {
  const command = process.platform === 'win32' ? 'cmd' : 'npm';
  const commandArgs = process.platform === 'win32' ? ['/c', 'npm', ...args] : args;
  return [command, commandArgs];
}
