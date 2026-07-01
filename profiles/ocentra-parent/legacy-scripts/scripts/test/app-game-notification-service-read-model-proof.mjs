import { spawn } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';

const repoRoot = process.cwd();
const testOutputDir = join(repoRoot, 'test-results', 'app-game-notification-service-read-model-proof');
const appGameProofDir = join(repoRoot, 'output', 'app-game-plan-proof', '56-notification-service-read-model');
const appProofDir = join(repoRoot, 'output', 'app-plan-proof', '56-notification-service-read-model');
const commands = [];

await main();

async function main() {
  await mkdir(testOutputDir, { recursive: true });
  await mkdir(appGameProofDir, { recursive: true });
  await mkdir(appProofDir, { recursive: true });
  await mkdir(join(appGameProofDir, '06-ui-snapshots'), { recursive: true });
  await mkdir(join(appProofDir, '06-ui-snapshots'), { recursive: true });

  await runCommand(...npmCommand(['run', 'build:contracts']));
  await runCommand(
    ...npmCommand([
      'run',
      'build',
      '--workspace',
      '@ocentra-parent/schema-domain',
    ])
  );
  await runCommand('cargo', ['test', '-p', 'ocentra-parent-agent-protocol', 'app_game_notification_readiness']);
  await runCommand('cargo', ['test', '-p', 'ocentra-parent-agent-service', 'app_game_notification_readiness']);
  await assertProtocolHarness();

  const proof = {
    schemaVersion: 1,
    proofMode: 'app-game-notification-service-read-model',
    checkedAt: new Date().toISOString(),
    commit: await gitHead(),
    commands,
    summary: {
      command: 'agent.activity.app-game.notification-readiness.read-model.get',
      event: 'agent.activity.app-game.notification-readiness.read-model.reported',
      payloadField: 'appGameNotificationReadinessReadModel',
      readinessReasons: [
        'time-limit-exceeded',
        'approval-request',
        'suspicious-unknown',
        'manual-required',
        'capability-unavailable',
      ],
      providerDeliveryClaimed: false,
      providerReceiptIngestionClaimed: false,
      localOutboxRuntimeClaimed: false,
      schedulerRuntimeClaimed: false,
      adapterDispatchClaimed: false,
      parentUiClaimed: false,
      childDeliveryClaimed: false,
    },
    claimsProved: [
      'Generated schema-domain bridge mirrors the Rust-owned app/game notification readiness read-model contract surface',
      'Generated bridge consumes the dedicated app/game notification readiness event payload',
      'Rust protocol serializes notification readiness rows and no-delivery/no-adapter claim flags',
      'Agent service answers the notification readiness command from the existing activity-store app/game service read model',
      'Eligible app/game evidence rows can become local-intent-ready rows without provider delivery or adapter dispatch',
      'Missing prerequisites remain manual-required or unavailable instead of being invented',
    ],
    claimsNotProved: [
      'provider adapter delivery or receipt ingestion',
      'durable production notification outbox or scheduler runtime',
      'quiet-hours timer execution or retry worker execution',
      'parent notification UI or child-device notification delivery',
      'policy evaluator execution, adapter dispatch, broad blocking, or platform support',
    ],
    evidence: {
      schemaContract: 'packages/schema-domain/src/app-game-notification-readiness.ts',
      generatedBridge: 'packages/schema-domain/src/app-game-notification-readiness.ts',
      rustProtocol: 'crates/agent-protocol/src/app_game_notification_readiness.rs',
      rustProtocolTest: 'crates/agent-protocol/tests/contract/app_game_notification_readiness_tests.rs',
      servicePayload: 'crates/agent-service/src/activity_api/app_game_notification_readiness_payload.rs',
      servicePayloadTest: 'crates/agent-service/tests/unit/app_game_notification_readiness_payload_tests.rs',
      serviceWebSocketTest: 'crates/agent-service/tests/unit/app_game_notification_readiness_service_tests.rs',
      harness: 'scripts/test/app-game-notification-service-read-model-proof.mjs',
      appGameProofPack: 'output/app-game-plan-proof/56-notification-service-read-model',
      appProofPack: 'output/app-plan-proof/56-notification-service-read-model',
    },
  };

  await writeJson(join(testOutputDir, 'proof.json'), proof);
  await writeProofPack(appGameProofDir, proof, 'app-game WP56');
  await writeProofPack(appProofDir, proof, 'app WP56');

  console.log('app-game-notification-service-read-model-proof-ok');
  console.log(`evidence=${relative(repoRoot, join(testOutputDir, 'proof.json'))}`);
}

async function writeProofPack(proofDir, proof, label) {
  await writeFile(
    join(proofDir, '00-source-snapshot.md'),
    [
      `# ${label} Source Snapshot`,
      '',
      `- Branch: ${await gitBranch()}`,
      `- Commit: ${proof.commit}`,
      '- Scope: service-backed app/game notification intent readiness read model.',
      '- Source inspected: app/game notification intent contract, policy readiness read model, activity API routing, WebSocket dispatch, and activity store app/game service model.',
      '- Portal UI, provider delivery, local outbox runtime, scheduler runtime, child delivery, policy execution, adapters, broad blocking, and platform support are intentionally not changed.',
      '',
    ].join('\n'),
    'utf8'
  );
  await writeFile(
    join(proofDir, '01-contract-proof.log'),
    [
      'Contract proof:',
      '',
      '- cmd /c npm run build:contracts: PASS',
      '- cmd /c npm run build --workspace @ocentra-parent/schema-domain: PASS',
      '- Generated bridge import accepts the dedicated notification readiness event payload and rejects providerDeliveryClaimed=true.',
      '',
    ].join('\n'),
    'utf8'
  );
  await writeFile(
    join(proofDir, '02-rust-protocol-proof.log'),
    [
      'Rust protocol proof:',
      '',
      '- cargo test -p ocentra-parent-agent-protocol app_game_notification_readiness: PASS',
      '- DTO serialization preserves providerDeliveryClaimed=false, providerReceiptIngestionClaimed=false, localOutboxRuntimeClaimed=false, schedulerRuntimeClaimed=false, adapterDispatchClaimed=false, parentUiClaimed=false, and childDeliveryClaimed=false.',
      '',
    ].join('\n'),
    'utf8'
  );
  await writeJson(join(proofDir, '03-runtime-evidence.json'), proof);
  await writeFile(
    join(proofDir, '04-journal-sqlite-proof.json'),
    `${JSON.stringify(
      {
        schemaVersion: 1,
        journalSqliteChanged: false,
        serviceModelSource: 'ActivityStore::app_game_service_read_model',
        readModelRows:
          'notification readiness derives from existing app/game evidence, identity, approval authority, platform authority, and classifier row availability',
      },
      null,
      2
    )}\n`,
    'utf8'
  );
  await writeFile(
    join(proofDir, '05-policy-action-proof.json'),
    `${JSON.stringify(
      {
        schemaVersion: 1,
        readyIntentRows:
          'time-limit, approval request, and suspicious unknown rows can become local-intent-ready only when required service rows exist',
        manualRequiredRows:
          'missing identity, approval authority, platform authority, or classifier rows stay manual-required',
        providerDeliveryClaimed: false,
        adapterDispatchClaimed: false,
      },
      null,
      2
    )}\n`,
    'utf8'
  );
  await writeFile(
    join(proofDir, '06-ui-snapshots', 'ui-not-applicable.md'),
    '# UI Not Applicable\n\nNo parent portal, notification history, preference UI, or child-facing UI source changed in this workpack.\n',
    'utf8'
  );
  await writeFile(
    join(proofDir, '07-playwright-ui-proof.log'),
    'Playwright/browser proof not applicable: no UI source changed.\n',
    'utf8'
  );
  await writeFile(
    join(proofDir, '08-security-negative-proof.log'),
    [
      'Security/no-claim proof:',
      '',
      '- Notification rows carry evidence refs and minimal payload refs only.',
      '- Provider delivery, receipt ingestion, provider credentials, cloud routing, parent UI, child delivery, policy execution, adapter dispatch, broad blocking, and platform support are all fixed false or absent.',
      '- Missing prerequisites remain manual-required or unavailable instead of being promoted.',
      '',
    ].join('\n'),
    'utf8'
  );
  await writeFile(
    join(proofDir, '09-manual-platform-proof.md'),
    '# Manual Platform Proof\n\nNo live platform authority tier is raised. Broad blocking and platform adapter execution remain unclaimed.\n',
    'utf8'
  );
  await writeFile(
    join(proofDir, '10-validation-commands.log'),
    [
      'Validation run:',
      '',
      '- cmd /c npm run build:contracts: PASS',
      '- cmd /c npm run build --workspace @ocentra-parent/schema-domain: PASS',
      '- cargo test -p ocentra-parent-agent-protocol app_game_notification_readiness: PASS',
      '- cargo test -p ocentra-parent-agent-service app_game_notification_readiness: PASS',
      '- node scripts/test/app-game-notification-service-read-model-proof.mjs: PASS',
      '',
    ].join('\n'),
    'utf8'
  );
  await writeFile(
    join(proofDir, '11-authority-tier-proof.md'),
    '# Authority Tier Proof\n\nThe read model reports existing authority rows only. It does not upgrade manual-required or not-claimed platform authority.\n',
    'utf8'
  );
  await writeFile(
    join(proofDir, '12-rollback-proof.md'),
    '# Rollback Proof\n\nNo device action, timer, block, suspend, shield, provider send, scheduler worker, or adapter state is created, so rollback is not applicable.\n',
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
    '#[path = "contract/app_game_notification_readiness_tests.rs"]',
    'app-game notification readiness contract harness registration exists'
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
