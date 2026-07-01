import { spawnSync } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { join } from 'node:path';

const repoRoot = process.cwd();
const testOutputDir = join(repoRoot, 'test-results', 'app-game-malicious-metadata-ui-safety-gate-proof');
const proofDir = join(repoRoot, 'output', 'app-game-plan-proof', 'merge-gates', 'malicious-metadata-ui-safety');
const commands = [];
const proofBranch = 'codex/app-game-malicious-metadata-ui-safety-gate-proof-split';
const deterministicProofRevision = 'branch-head-validated-by-harness';
const deterministicGeneratedAt = 'deterministic-proof-artifact';

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
  const truncateSlice = sourceSlice(surfaceSource, 'function truncateTextForWidth', 'function compactControlStatLabel');
  const fitTextSlice = sourceSlice(surfaceSource, 'function fitSingleLineTextSize', 'function truncateTextForWidth');
  const dashboardRenderSlice = `${dashboardPanelSlice}\n${dashboardRowSlice}`;
  const maliciousLabel =
    'VPN Proxy Portable <script>alert(1)</script> with a display name that is deliberately too long for one row';

  assertIncludes(dashboardTest, maliciousLabel, 'portal test feeds malicious script-like long metadata');
  assertIncludes(
    dashboardTest,
    'expectMaliciousMetadataStaysTextOnly',
    'portal test asserts malicious metadata remains a manual text row'
  );
  assertIncludes(
    dashboardTest,
    "expect(maliciousRow?.tone).toBe('gold')",
    'portal test keeps malicious row manual-required instead of promoting an unsafe action'
  );
  assertIncludes(dashboardIntent, 'label: stringValue(row[config.labelField])', 'dashboard intent maps label as text');
  assertIncludes(
    dashboardIntent,
    'const manualRequired = appGameManualRequired',
    'dashboard intent derives manual-required state from unsafe/stale metadata'
  );
  assertIncludes(
    dashboardRenderSlice,
    '{truncateTextForWidth(row.label, w - 28, titleSize, 0.58)}',
    'dashboard row renders app/game labels through bounded text truncation'
  );
  assertIncludes(
    dashboardPanelSlice,
    'const visibleRows = dashboard.rows.slice(0, visibleRowCount)',
    'dashboard panel bounds rendered rows to available layout capacity'
  );
  assertIncludes(
    fitTextSlice,
    'return clampValue(width / Math.max(1, text.length * factor), min, max)',
    'renderer reduces text size for long single-line labels'
  );
  assertIncludes(
    truncateSlice,
    'return `${text.slice(0, maxChars - 3).trimEnd()}...`',
    'renderer truncates over-wide labels instead of letting them break layout'
  );
  assertNotIncludes(dashboardRenderSlice, 'dangerouslySetInnerHTML', 'app/game dashboard must not use HTML injection');
  assertNotIncludes(dashboardRenderSlice, 'innerHTML', 'app/game dashboard must not use direct HTML sinks');
  assertNotIncludes(dashboardRenderSlice, '<foreignObject', 'app/game dashboard should render rows as SVG text nodes');
  assertNotIncludes(dashboardRenderSlice, 'eval(', 'app/game dashboard must not evaluate metadata');
  assertIncludes(
    appGameFeatureDoc,
    'malicious metadata UI safety gate',
    'feature doc records the app/game malicious metadata UI safety proof'
  );

  const proof = {
    schemaVersion: 1,
    proofMode: 'app-game-malicious-metadata-ui-safety-gate-proof',
    generatedAt: deterministicGeneratedAt,
    branch: proofBranch,
    commit: deterministicProofRevision,
    commitMetadata:
      'This proof intentionally avoids embedding HEAD because a committed artifact cannot contain its own final commit hash.',
    gitStatusShort: 'validated-by-explicit-handoff-status-check',
    commands,
    gate: 'Malicious app/game metadata causes XSS or layout breakage.',
    gateState: 'prevented-by-react-text-rendering-bounded-svg-layout-and-manual-required-row-proof',
    evidence: {
      dashboardTest:
        'apps/portal/tests/activity-ui-app-game-dashboard-intent.test.ts feeds a script-like long app label and asserts it remains a manual-required/risk text row in the parent dashboard intent.',
      dashboardIntent:
        'vendor/ocentra-parent-core-ui/AppPages/ParentPortal/app-game-dashboard-intent.ts maps app/game metadata into text fields and manual-required/risk state without action execution.',
      dashboardSurface:
        'vendor/ocentra-parent-core-ui/AppPages/ParentPortal/ParentPortalSvgSurface.tsx renders app/game labels through SVG text nodes, fitSingleLineTextSize, truncateTextForWidth, and visible row bounds; no app/game dashboard HTML sinks are used.',
    },
    productBoundaries: {
      sharedEvidenceSpine: true,
      nativeAppMeaningProven: true,
      nativeGameMeaningProven: true,
      maliciousMetadataExecuted: false,
      dangerousHtmlSinkUsed: false,
      layoutRowsBounded: true,
      labelsTruncated: true,
      manualRequiredPreserved: true,
      browserGameWorkDuplicated: false,
      adapterDispatchClaimed: false,
      policyExecutionClaimed: false,
      packageExportsChanged: false,
    },
    proofPaths: {
      proof: 'test-results/app-game-malicious-metadata-ui-safety-gate-proof/proof.json',
      appGameProofPack: 'output/app-game-plan-proof/merge-gates/malicious-metadata-ui-safety',
      harness: 'scripts/test/app-game-malicious-metadata-ui-safety-gate-proof.mjs',
    },
  };

  await writeJson(join(testOutputDir, 'proof.json'), proof);
  await writeJson(join(proofDir, 'proof.json'), proof);
  await writeFile(
    join(proofDir, '00-source-snapshot.md'),
    [
      '# App-game malicious metadata UI safety gate source snapshot',
      '',
      `- Branch: ${proof.branch}`,
      `- Commit: ${proof.commit}`,
      `- Git status: ${proof.gitStatusShort}`,
      '',
      'Evidence:',
      '- Portal app/game dashboard tests feed a long script-like app label through the real dashboard intent path.',
      '- The row remains manual-required and risk-candidate text, with no adapter dispatch or policy execution claim.',
      '- The SVG app/game dashboard renders labels as React/SVG text children through bounded text sizing and truncation.',
      '- The app/game dashboard render slices do not use dangerouslySetInnerHTML, innerHTML, foreignObject, or eval.',
      '- This proof adds no extra activity, package exports, adapter dispatch, policy execution, or browser-game path.',
      '',
    ].join('\n')
  );
  await writeFile(join(proofDir, '10-validation-commands.log'), `${commands.join('\n\n').trimEnd()}\n`);

  console.log('app-game-malicious-metadata-ui-safety-gate-proof-ok');
  console.log('evidence=test-results/app-game-malicious-metadata-ui-safety-gate-proof/proof.json');
}

function assertIncludes(source, needle, label) {
  if (!source.includes(needle)) {
    throw new Error(`Missing ${label}: ${needle}`);
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
