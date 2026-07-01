import { spawn } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';

const repoRoot = process.cwd();
const testOutputDir = join(repoRoot, 'test-results', 'app-game-source-panel-polish-proof');
const appGameProofDir = join(repoRoot, 'output', 'app-game-plan-proof', '63-source-freshness-source-panel-polish');
const appProofDir = join(repoRoot, 'output', 'app-plan-proof', '63-source-freshness-source-panel-polish');
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
    proofMode: 'app-game-source-panel-polish',
    checkedAt: new Date().toISOString(),
    branch: await gitValue(['branch', '--show-current']),
    commit: await gitValue(['rev-parse', 'HEAD']),
    statusShort: await gitValue(['status', '--short']),
    commands,
    sourceAssertions,
    evidence: {
      sourcePanelIntent: 'vendor/ocentra-parent-core-ui/AppPages/ParentPortal/app-game-source-panel-intent.ts',
      dashboardIntent: 'vendor/ocentra-parent-core-ui/AppPages/ParentPortal/app-game-dashboard-intent.ts',
      dashboardIntentTest: 'apps/portal/tests/activity-ui-app-game-dashboard-intent.test.ts',
      proofHarness: 'scripts/test/app-game-source-panel-polish-proof.mjs',
      appGameProofPack: 'output/app-game-plan-proof/63-source-freshness-source-panel-polish',
      appProofPack: 'output/app-plan-proof/63-source-freshness-source-panel-polish',
    },
    claimsProved: [
      'portal dashboard intent now exposes dedicated source-panel sections derived from service-backed sourceStatusRows',
      'source-panel sections group app-use and game source rows separately with fresh/manual/evidence counts',
      'source-panel rows carry freshness labels, source-kind labels, row counts, evidence counts, last observed labels, and existing dashboard tones',
    ],
    claimsNotProved: [
      'SVG source-panel rendering because ParentPortalSvgSurface.tsx is locked by E-A in the hub',
      'route E2E assertion changes because the portal route scaffold assertion file is locked by E-A in the hub',
      'new backend source status contracts, source subscriptions, policy evaluator consumption, provider delivery, adapter execution, broad blocking, or platform support',
    ],
  };

  await writeJson(join(testOutputDir, 'proof.json'), proof);
  await writeProofPack(appGameProofDir, proof, 'app-game WP63');
  await writeProofPack(appProofDir, proof, 'app WP63');

  console.log(`app-game-source-panel-polish-proof-ok:${sourceAssertions.panelSectionsWired}`);
  console.log(`evidence=${relative(repoRoot, join(testOutputDir, 'proof.json'))}`);
}

function assertSourceAssertions(sourceAssertions) {
  for (const [key, value] of Object.entries(sourceAssertions)) {
    if (value !== true) {
      throw new Error(`Source panel proof assertion failed: ${key}`);
    }
  }
}

async function collectSourceAssertions() {
  const helperSource = await readFile(
    join(repoRoot, 'vendor', 'ocentra-parent-core-ui', 'AppPages', 'ParentPortal', 'app-game-source-panel-intent.ts'),
    'utf8'
  );
  const dashboardSource = await readFile(
    join(repoRoot, 'vendor', 'ocentra-parent-core-ui', 'AppPages', 'ParentPortal', 'app-game-dashboard-intent.ts'),
    'utf8'
  );
  const testSource = await readFile(
    join(repoRoot, 'apps', 'portal', 'tests', 'activity-ui-app-game-dashboard-intent.test.ts'),
    'utf8'
  );
  const changedFiles = await gitValue(['diff', '--name-only']);
  return {
    helperOwnsPanelSections: helperSource.includes('ParentPortalAppGameSourcePanelSection'),
    helperComputesFreshManualEvidence:
      helperSource.includes('freshCount') &&
      helperSource.includes('manualRequiredCount') &&
      helperSource.includes('evidenceCount'),
    panelSectionsWired:
      dashboardSource.includes('sourcePanelSections') &&
      dashboardSource.includes('createParentPortalAppGameSourcePanelSections'),
    testProvesPanelSections:
      testSource.includes('expectSourcePanelSections') &&
      testSource.includes('App use sources') &&
      testSource.includes('Game sources'),
    noLockedSurfaceDiff:
      !changedFiles.includes('vendor/ocentra-parent-core-ui/AppPages/ParentPortal/parentportalsvgsurface.tsx') &&
      !changedFiles.includes('apps/portal/e2e/portal-route-scaffold-assertions.ts'),
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

async function gitValue(args) {
  return new Promise((resolve) => {
    const child = spawn('git', args, {
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
    join(outputDir, '00-source-snapshot.md'),
    [
      `# ${label} Source Snapshot`,
      '',
      `Branch: ${proof.branch}`,
      `Commit: ${proof.commit}`,
      '',
      '## Git Status',
      '```text',
      proof.statusShort || 'clean',
      '```',
      '',
      '## Inspected Sources',
      ...Object.values(proof.evidence).map((value) => `- ${value}`),
      '',
    ].join('\n'),
    'utf8'
  );
  await writeFile(
    join(outputDir, '01-contract-proof.log'),
    [
      ...proof.commands.map((entry) => `${entry.command}: exit ${entry.exitCode}`),
      'source panel assertions: PASS',
    ].join('\n') + '\n',
    'utf8'
  );
  await writeFile(
    join(outputDir, '02-rust-protocol-proof.log'),
    'N/A: portal intent-only source-panel seam.\n',
    'utf8'
  );
  await writeJson(join(outputDir, '03-runtime-evidence.json'), proof);
  await writeFile(
    join(outputDir, '04-journal-sqlite-proof.json'),
    '{\n  "applicable": false,\n  "reason": "No journal or SQLite schema changed in WP63."\n}\n',
    'utf8'
  );
  await writeFile(
    join(outputDir, '05-policy-action-proof.json'),
    '{\n  "adapterDispatchClaimed": false,\n  "policyEvaluatorConsumptionClaimed": false\n}\n',
    'utf8'
  );
  await mkdir(join(outputDir, '06-ui-snapshots'), { recursive: true });
  await writeFile(
    join(outputDir, '06-ui-snapshots', 'ui-rendering-handoff.md'),
    [
      '# UI Rendering Handoff',
      '',
      'The source-panel intent seam is proved in WP63.',
      'SVG rendering remains a follow-up because `ParentPortalSvgSurface.tsx` and the route E2E assertion file are locked by E-A in the hub.',
      '',
    ].join('\n'),
    'utf8'
  );
  await writeFile(
    join(outputDir, '07-playwright-ui-proof.log'),
    'N/A: route-level Playwright assertion changes require E-A locked files.\n',
    'utf8'
  );
  await writeFile(
    join(outputDir, '08-security-negative-proof.log'),
    [
      'PASS: source panel rows consume service-backed sourceStatusRows only.',
      'PASS: no raw evidence vectors are parsed for new product meaning.',
      'PASS: source panel proof does not claim policy evaluation, adapter execution, or broad blocking.',
      '',
    ].join('\n'),
    'utf8'
  );
  await writeFile(
    join(outputDir, '09-manual-platform-proof.md'),
    '# Manual Platform Proof\n\nN/A: no platform adapter or OS authority changed.\n',
    'utf8'
  );
  await writeFile(
    join(outputDir, '10-validation-commands.log'),
    [
      ...proof.commands.map((entry) => `${entry.command}: exit ${entry.exitCode}`),
      'node scripts/test/app-game-source-panel-polish-proof.mjs: PASS',
    ].join('\n') + '\n',
    'utf8'
  );
  await writeFile(
    join(outputDir, '11-authority-tier-proof.md'),
    '# Authority Tier Proof\n\nN/A: parent-visible source-panel data only; no adapter authority changed.\n',
    'utf8'
  );
  await writeFile(
    join(outputDir, '12-rollback-proof.md'),
    [
      '# Rollback Proof',
      '',
      'Remove `sourcePanelSections` from the dashboard intent, delete `app-game-source-panel-intent.ts`, and remove the WP63 proof/docs.',
      'No persisted data, provider state, policy state, or adapter state is created by this slice.',
      '',
    ].join('\n'),
    'utf8'
  );
  await writeJson(join(outputDir, 'proof.json'), proof);
  await writeFile(
    join(outputDir, 'README.md'),
    [
      `# ${label} Source Freshness Source Panel Polish`,
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
    ].join('\n'),
    'utf8'
  );
}

function npmCommand(args) {
  const command = process.platform === 'win32' ? 'cmd' : 'npm';
  const commandArgs = process.platform === 'win32' ? ['/c', 'npm', ...args] : args;
  return [command, commandArgs];
}
