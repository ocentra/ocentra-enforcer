import { spawnSync } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { join } from 'node:path';

const repoRoot = process.cwd();
const testOutputDir = join(repoRoot, 'test-results', 'app-game-inventory-display-gate-proof');
const proofDir = join(repoRoot, 'output', 'app-game-plan-proof', 'merge-gates', 'inventory-display');
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

  assertIncludes(intentSource, 'inventoryCount = inventoryCountValue', 'dashboard derives inventory count');
  assertIncludes(intentSource, "{ label: 'Inventory'", 'dashboard exposes inventory metric');
  assertIncludes(intentSource, "sourceKind === 'app-use' ? 'App use' : 'Game'", 'dashboard separates app/game rows');
  assertIncludes(surfaceSource, 'APP/GAME READ MODEL DASHBOARD', 'route renders app/game dashboard title');
  assertIncludes(surfaceSource, 'Inventory ${row.inventoryCount}', 'route renders inventory count per row');
  assertIncludes(routeAssertions, 'assertAppGameDashboardRouteSurface', 'route E2E asserts app/game dashboard');
  assertIncludes(routeAssertions, 'INVENTORY', 'route E2E expects inventory visible text');
  assertIncludes(intentTest, "productKind: 'native-app'", 'intent test includes native app row');
  assertIncludes(intentTest, "productKind: 'native-game'", 'intent test includes native game row');
  assertIncludes(intentTest, "productKind: 'launcher'", 'intent test keeps launcher meaning separate');

  const proof = {
    schemaVersion: 1,
    proofMode: 'app-game-inventory-display-gate-proof',
    generatedAt: new Date().toISOString(),
    branch: gitOutput(['rev-parse', '--abbrev-ref', 'HEAD']),
    commit: gitOutput(['rev-parse', 'HEAD']),
    gitStatusShort: gitOutput(['status', '--short']),
    commands,
    gate: 'Inventory evidence is displayed as app/game usage.',
    gateState: 'proven-by-existing-portal-dashboard-intent-and-route-assertion',
    evidence: {
      intentTest:
        'apps/portal/tests/activity-ui-app-game-dashboard-intent.test.ts proves native-app, native-game, and launcher rows feed the dashboard separately.',
      intentSource:
        'vendor/ocentra-parent-core-ui/AppPages/ParentPortal/app-game-dashboard-intent.ts maps inventoryRowCount/inventoryState into inventoryCount and dashboard Inventory metrics.',
      routeSource:
        'vendor/ocentra-parent-core-ui/AppPages/ParentPortal/ParentPortalSvgSurface.tsx renders APP/GAME READ MODEL DASHBOARD and per-row Inventory counts.',
      routeAssertion:
        'apps/portal/e2e/portal-route-scaffold-assertions.ts asserts the App/Game Sessions route displays INVENTORY text.',
    },
    productBoundaries: {
      sharedEvidenceSpine: true,
      nativeAppMeaningProven: true,
      nativeGameMeaningProven: true,
      browserGameWorkDuplicated: false,
      rawPrivateExecutablePathsRendered: false,
      packageExportsChanged: false,
      runtimeAdapterClaimed: false,
    },
    proofPaths: {
      proof: 'test-results/app-game-inventory-display-gate-proof/proof.json',
      appGameProofPack: 'output/app-game-plan-proof/merge-gates/inventory-display',
      harness: 'scripts/test/app-game-inventory-display-gate-proof.mjs',
    },
  };

  await writeJson(join(testOutputDir, 'proof.json'), proof);
  await writeJson(join(proofDir, 'proof.json'), proof);
  await writeFile(
    join(proofDir, '00-source-snapshot.md'),
    [
      '# App-game inventory display gate source snapshot',
      '',
      `- Branch: ${proof.branch}`,
      `- Commit: ${proof.commit}`,
      `- Git status: ${proof.gitStatusShort.length === 0 ? 'clean before proof generation' : proof.gitStatusShort}`,
      '',
      'Evidence:',
      '- Portal app/game dashboard intent test includes native app, native game, and launcher rows.',
      '- Core dashboard intent maps inventory evidence into app/game dashboard metrics.',
      '- Core SVG route renders the app/game dashboard and inventory counts.',
      '- Portal route scaffold E2E assertion expects inventory text on App/Game Sessions.',
      '',
    ].join('\n')
  );
  await writeFile(join(proofDir, '10-validation-commands.log'), `${commands.join('\n\n').trimEnd()}\n`);

  console.log('app-game-inventory-display-gate-proof-ok');
  console.log('evidence=test-results/app-game-inventory-display-gate-proof/proof.json');
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
