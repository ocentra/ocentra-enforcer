import { spawn } from 'node:child_process';
import { existsSync, mkdirSync, readdirSync, readFileSync, rmSync, statSync, writeFileSync } from 'node:fs';
import { join, relative } from 'node:path';

const repoRoot = process.cwd();
const scenarioId = process.env.OCENTRA_SCREEN_AI_ACTION_SCENARIO ?? 'native-owned-process-time-limit';
const pipelineRoot = join(repoRoot, 'output', 'screen-ai-pipeline-proof');
const scenarioDir = join(pipelineRoot, scenarioId);
const outputDir = join(pipelineRoot, 'action-dispatch');
const adapterResultsDir = join(repoRoot, 'test-results', 'v0-8-windows-app-time-limit-adapter-mvp');

rmSync(outputDir, { recursive: true, force: true });
mkdirSync(outputDir, { recursive: true });

const screenPolicyArtifact = readJson(join(scenarioDir, '08-policy-decision.json'));
const screenAiArtifact = readJson(join(scenarioDir, '07-ai-result.json'));
const deletionArtifact = readJson(join(scenarioDir, '12-deletion-proof.json'));
const policyDecision = screenPolicyArtifact.policyDecision;
const screenResult = screenAiArtifact.screenResult;
const localAiSafetyResult = screenAiArtifact.localAiSafetyResult;

assertScreenPolicyInput();

const commandEnv = adapterCommandEnv();
writeJson(join(outputDir, '00-screen-policy-source.json'), {
  scenarioId,
  screenAnalysisResultId: screenResult.screenAnalysisResultId,
  localAiResultId: localAiSafetyResult.resultId,
  policyDecisionId: policyDecision.decisionId,
  action: policyDecision.action,
  reasonCodes: policyDecision.reasonCodes,
  ruleIds: policyDecision.ruleIds,
  evidenceReferences: policyDecision.evidenceReferences,
  rawImageDeletedBeforeDispatch: deletionArtifact.rawImageDeletedAfterAnalysis,
});
writeJson(join(outputDir, '01-adapter-command-env.json'), {
  command: 'node scripts/test/v0-8-windows-app-time-limit-adapter-mvp.mjs',
  env: commandEnv,
});

await runAdapterProof(commandEnv);

const adapterProofPath = latestJsonPath(adapterResultsDir);
const adapterProof = readJson(adapterProofPath);
const adapterAction = JSON.parse(adapterProof.events.execute.payload.enforcementAction);
const adapterTimer = JSON.parse(adapterProof.events.recover.payload.enforcementTimerEvent);
const adapterExpireResult = adapterProof.assertions.expire.adapterResultCode;
const expectedEvidenceRefs = policyDecision.evidenceReferences.map((reference) => reference.evidenceReferenceId);
const adapterEvidenceRefs = adapterAction.evidenceReferences.map((reference) => reference.evidenceReferenceId);

assertAdapterProof(adapterProof, adapterAction, adapterTimer, expectedEvidenceRefs, adapterEvidenceRefs);

const summary = {
  proof: 'screen-ai-action-dispatch-proof',
  proofTier: 'P3_LOCAL_DEV_MACHINE',
  scenarioId,
  platform: process.platform,
  screenPolicyDecisionId: policyDecision.decisionId,
  screenPolicyAction: policyDecision.action,
  screenPolicyWasDryRunPreview: policyDecision.dryRun,
  adapterCommandPolicyDecisionId: adapterProof.inputRefs.policyDecisionId,
  policyDecisionLinkedToAdapter: adapterProof.inputRefs.policyDecisionId === policyDecision.decisionId,
  localAiResultLinkedToScreenPolicy: policyDecision.localAiResultId === localAiSafetyResult.resultId,
  evidenceRefsLinkedToAdapter: expectedEvidenceRefs.every((referenceId) => adapterEvidenceRefs.includes(referenceId)),
  rawImageDeletedBeforeDispatch: deletionArtifact.rawImageDeletedAfterAnalysis,
  realWindowsAdapterProof:
    process.platform === 'win32' && ['process-terminated', 'process-already-exited'].includes(adapterExpireResult),
  adapterResultCode: adapterExpireResult,
  serviceTimerCreateRecoverCancelExpireProven: adapterProof.serviceScope.timeLimitCreateRecoverCancelExpireProven,
  expiryAdapterReachedThroughService: adapterProof.serviceScope.expiryAdapterReachedThroughService,
  restartRecoveryBacked: adapterProof.assertions.recover.recoveredAfterRestart,
  parentCancelBacked: adapterProof.assertions.cancel.stateCleared,
  expiryStateCleared: adapterProof.assertions.expire.stateCleared,
  adapterProofPath: relative(repoRoot, adapterProofPath),
  nonClaims: [
    'The source screen policy artifact is still a dry-run preview; this proof verifies handoff into a real adapter command path.',
    'The real adapter claim is Windows owned-process time-limit only; it does not claim browser URL blocking, network/domain blocking, or mobile child enforcement.',
    'Non-Windows CI may honestly report unsupported-platform for the adapter expiry step; Windows local proof must terminate or observe an already exited owned process.',
  ],
};

writeJson(join(outputDir, '02-adapter-proof-ref.json'), {
  adapterProofPath: relative(repoRoot, adapterProofPath),
  adapterAction,
  adapterTimer,
  expireAssertion: adapterProof.assertions.expire,
});
writeJson(join(outputDir, 'proof-summary.json'), summary);

if (process.platform === 'win32' && !summary.realWindowsAdapterProof) {
  throw new Error(`Windows action adapter proof did not enforce: ${JSON.stringify(summary, null, 2)}`);
}

console.log(
  `screen-ai-action-dispatch-proof-ok:${scenarioId}:${policyDecision.action}:${adapterProof.assertions.expire.timerEventKind}:${adapterExpireResult}`
);

function assertScreenPolicyInput() {
  if (policyDecision.action !== 'time-limit') {
    throw new Error(`Expected screen policy time-limit decision, received ${policyDecision.action}.`);
  }
  if (screenResult.primaryCategory !== 'game') {
    throw new Error(`Expected screen analysis game category, received ${screenResult.primaryCategory}.`);
  }
  if (policyDecision.localAiResultId !== localAiSafetyResult.resultId) {
    throw new Error('Screen policy decision is not linked to the local AI result.');
  }
  if (policyDecision.evidenceReferences.length === 0 || policyDecision.ruleIds.length === 0) {
    throw new Error('Screen policy decision is missing evidence references or rule ids.');
  }
  if (deletionArtifact.rawImageDeletedAfterAnalysis !== true) {
    throw new Error('Raw image must be deleted before adapter dispatch proof.');
  }
}

function adapterCommandEnv() {
  const evidenceReferenceIds = policyDecision.evidenceReferences
    .map((reference) => reference.evidenceReferenceId)
    .join(',');
  return {
    OCENTRA_PARENT_V08_APP_TIME_LIMIT_POLICY_DECISION_ID: policyDecision.decisionId,
    OCENTRA_PARENT_V08_APP_TIME_LIMIT_ACTION_ID: `action-${scenarioId}`,
    OCENTRA_PARENT_V08_APP_TIME_LIMIT_RESULT_ID: `result-${scenarioId}`,
    OCENTRA_PARENT_V08_APP_TIME_LIMIT_AUDIT_EVENT_ID: `audit-${scenarioId}`,
    OCENTRA_PARENT_V08_APP_TIME_LIMIT_TIMER_EVENT_ID: `timer-${scenarioId}`,
    OCENTRA_PARENT_V08_APP_TIME_LIMIT_PARENT_ACTION_REFERENCE_ID: `parent-action-${scenarioId}`,
    OCENTRA_PARENT_V08_APP_TIME_LIMIT_EVIDENCE_REFERENCE_IDS: evidenceReferenceIds,
    OCENTRA_PARENT_V08_APP_TIME_LIMIT_RULE_IDS: policyDecision.ruleIds.join(','),
    OCENTRA_PARENT_V08_APP_TIME_LIMIT_REASON_CODES: policyDecision.reasonCodes.join(','),
    OCENTRA_PARENT_V08_APP_TIME_LIMIT_INTENT_ID: `intent-${scenarioId}`,
    OCENTRA_PARENT_V08_APP_TIME_LIMIT_TARGET_ID: `target-${scenarioId}`,
  };
}

async function runAdapterProof(commandEnv) {
  const before = new Set(jsonFiles(adapterResultsDir));
  await runCommand('node', ['scripts/test/v0-8-windows-app-time-limit-adapter-mvp.mjs'], {
    ...process.env,
    ...commandEnv,
  });
  const after = jsonFiles(adapterResultsDir);
  const created = after.filter((path) => !before.has(path));
  if (created.length === 0) {
    throw new Error('Action adapter proof did not write a new evidence JSON file.');
  }
}

function assertAdapterProof(adapterProof, adapterAction, adapterTimer, expectedEvidenceRefs, adapterEvidenceRefs) {
  if (adapterProof.inputRefs.policyDecisionId !== policyDecision.decisionId) {
    throw new Error('Adapter proof input policy decision id did not match screen policy decision id.');
  }
  if (
    adapterAction.policyDecisionId !== policyDecision.decisionId ||
    adapterTimer.policyDecisionId !== policyDecision.decisionId
  ) {
    throw new Error('Adapter action/timer did not preserve screen policy decision id.');
  }
  if (adapterAction.policyAction !== 'time-limit' || adapterAction.dryRun !== false) {
    throw new Error(`Adapter action did not execute a real time-limit path: ${JSON.stringify(adapterAction)}`);
  }
  if (!expectedEvidenceRefs.every((referenceId) => adapterEvidenceRefs.includes(referenceId))) {
    throw new Error(
      `Adapter action did not preserve screen evidence refs: expected=${expectedEvidenceRefs.join(',')} actual=${adapterEvidenceRefs.join(',')}`
    );
  }
  if (adapterProof.assertions.recover.recoveredAfterRestart !== true) {
    throw new Error('Adapter proof did not recover timer state after restart.');
  }
  if (adapterProof.assertions.cancel.stateCleared !== true || adapterProof.assertions.expire.stateCleared !== true) {
    throw new Error('Adapter proof did not clear state on cancel and expire.');
  }
  if (adapterProof.serviceScope.expiryAdapterReachedThroughService !== true) {
    throw new Error('Adapter expiry did not go through the Rust service path.');
  }
}

function latestJsonPath(directory) {
  const files = jsonFiles(directory);
  if (files.length === 0) {
    throw new Error(`No JSON adapter proof files found in ${directory}.`);
  }
  return files.sort((a, b) => statSync(b).mtimeMs - statSync(a).mtimeMs)[0];
}

function jsonFiles(directory) {
  if (!existsSync(directory)) {
    return [];
  }
  return readdirSync(directory, { withFileTypes: true })
    .filter((entry) => entry.isFile() && entry.name.endsWith('.json'))
    .map((entry) => join(directory, entry.name));
}

function readJson(path) {
  if (!existsSync(path)) {
    throw new Error(`Required proof artifact is missing: ${relative(repoRoot, path)}`);
  }
  return JSON.parse(readFileSync(path, 'utf8'));
}

function writeJson(path, value) {
  writeFileSync(path, `${JSON.stringify(value, null, 2)}\n`);
}

function runCommand(command, args, env) {
  return new Promise((resolve, reject) => {
    const child = spawn(command, args, {
      cwd: repoRoot,
      env,
      stdio: 'inherit',
      shell: process.platform === 'win32',
      windowsHide: true,
    });
    child.once('exit', (code) => {
      if (code === 0) {
        resolve();
        return;
      }
      reject(new Error(`${command} ${args.join(' ')} exited with ${code}`));
    });
    child.once('error', reject);
  });
}
