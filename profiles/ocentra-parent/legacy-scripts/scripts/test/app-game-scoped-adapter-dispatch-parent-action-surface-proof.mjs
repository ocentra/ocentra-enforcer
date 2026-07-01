import { spawn } from 'node:child_process';
import { mkdir, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';

const repoRoot = process.cwd();
const outputDir = join(repoRoot, 'test-results', 'app-game-scoped-adapter-dispatch-parent-action-surface-proof');
const proofPath = join(outputDir, 'proof.json');
const planOutputDir = join(
  repoRoot,
  'output',
  'app-game-plan-proof',
  '175-app-game-scoped-adapter-dispatch-parent-action-surface'
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
      '@ocentra-parent/portal',
      '--',
      'tests/unit/app-game-adapter-dispatch-route-panel.test.tsx',
    ])
  );
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
    proofMode: 'app-game-scoped-adapter-dispatch-parent-action-surface-proof',
    checkedAt: new Date().toISOString(),
    commit: await gitHead(),
    commands,
    sourceCommand: 'agent.activity.app-game.adapter-dispatch.execute',
    sourceEvent: 'agent.activity.app-game.adapter-dispatch.executed',
    sideEffectFreeCommands: [
      'agent.activity.app-game.adapter-dispatch-preflight.read-model.get',
      'agent.activity.app-game.adapter-dispatch-result.read-model.get',
    ],
    evidence: {
      routePanel: 'apps/portal/src/AppGameAdapterDispatchRoutePanel.tsx',
      routeMount: 'apps/portal/src/ParentPortalRoute.tsx',
      routePanelTest: 'apps/portal/tests/unit/app-game-adapter-dispatch-route-panel.test.tsx',
      portalPanelIntent: 'apps/portal/src/AppGameAdapterDispatchRoutePanel.tsx',
      portalPanelIntentTest: 'apps/portal/tests/unit/app-game-adapter-dispatch-route-panel.test.tsx',
    },
    summary: {
      appGameSessionsRouteMounted: true,
      executeButtonRequiresAcceptedScopedRow: true,
      executeButtonSendsTypedCommand: true,
      executeButtonSelectsExecutedEvent: true,
      overviewAutoExecute: false,
      broadInstalledAppBlockingClaimed: false,
      childDeviceDeliveryClaimed: false,
      platformEnforcementClaimed: false,
      providerDeliveryClaimed: false,
      privateDiagnosticsClaimed: false,
    },
    claimsProved: [
      'the App/Game Sessions product route mounts the adapter dispatch parent surface',
      'side-effect-free preflight and result read models remain refresh commands',
      'manual execute is exposed only when the scoped Windows owned-process app/game timer row is accepted',
      'manual execute sends agent.activity.app-game.adapter-dispatch.execute and selects the executed event',
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
  console.log('app-game-scoped-adapter-dispatch-parent-action-surface-proof-ok');
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
