import { spawn } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';

const repoRoot = process.cwd();
const testOutputDir = join(repoRoot, 'test-results', 'app-game-source-freshness-portal-proof');
const appGameProofDir = join(repoRoot, 'output', 'app-game-plan-proof', '48-portal-source-freshness-surface');
const appProofDir = join(repoRoot, 'output', 'app-plan-proof', '48-portal-source-freshness-surface');
const commands = [];

await main();

async function main() {
  await mkdir(testOutputDir, { recursive: true });
  await mkdir(appGameProofDir, { recursive: true });
  await mkdir(appProofDir, { recursive: true });

  await runCommand(
    ...npmCommand([
      'exec',
      '--workspace',
      '@ocentra-parent/portal',
      '--',
      'vitest',
      'run',
      'tests/activity-ui-app-game-dashboard-intent.test.ts',
    ])
  );

  const sourceAssertions = await collectSourceAssertions();
  assertSourceAssertions(sourceAssertions);
  const proof = {
    schemaVersion: 1,
    proofMode: 'app-game-source-freshness-portal',
    checkedAt: new Date().toISOString(),
    commit: await gitHead(),
    commands,
    sourceAssertions,
    evidence: {
      dashboardIntent: 'vendor/ocentra-parent-core-ui/AppPages/ParentPortal/app-game-dashboard-intent.ts',
      dashboardIntentTest: 'apps/portal/tests/activity-ui-app-game-dashboard-intent.test.ts',
      routeAssertion: 'apps/portal/e2e/portal-route-scaffold-assertions.ts',
      proofHarness: 'scripts/test/app-game-source-freshness-portal-proof.mjs',
      appGameProofPack: 'output/app-game-plan-proof/48-portal-source-freshness-surface',
      appProofPack: 'output/app-plan-proof/48-portal-source-freshness-surface',
    },
    claimsProved: [
      'portal dashboard intent consumes service-backed sourceStatusRows from app-use and games read models',
      'source rows and fresh source counts are rendered through the existing App/Game Sessions dashboard metric surface',
      'source-kind capability, latest observed timestamp, and evidence ref counts are visible through the existing evidence drawer rows',
    ],
    claimsNotProved: [
      'new backend source status contracts or service adapters',
      'policy evaluator consumption of source freshness rows',
      'live source crawling, launcher disambiguation, provider execution, or broad blocking',
      'new dedicated SVG panel section because ParentPortalSvgSurface.tsx is owned by another active lane',
    ],
  };

  await writeJson(join(testOutputDir, 'proof.json'), proof);
  await writeProofPack(appGameProofDir, proof, 'app-game WP48');
  await writeProofPack(appProofDir, proof, 'app WP48');

  console.log(`app-game-source-freshness-portal-proof-ok:${sourceAssertions.metricAssertion}`);
  console.log(`evidence=${relative(repoRoot, join(testOutputDir, 'proof.json'))}`);
}

function assertSourceAssertions(sourceAssertions) {
  for (const [key, value] of Object.entries(sourceAssertions)) {
    if (value !== true) {
      throw new Error(`Source freshness proof assertion failed: ${key}`);
    }
  }
}

async function collectSourceAssertions() {
  const intentSource = await readFile(
    join(repoRoot, 'vendor', 'ocentra-parent-core-ui', 'AppPages', 'ParentPortal', 'app-game-dashboard-intent.ts'),
    'utf8'
  );
  const routeAssertionSource = await readFile(
    join(repoRoot, 'apps', 'portal', 'e2e', 'portal-route-scaffold-assertions.ts'),
    'utf8'
  );
  return {
    metricAssertion: intentSource.includes('Source rows') && routeAssertionSource.includes('SOURCE ROWS'),
    freshnessAssertion: intentSource.includes('Fresh sources') && routeAssertionSource.includes('FRESH SOURCES'),
    nestedSourceRows: intentSource.includes('sourceStatusRows'),
  };
}

async function runCommand(command, args) {
  await new Promise((resolve, reject) => {
    const child = spawn(command, args, {
      cwd: repoRoot,
      shell: false,
      stdio: 'inherit',
      windowsHide: true,
    });
    child.on('error', reject);
    child.on('exit', (code) => {
      const commandLine = [command, ...args].join(' ');
      commands.push({
        command: commandLine,
        exitCode: code,
      });
      if (code === 0) {
        resolve();
      } else {
        reject(new Error(`${commandLine} exited ${code}`));
      }
    });
  });
}

async function gitHead() {
  return new Promise((resolve) => {
    const child = spawn('git', ['rev-parse', 'HEAD'], {
      cwd: repoRoot,
      shell: false,
      stdio: ['ignore', 'pipe', 'ignore'],
      windowsHide: true,
    });
    let output = '';
    child.stdout.on('data', (chunk) => {
      output += chunk.toString();
    });
    child.on('exit', () => {
      resolve(output.trim());
    });
  });
}

async function writeJson(path, value) {
  await writeFile(path, `${JSON.stringify(value, null, 2)}\n`, 'utf8');
}

async function writeProofPack(outputDir, proof, label) {
  await writeFile(
    join(outputDir, 'README.md'),
    [
      `# ${label} Portal Source Freshness Surface`,
      '',
      `Checked at: ${proof.checkedAt}`,
      `Commit: ${proof.commit}`,
      '',
      '## Claims Proved',
      ...proof.claimsProved.map((claim) => `- ${claim}`),
      '',
      '## Claims Not Proved',
      ...proof.claimsNotProved.map((claim) => `- ${claim}`),
      '',
      '## Evidence',
      ...Object.entries(proof.evidence).map(([key, value]) => `- ${key}: ${value}`),
      '',
    ].join('\n'),
    'utf8'
  );
  await writeJson(join(outputDir, 'proof.json'), proof);
}

function npmCommand(args) {
  const command = process.platform === 'win32' ? 'cmd' : 'npm';
  const commandArgs = process.platform === 'win32' ? ['/c', 'npm', ...args] : args;
  return [command, commandArgs];
}
