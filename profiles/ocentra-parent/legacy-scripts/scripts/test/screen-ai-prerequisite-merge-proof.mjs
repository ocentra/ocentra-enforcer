import { spawnSync } from 'node:child_process';
import { existsSync } from 'node:fs';
import { mkdir, writeFile } from 'node:fs/promises';
import { join, resolve } from 'node:path';

const repoRoot = process.cwd();
const outputRoot = resolve(repoRoot, 'output', 'screen-ai-pipeline-proof', 'prerequisite-merge');
const artifactSummaryPath = join(outputRoot, 'proof-summary.json');
const screenAiCheckpointCommit = '9cda19698206ee5c3d49b2fd152b1daf7af395c1';
const screenCaptureProof = resolve(
  repoRoot,
  'output',
  'screen-plan-proof',
  'real-capture',
  'manual-parent-test-active-window',
  'proof-summary.json'
);
const aiAnalysisProof = resolve(repoRoot, 'output', 'ai-plan-proof', 'real-analysis', 'proof-summary.json');

await mkdir(outputRoot, { recursive: true });

const head = git(['rev-parse', 'HEAD']);
const branch = git(['rev-parse', '--abbrev-ref', 'HEAD']);
const originMain = git(['rev-parse', 'origin/main']);
const checkpointMerged = commandSucceeds('git', ['merge-base', '--is-ancestor', screenAiCheckpointCommit, 'HEAD']);
const screenCaptureProofExists = existsSync(screenCaptureProof);
const aiAnalysisProofExists = existsSync(aiAnalysisProof);

if (!checkpointMerged || !screenCaptureProofExists || !aiAnalysisProofExists) {
  throw new Error(
    JSON.stringify(
      {
        checkpointMerged,
        screenCaptureProofExists,
        aiAnalysisProofExists,
      },
      null,
      2
    )
  );
}

const summary = {
  status: 'ok',
  proofKind: 'screen-ai-prerequisite-merge-proof',
  artifact: artifactSummaryPath,
  branch,
  head,
  originMain,
  screenAiCheckpointCommit,
  checkpointMerged,
  screenCaptureProof,
  screenCaptureProofExists,
  aiAnalysisProof,
  aiAnalysisProofExists,
  assertions: [
    'The merged PR258 checkpoint commit is an ancestor of the current screen AI continuation branch.',
    'The branch contains the screen capture proof artifact.',
    'The branch contains the AI analysis proof artifact.',
  ],
  nonClaims: [
    'This records prerequisite merge state only.',
    'It does not claim npm run validate, live external URL/account proof, or service-owned timer wiring.',
  ],
};

await writeFile(artifactSummaryPath, `${JSON.stringify(summary, null, 2)}\n`);
console.log(`screen-ai-prerequisite-merge-proof-ok ${artifactSummaryPath}`);

function git(args) {
  return runCommand('git', args).stdout.trim();
}

function commandSucceeds(command, args) {
  return (
    spawnSync(command, args, {
      cwd: repoRoot,
      encoding: 'utf8',
      shell: process.platform === 'win32',
    }).status === 0
  );
}

function runCommand(command, args) {
  const result = spawnSync(command, args, {
    cwd: repoRoot,
    encoding: 'utf8',
    shell: process.platform === 'win32',
  });
  if (result.status !== 0) {
    throw new Error(`${command} ${args.join(' ')} failed\n${result.stdout}\n${result.stderr}`);
  }
  return result;
}
