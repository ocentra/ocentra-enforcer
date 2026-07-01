import { spawnSync } from 'node:child_process';
import { mkdirSync, readdirSync, readFileSync, writeFileSync } from 'node:fs';
import { extname, join } from 'node:path';

const repoRoot = process.cwd();
const proofRoot = join('output', 'network-plan-proof', 'portal-risk-performance-platform-status');
const testRoot = join('test-results', 'network-portal-risk-performance-platform-proof');
const proofPath = join(testRoot, 'proof.json');
const planProofPath = join(proofRoot, 'proof-summary.json');
const sourceRoots = ['apps/portal/src'];
const sourceExtensions = new Set(['.ts', '.tsx']);
const commands = [];
const proofLabels = [];

const forbiddenPortalAuthorityPatterns = [
  {
    label: 'no portal-owned network event publishing',
    pattern: /(?:publishNetworkEvent|publishEnforcementCommand|publishDomainEvent|publishBusinessEvent)\s*\(/u,
  },
  {
    label: 'no portal-owned network adapter execution',
    pattern: /(?:executeNetworkAdapter|applyNetworkAdapter|dispatchEnforcement|authorizeNetworkAdapter)\s*\(/u,
  },
  {
    label: 'no portal-owned network policy evaluator',
    pattern: /(?:evaluateNetworkPolicy|decideNetworkPolicy|computeNetworkPolicy)\s*\(/u,
  },
  {
    label: 'no portal-owned network risk scoring',
    pattern: /(?:computeNetworkRisk|scoreNetworkRisk|calculateRiskBudget)\s*\(/u,
  },
  {
    label: 'no portal-owned evidence grade computation',
    pattern: /(?:computeNetworkEvidenceGrade|ActivityNetworkEvidenceGradeSchema|NetworkEvidenceGradeSchema)/u,
  },
];

mkdirSync(proofRoot, { recursive: true });
mkdirSync(testRoot, { recursive: true });

await main();

async function main() {
  runCommand(...npmCommand(['run', 'build', '--workspace', '@ocentra-parent/portal']));
  runCommand(
    ...npmCommand([
      'exec',
      '--workspace',
      '@ocentra-parent/portal',
      '--',
      'vitest',
      'run',
      'tests/live-activity/live-activity-network-flow.test.ts',
    ])
  );
  runCommand(
    ...npmCommand([
      'exec',
      '--workspace',
      '@ocentra-parent/portal',
      '--',
      'eslint',
      'src/NetworkEvidenceDrawerRoutePanel.tsx',
      'tests/live-activity/live-activity-network-flow.test.ts',
    ])
  );
  runCommand('node', ['scripts/check-source-shape.mjs']);
  runCommand('git', ['diff', '--check']);

  const scannedFiles = assertPortalStatusProjection();
  const proof = {
    schemaVersion: 1,
    proof: 'network-portal-risk-performance-platform-proof',
    checkedAt: new Date().toISOString(),
    branch: runText('git', ['branch', '--show-current']).trim(),
    commit: runText('git', ['rev-parse', 'HEAD']).trim(),
    statusShort: runText('git', ['status', '--short']),
    originMain: runText('git', ['rev-parse', 'origin/main']).trim(),
    proofRoot,
    testRoot,
    commands,
    proofLabels,
    artifacts: {
      proofSummary: planProofPath,
      testProof: proofPath,
    },
    evidence: {
      networkEvidenceDrawerRoutePanel: 'apps/portal/src/NetworkEvidenceDrawerRoutePanel.tsx',
    portalNetworkFlowTest: 'apps/portal/tests/live-activity/live-activity-network-flow.test.ts',
      scannedSourceRoots: sourceRoots,
      scannedFiles,
    },
    claimsProved: [
      'portal route renders platform/capability status from the service-backed network read model',
      'portal drawer renders active/tombstone/exportable row counts without inventing evidence',
      'portal drawer renders retention tombstone and deleted evidence refs when the service reports them',
      'portal drawer renders degraded adapter/platform states from read-model capability status',
      'portal keeps exact URL, policy, and intervention facets Not reported without service refs',
      'portal source contains no network event publish, policy evaluation, evidence grade computation, adapter execution, or enforcement dispatch authority',
    ],
    notClaimed: [
      'live packet capture or raw PCAP custody',
      'risk-budget calculation inside the portal',
      'policy engine execution or parent-rule authoring',
      'adapter execution, host filtering, or enforcement command publication',
      'production performance or real-time SLO validation',
      'exact URL, page content, private message, search query, or decrypted payload availability',
    ],
  };

  const serialized = `${JSON.stringify(proof, null, 2)}\n`;
  writeFileSync(proofPath, serialized);
  writeFileSync(planProofPath, serialized);
  console.log(`network-portal-risk-performance-platform-proof-ok:${proofLabels.join(',')}`);
  console.log(`proof=${proofPath}`);
}

function assertPortalStatusProjection() {
  const panel = readText('apps/portal/src/NetworkEvidenceDrawerRoutePanel.tsx');
  const test = readText('apps/portal/tests/live-activity/live-activity-network-flow.test.ts');

  for (const expected of [
    'networkEvidenceDrawerSummary(liveActivity.networkFlowReadModel, {',
    'PortalDetails.PlatformState',
    'PortalDetails.ReadModelRows',
    'PortalDetails.DeletedEvidenceReferences',
    'PortalDetails.PerformanceState',
  ]) {
    assertIncludes(panel, expected, `drawer route renders field: ${expected}`);
  }
  proofLabels.push('drawer.route-renders-platform-performance-retention-fields');

  for (const expected of [
    "expect(summary.platformState).toBe('child-device-query-store | available')",
    "expect(summary.readModelRows).toBe('1 | 1 | 0 | 1')",
    "expect(summary.degradedState).toBe('available | domain-observed | process-attributed')",
    "expect(summary.retentionState).toBe('activity-network-flow-deleted | 2026-05-21T02:05:00Z | network-evidence-1')",
    "expect(summary.policyDecisionRef).toBe('Not reported')",
    "expect(summary.interventionResultRef).toBe('Not reported')",
  ]) {
    assertIncludes(test, expected, `portal test assertion: ${expected}`);
  }
  proofLabels.push('portal.tests-active-and-deleted-service-readmodel-states');

  const scannedFiles = sourceFiles(sourceRoots);
  for (const file of scannedFiles) {
    const source = readText(file);
    for (const forbidden of forbiddenPortalAuthorityPatterns) {
      assertPatternAbsent(source, forbidden.pattern, `${forbidden.label}: ${file}`);
    }
  }
  proofLabels.push('portal.source-no-policy-adapter-or-risk-authority');
  return scannedFiles;
}

function sourceFiles(roots) {
  return roots.flatMap((root) => collectSourceFiles(root)).sort();
}

function collectSourceFiles(path) {
  const entries = readdirSync(join(repoRoot, path), { withFileTypes: true });
  return entries.flatMap((entry) => {
    const entryPath = `${path}/${entry.name}`;
    if (entry.isDirectory()) {
      return collectSourceFiles(entryPath);
    }
    return sourceExtensions.has(extname(entry.name)) ? [entryPath] : [];
  });
}

function runCommand(command, args) {
  const commandLine = [command, ...args].join(' ');
  const result = spawnSync(command, args, { cwd: repoRoot, encoding: 'utf8', shell: false });
  commands.push({
    command: commandLine,
    status: result.status ?? 1,
    stdout: trimForProof(result.stdout),
    stderr: trimForProof(result.stderr),
  });
  if (result.status !== 0) {
    throw new Error(`${commandLine} failed with exit ${result.status}`);
  }
}

function runText(command, args) {
  const result = spawnSync(command, args, { cwd: repoRoot, encoding: 'utf8', shell: false });
  if (result.status !== 0) {
    throw new Error(`${command} ${args.join(' ')} failed with exit ${result.status}`);
  }
  return `${result.stdout ?? ''}${result.stderr ?? ''}`;
}

function readText(path) {
  return readFileSync(join(repoRoot, path), 'utf8');
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

function trimForProof(value) {
  const text = String(value ?? '').trimEnd();
  return text.length <= 2000 ? text : `${text.slice(0, 2000)}\n...[truncated]`;
}

function npmCommand(args) {
  const command = process.platform === 'win32' ? 'cmd' : 'npm';
  const commandArgs = process.platform === 'win32' ? ['/c', 'npm', ...args] : args;
  return [command, commandArgs];
}
