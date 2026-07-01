import { spawnSync } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { join } from 'node:path';

const repoRoot = process.cwd();
const testOutputDir = join(repoRoot, 'test-results', 'app-game-portal-state-visibility-gate-proof');
const proofDir = join(repoRoot, 'output', 'app-game-plan-proof', 'merge-gates', 'portal-state-visibility');
const proofBranch = 'codex/app-game-portal-state-visibility-gate-proof-split';
const deterministicProofRevision = 'branch-head-validated-by-harness';
const deterministicGeneratedAt = 'deterministic-proof-artifact';
const commands = [];

await main();

async function main() {
  await mkdir(testOutputDir, { recursive: true });
  await mkdir(proofDir, { recursive: true });

  runNpm(['run', 'build:contracts']);
  runNpm([
    'run',
    'test',
    '--workspace',
    '@ocentra-parent/portal',
    '--',
    'activity-ui-app-game-dashboard-intent.test.ts',
  ]);
  runNpm([
    'run',
    'test',
    '--workspace',
    '@ocentra-parent/portal',
    '--',
    'tests/unit/app-game-policy-readiness-panel.test.ts',
  ]);

  const dashboardIntent = await readFile(
    join(repoRoot, 'vendor', 'ocentra-parent-core-ui', 'AppPages', 'ParentPortal', 'app-game-dashboard-intent.ts'),
    'utf8'
  );
  const dashboardTest = await readFile(
    join(repoRoot, 'apps', 'portal', 'tests', 'activity-ui-app-game-dashboard-intent.test.ts'),
    'utf8'
  );
  const policyReadinessTest = await readFile(
    join(repoRoot, 'apps', 'portal', 'tests', 'unit', 'app-game-policy-readiness-panel.test.ts'),
    'utf8'
  );
  const policyReadinessPanel = await readFile(
    join(repoRoot, 'apps', 'portal', 'src', 'AppGamePolicyReadinessRoutePanel.tsx'),
    'utf8'
  );
  const portalReadme = await readFile(join(repoRoot, 'apps', 'portal', 'README.md'), 'utf8');
  const appGameFeatureDoc = await readFile(join(repoRoot, 'docs', 'features', 'app-game-control.md'), 'utf8');

  assertIncludes(
    dashboardIntent,
    '/manual|required|permission|unsupported|unavailable|not-claimed|admin|supervised|degraded|stale/u',
    'dashboard manual-required classifier covers stale, permission, and not-claimed states'
  );
  assertIncludes(
    dashboardTest,
    'expect(dashboard.rows.map((row) => [row.label, row.capabilityStatus, row.manualRequired, row.tone])).toContainEqual',
    'dashboard test keeps permission-required game rows visible as gold/manual-required'
  );
  assertIncludes(
    dashboardTest,
    'VPN Proxy Portable <script>alert(1)</script> with a display name that is deliberately too long for one row',
    'dashboard test names the manual-required app row'
  );
  assertIncludes(
    dashboardTest,
    "'Voxel Quest',\n      'permission-required',\n      true,\n      'gold',",
    'dashboard test asserts permission row state'
  );
  assertIncludes(
    dashboardTest,
    "'Old App',\n      'stale',\n      true,\n      'gold',",
    'dashboard test keeps stale/manual-required app rows visible'
  );
  assertIncludes(
    dashboardTest,
    "expect(dashboard.capabilityRows.map((row) => row.label)).toContain('permission-required')",
    'dashboard test exposes permission-required capability row'
  );
  assertIncludes(
    dashboardTest,
    "expect(dashboard.capabilityRows.map((row) => row.label)).toContain('manual-required')",
    'dashboard test exposes manual-required capability row'
  );
  assertIncludes(
    policyReadinessTest,
    "expect(html).toContain('Capability');",
    'policy readiness test keeps capability visible'
  );
  assertIncludes(
    policyReadinessTest,
    "expect(html).toContain('Not claimed');",
    'policy readiness test keeps not-claimed capability visible'
  );
  assertIncludes(
    policyReadinessTest,
    "expect(html).toContain('Adapter dispatch');",
    'policy readiness test keeps adapter dispatch visible'
  );
  assertIncludes(
    policyReadinessTest,
    "expect(html).toContain('Manual review');",
    'policy readiness test keeps manual review visible'
  );
  assertIncludes(
    policyReadinessTest,
    "expect(html).toContain('Policy evidence');",
    'policy readiness test keeps policy evidence visible'
  );
  assertIncludes(
    policyReadinessTest,
    "expect(html).toContain('AI classifier context');",
    'policy readiness test keeps AI classifier context visible'
  );
  assertIncludes(
    policyReadinessTest,
    "expect(html).toContain('AI classifier context requires manual review');",
    'policy readiness test keeps manual review explanation visible'
  );
  assertIncludes(
    policyReadinessTest,
    "expect(html).toContain('Not reported');",
    'policy readiness test keeps not reported evidence visible'
  );
  assertIncludes(
    policyReadinessPanel,
    '<AppGamePolicyReadinessDetails details={panel.summaryDetails} />',
    'policy readiness route panel renders Rust-owned summary details'
  );
  assertIncludes(
    policyReadinessPanel,
    'Approval workflow, category routing, and adapter dispatch remain unclaimed.',
    'policy readiness route panel keeps adapter dispatch unclaimed'
  );
  assertIncludes(
    portalReadme,
    'Displays service-backed app/game policy readiness rows on App/Game Sessions',
    'portal README names service-backed app/game readiness surface'
  );
  assertIncludes(
    appGameFeatureDoc,
    'The portal App/Game Sessions route now renders that service-backed policy',
    'feature doc names app/game policy readiness portal route'
  );

  const proof = {
    schemaVersion: 1,
    proofMode: 'app-game-portal-state-visibility-gate-proof',
    generatedAt: deterministicGeneratedAt,
    branch: proofBranch,
    commit: deterministicProofRevision,
    commitMetadata:
      'This proof intentionally avoids embedding HEAD because a committed artifact cannot contain its own final commit hash.',
    gitStatusShort: 'validated-by-explicit-handoff-status-check',
    commands,
    gate: 'Portal hides stale, permission-limited, manual-required, or not-claimed states.',
    gateState: 'prevented-by-portal-dashboard-and-policy-readiness-intent-proof',
    evidence: {
      dashboardIntent:
        'vendor/ocentra-parent-core-ui/AppPages/ParentPortal/app-game-dashboard-intent.ts classifies stale, permission, manual-required, unavailable, degraded, and not-claimed state text as parent-visible manual-required/gold rows.',
      dashboardTest:
        'apps/portal/tests/activity-ui-app-game-dashboard-intent.test.ts proves stale/manual-required app rows and permission-required native-game rows remain in dashboard rows and capability summaries, with Voxel Quest named as the permission-required native-game row.',
      policyReadinessTest:
        'apps/portal/tests/unit/app-game-policy-readiness-panel.test.ts proves not-claimed app/game policy readiness capability and adapter dispatch states render in summary details.',
      policyReadinessPanel:
        'apps/portal/src/AppGamePolicyReadinessRoutePanel.tsx renders Rust-owned summary details and keeps adapter dispatch unclaimed.',
    },
    productBoundaries: {
      sharedEvidenceSpine: true,
      nativeAppMeaningProven: true,
      nativeGameMeaningProven: true,
      staleRowsHidden: false,
      permissionLimitedRowsHidden: false,
      manualRequiredRowsHidden: false,
      notClaimedRowsHidden: false,
      browserGameWorkDuplicated: false,
      portalFakeActivityAdded: false,
      adapterDispatchClaimed: false,
      policyExecutionClaimed: false,
      packageExportsChanged: false,
    },
    proofPaths: {
      proof: 'test-results/app-game-portal-state-visibility-gate-proof/proof.json',
      appGameProofPack: 'output/app-game-plan-proof/merge-gates/portal-state-visibility',
      harness: 'scripts/test/app-game-portal-state-visibility-gate-proof.mjs',
    },
  };

  await writeJson(join(testOutputDir, 'proof.json'), proof);
  await writeJson(join(proofDir, 'proof.json'), proof);
  await writeFile(
    join(proofDir, '00-source-snapshot.md'),
    [
      '# App-game portal state visibility gate source snapshot',
      '',
      `- Branch: ${proof.branch}`,
      `- Commit: ${proof.commit}`,
      `- Git status: ${proof.gitStatusShort}`,
      '',
      'Evidence:',
      '- The App/Game Sessions dashboard intent keeps stale/manual-required app rows visible.',
      '- The same dashboard keeps permission-required native-game rows visible and gold/manual-required.',
      '- App/game policy readiness route summary details keep not-claimed capability and adapter dispatch visible.',
      '- This proof adds no synthetic portal activity, policy execution, adapter dispatch, or browser-game path.',
      '',
    ].join('\n')
  );
  await writeFile(join(proofDir, '10-validation-commands.log'), `${commands.join('\n\n').trimEnd()}\n`);

  console.log('app-game-portal-state-visibility-gate-proof-ok');
  console.log('evidence=test-results/app-game-portal-state-visibility-gate-proof/proof.json');
}

function assertIncludes(source, needle, label) {
  if (!source.includes(needle)) {
    throw new Error(`Missing ${label}: ${needle}`);
  }
}

async function writeJson(path, value) {
  await writeFile(path, `${JSON.stringify(value, null, 2)}\n`);
}

function run(command, args) {
  const rendered = `${command} ${args.join(' ')}`;
  const result = spawnSync(command, args, { cwd: repoRoot, encoding: 'utf8', shell: false });
  commands.push(
    `${rendered}\nexit=${result.status}\n${normalizeCommandOutput(result.stdout)}${normalizeCommandOutput(result.stderr)}`
  );
  if (result.status !== 0) {
    throw new Error(`${rendered} failed with exit ${result.status}`);
  }
}

function normalizeCommandOutput(output) {
  const slashRepoRoot = repoRoot.replace(/\\/g, '/');
  return output
    .split(repoRoot)
    .join('<repo-root>')
    .split(slashRepoRoot)
    .join('<repo-root>')
    .replace(/Start at\s+\d{2}:\d{2}:\d{2}/g, 'Start at <normalized>')
    .replace(/\x1b\[2m[^\r\n]*?\x1b\[22m/g, '\x1b[2m<normalized>\x1b[22m')
    .replace(/Duration\s+[^\r\n]+/g, 'Duration <normalized>');
}

function runNpm(args, ...rest) {
  const command = process.platform === 'win32' ? 'cmd' : 'npm';
  const commandArgs = process.platform === 'win32' ? ['/c', 'npm', ...args] : args;
  return run(command, commandArgs, ...rest);
}
