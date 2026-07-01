import { spawn } from 'node:child_process';
import { mkdir, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';

const repoRoot = process.cwd();
const outputDir = join(repoRoot, 'test-results', 'app-game-adapter-execution-readiness-live-surface-proof');
const proofPath = join(outputDir, 'proof.json');
const planOutputDir = join(
  repoRoot,
  'output',
  'app-game-plan-proof',
  '167-app-game-adapter-execution-readiness-live-surface'
);
const planProofPath = join(planOutputDir, 'proof.json');
const commands = [];

await main();

async function main() {
  await mkdir(outputDir, { recursive: true });
  await mkdir(planOutputDir, { recursive: true });

  await runCommand(...npmCommand(['run', 'build:contracts']));
  await runCommand('cargo', ['test', '-p', 'ocentra-parent-agent-protocol', 'app_game_adapter_execution_readiness']);
  await runCommand('cargo', ['test', '-p', 'ocentra-parent-agent-service', 'app_game_adapter_execution_readiness']);
  await runCommand(
    ...npmCommand([
      'run',
      'test',
      '--workspace',
      '@ocentra-parent/portal-domain',
      '--',
      'app-game-adapter-execution-readiness-panel',
      'contracts',
    ])
  );
  await runCommand(...npmCommand(['run', 'build', '--workspace', '@ocentra-parent/portal']));

  const proof = {
    schemaVersion: 1,
    proofMode: 'app-game-adapter-execution-readiness-live-surface-proof',
    checkedAt: new Date().toISOString(),
    commit: await gitHead(),
    commands,
    serviceCommand: 'agent.activity.app-game.adapter-execution-readiness.read-model.get',
    serviceEvent: 'agent.activity.app-game.adapter-execution-readiness.read-model.reported',
    payloadField: 'appGameAdapterExecutionReadinessReadModel',
    evidence: {
      generatedBridge: 'packages/schema-domain/src/app-game-adapter-execution-readiness.ts',
      rustProtocol: 'crates/agent-protocol/src/app_game_adapter_execution_readiness.rs',
      servicePayload: 'crates/agent-service/src/activity_api/app_game_adapter_execution_readiness_payload.rs',
      portalPanel: 'packages/portal-domain/src/app-game-adapter-execution-readiness-panel.ts',
      liveState: 'packages/portal-domain/src/live-activity-state.ts',
      commandSurface: 'packages/portal-domain/src/commands.ts',
    },
    summary: {
      expectedRows: 8,
      executionAllowedRows: 1,
      blockedBeforeExecutionRows: 7,
      adapterExecutionClaimedRows: 1,
      broadInstalledAppBlockingClaimed: false,
      childDeviceDeliveryClaimed: false,
      platformEnforcementClaimed: false,
      providerDeliveryClaimed: false,
      privateDiagnosticsClaimed: false,
    },
    claimsProved: [
      'Generated bridge parses a dedicated app/game adapter execution readiness event',
      'Rust protocol exposes stable command and event names for the read model',
      'Rust service WebSocket command emits the service-backed read model payload',
      'portal-domain renders parent-safe summary and row details without claim upgrades',
      'portal live state stores the parsed adapter execution readiness result',
    ],
    claimsNotProved: [
      'broad installed-app blocking execution',
      'platform enforcement outside the scoped Windows owned-process boundary',
      'provider delivery or provider receipt ingestion',
      'child-device runtime delivery',
      'private diagnostics or raw target/source row exposure',
    ],
  };

  await writeFile(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
  await writeFile(planProofPath, `${JSON.stringify(proof, null, 2)}\n`);
  console.log('app-game-adapter-execution-readiness-live-surface-proof-ok');
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
