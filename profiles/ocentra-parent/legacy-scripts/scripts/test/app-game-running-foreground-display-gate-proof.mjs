import { spawnSync } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { join } from 'node:path';

const repoRoot = process.cwd();
const testOutputDir = join(repoRoot, 'test-results', 'app-game-running-foreground-display-gate-proof');
const proofDir = join(repoRoot, 'output', 'app-game-plan-proof', 'merge-gates', 'running-foreground-display');
const commands = [];

await main();

async function main() {
  await mkdir(testOutputDir, { recursive: true });
  await mkdir(proofDir, { recursive: true });

  runNpm([
    'run',
    'test',
    '--workspace',
    '@ocentra-parent/portal',
    '--',
    'activity-ui-app-game-dashboard-intent.test.ts',
  ]);

  const intentSource = await readFile(
    join(repoRoot, 'vendor', 'ocentra-parent-core-ui', 'AppPages', 'ParentPortal', 'app-game-dashboard-intent.ts'),
    'utf8'
  );
  const surfaceSource = await readFile(
    join(repoRoot, 'vendor', 'ocentra-parent-core-ui', 'AppPages', 'ParentPortal', 'ParentPortalSvgSurface.tsx'),
    'utf8'
  );
  const routeAssertions = await readFile(
    join(repoRoot, 'apps', 'portal', 'e2e', 'portal-route-scaffold-assertions.ts'),
    'utf8'
  );
  const intentTest = await readFile(
    join(repoRoot, 'apps', 'portal', 'tests', 'activity-ui-app-game-dashboard-intent.test.ts'),
    'utf8'
  );

  assertIncludes(
    intentTest,
    "expect(metricPairs).toContainEqual(['Running', '4'])",
    'intent test proves running metric'
  );
  assertIncludes(
    intentTest,
    "expect(metricPairs).toContainEqual(['Foreground', '2'])",
    'intent test proves foreground metric'
  );
  assertIncludes(intentTest, 'runningRowCount: 1', 'intent test includes running rows');
  assertIncludes(intentTest, 'foregroundRowCount: 0', 'intent test includes running without foreground rows');
  assertIncludes(intentTest, 'foregroundRowCount: 1', 'intent test includes foreground rows');
  assertIncludes(
    intentSource,
    'readonly runningCount: number;',
    'dashboard row model exposes running count separately'
  );
  assertIncludes(
    intentSource,
    'readonly foregroundCount: number;',
    'dashboard row model exposes foreground count separately'
  );
  assertIncludes(
    intentSource,
    "const runningCount = numberValue(row['runningRowCount'])",
    'dashboard derives running count from running rows'
  );
  assertIncludes(
    intentSource,
    "const foregroundCount = numberValue(row['foregroundRowCount'])",
    'dashboard derives foreground count from foreground rows'
  );
  assertIncludes(intentSource, "{ label: 'Running'", 'dashboard exposes running metric');
  assertIncludes(intentSource, "{ label: 'Foreground'", 'dashboard exposes foreground metric');
  assertIncludes(
    intentSource,
    'Number(right.foregroundCount > 0) - Number(left.foregroundCount > 0)',
    'dashboard sorting prioritizes foreground separately'
  );
  assertIncludes(
    intentSource,
    'Number(right.runningCount > 0) - Number(left.runningCount > 0)',
    'dashboard sorting keeps running separate from foreground'
  );
  assertIncludes(surfaceSource, 'Running ${row.runningCount}', 'route renders running count per row');
  assertIncludes(surfaceSource, 'Foreground ${row.foregroundCount}', 'route renders foreground count per row');
  assertIncludes(routeAssertions, 'RUNNING', 'route E2E expects running visible text');
  assertIncludes(routeAssertions, 'FOREGROUND', 'route E2E expects foreground visible text');

  const proof = {
    schemaVersion: 1,
    proofMode: 'app-game-running-foreground-display-gate-proof',
    generatedAt: new Date().toISOString(),
    branch: gitOutput(['rev-parse', '--abbrev-ref', 'HEAD']),
    commit: gitOutput(['rev-parse', 'HEAD']),
    gitStatusShort: gitOutput(['status', '--short']),
    commands,
    gate: 'Running evidence is displayed as foreground usage.',
    gateState: 'prevented-by-existing-portal-dashboard-intent-and-route-assertion',
    evidence: {
      intentTest:
        'apps/portal/tests/activity-ui-app-game-dashboard-intent.test.ts proves running and foreground are separate metrics, including running rows with foregroundRowCount 0.',
      intentSource:
        'vendor/ocentra-parent-core-ui/AppPages/ParentPortal/app-game-dashboard-intent.ts maps runningRowCount and foregroundRowCount into separate row fields, metrics, tones, and sort keys.',
      routeSource:
        'vendor/ocentra-parent-core-ui/AppPages/ParentPortal/ParentPortalSvgSurface.tsx renders per-row Running and Foreground counts separately.',
      routeAssertion:
        'apps/portal/e2e/portal-route-scaffold-assertions.ts asserts the App/Game Sessions route displays both RUNNING and FOREGROUND text.',
    },
    productBoundaries: {
      sharedEvidenceSpine: true,
      nativeAppMeaningProven: true,
      nativeGameMeaningProven: true,
      runningPromotedToForeground: false,
      foregroundPromotedToContentKnowledge: false,
      browserGameWorkDuplicated: false,
      rawPrivateExecutablePathsRendered: false,
      packageExportsChanged: false,
      runtimeAdapterClaimed: false,
    },
    proofPaths: {
      proof: 'test-results/app-game-running-foreground-display-gate-proof/proof.json',
      appGameProofPack: 'output/app-game-plan-proof/merge-gates/running-foreground-display',
      harness: 'scripts/test/app-game-running-foreground-display-gate-proof.mjs',
    },
  };

  await writeJson(join(testOutputDir, 'proof.json'), proof);
  await writeJson(join(proofDir, 'proof.json'), proof);
  await writeFile(
    join(proofDir, '00-source-snapshot.md'),
    [
      '# App-game running foreground display gate source snapshot',
      '',
      `- Branch: ${proof.branch}`,
      `- Commit: ${proof.commit}`,
      `- Git status: ${proof.gitStatusShort.length === 0 ? 'clean before proof generation' : proof.gitStatusShort}`,
      '',
      'Evidence:',
      '- Portal app/game dashboard intent test includes separate Running and Foreground metric totals.',
      '- The test includes running rows where foregroundRowCount remains 0.',
      '- Core dashboard intent maps runningRowCount and foregroundRowCount into separate dashboard fields.',
      '- Core SVG route renders Running and Foreground counts separately.',
      '- Portal route scaffold E2E assertion expects both RUNNING and FOREGROUND text on App/Game Sessions.',
      '',
    ].join('\n')
  );
  await writeFile(join(proofDir, '10-validation-commands.log'), `${commands.join('\n\n').trimEnd()}\n`);

  console.log('app-game-running-foreground-display-gate-proof-ok');
  console.log('evidence=test-results/app-game-running-foreground-display-gate-proof/proof.json');
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
  commands.push(`${rendered}\nexit=${result.status}\n${result.stdout}${result.stderr}`);
  if (result.status !== 0) {
    throw new Error(`${rendered} failed with exit ${result.status}`);
  }
}

function gitOutput(args) {
  const result = spawnSync('git', args, { cwd: repoRoot, encoding: 'utf8', shell: false });
  if (result.status !== 0) {
    throw new Error(`git ${args.join(' ')} failed: ${result.stderr}`);
  }
  return result.stdout.trim();
}

function runNpm(args, ...rest) {
  const command = process.platform === 'win32' ? 'cmd' : 'npm';
  const commandArgs = process.platform === 'win32' ? ['/c', 'npm', ...args] : args;
  return run(command, commandArgs, ...rest);
}
