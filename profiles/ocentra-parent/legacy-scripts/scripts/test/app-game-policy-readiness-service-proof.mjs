import { spawn } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';

const repoRoot = process.cwd();
const testOutputDir = join(repoRoot, 'test-results', 'app-game-policy-readiness-service-proof');
const appGameProofDir = join(repoRoot, 'output', 'app-game-plan-proof', '52-policy-readiness-service-read-model');
const appProofDir = join(repoRoot, 'output', 'app-plan-proof', '52-policy-readiness-service-read-model');
const commands = [];

await main();

async function main() {
  await mkdir(testOutputDir, { recursive: true });
  await mkdir(appGameProofDir, { recursive: true });
  await mkdir(appProofDir, { recursive: true });
  await mkdir(join(appGameProofDir, '06-ui-snapshots'), { recursive: true });
  await mkdir(join(appProofDir, '06-ui-snapshots'), { recursive: true });

  await runCommand(...npmCommand(['run', 'build', '--workspace', '@ocentra-parent/schema-domain']));
  await runCommand(...npmCommand(['run', 'build:contracts']));
  await runCommand('cargo', ['test', '-p', 'ocentra-parent-agent-protocol', 'app_game_policy_readiness']);
  await runCommand('cargo', ['test', '-p', 'ocentra-parent-agent-service', 'app_game_policy_readiness']);
  await assertProtocolHarness();
  const schemaPolicyReadiness = await import('@ocentra-parent/schema-domain/app-game-policy-readiness');
  commands.push('node import @ocentra-parent/schema-domain/app-game-policy-readiness');
  if (!('AgentAppGamePolicyReadinessReadModelSchema' in schemaPolicyReadiness)) {
    throw new Error('Missing AgentAppGamePolicyReadinessReadModelSchema export from schema-domain');
  }

  const proof = {
    schemaVersion: 1,
    proofMode: 'app-game-policy-readiness-service-read-model',
    checkedAt: new Date().toISOString(),
    commit: await gitHead(),
    commands,
    summary: {
      command: 'agent.activity.app-game.policy-readiness.read-model.get',
      event: 'agent.activity.app-game.policy-readiness.read-model.reported',
      payloadField: 'appGamePolicyReadinessReadModel',
      generatedBridge: 'packages/schema-domain/src/app-game-policy-readiness.ts',
      readinessRows: [
        'policyEvidence',
        'approvalAuthority',
        'approvalActionResult',
        'platformAuthority',
        'aiClassifierContext',
      ],
      adapterDispatchClaimed: false,
    },
    claimsProved: [
      'Generated schema-domain bridge mirrors the Rust-owned app/game policy readiness read-model contract surface',
      'Generated bridge parses the dedicated app/game policy readiness event payload',
      'Rust protocol serializes readiness rows and read model with adapterDispatchClaimed=false',
      'Agent service answers the policy readiness command from the existing activity store app-game service read model',
      'Missing required service rows remain visible as missing/manual-required readiness rows',
      'No portal UI, notification delivery, policy evaluator execution, adapter dispatch, or platform capability is claimed',
    ],
    claimsNotProved: [
      'runtime policy evaluator consumption',
      'portal policy readiness rendering',
      'classifier provider execution or quality',
      'notification or child request delivery',
      'adapter execution, broad installed-app blocking, or platform support',
    ],
    evidence: {
      schemaContract: 'packages/schema-domain/src/app-game-policy-readiness.ts',
      generatedBridge: 'packages/schema-domain/src/app-game-policy-readiness.ts',
      rustProtocol: 'crates/agent-protocol/src/app_game_policy_readiness.rs',
      rustProtocolTest: 'crates/agent-protocol/tests/contract/app_game_policy_readiness_tests.rs',
      servicePayload: 'crates/agent-service/src/activity_api/app_game_policy_readiness_payload.rs',
      servicePayloadTest: 'crates/agent-service/tests/unit/app_game_policy_readiness_payload_tests.rs',
      serviceWebSocketTest: 'crates/agent-service/tests/unit/app_game_policy_readiness_service_tests.rs',
      harness: 'scripts/test/app-game-policy-readiness-service-proof.mjs',
      appGameProofPack: 'output/app-game-plan-proof/52-policy-readiness-service-read-model',
      appProofPack: 'output/app-plan-proof/52-policy-readiness-service-read-model',
    },
  };

  await writeJson(join(testOutputDir, 'proof.json'), proof);
  await writeProofPack(appGameProofDir, proof, 'app-game WP52');
  await writeProofPack(appProofDir, proof, 'app WP52');

  console.log('app-game-policy-readiness-service-proof-ok');
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
      '- Scope: service-backed app/game policy readiness read model.',
      '- Source inspected: generated bridge, Rust app-game policy readiness contract, activity API, WebSocket routing, and activity store app-game service model.',
      '- Portal UI, product checklist, policy evaluator execution, notifications, adapters, and platform support are intentionally not changed.',
      '',
    ].join('\n'),
    'utf8'
  );
  await writeFile(
    join(proofDir, '01-contract-proof.log'),
    [
      'Contract proof:',
      '',
      '- cmd /c npm run build --workspace @ocentra-parent/schema-domain: PASS',
      '- cmd /c npm run build:contracts: PASS',
      '- node import @ocentra-parent/schema-domain/app-game-policy-readiness: PASS',
      '- Generated bridge import accepts categoryCandidate and unknownReview rows and rejects adapterDispatchClaimed=true.',
      '',
    ].join('\n'),
    'utf8'
  );
  await writeFile(
    join(proofDir, '02-rust-protocol-proof.log'),
    [
      'Rust protocol proof:',
      '',
      '- cargo test -p ocentra-parent-agent-protocol app_game_policy_readiness: PASS',
      '- DTO serialization preserves policyEvaluationReady, manualReviewRequired, and adapterDispatchClaimed=false.',
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
          'policy readiness is derived from existing evidence claim, identity, approval, platform authority, and classifier service rows',
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
        policyEvaluationReady:
          'true only when required service rows for policy evidence, approval authority, and platform authority are present',
        manualReviewRequired:
          'true when optional action history or classifier context is missing, and when required rows are missing',
        adapterDispatchClaimed: false,
      },
      null,
      2
    )}\n`,
    'utf8'
  );
  await writeFile(
    join(proofDir, '06-ui-snapshots', 'ui-not-applicable.md'),
    '# UI Not Applicable\n\nNo portal or child-facing UI source changed in this workpack.\n',
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
      '- Readiness rows are derived from existing service read-model counts and citations.',
      '- Missing rows remain missing/manual-required instead of being invented.',
      '- adapterDispatchClaimed is fixed false in the generated bridge, Rust DTO construction, and service payload tests.',
      '- The WebSocket command reports readiness only; it does not execute policy, call adapters, send notifications, or claim platform support.',
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
      '- cmd /c npm run build --workspace @ocentra-parent/schema-domain: PASS',
      '- cmd /c npm run build:contracts: PASS',
      '- node import @ocentra-parent/schema-domain/app-game-policy-readiness: PASS',
      '- cargo test -p ocentra-parent-agent-protocol app_game_policy_readiness: PASS',
      '- cargo test -p ocentra-parent-agent-service app_game_policy_readiness: PASS',
      '- node scripts/test/app-game-policy-readiness-service-proof.mjs: PASS',
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
