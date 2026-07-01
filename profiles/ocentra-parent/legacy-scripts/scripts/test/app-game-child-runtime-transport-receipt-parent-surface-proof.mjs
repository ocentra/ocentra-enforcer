import { spawn } from 'node:child_process';
import { mkdir, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';

const repoRoot = process.cwd();
const proofMode = 'app-game-child-runtime-transport-receipt-parent-surface-proof';
const outputDir = join(repoRoot, 'test-results', proofMode);
const proofPath = join(outputDir, 'proof.json');
const appGameProofDir = join(
  repoRoot,
  'output',
  'app-game-plan-proof',
  '210-app-game-child-runtime-transport-receipt-parent-surface'
);
const commands = [];

await main();

async function main() {
  await mkdir(outputDir, { recursive: true });
  await mkdir(appGameProofDir, { recursive: true });

  await runCommand(...npmCommand(['run', 'build:contracts']));
  await runCommand(
    ...npmCommand([
      'exec',
      '--workspace',
      '@ocentra-parent/portal-domain',
      '--',
      'vitest',
      'run',
      'tests/unit/contracts.test.ts',
      '-t',
      'PortalCommandButtons',
    ])
  );
  await runCommand(
    ...npmCommand([
      'exec',
      '--workspace',
      '@ocentra-parent/portal-domain',
      '--',
      'vitest',
      'run',
      'tests/unit/contracts.test.ts',
      '-t',
      'PortalOverviewCommands',
    ])
  );
  await runCommand(
    ...npmCommand([
      'run',
      'test',
      '--workspace',
      '@ocentra-parent/portal',
      '--',
      'app-game-child-runtime-transport-receipt-route-panel',
    ])
  );

  const branch = await gitBranch();
  const commit = await gitHead();
  const proof = {
    schemaVersion: 1,
    proofMode,
    checkedAt: 'deterministic-proof-artifact',
    branch,
    commit,
    commands,
    parentSurface: {
      route: 'app-game-sessions',
      command: 'agent.activity.app-game.child-runtime-transport-receipt.read-model.get',
      event: 'agent.activity.app-game.child-runtime-transport-receipt.read-model.reported',
      payloadField: 'appGameChildRuntimeTransportReceiptReadModel',
      rowsRendered: ['child-runtime-transport-required', 'manual-required', 'unavailable'],
      runtimeTransportExecuted: false,
      runtimeReceiptIngested: false,
      providerDeliveryExecuted: false,
      platformDeliveryChannelClaimed: false,
      adapterDispatchClaimed: false,
      platformEnforcementClaimed: false,
      rawPrivateSourceRowsIncluded: false,
    },
    evidence: {
      portalCommands: 'packages/portal-domain/src/commands.ts',
      portalCommandsTest: 'packages/portal-domain/tests/unit/contracts.test.ts',
      portalRoutePanel: 'apps/portal/src/AppGameChildRuntimeTransportReceiptRoutePanel.tsx',
      portalRoutePanelTest: 'apps/portal/tests/unit/app-game-child-runtime-transport-receipt-route-panel.test.ts',
      portalRouteBridgeGuard: 'apps/portal/tests/unit/parent-ui-bridge.test.ts',
    },
    claimsProved: [
      'The portal-domain command catalog includes the child runtime transport receipt read-model request and result event',
      'The product route snapshot bridge exposes the Rust-owned child runtime transport receipt panel to the portal surface',
      'The portal route panel renders transport-required, manual-required, and unavailable rows with parent-safe refs',
      'The portal parent surface keeps runtime transport, receipt ingestion, provider delivery, platform channel delivery, adapter dispatch, platform enforcement, and raw private rows unclaimed',
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
      '# WP210 app/game child runtime transport receipt parent surface source snapshot',
      '',
      `- Branch: ${proof.branch}`,
      `- Commit: ${proof.commit}`,
      '- Portal commands: packages/portal-domain/src/commands.ts',
      '- Portal route panel: apps/portal/src/AppGameChildRuntimeTransportReceiptRoutePanel.tsx',
      '- Route bridge guard: apps/portal/tests/unit/parent-ui-bridge.test.ts',
      '',
      'Evidence:',
      '- The portal-domain command catalog requests the service-backed child runtime transport receipt read model.',
      '- The portal-domain panel renders transport-required, manual-required, and unavailable rows.',
      '- Runtime transport execution, receipt ingestion, provider delivery, adapter dispatch, and platform enforcement stay unclaimed.',
      '',
    ].join('\n')
  );
  await writeFile(join(appGameProofDir, '10-validation-commands.log'), `${commands.join('\n')}\n`);

  console.log('app-game-child-runtime-transport-receipt-parent-surface-proof-ok');
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

async function gitBranch() {
  const chunks = [];
  await new Promise((resolve, reject) => {
    const child = spawn('git', ['rev-parse', '--abbrev-ref', 'HEAD'], { cwd: repoRoot, stdio: ['ignore', 'pipe', 'pipe'] });
    child.stdout.on('data', (chunk) => chunks.push(String(chunk)));
    child.once('exit', (code) => (code === 0 ? resolve() : reject(new Error('git rev-parse --abbrev-ref HEAD failed'))));
    child.once('error', reject);
  });
  return chunks.join('').trim();
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
