import { spawn } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';

const repoRoot = process.cwd();
const proofMode = 'app-game-child-runtime-transport-receipt-service-proof';
const outputDir = join(repoRoot, 'test-results', proofMode);
const proofPath = join(outputDir, 'proof.json');
const appGameProofDir = join(
  repoRoot,
  'output',
  'app-game-plan-proof',
  '209-app-game-child-runtime-transport-receipt-service'
);
const commands = [];

await main();

async function main() {
  await mkdir(outputDir, { recursive: true });
  await mkdir(appGameProofDir, { recursive: true });

  await runCommand(...npmCommand(['run', 'build:contracts']));
  await runCommand('cargo', [
    'test',
    '-p',
    'ocentra-parent-agent-protocol',
    'app_game_child_runtime_transport_receipt',
  ]);
  await runCommand('cargo', ['test', '-p', 'ocentra-parent-agent-service', 'app_game_child_runtime_transport_receipt']);
  await assertProtocolHarness();

  const proof = {
    schemaVersion: 1,
    proofMode,
    checkedAt: 'deterministic-proof-artifact',
    commit: await gitHead(),
    commands,
    serviceSurface: {
      command: 'agent.activity.app-game.child-runtime-transport-receipt.read-model.get',
      event: 'agent.activity.app-game.child-runtime-transport-receipt.read-model.reported',
      payloadField: 'appGameChildRuntimeTransportReceiptReadModel',
      readModelId: 'app-game-child-runtime-transport-receipt',
      returnedRows: 4,
      transportRequiredRows: 2,
      manualRequiredRows: 1,
      unavailableRows: 1,
      runtimeTransportExecuted: false,
      runtimeReceiptIngested: false,
      providerDeliveryExecuted: false,
      platformDeliveryChannelClaimed: false,
      adapterDispatchClaimed: false,
      platformEnforcementClaimed: false,
      rawPrivateSourceRowsIncluded: false,
    },
    evidence: {
      schemaContract: 'packages/schema-domain/src/app-game-child-runtime-transport-receipt.ts',
      generatedBridge: 'packages/schema-domain/src/app-game-child-runtime-transport-receipt.ts',
      rustProtocol: 'crates/agent-protocol/src/app_game_child_runtime_transport_receipt.rs',
      rustProtocolTest:
        'crates/agent-protocol/tests/contract/app_game_child_runtime_transport_receipt_tests.rs',
      rustService: 'crates/agent-service/src/activity_api/app_game_child_runtime_transport_receipt_payload.rs',
      rustPayloadTest:
        'crates/agent-service/tests/unit/app_game_child_runtime_transport_receipt_payload_tests.rs',
      rustServiceTest:
        'crates/agent-service/tests/unit/app_game_child_runtime_transport_receipt_service_tests.rs',
      websocketRoute: 'crates/agent-service/src/websocket.rs',
    },
    claimsProved: [
      'The generated schema-domain bridge mirrors the Rust-owned app/game child runtime transport receipt payload field and read-model shape',
      'The Rust agent service accepts agent.activity.app-game.child-runtime-transport-receipt.read-model.get',
      'The service reports agent.activity.app-game.child-runtime-transport-receipt.read-model.reported',
      'The generated bridge accepts the reported payload field and rejects malformed or overclaiming payloads',
      'The Rust protocol serializes the read model with runtime transport, receipt ingestion, provider delivery, platform channel, adapter dispatch, platform enforcement, and raw private source rows unclaimed',
    ],
    claimsNotProved: [
      'Child runtime transport execution',
      'Child runtime receipt ingestion',
      'Provider delivery execution',
      'Platform delivery channel execution',
      'Adapter dispatch or platform enforcement',
    ],
  };

  await writeJson(proofPath, proof);
  await writeJson(join(appGameProofDir, 'proof.json'), proof);
  await writeFile(
    join(appGameProofDir, '00-source-snapshot.md'),
    [
      '# WP209 app/game child runtime transport receipt service source snapshot',
      '',
      '- Branch: codex/app-game-control-product-completion',
      '- Commit: uncommitted full-goal batch, validated by harness before final checkpoint commit',
      '- Schema contract: packages/schema-domain/src/app-game-child-runtime-transport-receipt.ts',
      '- Generated bridge: packages/schema-domain/src/app-game-child-runtime-transport-receipt.ts',
      '- Rust protocol: crates/agent-protocol/src/app_game_child_runtime_transport_receipt.rs',
      '- Rust service: crates/agent-service/src/activity_api/app_game_child_runtime_transport_receipt_payload.rs',
      '- WebSocket route: crates/agent-service/src/websocket.rs',
      '',
      'Evidence:',
      '- The command/event pair is registered in the generated bridge and Rust protocol contracts.',
      '- The Rust service returns a live read model through the WebSocket command handler.',
      '- Runtime transport execution, receipt ingestion, provider delivery, adapter dispatch, and platform enforcement stay unclaimed.',
      '',
    ].join('\n')
  );
  await writeFile(join(appGameProofDir, '10-validation-commands.log'), `${commands.join('\n')}\n`);

  console.log('app-game-child-runtime-transport-receipt-service-proof-ok');
  console.log(`evidence=${relativePath(proofPath)}`);
}

async function runCommand(command, args) {
  const commandLine = [command, ...args].join(' ');
  commands.push(commandLine);
  await new Promise((resolve, reject) => {
    const child = spawn(command, args, { cwd: repoRoot, stdio: 'inherit', windowsHide: true });
    child.once('exit', (code) => (code === 0 ? resolve() : reject(new Error(`${commandLine} exited with ${code}`))));
    child.once('error', reject);
  });
}

async function gitHead() {
  const chunks = [];
  await new Promise((resolve, reject) => {
    const child = spawn('git', ['rev-parse', 'HEAD'], { cwd: repoRoot, stdio: ['ignore', 'pipe', 'pipe'] });
    child.stdout.on('data', (chunk) => chunks.push(String(chunk)));
    child.once('exit', (code) => (code === 0 ? resolve() : reject(new Error('git rev-parse HEAD failed'))));
    child.once('error', reject);
  });
  return chunks.join('').trim();
}

async function assertProtocolHarness() {
  const harness = await readFile(join(repoRoot, 'crates/agent-protocol/tests/contract.rs'), 'utf8');
  assertIncludes(
    harness,
    '#[path = "contract/app_game_child_runtime_transport_receipt_tests.rs"]',
    'app-game child runtime transport receipt contract harness registration exists'
  );
}

async function writeJson(path, value) {
  await writeFile(path, `${JSON.stringify(value, null, 2)}\n`);
}

function relativePath(path) {
  return relative(repoRoot, path).replaceAll('\\', '/');
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
