import { spawn } from 'node:child_process';
import { mkdir, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';

const repoRoot = process.cwd();
const outputDir = join(repoRoot, 'test-results', 'app-game-adapter-dispatch-preflight-live-handoff-proof');
const proofPath = join(outputDir, 'proof.json');
const planOutputDir = join(
  repoRoot,
  'output',
  'app-game-plan-proof',
  '168-app-game-adapter-dispatch-preflight-live-handoff'
);
const planProofPath = join(planOutputDir, 'proof.json');
const commands = [];

await main();

async function main() {
  await mkdir(outputDir, { recursive: true });
  await mkdir(planOutputDir, { recursive: true });

  await runCommand(...npmCommand(['run', 'build:contracts']));
  await runCommand('cargo', ['test', '-p', 'ocentra-parent-agent-protocol', 'app_game_adapter_dispatch_preflight']);
  await runCommand('cargo', ['test', '-p', 'ocentra-parent-agent-service', 'app_game_adapter_dispatch_preflight']);
  await runCommand(
    ...npmCommand([
      'run',
      'test',
      '--workspace',
      '@ocentra-parent/portal',
      '--',
      'tests/unit/app-game-adapter-dispatch-route-panel.test.tsx',
    ])
  );
  await runCommand(...npmCommand(['run', 'build', '--workspace', '@ocentra-parent/portal']));

  const proof = {
    schemaVersion: 1,
    proofMode: 'app-game-adapter-dispatch-preflight-live-handoff-proof',
    checkedAt: new Date().toISOString(),
    commit: await gitHead(),
    commands,
    serviceCommand: 'agent.activity.app-game.adapter-dispatch-preflight.read-model.get',
    serviceEvent: 'agent.activity.app-game.adapter-dispatch-preflight.read-model.reported',
    payloadField: 'appGameAdapterDispatchPreflightReadModel',
    evidence: {
      generatedBridge: 'packages/schema-domain/src/app-game-adapter-dispatch-preflight.ts',
      rustProtocol: 'crates/agent-protocol/src/app_game_adapter_dispatch_preflight.rs',
      servicePayload: 'crates/agent-service/src/activity_api/app_game_adapter_dispatch_preflight_payload.rs',
      portalPanel: 'apps/portal/src/AppGameAdapterDispatchRoutePanel.tsx',
      liveState: 'apps/portal/src/live-activity-state.ts',
      commandSurface: 'apps/portal/src/AppGameAdapterDispatchRoutePanel.tsx',
    },
    summary: {
      expectedRows: 8,
      dispatchEligibleRows: 1,
      blockedBeforeDispatchRows: 7,
      adapterDispatchEligibleRows: 1,
      adapterDispatchExecutedClaimedRows: 0,
      broadInstalledAppBlockingClaimed: false,
      childDeviceDeliveryClaimed: false,
      platformEnforcementClaimed: false,
      providerDeliveryClaimed: false,
      privateDiagnosticsClaimed: false,
    },
    claimsProved: [
      'Generated bridge parses a dedicated app/game adapter dispatch preflight event',
      'Rust protocol exposes stable command and event names for the dispatch preflight read model',
      'Rust service WebSocket command emits the service-backed dispatch preflight payload',
      'portal-domain renders parent-safe summary and row details without adapter execution claim upgrades',
      'portal live state stores the parsed adapter dispatch preflight result',
    ],
    claimsNotProved: [
      'adapter dispatch execution',
      'broad installed-app blocking execution',
      'platform enforcement outside the scoped Windows owned-process boundary',
      'provider delivery or provider receipt ingestion',
      'child-device runtime delivery',
      'private diagnostics or raw target/source row exposure',
    ],
  };

  await writeFile(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
  await writeFile(planProofPath, `${JSON.stringify(proof, null, 2)}\n`);
  console.log('app-game-adapter-dispatch-preflight-live-handoff-proof-ok');
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
