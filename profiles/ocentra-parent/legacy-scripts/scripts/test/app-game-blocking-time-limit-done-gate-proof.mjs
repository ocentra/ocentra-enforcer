import { spawn } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';

const repoRoot = process.cwd();
const outputDir = join(repoRoot, 'test-results', 'app-game-blocking-time-limit-done-gate-proof');
const proofPath = join(outputDir, 'proof.json');
const planOutputDir = join(repoRoot, 'output', 'app-game-plan-proof', '176-app-game-blocking-time-limit-done-gate');
const planProofPath = join(planOutputDir, 'proof.json');
const commands = [];

await main();

async function main() {
  await mkdir(outputDir, { recursive: true });
  await mkdir(planOutputDir, { recursive: true });

  await runCommand('node', ['scripts/test/app-game-scoped-adapter-dispatch-execute-command-proof.mjs']);
  await runCommand('node', ['scripts/test/app-game-scoped-adapter-dispatch-parent-action-surface-proof.mjs']);
  await runCommand('node', ['scripts/test/app-game-broad-blocking-proof-gates.mjs']);

  const executeProof = await readJson(
    'output/app-game-plan-proof/173-app-game-scoped-adapter-dispatch-execute-command/proof.json'
  );
  const parentActionProof = await readJson(
    'output/app-game-plan-proof/175-app-game-scoped-adapter-dispatch-parent-action-surface/proof.json'
  );
  const broadBlockingProof = await readJson(
    'output/app-game-plan-proof/23-broad-blocking-proof-gates/03-runtime-evidence.json'
  );

  assertScopedExecuteProof(executeProof);
  assertParentActionProof(parentActionProof);
  assertBroadBlockingProof(broadBlockingProof);

  const proof = {
    schemaVersion: 1,
    proofMode: 'app-game-blocking-time-limit-done-gate-proof',
    checkedAt: new Date().toISOString(),
    commit: await gitHead(),
    commands,
    sourceProofs: {
      scopedExecute: 'output/app-game-plan-proof/173-app-game-scoped-adapter-dispatch-execute-command/proof.json',
      parentAction: 'output/app-game-plan-proof/175-app-game-scoped-adapter-dispatch-parent-action-surface/proof.json',
      broadBlocking: 'output/app-game-plan-proof/23-broad-blocking-proof-gates/03-runtime-evidence.json',
    },
    summary: {
      scopedWindowsOwnedProcessTimeLimitReady: true,
      parentActionRequiresExplicitClick: true,
      readModelCommandsSideEffectFree: true,
      broadInstalledAppBlockingClaimed: false,
      broadBlockingAdapterCallsAllowed: 0,
      broadBlockingDispatchEligibleRows: 0,
      platformEnforcementClaimedOutsideScopedWindowsOwnedProcess: false,
      providerDeliveryClaimed: false,
      childDeviceDeliveryClaimed: false,
      rawPrivateRowsOrTargetsClaimed: false,
      privateDiagnosticsClaimed: false,
    },
    doneGateDecision: {
      featureDocItemClosed: true,
      centralProductChecklistUpdated: false,
      centralProductChecklistReason:
        'This closes the app-game feature-local blocking/time-limit done gate without changing the central product capability row while another lane may own checklist churn.',
      remainingNoClaims: [
        'broad installed-app blocking execution',
        'non-scoped platform enforcement',
        'provider delivery or provider receipt ingestion',
        'child-device runtime delivery',
        'raw private source rows or target values',
        'private diagnostics',
      ],
    },
    claimsProved: [
      'the scoped Windows owned-process app/game timer path can run the typed manual adapter-dispatch execute command',
      'the App/Game Sessions parent surface exposes execution only as an explicit parent action',
      'side-effect-free read-model refresh commands do not execute adapter dispatch',
      'broad/platform blocking gates still have zero dispatch-eligible rows and zero adapter calls',
    ],
  };

  await writeJson(proofPath, proof);
  await writeJson(planProofPath, proof);
  console.log('app-game-blocking-time-limit-done-gate-proof-ok');
  console.log(`evidence=${relative(repoRoot, proofPath)}`);
  console.log(`planEvidence=${relative(repoRoot, planProofPath)}`);
}

async function readJson(path) {
  return JSON.parse(await readFile(join(repoRoot, path), 'utf8'));
}

function assertScopedExecuteProof(proof) {
  assertEqual(proof.summary.dispatchExecuteRows, 1, 'scoped execute dispatch rows');
  assertEqual(proof.summary.blockedBeforeExecutionRows, 7, 'scoped execute blocked rows');
  assertEqual(proof.summary.executionStatus, 'actually-enforced', 'scoped execute status');
  assertEqual(proof.summary.readModelCommandSideEffectFree, true, 'execute read-model side effects');
  assertNoBroadClaim(proof.summary);
}

function assertParentActionProof(proof) {
  assertEqual(proof.summary.appGameSessionsRouteMounted, true, 'parent route mounted');
  assertEqual(proof.summary.executeButtonRequiresAcceptedScopedRow, true, 'execute button scoped gate');
  assertEqual(proof.summary.executeButtonSendsTypedCommand, true, 'execute typed command');
  assertEqual(proof.summary.executeButtonSelectsExecutedEvent, true, 'execute result event');
  assertEqual(proof.summary.overviewAutoExecute, false, 'overview auto execute');
  assertNoBroadClaim(proof.summary);
}

function assertBroadBlockingProof(proof) {
  assertEqual(proof.counts.gateCount, 7, 'broad blocking gate count');
  assertEqual(proof.counts.dispatchEligible, 0, 'broad blocking dispatch eligible');
  assertEqual(proof.counts.adapterCallAllowed, 0, 'broad blocking adapter calls');
  assertEqual(proof.counts.broadBlockingClaimed, 0, 'broad blocking claimed');
  assertEqual(proof.counts.byOutcomeState['manual-required'], 5, 'manual-required broad gates');
  assertEqual(proof.counts.byOutcomeState.unavailable, 1, 'unavailable broad gates');
  assertEqual(proof.counts.byOutcomeState['not-claimed'], 1, 'not-claimed broad gates');
}

function assertNoBroadClaim(summary) {
  assertEqual(summary.broadInstalledAppBlockingClaimed, false, 'broad installed app blocking claim');
  assertEqual(summary.childDeviceDeliveryClaimed, false, 'child delivery claim');
  assertEqual(summary.platformEnforcementClaimed, false, 'platform enforcement claim');
  assertEqual(summary.providerDeliveryClaimed, false, 'provider delivery claim');
  assertEqual(summary.privateDiagnosticsClaimed, false, 'private diagnostics claim');
}

function assertEqual(actual, expected, label) {
  if (actual !== expected) {
    throw new Error(`${label}: expected ${expected}, received ${actual}`);
  }
}

async function writeJson(path, value) {
  await writeFile(path, `${JSON.stringify(value, null, 2)}\n`);
}

async function runCommand(command, args) {
  const commandLine = [command, ...args].join(' ');
  commands.push(commandLine);
  await new Promise((resolve, reject) => {
    const child = spawn(command, args, { cwd: repoRoot, stdio: 'inherit', windowsHide: true });
    child.once('exit', (code) => (code === 0 ? resolve() : reject(new Error(`${commandLine} exited with ${code}`))));
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
