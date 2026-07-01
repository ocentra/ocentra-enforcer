import { spawn } from 'node:child_process';
import { mkdir, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';

const repoRoot = process.cwd();
const outputDir = join(repoRoot, 'test-results', 'app-game-scoped-adapter-execution-store-readback-proof');
const proofPath = join(outputDir, 'proof.json');
const planOutputDir = join(
  repoRoot,
  'output',
  'app-game-plan-proof',
  '172-app-game-scoped-adapter-execution-store-readback'
);
const planProofPath = join(planOutputDir, 'proof.json');
const commands = [];

await main();

async function main() {
  await mkdir(outputDir, { recursive: true });
  await mkdir(planOutputDir, { recursive: true });

  await runCommand(...npmCommand(['run', 'build:contracts']));
  await runCommand(
    ...npmCommand([
      'run',
      'test',
      '--workspace',
      '@ocentra-parent/agent-protocol-domain',
      '--',
      'app-game-adapter-dispatch-result',
    ])
  );
  await runCommand('cargo', ['test', '-p', 'ocentra-parent-agent-protocol', 'app_game_adapter_dispatch_result']);
  await runCommand('cargo', ['test', '-p', 'ocentra-parent-agent-core', 'activity_store_enforcement_audit']);
  await runCommand('cargo', ['test', '-p', 'ocentra-parent-agent-service', 'app_game_adapter_dispatch_result']);
  await runCommand('cargo', [
    'test',
    '-p',
    'ocentra-parent-agent-service',
    'enforcement_execute_records_audit_event_to_journal_and_store',
  ]);
  await runCommand(
    ...npmCommand([
      'run',
      'test',
      '--workspace',
      '@ocentra-parent/portal',
      '--',
      'app-game-adapter-dispatch-route-panel',
    ])
  );
  await runCommand(...npmCommand(['run', 'build', '--workspace', '@ocentra-parent/portal']));

  const proof = {
    schemaVersion: 1,
    proofMode: 'app-game-scoped-adapter-execution-store-readback-proof',
    checkedAt: new Date().toISOString(),
    commit: await gitHead(),
    commands,
    serviceCommand: 'agent.activity.app-game.adapter-dispatch-result.read-model.get',
    executionCommand: 'agent.enforcement.execute',
    executionEvent: 'agent.enforcement.audit.reported',
    payloadField: 'appGameAdapterDispatchResultReadModel',
    evidence: {
      storeQuery: 'crates/agent-core/src/activity_store_enforcement_audit.rs',
      sqliteConstant: 'crates/agent-protocol/src/constants/sqlite.rs',
      servicePayload: 'crates/agent-service/src/activity_api/app_game_adapter_dispatch_result_payload.rs',
      serviceReadbackTest: 'crates/agent-service/tests/unit/app_game_adapter_dispatch_result_service_tests.rs',
      realExecutionPath: 'crates/agent-service/src/enforcement_api.rs',
      portalPanel: 'apps/portal/src/AppGameAdapterDispatchRoutePanel.tsx',
    },
    summary: {
      expectedRows: 8,
      commandAcceptedRows: 1,
      liveStoreExecutionEvidenceRows: 1,
      liveStoreExecutionEvidenceMissingRowsAfterAudit: 0,
      blockedBeforeAdapterExecutionRows: 7,
      broadInstalledAppBlockingClaimed: false,
      childDeviceDeliveryClaimed: false,
      platformEnforcementClaimed: false,
      providerDeliveryClaimed: false,
      privateDiagnosticsClaimed: false,
    },
    claimsProved: [
      'ActivityStore exposes the latest persisted enforcement audit fields through a typed query boundary',
      'the app/game adapter dispatch-result service command reads real persisted enforcement audit evidence from ActivityStore',
      'a real agent.enforcement.execute call can seed the store and the next dispatch-result read-model command reports that evidence',
      'only the scoped Windows owned-process app/game timer row can report store-backed adapter execution evidence',
      'broad/manual/degraded/unavailable/unsupported rows remain blocked before adapter execution',
      'portal-domain rendering remains parent-safe for result/status/refs',
    ],
    claimsNotProved: [
      'broad installed-app blocking execution',
      'platform enforcement outside the scoped Windows owned-process boundary',
      'provider delivery or provider receipt ingestion',
      'child-device runtime delivery',
      'raw private source rows, raw target values, or private diagnostics',
    ],
  };

  await writeFile(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
  await writeFile(planProofPath, `${JSON.stringify(proof, null, 2)}\n`);
  console.log('app-game-scoped-adapter-execution-store-readback-proof-ok');
  console.log(`evidence=${relative(repoRoot, proofPath)}`);
  console.log(`planEvidence=${relative(repoRoot, planProofPath)}`);
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

function npmCommand(args) {
  const command = process.platform === 'win32' ? 'cmd' : 'npm';
  const commandArgs = process.platform === 'win32' ? ['/c', 'npm', ...args] : args;
  return [command, commandArgs];
}
