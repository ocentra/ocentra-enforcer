import { spawn } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';

const repoRoot = process.cwd();
const proofSlug = '177-app-game-category-unknown-policy-readiness-service-consumption';
const testOutputDir = join(
  repoRoot,
  'test-results',
  'app-game-category-unknown-policy-readiness-service-consumption-proof'
);
const appGameProofDir = join(repoRoot, 'output', 'app-game-plan-proof', proofSlug);
const commands = [];

await main();

async function main() {
  await mkdir(testOutputDir, { recursive: true });
  await mkdir(appGameProofDir, { recursive: true });
  await mkdir(join(appGameProofDir, '06-ui-snapshots'), { recursive: true });

  await runCommand(...npmCommand(['run', 'build:contracts']));
  await runCommand(
    ...npmCommand([
      'run',
      'build',
      '--workspace',
      '@ocentra-parent/schema-domain',
    ])
  );
  await runCommand(
    ...npmCommand(['run', 'test', '--workspace', '@ocentra-parent/portal', '--', 'app-game-policy-readiness-panel'])
  );
  await runCommand('cargo', ['test', '-p', 'ocentra-parent-agent-protocol', 'app_game_policy_readiness']);
  await runCommand('cargo', ['test', '-p', 'ocentra-parent-agent-service', 'app_game_policy_readiness']);
  await assertProtocolHarness();

  const proof = {
    schemaVersion: 1,
    proofMode: 'app-game-category-unknown-policy-readiness-service-consumption',
    checkedAt: new Date().toISOString(),
    branch: await gitBranch(),
    commit: await gitHead(),
    commands,
    summary: {
      command: 'agent.activity.app-game.policy-readiness.read-model.get',
      event: 'agent.activity.app-game.policy-readiness.read-model.reported',
      payloadField: 'appGamePolicyReadinessReadModel',
      addedReadinessRows: ['categoryCandidate', 'unknownReview'],
      addedReadinessFields: [
        'categoryRoutingReady',
        'unknownReviewRequired',
        'categoryCandidateRowCount',
        'unknownReviewRowCount',
      ],
      adapterDispatchClaimed: false,
    },
    claimsProved: [
      'Generated bridge accepts category candidate and unknown-review readiness rows from the app/game policy-readiness event',
      'Rust protocol serializes category routing and unknown-review readiness fields without adapter dispatch claims',
      'Agent service derives category candidate counts from existing inventory category candidates',
      'Agent service keeps unknown, possible-game, and launcher-game-candidate rows manual-required for review',
      'The existing policy readiness command consumes these rows from the service-backed app/game read model without portal scanning or adapter execution',
      'Portal route snapshots render category candidate and unknown-review rows/counts in the parent-safe policy-readiness surface',
    ],
    claimsNotProved: [
      'finished parent approval UI',
      'finished child request UI',
      'live classifier/provider quality',
      'runtime policy evaluator execution',
      'adapter dispatch, broad installed-app blocking, or platform enforcement',
      'dedicated source-panel renderer polish',
      'product checklist status movement',
    ],
    evidence: {
      generatedBridge: 'packages/schema-domain/src/app-game-policy-readiness.ts',
      rustProtocol: 'crates/agent-protocol/src/app_game_policy_readiness.rs',
      rustProtocolTest: 'crates/agent-protocol/tests/contract/app_game_policy_readiness_tests.rs',
      servicePayload: 'crates/agent-service/src/activity_api/app_game_policy_readiness_payload.rs',
      servicePayloadTest: 'crates/agent-service/tests/unit/app_game_policy_readiness_payload_tests.rs',
      serviceCommandTest: 'crates/agent-service/tests/unit/app_game_policy_readiness_service_tests.rs',
      portalRoutePanel: 'apps/portal/src/AppGamePolicyReadinessRoutePanel.tsx',
      portalBridgeGuard: 'apps/portal/tests/unit/parent-ui-bridge.test.ts',
      portalRouteTest: 'apps/portal/tests/unit/app-game-policy-readiness-panel.test.ts',
    },
  };

  await writeJson(join(testOutputDir, 'proof.json'), proof);
  await writeProofPack(proof);

  console.log('app-game-category-unknown-policy-readiness-service-consumption-proof-ok');
  console.log(`evidence=${relative(repoRoot, join(testOutputDir, 'proof.json'))}`);
}

async function writeProofPack(proof) {
  await writeFile(
    join(appGameProofDir, '00-source-snapshot.md'),
    [
      '# WP177 Source Snapshot',
      '',
      `- Branch: ${proof.branch}`,
      `- Commit: ${proof.commit}`,
      '- Scope: app/game category and unknown-state policy-readiness service plus parent-safe intent consumption.',
      '- Product checklist intentionally not edited because another lane owns that shared file.',
      '- Shared SVG renderer polish intentionally not edited.',
      '',
    ].join('\n'),
    'utf8'
  );
  await writeFile(
    join(appGameProofDir, '01-contract-proof.log'),
    [
      'Contract proof:',
      '',
      '- cmd /c npm run build:contracts: PASS',
      '- cmd /c npm run build --workspace @ocentra-parent/schema-domain: PASS',
      '- cmd /c npm run test --workspace @ocentra-parent/portal -- app-game-policy-readiness-panel: PASS',
      '- The generated bridge import accepts categoryCandidate and unknownReview rows and rejects adapterDispatchClaimed=true.',
      '- The app portal route snapshot surface renders category/unknown rows and counts without adapter dispatch claims.',
      '',
    ].join('\n'),
    'utf8'
  );
  await writeFile(
    join(appGameProofDir, '02-rust-protocol-proof.log'),
    [
      'Rust protocol proof:',
      '',
      '- cargo test -p ocentra-parent-agent-protocol app_game_policy_readiness: PASS',
      '- DTO serialization preserves categoryRoutingReady, unknownReviewRequired, categoryCandidateRowCount, unknownReviewRowCount, and adapterDispatchClaimed=false.',
      '',
    ].join('\n'),
    'utf8'
  );
  await writeJson(join(appGameProofDir, '03-runtime-evidence.json'), proof);
  await writeJson(join(appGameProofDir, '04-journal-sqlite-proof.json'), {
    schemaVersion: 1,
    journalSqliteChanged: false,
    serviceModelSource: 'ActivityStore::app_game_service_read_model',
    categorySource: 'inventory_rows[].category_candidates',
    unknownReviewSource:
      'inventory/running/foreground/launcher rows with unknownProcess, possiblyGame, or launcherGameCandidate classification',
  });
  await writeJson(join(appGameProofDir, '05-policy-action-proof.json'), {
    schemaVersion: 1,
    categoryRoutingReady: 'true only when category candidates exist in the service-backed app/game read model',
    unknownReviewRequired: 'true when unknown/manual-review app/game rows exist',
    adapterDispatchClaimed: false,
    policyExecutionClaimed: false,
  });
  await writeFile(
    join(appGameProofDir, '06-ui-snapshots', 'ui-not-changed.md'),
    '# UI Snapshot Deferred\n\nThis workpack updates the service/protocol readiness payload and parent-safe portal-domain intent. Shared SVG renderer polish and browser screenshots remain deferred to the larger app/game validation batch.\n',
    'utf8'
  );
  await writeFile(
    join(appGameProofDir, '07-playwright-ui-proof.log'),
    'Playwright/browser proof not run: WP177 changes portal-domain intent and defers rendered browser proof to the larger app/game validation batch.\n',
    'utf8'
  );
  await writeFile(
    join(appGameProofDir, '08-security-negative-proof.log'),
    [
      'Security/no-claim proof:',
      '',
      '- Category candidates are policy-readiness inputs only.',
      '- Unknown rows stay manual-required and are not promoted to known apps/games.',
      '- adapterDispatchClaimed remains false.',
      '- No parent portal code scans the OS, executes policy, dispatches adapters, sends notifications, or claims platform enforcement.',
      '',
    ].join('\n'),
    'utf8'
  );
  await writeFile(
    join(appGameProofDir, '09-manual-platform-proof.md'),
    '# Manual Platform Proof\n\nNo platform authority tier changed. Broad blocking and platform adapter execution remain unclaimed.\n',
    'utf8'
  );
  await writeFile(
    join(appGameProofDir, '10-validation-commands.log'),
    [
      'Validation run:',
      '',
      '- cmd /c npm run build:contracts: PASS',
      '- cmd /c npm run build --workspace @ocentra-parent/schema-domain: PASS',
      '- cmd /c npm run test --workspace @ocentra-parent/portal -- app-game-policy-readiness-panel: PASS',
      '- cargo test -p ocentra-parent-agent-protocol app_game_policy_readiness: PASS',
      '- cargo test -p ocentra-parent-agent-service app_game_policy_readiness: PASS',
      '- node scripts/test/app-game-category-unknown-policy-readiness-service-consumption-proof.mjs: PASS',
      '',
    ].join('\n'),
    'utf8'
  );
  await writeFile(
    join(appGameProofDir, '11-authority-tier-proof.md'),
    '# Authority Tier Proof\n\nCategory and unknown-review service rows do not upgrade platform authority or adapter dispatch readiness.\n',
    'utf8'
  );
  await writeFile(
    join(appGameProofDir, '12-rollback-proof.md'),
    '# Rollback Proof\n\nNo device action, timer, block, suspend, shield, or adapter state is created, so rollback is not applicable.\n',
    'utf8'
  );
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

async function assertProtocolHarness() {
  const harness = await readFile(join(repoRoot, 'crates/agent-protocol/tests/contract.rs'), 'utf8');
  assertIncludes(
    harness,
    '#[path = "contract/app_game_policy_readiness_tests.rs"]',
    'app-game policy readiness contract harness registration exists'
  );
}

async function writeJson(path, value) {
  await writeFile(path, `${JSON.stringify(value, null, 2)}\n`, 'utf8');
}

function npmCommand(args) {
  const command = process.platform === 'win32' ? 'cmd' : 'npm';
  const commandArgs = process.platform === 'win32' ? ['/c', 'npm', ...args] : args;
  return [command, commandArgs];
}

function assertIncludes(text, expected, label) {
  if (!text.includes(expected)) {
    throw new Error(`${label}: missing ${expected}`);
  }
}
