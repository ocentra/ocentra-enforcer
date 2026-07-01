import { spawn } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';

const repoRoot = process.cwd();
const outputDir = join(repoRoot, 'test-results', 'activity-mia-report-history-action-preview-proof');
const proofPath = join(outputDir, 'proof.json');
const finalPassProofPath = join(repoRoot, 'test-results', 'activity-mia-evidence-final-pass', 'proof.json');
const commands = [];

await main();

async function main() {
  await mkdir(outputDir, { recursive: true });

  await runCommand('node', ['scripts/test/activity-mia-evidence-final-pass.mjs']);
  await runCommand('node', ['scripts/test/parent-assistant-action-preview-proof.mjs']);

  const finalPassProof = await readJson(finalPassProofPath);
  assertFinalPassProof(finalPassProof);

  const proof = {
    schemaVersion: 1,
    proofMode: 'activity-mia-report-history-action-preview-proof',
    checkedAt: new Date().toISOString(),
    commit: await gitHead(),
    commands,
    proofLabels: [
      'activity-mia-report-history.saved-metadata',
      'activity-mia-report-history.degraded-storage',
      'activity-mia-report-history.source-state-summary',
      'activity-mia-action-preview.saved-report-citations',
      'activity-mia-action-preview.report-source-id-citations',
      'activity-mia-action-preview.stale-unreachable-source-id-citations',
      'activity-mia-action-preview.child-contract-non-enforcement',
    ],
    evidence: {
      upstreamFinalPassProof: relative(repoRoot, finalPassProofPath),
      activityDomain: 'packages/schema-domain/src/activity-surface.ts',
      parentDomain: 'packages/schema-domain/src/parent-assistant.ts',
      rustProtocolActivity: 'crates/agent-protocol/src/activity_surface.rs',
      rustProtocolParentAssistant: 'crates/agent-protocol/src/parent_assistant.rs',
      rustReportStore: 'crates/agent-service/src/activity_surface_report_store.rs',
      rustParentAssistantApi: 'crates/agent-service/src/parent_assistant_api.rs',
      actionPreviewRuntimeProof: 'scripts/test/parent-assistant-action-preview-proof.mjs',
      checkpoint: 'docs/checkpoints/activity-mia-report-history-action-preview-proof-2026-05-30.md',
    },
    coverage: {
      savedReportHistory:
        'Activity history rows expose saved report metadata, parsed reports, and per-row source-state summary counts.',
      degradedStorage:
        'Partially unreadable or unparsable saved report storage stays renderable with storageState=degraded and an explicit reason.',
      savedReportCitations:
        'Parent Assistant/MIA action-preview results carry evidenceContext from saved Activity reports when a report is supplied, including stale/unreachable/unavailable source ids from family fan-out records where available.',
      childContractBoundary:
        'Action preview and confirm remain non-enforcing and require child-agent/controller contracts before policy or enforcement writes.',
      nonVisualScope:
        'No C-owned UI, vendor portal, temp-scratchpad, API AI implementation, policy writes, or enforcement writes are part of this proof.',
    },
    counts: {
      upstreamFinalPassProofLabels: finalPassProof.proofLabels.length,
      proofLabels: 7,
      cOwnedPathsTouched: 0,
      actionPreviewRuntimeProofs: 1,
    },
    knownGaps: [
      'C-owned visual Activity UI still needs to render the enriched history metadata and source summaries.',
      'Action-preview evidence is citation context only; it does not authorize API AI, write policy, or enforce on the child agent.',
      'Physical multi-device source aggregation remains represented by typed source states until real household devices are connected.',
    ],
  };

  await writeFile(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
  console.log(`activity-mia-report-history-action-preview-proof-ok:${proof.proofLabels.join(',')}`);
  console.log(`evidence=${relative(repoRoot, proofPath)}`);
}

function assertFinalPassProof(finalPassProof) {
  for (const label of [
    'activity-mia-final-pass.report-persistence',
    'activity-mia-final-pass.family-device-request-builders',
    'activity-mia-final-pass.adapter-operation-manifest',
    'activity-mia-final-pass.adapter-failure-metadata',
    'activity-mia-final-pass.c-consumption-helper-map',
    'activity-mia-final-pass.parent-assistant-evidence',
    'activity-mia-final-pass.c-owned-paths-not-touched',
  ]) {
    if (!finalPassProof.proofLabels.includes(label)) {
      throw new Error(`Upstream Activity/MIA final-pass proof is missing ${label}.`);
    }
  }
}

async function readJson(path) {
  return JSON.parse(await readFile(path, 'utf8'));
}

async function runCommand(command, args) {
  commands.push([command, ...args].join(' '));
  await new Promise((resolve, reject) => {
    const child = spawn(command, args, { cwd: repoRoot, stdio: 'inherit', windowsHide: true });
    child.once('exit', (code) => (code === 0 ? resolve() : reject(new Error(`${command} exited with ${code}`))));
    child.once('error', reject);
  });
}

async function gitHead() {
  const chunks = [];
  await new Promise((resolve, reject) => {
    const child = spawn('git', ['rev-parse', 'HEAD'], { cwd: repoRoot, stdio: ['ignore', 'pipe', 'pipe'] });
    child.stdout.on('data', (chunk) => chunks.push(String(chunk)));
    child.once('exit', (code) => (code === 0 ? resolve() : reject(new Error('git rev-parse HEAD failed'))));
    child.once('error', reject);
  });
  return chunks.join('').trim();
}
