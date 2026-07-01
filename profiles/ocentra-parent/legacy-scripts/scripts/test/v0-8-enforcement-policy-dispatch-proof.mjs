import { spawn } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';

const repoRoot = process.cwd();
const outputDir = join(repoRoot, 'test-results', 'v0-8-enforcement-policy-dispatch-proof');
const proofPath = join(outputDir, 'proof.json');
const commands = [];
const proofLabels = [];

await main();

async function main() {
  await mkdir(outputDir, { recursive: true });

  await runCommand(...npmCommand(['run', 'build', '--workspace', '@ocentra-parent/schema-domain']));
  await runCommand('cargo', ['test', '-p', 'ocentra-parent-agent-protocol', 'enforcement_policy_dispatch']);
  await runCommand('cargo', ['test', '-p', 'ocentra-parent-agent-core', 'enforcement_policy_dispatch']);
  await runCommand('cargo', ['test', '-p', 'ocentra-parent-agent-service', 'enforcement_policy_dispatch']);
  await assertProtocolHarness();

  const { EnforcementPolicyDispatchReadModel } =
    await import('@ocentra-parent/schema-domain/enforcement-policy-dispatch');
  const summary = summarizeReadModel(EnforcementPolicyDispatchReadModel);

  assertReadModel(EnforcementPolicyDispatchReadModel, summary);

  const proof = {
    schemaVersion: 1,
    proofMode: 'v0-8-enforcement-policy-dispatch-proof',
    checkedAt: new Date().toISOString(),
    commit: await gitHead(),
    commands,
    proofLabels,
    evidence: {
      generatedContract: 'packages/schema-domain/src/enforcement-policy-dispatch.ts',
      rustProtocol: 'crates/agent-protocol/src/enforcement_policy_dispatch.rs',
      rustProtocolTest: 'crates/agent-protocol/tests/unit/enforcement_policy_dispatch_tests.rs',
      rustCoreValidator: 'crates/agent-core/src/enforcement_policy_dispatch.rs',
      rustCoreTest: 'crates/agent-core/tests/unit/enforcement_policy_dispatch_tests.rs',
      rustServiceReadModel: 'crates/agent-service/src/enforcement_policy_dispatch_read_model.rs',
      rustServiceTest: 'crates/agent-service/tests/unit/enforcement_policy_dispatch_read_model_tests.rs',
      rustServiceCommand: 'agent.enforcement.policy-dispatch.get',
      rustServiceEvent: 'agent.enforcement.policy-dispatch.reported',
      proofHarness: 'scripts/test/v0-8-enforcement-policy-dispatch-proof.mjs',
    },
    counts: summary,
    claimsProved: [
      'Parent-authored policy dispatch intents are schema-backed in schema-domain before service/runtime use',
      'Service read model validates actor, target device, stable policy decision refs, schedule refs, evidence refs, route/source state, adapter capability, and proof level before dispatch-ready states',
      'Capability matrix preserves ask-parent dry-run-only, report-only, manual-required, stale-policy rejection, missing-source rejection, and scaffold states without upgrading them into adapter execution',
      'Malformed or missing policy decision references are rejected by the schema-domain and Rust-core validation path before dispatch-ready states',
      'Owned-process and app/game time-limit rows are dispatch-ready only with evidence refs and child reason codes',
      'Network/domain blocking, source-not-ready rows, and tamper/uninstall stay manual-required, rejected, or scaffold, not product-complete claims',
      'The generated schema-domain read model stays aligned with the Rust service-backed policy-dispatch contract',
    ],
    claimsNotProved: [
      'broad installed-app blocking',
      'host network or domain blocking',
      'managed active-tab exact URL enforcement',
      'unmanaged browser exact URL evidence',
      'notification delivery',
      'tamper resistance or uninstall hardening',
      'mobile enforcement parity',
    ],
  };

  await writeFile(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
  console.log(`v0-8-enforcement-policy-dispatch-proof-ok:${proofLabels.join(',')}`);
  console.log(`evidence=${relative(repoRoot, proofPath)}`);
}

function summarizeReadModel(readModel) {
  return {
    entries: readModel.entries.length,
    byProofLevel: countBy(readModel.entries.map((entry) => entry.matrixRow.proofLevel)),
    byOutcomeState: countBy(readModel.entries.map((entry) => entry.matrixRow.outcomeState)),
    byApprovalState: countBy(readModel.entries.map((entry) => entry.approvalState)),
    byRequestedParentAction: countBy(readModel.entries.map((entry) => entry.intent.requestedParentAction)),
    byRejectionReason: countBy(readModel.entries.map((entry) => entry.matrixRow.rejectionReason)),
    byTimerState: countBy(readModel.entries.map((entry) => entry.timerState)),
    bySourceState: countBy(readModel.entries.map((entry) => entry.intent.sourceState)),
    dispatchReady: readModel.entries.filter((entry) => entry.matrixRow.outcomeState === 'dispatch-ready').length,
    dryRunOnly: readModel.entries.filter((entry) => entry.matrixRow.outcomeState === 'dry-run-only').length,
    manualRequired: readModel.entries.filter((entry) => entry.matrixRow.outcomeState === 'manual-required').length,
    reportOnly: readModel.entries.filter((entry) => entry.matrixRow.outcomeState === 'report-only').length,
    rejected: readModel.entries.filter((entry) => entry.matrixRow.outcomeState === 'rejected').length,
  };
}

function assertReadModel(readModel, summary) {
  assertEqual(readModel.readModelId, 'v0-8-enforcement-policy-dispatch', 'read model id');
  assertEqual(summary.entries, 8, 'entry count');
  assertEqual(summary.byProofLevel.implemented, 2, 'implemented proof count');
  assertEqual(summary.byProofLevel.scaffold, 4, 'scaffold proof count');
  assertEqual(summary.byProofLevel['report-only'], 1, 'report-only proof count');
  assertEqual(summary.byProofLevel['manual-required'], 1, 'manual-required proof count');
  assertEqual(summary.dispatchReady, 2, 'dispatch-ready count');
  assertEqual(summary.dryRunOnly, 1, 'dry-run-only count');
  assertEqual(summary.reportOnly, 1, 'report-only count');
  assertEqual(summary.manualRequired, 1, 'manual-required count');
  assertEqual(summary.rejected, 3, 'rejected count');
  assertEqual(summary.byTimerState['restart-recovered'], 1, 'restart recovered timer count');
  assertEqual(summary.byTimerState['recovery-needed'], 1, 'recovery needed timer count');
  assertEqual(summary.bySourceState.ready, 5, 'ready source count');
  assertEqual(summary.bySourceState.stale, 1, 'stale source count');
  assertEqual(summary.bySourceState.missing, 1, 'missing source count');
  assertEqual(summary.bySourceState.unavailable, 1, 'unavailable source count');
  assertEqual(summary.byRequestedParentAction['ask-parent'], 1, 'ask-parent count');
  assertEqual(summary.byRejectionReason['stale-policy-version'], 1, 'stale rejection count');
  assertEqual(summary.byRejectionReason['source-not-ready'], 1, 'source-not-ready rejection count');

  assertEntry(readModel, 'dispatch-owned-process-time-limit', {
    proofLevel: 'implemented',
    outcomeState: 'dispatch-ready',
    evidenceReferenceId: 'evidence-app-session-owned-process',
    childReasonCode: 'child-reason-time-limit-reached',
  });
  assertEntry(readModel, 'dispatch-app-game-session-handoff', {
    proofLevel: 'implemented',
    outcomeState: 'dispatch-ready',
    evidenceReferenceId: 'evidence-app-game-session-summary',
    childReasonCode: 'child-reason-parent-approval-bonus-time',
  });
  assertEntry(readModel, 'dispatch-ask-parent-dry-run', {
    proofLevel: 'scaffold',
    outcomeState: 'dry-run-only',
    evidenceReferenceId: 'evidence-app-game-session-summary',
    childReasonCode: 'child-reason-ask-parent-review-required',
    requestedParentAction: 'ask-parent',
    requestedPolicyAction: 'ask-parent',
    dryRun: true,
    approvalState: 'pending',
    sourceState: 'ready',
  });
  assertEntry(readModel, 'dispatch-network-domain-manual-required', {
    proofLevel: 'manual-required',
    outcomeState: 'manual-required',
    evidenceReferenceId: 'evidence-network-flow-domain-summary',
    childReasonCode: 'child-reason-adapter-manual-required',
  });
  assertEntry(readModel, 'dispatch-stale-policy-version-rejected', {
    proofLevel: 'scaffold',
    outcomeState: 'rejected',
    evidenceReferenceId: 'evidence-policy-decision-stale',
    childReasonCode: 'child-reason-policy-version-stale',
    rejectionReason: 'stale-policy-version',
    sourceState: 'stale',
  });
  assertEntry(readModel, 'dispatch-missing-source-rejected', {
    proofLevel: 'scaffold',
    outcomeState: 'rejected',
    evidenceReferenceId: 'evidence-policy-source-missing',
    childReasonCode: 'child-reason-source-not-ready',
    rejectionReason: 'source-not-ready',
    sourceState: 'missing',
  });
  assertEntry(readModel, 'dispatch-tamper-alert-scaffold', {
    proofLevel: 'scaffold',
    outcomeState: 'rejected',
    evidenceReferenceId: 'evidence-integrity-heartbeat-gap',
    childReasonCode: 'child-reason-integrity-proof-required',
  });

  proofLabels.push('v0.8.policy-dispatch.contract-boundary');
  proofLabels.push('v0.8.policy-dispatch.service-read-model');
  proofLabels.push('v0.8.policy-dispatch.capability-matrix');
  proofLabels.push('v0.8.policy-dispatch.ask-parent-and-stale-rejections');
  proofLabels.push('v0.8.policy-dispatch.timer-approval-audit-reasons');
  proofLabels.push('v0.8.policy-dispatch.no-claim-upgrade');
}

function assertEntry(readModel, intentId, expected) {
  const entry = readModel.entries.find((candidate) => candidate.intent.intentId === intentId);
  if (entry === undefined) {
    throw new Error(`missing dispatch entry ${intentId}`);
  }
  assertEqual(entry.matrixRow.proofLevel, expected.proofLevel, `${intentId} proof level`);
  assertEqual(entry.matrixRow.outcomeState, expected.outcomeState, `${intentId} outcome state`);
  assertEqual(
    entry.intent.evidenceReferences[0]?.evidenceReferenceId,
    expected.evidenceReferenceId,
    `${intentId} evidence ref`
  );
  assertEqual(entry.childReasonCode, expected.childReasonCode, `${intentId} child reason`);
  if (expected.rejectionReason !== undefined) {
    assertEqual(entry.matrixRow.rejectionReason, expected.rejectionReason, `${intentId} rejection reason`);
  }
  if (expected.requestedParentAction !== undefined) {
    assertEqual(entry.intent.requestedParentAction, expected.requestedParentAction, `${intentId} parent action`);
  }
  if (expected.requestedPolicyAction !== undefined) {
    assertEqual(entry.intent.requestedPolicyAction, expected.requestedPolicyAction, `${intentId} policy action`);
  }
  if (expected.dryRun !== undefined) {
    assertEqual(entry.intent.dryRun, expected.dryRun, `${intentId} dry-run flag`);
  }
  if (expected.approvalState !== undefined) {
    assertEqual(entry.approvalState, expected.approvalState, `${intentId} approval state`);
  }
  if (expected.sourceState !== undefined) {
    assertEqual(entry.intent.sourceState, expected.sourceState, `${intentId} source state`);
  }
}

function countBy(values) {
  return values.reduce((counts, value) => {
    counts[value] = (counts[value] ?? 0) + 1;
    return counts;
  }, {});
}

function assertEqual(actual, expected, label) {
  if (actual !== expected) {
    throw new Error(`${label}: expected ${expected}, received ${actual}`);
  }
}

async function assertProtocolHarness() {
  const harness = await readFile(join(repoRoot, 'crates/agent-protocol/tests/unit.rs'), 'utf8');
  assertIncludes(
    harness,
    '#[path = "unit/enforcement_policy_dispatch_tests.rs"]',
    'enforcement policy dispatch unit harness registration exists'
  );
}

async function runCommand(command, args) {
  const commandLine = [command, ...args].join(' ');
  commands.push(commandLine);
  await new Promise((resolve, reject) => {
    const child = spawn(command, args, { cwd: repoRoot, shell: false, stdio: 'inherit' });
    child.on('error', reject);
    child.on('exit', (code) => {
      if (code === 0) {
        resolve();
      } else {
        reject(new Error(`${commandLine} exited with ${code}`));
      }
    });
  });
}

async function gitHead() {
  const chunks = [];
  await new Promise((resolve, reject) => {
    const child = spawn('git', ['rev-parse', 'HEAD'], {
      cwd: repoRoot,
      shell: false,
      stdio: ['ignore', 'pipe', 'pipe'],
    });
    child.stdout.on('data', (chunk) => chunks.push(chunk));
    child.on('error', reject);
    child.on('exit', (code) => {
      if (code === 0) {
        resolve();
      } else {
        reject(new Error(`git rev-parse HEAD exited with ${code}`));
      }
    });
  });
  return Buffer.concat(chunks).toString('utf8').trim();
}

function npmCommand(args) {
  const command = process.platform === 'win32' ? 'cmd' : 'npm';
  const commandArgs = process.platform === 'win32' ? ['/c', 'npm', ...args] : args;
  return [command, commandArgs];
}

function assertIncludes(text, expected, label) {
  if (!text.includes(expected)) {
    throw new Error(`${label}: missing ${expected}`);
  }
}
