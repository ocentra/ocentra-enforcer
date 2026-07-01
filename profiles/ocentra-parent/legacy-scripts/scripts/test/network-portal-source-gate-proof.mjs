import { spawn } from 'node:child_process';
import { mkdir, readdir, readFile, writeFile } from 'node:fs/promises';
import { extname, join, relative } from 'node:path';

const repoRoot = process.cwd();
const testOutputDir = join(repoRoot, 'test-results', 'network-portal-source-gate-proof');
const planOutputDir = join(repoRoot, 'output', 'network-plan-proof', '36-portal-source-gate');
const proofPath = join(testOutputDir, 'proof.json');
const planProofPath = join(planOutputDir, 'proof-summary.json');
const sourceRoots = ['apps/portal/src'];
const sourceShapeFiles = [
  'apps/portal/src/use-portal-network-activity-refresh.ts',
  'apps/portal/src/portal-route-refresh.ts',
  'apps/portal/src/PolicyPreviewRoutePanel.tsx',
  'apps/portal/src/PortalApp.tsx',
  'apps/portal/src/route-live-activity-state.ts',
  'apps/portal/src/live-activity-state.ts',
  'apps/portal/src/NetworkEvidenceDrawerRoutePanel.tsx',
  'packages/portal-domain/src/live-activity-state.ts',
  'crates/agent-protocol/tests/contract/network_flow_tests.rs',
];
const sourceExtensions = new Set(['.ts', '.tsx']);
const commands = [];
const proofLabels = [];

const forbiddenSourcePatterns = [
  {
    label: 'no Rust or private event bus ownership in portal source',
    pattern: /(?:ocentra-eventing|NetworkEventBus|EventContext<|EventPublisher|createEventPublisher)/u,
  },
  {
    label: 'no portal business event publish function',
    pattern: /(?:^|[^\w])(?:publishEvent|publishBusinessEvent|publishDomainEvent|publishNetworkEvent)\s*\(/u,
  },
  {
    label: 'no portal event bus publish call',
    pattern: /(?:eventBus|networkBus|bus|publisher)\.publish\s*\(/u,
  },
  {
    label: 'no portal event subscription ownership',
    pattern: /(?:eventBus|networkBus|bus)\.subscribe\s*\(/u,
  },
  {
    label: 'no portal-owned network policy evaluator',
    pattern: /(?:evaluateNetworkPolicy|decideNetworkPolicy|computeNetworkPolicy|authorizeNetworkAdapter)\s*\(/u,
  },
  {
    label: 'no portal-owned adapter or enforcement execution',
    pattern: /(?:executeNetworkAdapter|applyNetworkAdapter|publishEnforcementCommand|dispatchEnforcement)\s*\(/u,
  },
  {
    label: 'no portal-owned network evidence grade computation',
    pattern: /(?:computeNetworkEvidenceGrade|ActivityNetworkEvidenceGradeSchema|NetworkEvidenceGradeSchema)/u,
  },
];

await main();

async function main() {
  await mkdir(testOutputDir, { recursive: true });
  await mkdir(planOutputDir, { recursive: true });

  await runCommand(...npmCommand(['--workspace', '@ocentra-parent/agent-protocol-domain', 'run', 'build']));
  await runCommand(...npmCommand(['--workspace', '@ocentra-parent/portal-domain', 'run', 'build']));
  await runCommand(
    ...npmCommand([
      'exec',
      '--workspace',
      '@ocentra-parent/portal',
      '--',
      'vitest',
      'run',
      'tests/live-activity/live-activity-network-flow.test.ts',
      'tests/network/network-evidence-drawer-route-panel.test.ts',
      'tests/live-activity/live-activity-state.test.ts',
    ])
  );
  await runCommand('cargo', ['test', '-p', 'ocentra-parent-agent-protocol', 'network_flow_tests', '--quiet']);
  await runCommand(
    ...npmCommand([
      'exec',
      '--workspace',
      '@ocentra-parent/portal',
      '--',
      'eslint',
      'src/live-activity-state.ts',
      'src/route-live-activity-state.ts',
      'src/NetworkEvidenceDrawerRoutePanel.tsx',
      'src/use-portal-network-activity-refresh.ts',
      'src/portal-route-refresh.ts',
      'src/PolicyPreviewRoutePanel.tsx',
      'src/PortalApp.tsx',
      'tests/live-activity/live-activity-network-flow.test.ts',
    ])
  );
  await runCommand('node', ['scripts/check-source-shape.mjs', '--files', ...sourceShapeFiles]);

  const scannedFiles = await assertNetworkPortalSourceGate();
  const proof = {
    schemaVersion: 1,
    proofMode: 'network-portal-source-gate-proof',
    checkedAt: new Date().toISOString(),
    commit: await gitHead(),
    commands,
    proofLabels,
    evidence: {
      portalNetworkRefreshHook: 'apps/portal/src/use-portal-network-activity-refresh.ts',
      portalRouteRefresh: 'apps/portal/src/portal-route-refresh.ts',
      portalPolicyPreviewRoutePanel: 'apps/portal/src/PolicyPreviewRoutePanel.tsx',
      portalApp: 'apps/portal/src/PortalApp.tsx',
      routeLiveActivityState: 'apps/portal/src/route-live-activity-state.ts',
      liveActivityState: 'apps/portal/src/live-activity-state.ts',
      portalDomainLiveActivityState: 'packages/portal-domain/src/live-activity-state.ts',
      rustNetworkFlowReadModelContractTest: 'crates/agent-protocol/tests/contract/network_flow_tests.rs',
      networkEvidenceDrawerRoutePanel: 'apps/portal/src/NetworkEvidenceDrawerRoutePanel.tsx',
      portalNetworkFlowTest: 'apps/portal/tests/live-activity/live-activity-network-flow.test.ts',
      networkEvidenceDrawerRoutePanelTest: 'apps/portal/tests/network/network-evidence-drawer-route-panel.test.ts',
      portalDomainNetworkFlowTest: 'packages/portal-domain/tests/unit/live-activity-network-flow.test.ts',
      proofHarness: 'scripts/test/network-portal-source-gate-proof.mjs',
      scannedSourceRoots: sourceRoots,
      scannedFiles,
    },
    claimsProved: [
      'portal network refresh surfaces a thin read-model refresh callback only',
      'portal policy-preview route refreshes the snapshot through the thin callback only',
      'portal app live-activity state stays a thin portal-domain type alias and route-snapshot resolver before rendering',
      'portal-domain owns the empty live-activity defaults consumed by the route resolver',
      'network read-model parser validates the service payload and embedded activity digest before app rendering',
      'network evidence drawer route renders service-provided endpoint, domain, process, custody, and evidence refs',
      'network evidence drawer keeps exact URL, policy decision, and intervention facets Not reported when service refs are absent',
      'portal source contains no event bus import, event publish call, event subscription ownership, adapter execution, enforcement dispatch, or local network policy/evidence-grade computation',
      'network evidence drawer is mounted on canonical network product routes only',
    ],
    claimsNotProved: [
      'live packet capture, raw PCAP custody, quota rotation, deletion, or export',
      'broker or family-hub network event-chain delivery',
      'local AI model execution or portal AI audit rendering',
      'policy engine execution, rule authoring, or final policy authority',
      'adapter execution, host DNS mutation, firewall mutation, packet blocking, or enforcement command publication',
      'new portal UI behavior beyond the existing service-backed network evidence drawer',
    ],
  };

  const serialized = `${JSON.stringify(proof, null, 2)}\n`;
  await writeFile(proofPath, serialized);
  await writeFile(planProofPath, serialized);
  console.log(`network-portal-source-gate-proof-ok:${proofLabels.join(',')}`);
  console.log(`evidence=${relative(repoRoot, proofPath)}`);
  console.log(`planEvidence=${relative(repoRoot, planProofPath)}`);
}

async function assertNetworkPortalSourceGate() {
  const networkRefreshHook = await readText('apps/portal/src/use-portal-network-activity-refresh.ts');
  const routeRefresh = await readText('apps/portal/src/portal-route-refresh.ts');
  const policyPreviewRoutePanel = await readText('apps/portal/src/PolicyPreviewRoutePanel.tsx');
  const portalApp = await readText('apps/portal/src/PortalApp.tsx');
  const routeLiveActivityState = await readText('apps/portal/src/route-live-activity-state.ts');
  const liveActivityState = await readText('apps/portal/src/live-activity-state.ts');
  const portalDomainLiveActivityState = await readText('packages/portal-domain/src/live-activity-state.ts');
  const networkFlowContractTest = await readText('crates/agent-protocol/tests/contract/network_flow_tests.rs');
  const parentRuntimeNetworkFlowSamples = await readText(
    'crates/parent-runtime-core/tests/unit/parent_ui_bridge/common_events_samples.rs'
  );
  const drawerRoutePanel = await readText('apps/portal/src/NetworkEvidenceDrawerRoutePanel.tsx');
  const networkEvidenceDrawerRoutePanelTest = await readText(
    'apps/portal/tests/network/network-evidence-drawer-route-panel.test.ts'
  );
  const portalDomainNetworkFlowTest = await readText('packages/portal-domain/tests/unit/live-activity-network-flow.test.ts');

  assertIncludes(
    networkRefreshHook,
    'void actions.requestNetworkFlowReadModelRefresh?.();',
    'network flow route requests the read-model refresh callback only'
  );
  assertIncludes(
    policyPreviewRoutePanel,
    'onClick={() => void actions.refreshRouteSnapshot?.()}',
    'policy preview route refreshes the snapshot through the thin callback only'
  );
  assertIncludes(
    policyPreviewRoutePanel,
    'refreshRouteSnapshot',
    'policy preview route exposes the snapshot refresh callback only'
  );
  assertIncludes(
    routeRefresh,
    '!hasNetworkFlowReadModelEvent',
    'network refresh waits for a missing service-backed read-model event'
  );
  assertIncludes(
    portalApp,
    'const hasNetworkFlowReadModelEvent =',
    'portal app tracks whether the current route snapshot already carries a network read-model'
  );
  proofLabels.push('portal.routes.network-query-only');

  assertIncludes(
    liveActivityState,
    'PortalLiveActivityState as PortalDomainPortalLiveActivityState,',
    'live activity state stays a thin portal-domain type alias'
  );
  assertIncludes(
    liveActivityState,
    'export type PortalLiveActivityState = PortalDomainPortalLiveActivityState;',
    'live activity state re-exports the shared portal-domain live activity type'
  );
  assertIncludes(
    routeLiveActivityState,
    'EMPTY_PORTAL_LIVE_ACTIVITY_STATE,',
    'route live activity state reads the shared portal-domain empty state'
  );
  assertIncludes(
    routeLiveActivityState,
    '...EMPTY_PORTAL_LIVE_ACTIVITY_STATE,',
    'route live activity state composes from the shared portal-domain empty state'
  );
  assertIncludes(
    routeLiveActivityState,
    'const resolvedSnapshot = snapshot as Partial<ResolvedPortalLiveActivityState>;',
    'route live activity state normalizes the route snapshot locally'
  );
  assertIncludes(
    routeLiveActivityState,
    'export const EMPTY_ROUTE_LIVE_ACTIVITY_STATE = {',
    'route live activity state exports the resolved empty route snapshot'
  );
  assertIncludes(
    portalDomainLiveActivityState,
    'export const EMPTY_PORTAL_LIVE_ACTIVITY_STATE = {',
    'portal domain owns the empty live activity defaults'
  );
  proofLabels.push('portal.state.thin-alias-and-route-snapshot-resolver');

  assertIncludes(
    networkFlowContractTest,
    'network_flow_read_model_serializes_rows_without_payload_claims',
    'network read-model contract test validates Rust-owned serialization'
  );
  assertIncludes(
    networkFlowContractTest,
    'ActivityNetworkFlowReadModel',
    'network read-model contract test covers the Rust read model type'
  );
  assertIncludes(
    parentRuntimeNetworkFlowSamples,
    'AgentEventName::AgentNetworkFlowReadModelReported',
    'parent runtime exposes the Rust-owned network flow read-model event'
  );
  proofLabels.push('portal.network-flow.rust-contract-validated');

  assertIncludes(
    drawerRoutePanel,
    'networkEvidenceDrawerSummary(liveActivity.networkFlowReadModel, {',
    'network route uses the current app-owned drawer surface'
  );
  assertIncludes(
    drawerRoutePanel,
    'PortalDetails.EvidenceReferences',
    'network drawer route renders service evidence refs'
  );
  assertIncludes(drawerRoutePanel, 'PortalDetails.Custody', 'network drawer route renders service custody details');
  proofLabels.push('portal.route.renders-network-surface-details');

  assertIncludes(
    drawerRoutePanel,
    'return isNetworkEvidenceDrawerRoute(route);',
    'network drawer route uses portal-domain route predicate'
  );
  assertIncludes(
    networkEvidenceDrawerRoutePanelTest,
    "expect(html).toContain('Exact URL claim</dt><dd>Not reported');",
    'network drawer test proves exact URL stays unsupported without service ref'
  );
  assertIncludes(
    portalDomainNetworkFlowTest,
    "expect(summary.policyDecisionRef).toBe('Not reported')",
    'network drawer test proves policy facet is unsupported without service ref'
  );
  assertIncludes(
    portalDomainNetworkFlowTest,
    "expect(summary.interventionResultRef).toBe('Not reported')",
    'network drawer test proves intervention facet is unsupported without service ref'
  );
  proofLabels.push('portal.route-and-test.network-drawer-boundary');

  const scannedFiles = await sourceFiles(sourceRoots);
  for (const file of scannedFiles) {
    const source = await readText(file);
    for (const forbidden of forbiddenSourcePatterns) {
      assertPatternAbsent(source, forbidden.pattern, `${forbidden.label}: ${file}`);
    }
  }
  proofLabels.push('portal.source.no-network-business-authority');
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

function assertIncludes(text, expected, label) {
  if (!text.includes(expected)) {
    throw new Error(`${label}: missing ${expected}`);
  }
}

function assertPatternAbsent(text, pattern, label) {
  if (pattern.test(text)) {
    throw new Error(`${label}: matched ${pattern}`);
  }
}

function npmCommand(args) {
  const command = process.platform === 'win32' ? 'cmd' : 'npm';
  const commandArgs = process.platform === 'win32' ? ['/c', 'npm', ...args] : args;
  return [command, commandArgs];
}
