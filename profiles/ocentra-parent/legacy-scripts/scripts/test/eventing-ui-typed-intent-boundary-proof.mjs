import { spawn } from 'node:child_process';
import { mkdir, readdir, readFile, writeFile } from 'node:fs/promises';
import { extname, join, relative } from 'node:path';

const repoRoot = process.cwd();
const testOutputDir = join(repoRoot, 'test-results', 'eventing-ui-typed-intent-boundary-proof');
const planOutputDir = join(repoRoot, 'output', 'eventing-plan-proof', '52-ui-typed-intent-boundary');
const proofPath = join(testOutputDir, 'proof.json');
const planProofPath = join(planOutputDir, 'proof-summary.json');
const commands = [];
const proofLabels = [];

const sourceRoots = ['apps/portal/src'];
const sourceExtensions = new Set(['.ts', '.tsx']);
const forbiddenPublisherPatterns = [
  {
    label: 'no reusable Rust eventing import in portal',
    pattern: /ocentra-eventing|@ocentra-parent\/eventing|EventBus|NetworkEventBus/u,
  },
  {
    label: 'no portal event publish function',
    pattern: /(?:^|[^\w])(?:publishEvent|publishBusinessEvent|publishDomainEvent|createEventPublisher)\s*\(/u,
  },
  {
    label: 'no portal event bus publish call',
    pattern: /(?:eventBus|bus|publisher)\.publish\s*\(/u,
  },
  {
    label: 'no portal event subscription ownership',
    pattern: /(?:eventBus|bus)\.subscribe\s*\(/u,
  },
  {
    label: 'no portal event envelope send',
    pattern: /AgentEventEnvelopeSchema\.parse\s*\(\s*\{/u,
  },
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
      'src/host-bridge.ts',
      'src/main.ts',
      'src/portal-actions.ts',
      'src/portal-command-controls.ts',
      'tests/unit/transport-lan-target.test.ts',
    ])
  );
  await runCommand('node', ['scripts/check-source-shape.mjs', 'crates/ocentra-eventing']);

  const scannedFiles = await assertPortalTypedIntentBoundary();

  const proof = {
    schemaVersion: 1,
    proofMode: 'eventing-ui-typed-intent-boundary-proof',
    checkedAt: new Date().toISOString(),
    commit: await gitHead(),
    commands,
    proofLabels,
    evidence: {
      portalHostBridge: 'apps/portal/src/host-bridge.ts',
      portalMain: 'apps/portal/src/main.ts',
      portalActions: 'apps/portal/src/portal-actions.ts',
      portalCommandControls: 'apps/portal/src/portal-command-controls.ts',
      portalTransportTests: 'apps/portal/tests/unit/transport-lan-target.test.ts',
      proofHarness: 'scripts/test/eventing-ui-typed-intent-boundary-proof.mjs',
      scannedSourceRoots: sourceRoots,
      scannedFiles,
    },
    claimsProved: [
      'portal host bridge routes Tauri and dev-web traffic through typed bridge commands and dev bridge URLs',
      'portal main dispatches typed AgentCommandRequested actions through the host bridge',
      'portal command controls send AgentCommandName intents and keep AgentEventName values as result-selection metadata',
      'portal LAN transport unit test covers the current dev-bridge path and LAN target dispatch surface',
      'portal source contains no event bus imports, event publish calls, or event subscription ownership',
      'Rust service remains the owner of business event publishing for this boundary',
    ],
    claimsNotProved: [
      'Parent-specific event contracts outside the portal bridge and typed-intent surfaces',
      'Rust parent/controller validated intent publisher outside the portal bridge',
      'child-agent command transport and local publish outside the portal bridge',
      'journal-before-action enforcement or adapter-result audit/read-model integration outside the portal bridge',
    ],
  };

  const serialized = `${JSON.stringify(proof, null, 2)}\n`;
  await writeFile(proofPath, serialized);
  await writeFile(planProofPath, serialized);
  console.log(`eventing-ui-typed-intent-boundary-proof-ok:${proofLabels.join(',')}`);
  console.log(`evidence=${relative(repoRoot, proofPath)}`);
  console.log(`planEvidence=${relative(repoRoot, planProofPath)}`);
}

async function assertPortalTypedIntentBoundary() {
  const hostBridge = await readText('apps/portal/src/host-bridge.ts');
  const main = await readText('apps/portal/src/main.ts');
  const portalActions = await readText('apps/portal/src/portal-actions.ts');
  const commandControls = await readText('apps/portal/src/portal-command-controls.ts');
  const transportTest = await readText('apps/portal/tests/unit/transport-lan-target.test.ts');

  assertIncludes(
    hostBridge,
    'export function createHostBridge',
    'portal host bridge exports the bridge factory'
  );
  assertIncludes(
    hostBridge,
    'createDevWebHostBridge',
    'portal host bridge exposes the dev web bridge'
  );
  assertIncludes(
    hostBridge,
    'createUnavailableDevWebHostBridge',
    'portal host bridge rejects missing dev bridge URLs'
  );
  assertIncludes(
    hostBridge,
    'invokeParentDevBridgeCommandOrThrow',
    'portal host bridge guards dev bridge dispatch failures'
  );
  assertIncludes(
    hostBridge,
    'loadTauriCoreModule',
    'portal host bridge loads the Tauri core module lazily'
  );
  assertIncludes(
    hostBridge,
    'loadTauriEventModule',
    'portal host bridge loads the Tauri event module lazily'
  );
  assertIncludes(
    hostBridge,
    'ParentBridgeCommand.LoadRoute',
    'portal host bridge loads routes through typed bridge commands'
  );
  assertIncludes(
    hostBridge,
    'ParentBridgeCommand.Dispatch',
    'portal host bridge dispatches typed commands through bridge commands'
  );
  proofLabels.push('portal.host-bridge.typed-bridge-commands');

  assertIncludes(main, 'const bridge = createHostBridge();', 'portal main creates the host bridge');
  assertIncludes(main, 'sendCommand(command, payload)', 'portal main dispatches typed command intents');
  assertIncludes(
    main,
    'ParentUiActionKind.AgentCommandRequested',
    'portal main maps command sends to agent-command-requested actions'
  );
  assertIncludes(main, 'dispatchHostAction({', 'portal main funnels actions through dispatchHostAction');
  proofLabels.push('portal.main.typed-command-intents');

  assertIncludes(portalActions, 'sendCommand(command: AgentCommandName', 'portal actions expose command names only');
  assertIncludes(
    commandControls,
    'actions.selectCommandResult(command.resultEvent)',
    'command controls select result event'
  );
  assertIncludes(
    commandControls,
    'actions.sendCommand(command.command, command.payload)',
    'command controls send commands'
  );
  assertIncludes(commandControls, 'PortalCommandButtons', 'command controls consume shared command inventory');
  assertIncludes(
    commandControls,
    'actions.selectCommandResult(command.resultEvent)',
    'command controls keep events as result metadata'
  );
  assertIncludes(
    commandControls,
    'actions.sendCommand(command.command, command.payload)',
    'command controls send typed command intents'
  );
  proofLabels.push('portal.actions.typed-command-intents');
  proofLabels.push('portal.command-controls.events-are-result-metadata');

  assertIncludes(
    transportTest,
    'createDevWebHostBridge(null)',
    'portal transport test covers missing dev bridge URLs'
  );
  assertIncludes(
    transportTest,
    "createDevWebHostBridge('http://127.0.0.1:4779/api/parent-ui')",
    'portal transport test covers the configured dev bridge URL'
  );
  assertIncludes(
    transportTest,
    'ParentAgentCommand.LanPairingBrowserDiscoveryScan',
    'portal transport test covers LAN target dispatch'
  );
  assertIncludes(
    transportTest,
    'ParentAgentTargetDefaults.LocalNetworkWindowsAgent.route',
    'portal transport test covers local-network target routing'
  );
  assertIncludes(
    transportTest,
    'expect(events).toMatchObject([',
    'portal transport test proves changed snapshot emission only'
  );
  proofLabels.push('portal.transport.lan-target-dispatch');

  const scannedFiles = await sourceFiles(sourceRoots);
  for (const file of scannedFiles) {
    const source = await readText(file);
    for (const forbidden of forbiddenPublisherPatterns) {
      if (forbidden.pattern.test(source)) {
        throw new Error(`${forbidden.label}: ${file}`);
      }
    }
  }
  proofLabels.push('portal.source.no-business-event-publisher');
  return scannedFiles;
}

async function sourceFiles(roots) {
  const files = [];
  for (const root of roots) {
    await collectSourceFiles(root, files);
  }
  return files.sort();
}

async function collectSourceFiles(path, files) {
  const entries = await readdir(join(repoRoot, path), { withFileTypes: true });
  for (const entry of entries) {
    const entryPath = `${path}/${entry.name}`;
    if (entry.isDirectory()) {
      await collectSourceFiles(entryPath, files);
      continue;
    }
    if (sourceExtensions.has(extname(entry.name))) {
      files.push(entryPath);
    }
  }
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

function countMatches(text, needle) {
  let count = 0;
  let index = text.indexOf(needle);
  while (index !== -1) {
    count += 1;
    index = text.indexOf(needle, index + needle.length);
  }
  return count;
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
