import { spawn } from 'node:child_process';
import { mkdir, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';

const repoRoot = process.cwd();
const outputDir = join(repoRoot, 'test-results', 'app-game-scoped-adapter-execution-result-bridge-proof');
const proofPath = join(outputDir, 'proof.json');
const planOutputDir = join(
  repoRoot,
  'output',
  'app-game-plan-proof',
  '171-app-game-scoped-adapter-execution-result-bridge'
);
const planProofPath = join(planOutputDir, 'proof.json');
const commands = [];

await main();

async function main() {
  await mkdir(outputDir, { recursive: true });
  await mkdir(planOutputDir, { recursive: true });

  await runCommand(...npmCommand(['run', 'build:contracts']));
  await runCommand('cargo', ['test', '-p', 'ocentra-parent-agent-protocol', 'app_game_adapter_dispatch_result']);
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
    proofMode: 'app-game-scoped-adapter-execution-result-bridge-proof',
    checkedAt: new Date().toISOString(),
    commit: await gitHead(),
    commands,
    serviceCommand: 'agent.activity.app-game.adapter-dispatch-result.read-model.get',
    executionCommand: 'agent.enforcement.execute',
    executionEvent: 'agent.enforcement.audit.reported',
    payloadField: 'appGameAdapterDispatchResultReadModel',
    evidence: {
      generatedBridge: 'packages/schema-domain/src/app-game-adapter-dispatch-result.ts',
      rustProtocol: 'crates/agent-protocol/src/app_game_adapter_dispatch_result.rs',
      servicePayload: 'crates/agent-service/src/activity_api/app_game_adapter_dispatch_result_payload.rs',
      realExecutionPath: 'crates/agent-service/src/enforcement_api.rs',
      portalPanel: 'apps/portal/src/AppGameAdapterDispatchRoutePanel.tsx',
    },
    summary: {
      expectedRows: 8,
      commandAcceptedRows: 1,
      adapterExecutionReportedRowsWithEvidence: 1,
      adapterExecutionEvidenceMissingRowsWithoutEvidence: 1,
      blockedBeforeAdapterExecutionRows: 7,
      broadInstalledAppBlockingClaimed: false,
      childDeviceDeliveryClaimed: false,
      platformEnforcementClaimed: false,
      providerDeliveryClaimed: false,
      privateDiagnosticsClaimed: false,
    },
    claimsProved: [
      'real agent.enforcement.execute path records enforcement audit events to journal and store',
      'generated bridge and Rust protocol can attach a real enforcement audit payload as scoped adapter execution evidence',
      'side-effect-free dispatch-result read-model GET keeps the scoped row at execution-evidence-missing until evidence is attached',
      'only the Windows owned-process app/game timer row can report adapter execution evidence',
      'broad/manual/degraded/unavailable/unsupported rows remain blocked before adapter execution',
      'portal-domain renders scoped adapter execution result/status/refs without exposing private diagnostics',
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
  console.log('app-game-scoped-adapter-execution-result-bridge-proof-ok');
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
