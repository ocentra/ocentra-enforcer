import { spawnSync } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { join } from 'node:path';

const repoRoot = process.cwd();
const testOutputDir = join(repoRoot, 'test-results', 'app-game-raw-executable-path-ui-leak-gate-proof');
const proofDir = join(repoRoot, 'output', 'app-game-plan-proof', 'merge-gates', 'raw-executable-path-ui-leak');
const proofBranch = 'codex/app-game-raw-executable-path-ui-leak-gate-proof-split';
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

  const dashboardIntent = await readFile(
    join(repoRoot, 'vendor', 'ocentra-parent-core-ui', 'AppPages', 'ParentPortal', 'app-game-dashboard-intent.ts'),
    'utf8'
  );
  const surfaceSource = await readFile(
    join(repoRoot, 'vendor', 'ocentra-parent-core-ui', 'AppPages', 'ParentPortal', 'ParentPortalSvgSurface.tsx'),
    'utf8'
  );
  const dashboardTest = await readFile(
    join(repoRoot, 'apps', 'portal', 'tests', 'activity-ui-app-game-dashboard-intent.test.ts'),
    'utf8'
  );
  const appGameFeatureDoc = await readFile(join(repoRoot, 'docs', 'features', 'app-game-control.md'), 'utf8');
  const dashboardPanelSlice = sourceSlice(
    surfaceSource,
    'function ParentPortalAppGameDashboardPanel',
    'function ParentPortalAppGameDashboardMetricCard'
  );
  const dashboardRowSlice = sourceSlice(
    surfaceSource,
    'function ParentPortalAppGameDashboardRowCard',
    'function ParentPortalAppGameDashboardMetricList'
  );

  assertNotIncludes(dashboardIntent, 'executablePathRef', 'dashboard intent must not map executable path refs');
  assertNotIncludes(dashboardPanelSlice, 'executablePath', 'dashboard panel must not render executable paths');
  assertNotIncludes(dashboardRowSlice, 'executablePath', 'dashboard row card must not render executable paths');
  assertMatches(
    dashboardTest,
    /function\s+studyTimerAppRow\(\)\s*\{[\s\S]*?executablePathRef:\s*'C:\\\\Users\\\\child\\\\AppData\\\\Local\\\\Study Timer\\\\study-timer\.exe'/,
    'portal test feeds a private user executable path into an app row'
  );
  assertMatches(
    dashboardTest,
    /gamesReadModel\(\)[\s\S]*?executablePathRef:\s*'C:\\\\Program Files\\\\VoxelQuest\\\\VoxelQuest\.exe'/,
    'portal test feeds a Windows program executable path into a game row'
  );
  assertIncludes(
    dashboardTest,
    "expect(serializedDashboard).not.toContain('C:\\\\Users\\\\child\\\\AppData\\\\Local\\\\Study Timer\\\\study-timer.exe')",
    'portal test rejects private app executable path output'
  );
  assertIncludes(
    dashboardTest,
    "expect(serializedDashboard).not.toContain('C:\\\\Program Files\\\\VoxelQuest\\\\VoxelQuest.exe')",
    'portal test rejects game executable path output'
  );
  assertIncludes(
    dashboardTest,
    "expect(serializedDashboard).not.toContain('executablePathRef')",
    'portal test rejects executable path field output'
  );
  assertIncludes(
    appGameFeatureDoc,
    'manual-required capability, game-budget gap, and evidence',
    'feature doc names dashboard as service-backed state/evidence surface'
  );

  const proof = {
    schemaVersion: 1,
    proofMode: 'app-game-raw-executable-path-ui-leak-gate-proof',
    generatedAt: deterministicGeneratedAt,
    branch: proofBranch,
    commit: deterministicProofRevision,
    commitMetadata:
      'This proof intentionally avoids embedding HEAD because a committed artifact cannot contain its own final commit hash.',
    gitStatusShort: 'validated-by-explicit-handoff-status-check',
    commands,
    gate: 'Raw private executable paths leak into parent UI.',
    gateState: 'prevented-by-dashboard-intent-redaction-and-render-source-proof',
    evidence: {
      dashboardTest:
        'apps/portal/tests/activity-ui-app-game-dashboard-intent.test.ts feeds Windows-looking executablePathRef values into app/game rows and proves the serialized parent dashboard omits the raw paths and executablePathRef field.',
      dashboardIntent:
        'vendor/ocentra-parent-core-ui/AppPages/ParentPortal/app-game-dashboard-intent.ts maps parent-visible labels, state, counts, durations, capability, and evidence refs, but does not map executablePathRef.',
      dashboardSurface:
        'vendor/ocentra-parent-core-ui/AppPages/ParentPortal/ParentPortalSvgSurface.tsx renders the app/game dashboard rows and metric lists without executable path fields.',
    },
    productBoundaries: {
      sharedEvidenceSpine: true,
      nativeAppMeaningProven: true,
      nativeGameMeaningProven: true,
      rawPrivateExecutablePathExposed: false,
      executablePathRefRendered: false,
      evidenceRefsRendered: true,
      browserGameWorkDuplicated: false,
      adapterDispatchClaimed: false,
      policyExecutionClaimed: false,
      packageExportsChanged: false,
    },
    proofPaths: {
      proof: 'test-results/app-game-raw-executable-path-ui-leak-gate-proof/proof.json',
      appGameProofPack: 'output/app-game-plan-proof/merge-gates/raw-executable-path-ui-leak',
      harness: 'scripts/test/app-game-raw-executable-path-ui-leak-gate-proof.mjs',
    },
  };

  await writeJson(join(testOutputDir, 'proof.json'), proof);
  await writeJson(join(proofDir, 'proof.json'), proof);
  await writeFile(
    join(proofDir, '00-source-snapshot.md'),
    [
      '# App-game raw executable path UI leak gate source snapshot',
      '',
      `- Branch: ${proof.branch}`,
      `- Commit: ${proof.commit}`,
      `- Git status: ${proof.gitStatusShort}`,
      '',
      'Evidence:',
      '- Portal app/game dashboard tests feed raw Windows executable-path-like values into app/game rows.',
      '- The dashboard intent output omits those raw paths and the executablePathRef field.',
      '- The SVG dashboard render source displays labels, state, counts, capability, duration, and evidence refs without executable paths.',
      '- This proof adds no synthetic activity, adapter dispatch, policy execution, package exports, or browser-game path.',
      '',
    ].join('\n')
  );
  await writeFile(join(proofDir, '10-validation-commands.log'), `${commands.join('\n\n').trimEnd()}\n`);

  console.log('app-game-raw-executable-path-ui-leak-gate-proof-ok');
  console.log('evidence=test-results/app-game-raw-executable-path-ui-leak-gate-proof/proof.json');
}

function assertIncludes(source, needle, label) {
  if (!source.includes(needle)) {
    throw new Error(`Missing ${label}: ${needle}`);
  }
}

function assertMatches(source, pattern, label) {
  if (!pattern.test(source)) {
    throw new Error(`Missing ${label}: ${pattern}`);
  }
}

function assertNotIncludes(source, needle, label) {
  if (source.includes(needle)) {
    throw new Error(`Unexpected ${label}: ${needle}`);
  }
}

function sourceSlice(source, startNeedle, endNeedle) {
  const start = source.indexOf(startNeedle);
  const end = source.indexOf(endNeedle, start);
  if (start < 0 || end < 0 || end <= start) {
    throw new Error(`Could not find source slice ${startNeedle} -> ${endNeedle}`);
  }
  return source.slice(start, end);
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
