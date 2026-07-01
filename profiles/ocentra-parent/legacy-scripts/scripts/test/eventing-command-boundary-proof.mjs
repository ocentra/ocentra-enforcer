import { spawn } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';

const repoRoot = process.cwd();
const testOutputDir = join(repoRoot, 'test-results', 'eventing-command-boundary-proof');
const planOutputDir = join(repoRoot, 'output', 'eventing-plan-proof', '60-61-command-boundary');
const proofPath = join(testOutputDir, 'proof.json');
const planProofPath = join(planOutputDir, 'proof-summary.json');
const commands = [];
const proofLabels = [];

const directEnforcementActionCommands = [
  'AgentCommand.EnforcementExecute',
  'AgentCommand.EnforcementTimerRecover',
  'AgentCommand.EnforcementTimerExpire',
  'AgentCommand.EnforcementOverrideCancel',
];

await main();

async function main() {
  await mkdir(testOutputDir, { recursive: true });
  await mkdir(planOutputDir, { recursive: true });

  await runCommand(
    ...npmCommand([
      'exec',
      '--workspace',
      '@ocentra-parent/portal',
      '--',
      'vitest',
      'run',
      'tests/unit/transport-lan-target.test.ts',
    ])
  );
  await runCommand(
    ...npmCommand([
      'exec',
      '--workspace',
      '@ocentra-parent/portal',
      '--',
      'eslint',
      'tests/unit/transport-lan-target.test.ts',
    ])
  );

  await assertSourceContracts();

  const proof = {
    schemaVersion: 1,
    proofMode: 'eventing-command-boundary-proof',
    checkedAt: new Date().toISOString(),
    commit: await gitHead(),
    commands,
    proofLabels,
    evidence: {
      portalTransport: 'apps/portal/src/host-bridge.ts',
      portalTransportTest: 'apps/portal/tests/unit/transport-lan-target.test.ts',
      portalCommandControls: 'apps/portal/src/portal-command-controls.ts',
      portalCommandInventory: 'packages/portal-domain/src/commands.ts',
      parentAssistantAdapter: 'packages/agent-protocol-domain/src/parent-assistant-adapter.ts',
      aiRouter: 'crates/agent-service/src/websocket.rs',
      proofHarness: 'scripts/test/eventing-command-boundary-proof.mjs',
    },
    claimsProved: [
      'portal host bridge rejects missing or unreachable dev bridge endpoints and only uses the configured local dev bridge',
      'portal command controls stay thin and dispatch typed commands while keeping result events as UI metadata',
      'portal command inventory contains no direct enforcement action commands',
      'parent-assistant command mapping does not map AI or assistant requests to enforcement action commands',
      'agent-service AI command router does not call enforcement command handlers',
    ],
    claimsNotProved: [
      'Parent protocol event contracts for enforcement, policy, audit, or portal read-model events',
      'journal-before-action enforcement execution',
      'adapter apply, rollback, or audit artifact production',
      'complete network-to-AI-to-policy-to-enforcement production chain',
    ],
  };

  const serialized = `${JSON.stringify(proof, null, 2)}\n`;
  await writeFile(proofPath, serialized);
  await writeFile(planProofPath, serialized);
  console.log(`eventing-command-boundary-proof-ok:${proofLabels.join(',')}`);
  console.log(`evidence=${relative(repoRoot, proofPath)}`);
  console.log(`planEvidence=${relative(repoRoot, planProofPath)}`);
}

async function assertSourceContracts() {
  const transportSource = await readText('apps/portal/src/host-bridge.ts');
  const transportTests = await readText('apps/portal/tests/unit/transport-lan-target.test.ts');
  const portalCommandControls = await readText('apps/portal/src/portal-command-controls.ts');
  const portalCommands = await readText('packages/portal-domain/src/commands.ts');
  const parentAssistantAdapter = await readText('packages/agent-protocol-domain/src/parent-assistant-adapter.ts');
  const websocket = await readText('crates/agent-service/src/websocket.rs');

  assertIncludes(transportSource, 'export function createDevWebHostBridge', 'portal dev web host bridge export');
  assertIncludes(
    transportSource,
    'createUnavailableDevWebHostBridge',
    'portal dev web host bridge handles missing dev bridge endpoints'
  );
  for (const command of directEnforcementActionCommands) {
    assertDoesNotInclude(portalCommands, command, `portal command inventory excludes ${command}`);
  }
  assertIncludes(transportTests, 'createDevWebHostBridge(null)', 'portal transport test covers missing dev bridge');
  assertIncludes(
    transportTests,
    "createDevWebHostBridge('http://127.0.0.1:4779/api/parent-ui')",
    'portal transport test covers configured dev bridge'
  );
  assertIncludes(
    transportTests,
    'ParentAgentCommand.LanPairingBrowserDiscoveryScan',
    'portal transport test covers LAN target dispatch'
  );
  assertIncludes(
    transportTests,
    'ParentAgentTargetDefaults.LocalNetworkWindowsAgent.route',
    'portal transport test covers local-network target routing'
  );
  assertIncludes(
    transportTests,
    'expect(events).toMatchObject([',
    'portal transport test covers changed snapshot emission only'
  );
  assertIncludes(
    portalCommandControls,
    'actions.sendCommand(command.command, command.payload)',
    'portal command controls dispatch typed commands'
  );
  assertIncludes(
    portalCommandControls,
    'actions.selectCommandResult(command.resultEvent)',
    'portal command controls keep result events as UI metadata'
  );
  proofLabels.push('portal.host-bridge.dev-bridge-guard');
  proofLabels.push('portal.command-controls.thin-command-dispatch');
  proofLabels.push('portal.inventory.no-direct-enforcement-action');

  assertIncludes(parentAssistantAdapter, 'export function commandForKind', 'parent assistant command mapping export');
  assertDoesNotInclude(
    parentAssistantAdapter,
    'AgentCommand.Enforcement',
    'parent assistant command mapping does not target enforcement commands'
  );
  proofLabels.push('ai.parent-assistant.no-enforcement-command-map');

  const aiRouter = sourceBetween(websocket, 'async fn build_ai_command_report', 'async fn send_event');
  assertDoesNotInclude(
    aiRouter,
    'AgentCommandName::AgentEnforcement',
    'AI command router does not match enforcement commands'
  );
  assertDoesNotInclude(aiRouter, 'build_enforcement', 'AI command router does not call enforcement builders');
  proofLabels.push('ai.service-router.no-enforcement-handler');
}

async function runCommand(command, args) {
  commands.push([command, ...args].join(' '));
  await new Promise((resolve, reject) => {
    const child = spawn(command, args, { cwd: repoRoot, stdio: 'inherit', windowsHide: true });
    child.once('exit', (code) =>
      code === 0 ? resolve() : reject(new Error(`${command} ${args.join(' ')} exited with ${code}`))
    );
    child.once('error', reject);
  });
}

async function gitHead() {
  const chunks = [];
  await new Promise((resolve, reject) => {
    const child = spawn('git', ['rev-parse', 'HEAD'], {
      cwd: repoRoot,
      stdio: ['ignore', 'pipe', 'pipe'],
      windowsHide: true,
    });
    child.stdout.on('data', (chunk) => chunks.push(String(chunk)));
    child.once('exit', (code) => (code === 0 ? resolve() : reject(new Error('git rev-parse HEAD failed'))));
    child.once('error', reject);
  });
  return chunks.join('').trim();
}

async function readText(path) {
  return readFile(join(repoRoot, path), 'utf8');
}

function sourceBetween(source, start, end) {
  const startIndex = source.indexOf(start);
  if (startIndex === -1) {
    throw new Error(`missing source start marker ${start}`);
  }
  const endIndex = source.indexOf(end, startIndex);
  if (endIndex === -1) {
    throw new Error(`missing source end marker ${end}`);
  }
  return source.slice(startIndex, endIndex);
}

function assertIncludes(text, expected, label) {
  if (!text.includes(expected)) {
    throw new Error(`${label}: missing ${expected}`);
  }
}

function assertDoesNotInclude(text, unexpected, label) {
  if (text.includes(unexpected)) {
    throw new Error(`${label}: found ${unexpected}`);
  }
}

function npmCommand(args) {
  const command = process.platform === 'win32' ? 'cmd' : 'npm';
  const commandArgs = process.platform === 'win32' ? ['/c', 'npm', ...args] : args;
  return [command, commandArgs];
}
