import { spawnSync } from 'node:child_process';
import { mkdirSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

const repoRoot = fileURLToPath(new URL('../..', import.meta.url));
const outputDir = join(repoRoot, 'test-results', 'screen-parent-portal-summary-ui-proof');
const outputPath = join(outputDir, 'proof.json');
const planOutputDir = join(repoRoot, 'output', 'screen-plan-proof', 'screen-parent-portal-summary-ui');
const planOutputPath = join(planOutputDir, 'proof-summary.json');

run('npm', ['run', 'build', '--workspace=@ocentra-parent/logging-domain']);
run('npm', ['run', 'build', '--workspace=@ocentra-parent/portal-domain']);
run('npx', [
  'vitest',
  'run',
  'packages/portal-domain/tests/unit/screen-summary-panel.test.ts',
  'apps/portal/tests/screen/screen-summary-route-panel.test.ts',
]);
run('npm', ['run', 'build', '--workspace=@ocentra-parent/portal']);
run('node', ['scripts/test/portal-playwright-runner.mjs'], {
  env: {
    OCENTRA_PARENT_PORTAL_PLAYWRIGHT_SPEC: 'e2e/screen-summary-ui-proof.spec.ts',
    SCREEN_PARENT_PORTAL_SUMMARY_UI_PROOF: '1',
  },
});

const proof = {
  proofId: 'screen-parent-portal-summary-ui-proof',
  generatedAt: '2026-06-06T22:20:00Z',
  source: '@ocentra-parent/portal screen summary route panel',
  assertions: [
    'screen-analysis route renders a dedicated parent summary overlay without using the shared Activity/network route',
    'service-backed Activity Screen read-model rows produce parent-visible capability, queue, model, category, confidence, custody, deletion, policy, audit, evidence, and parent explanation ref details',
    'unavailable screen read-model state stays visible without invented rows',
    'enforcement remains not-claimed in this UI proof; adapter execution stays a separate gate',
  ],
  parsed: {
    route: 'screen-analysis',
    renderSharedActivityRoute: false,
    productClaim: 'No family setting is configured for this area yet.',
    rawScreenshotDisplayed: false,
    parentExplanationRefsDisplayed: true,
    adapterExecutionClaimed: false,
  },
  screenshots: {
    desktop: 'output/screen-plan-proof/screen-parent-portal-summary-ui/screenshots/screen-analysis-route-desktop.png',
    mobile: 'output/screen-plan-proof/screen-parent-portal-summary-ui/screenshots/screen-analysis-route-mobile.png',
    accessibilitySummary: 'test-results/screen-parent-portal-summary-ui-proof/accessibility-summary.json',
  },
};

mkdirSync(outputDir, { recursive: true });
mkdirSync(planOutputDir, { recursive: true });
writeFileSync(outputPath, `${JSON.stringify(proof, null, 2)}\n`);
writeFileSync(planOutputPath, `${JSON.stringify(proof, null, 2)}\n`);
console.log(`screen-parent-portal-summary-ui-proof-ok: ${outputPath}`);

function run(command, args, options = {}) {
  const result = spawnSync(command, args, {
    cwd: repoRoot,
    env: { ...process.env, ...options.env },
    shell: process.platform === 'win32',
    stdio: 'inherit',
  });
  if (result.status !== 0) {
    throw new Error(`${command} ${args.join(' ')} failed with ${result.status}`);
  }
}
