import { createHash } from 'node:crypto';
import { spawn } from 'node:child_process';
import { existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { mkdtemp, readFile, writeFile } from 'node:fs/promises';
import { createServer } from 'node:net';
import { basename, join, relative } from 'node:path';
import { setTimeout as delay } from 'node:timers/promises';

import { resolveDebugAgentServicePath, stopProcessTreeAndWait } from './agent-service-process.mjs';

const repoRoot = process.cwd();
const scenarioId = process.env.OCENTRA_SCREEN_AI_BLOCK_SCENARIO ?? 'bypass-tool';
const timeoutMs = envNumber('OCENTRA_SCREEN_AI_BLOCK_ACTION_TIMEOUT_MS', 20_000);
const pipelineRoot = join(repoRoot, 'output', 'screen-ai-pipeline-proof');
const scenarioDir = join(pipelineRoot, scenarioId);
const outputDir = join(pipelineRoot, 'block-action-dispatch');
const safeScenarioId = proofToken(scenarioId, 'scenario id');

rmSync(outputDir, { recursive: true, force: true });
mkdirSync(outputDir, { recursive: true });

const screenPolicyArtifact = readJson(join(scenarioDir, '08-policy-decision.json'));
const screenAiArtifact = readJson(join(scenarioDir, '07-ai-result.json'));
const deletionArtifact = readJson(join(scenarioDir, '12-deletion-proof.json'));
const policyDecision = screenPolicyArtifact.policyDecision;
const screenResult = screenAiArtifact.screenResult;
const localAiSafetyResult = screenAiArtifact.localAiSafetyResult;
const sourcePolicyTarget = policyRuleTarget();

assertScreenBlockPolicyInput();
writeJson(join(outputDir, '00-screen-block-source.json'), screenBlockSource());

await main();

async function main() {
  await runCommand('cargo', ['build', '-p', 'ocentra-parent-agent-service']);
  const runRoot = await mkdtemp(join(outputDir, 'run-'));
  const agentPort = await freePort();
  const ownedChild = spawnOwnedChildProcess();
  const service = spawnAgentService(runRoot, agentPort);
  const serviceOutput = collectOutput(service);

  try {
    await waitForHealth(agentPort, serviceOutput);
    const event = await requestBlock(agentPort, ownedChild);
    const assertion = await assertBlockResult(event, ownedChild);
    const journalText = await readFile(join(runRoot, 'activity.ndjson'), 'utf8');
    if (journalText.includes(policyDecision.decisionId) || journalText.includes(assertion.expectedProcessName)) {
      throw new Error('Encrypted journal contains plaintext screen block proof identifiers.');
    }

    const adapterProof = {
      schemaVersion: 1,
      generatedAt: new Date().toISOString(),
      platform: process.platform,
      runRoot: relative(repoRoot, runRoot),
      childProcess: {
        pid: ownedChild.pid ?? null,
        expectedProcessName: assertion.expectedProcessName,
      },
      assertion,
      event: {
        event: proofToken(event.event, 'adapter event name'),
        severity: event.severity === undefined ? null : proofToken(event.severity, 'adapter event severity'),
        payloadSha256: sha256(JSON.stringify(event.payload ?? null)),
        payloadFieldNames: Object.keys(event.payload ?? {}).map((fieldName) =>
          proofToken(fieldName, 'adapter payload field')
        ),
      },
      artifacts: {
        activityJournal: relative(repoRoot, join(runRoot, 'activity.ndjson')),
        activityStore: relative(repoRoot, join(runRoot, 'activity.sqlite')),
        devLogDirectory: relative(repoRoot, join(runRoot, 'logs')),
      },
    };
    await writeFile(join(outputDir, '02-adapter-proof.json'), `${JSON.stringify(adapterProof, null, 2)}\n`);
    await writeFile(join(outputDir, 'proof-summary.json'), `${JSON.stringify(proofSummary(assertion), null, 2)}\n`);
    printSummary(assertion);
  } finally {
    await stopProcessTreeAndWait(service);
    await stopProcessTreeAndWait(ownedChild);
  }
}

function assertScreenBlockPolicyInput() {
  if (policyDecision.action !== 'block') {
    throw new Error(`Expected screen policy block decision, received ${policyDecision.action}.`);
  }
  if (localAiSafetyResult.action !== 'block') {
    throw new Error(`Expected local AI block result, received ${localAiSafetyResult.action}.`);
  }
  if (!['bypassTool', 'adultContent', 'violence'].includes(screenResult.primaryCategory)) {
    throw new Error(`Expected block-worthy screen category, received ${screenResult.primaryCategory}.`);
  }
  if (policyDecision.localAiResultId !== localAiSafetyResult.resultId) {
    throw new Error('Screen policy decision is not linked to the local AI result.');
  }
  if (policyDecision.evidenceReferences.length === 0 || policyDecision.ruleIds.length === 0) {
    throw new Error('Screen policy decision is missing evidence references or rule ids.');
  }
  if (policyDecision.dryRun !== true) {
    throw new Error('Source screen policy decision must remain a dry-run preview before adapter dispatch.');
  }
  if (deletionArtifact.rawImageDeletedAfterAnalysis !== true) {
    throw new Error('Raw image must be deleted before block adapter dispatch proof.');
  }
}

function screenBlockSource() {
  return {
    schemaVersion: 1,
    scenarioId: safeScenarioId,
    sourceScreenCategory: screenResult.primaryCategory,
    screenAnalysisResultId: screenResult.screenAnalysisResultId,
    localAiResultId: localAiSafetyResult.resultId,
    localAiAction: localAiSafetyResult.action,
    policyDecisionId: policyDecision.decisionId,
    sourcePolicyTarget,
    sourcePolicyAction: policyDecision.action,
    sourcePolicyDryRun: policyDecision.dryRun,
    reasonCodes: policyDecision.reasonCodes,
    ruleIds: policyDecision.ruleIds,
    evidenceReferences: policyDecision.evidenceReferences,
    rawImageDeletedBeforeDispatch: deletionArtifact.rawImageDeletedAfterAnalysis,
    adapterProofTarget: {
      targetType: 'process',
      targetValue: basename(process.execPath),
      controlledOwnedProcess: true,
    },
  };
}

function spawnOwnedChildProcess() {
  return spawn(process.execPath, ['-e', 'setInterval(() => {}, 1000);'], {
    cwd: repoRoot,
    stdio: 'ignore',
    windowsHide: true,
  });
}

function spawnAgentService(runRoot, agentPort) {
  return spawn(resolveDebugAgentServicePath(repoRoot), [], {
    cwd: repoRoot,
    env: {
      ...process.env,
      OCENTRA_PARENT_AGENT_ADDR: `127.0.0.1:${agentPort}`,
      OCENTRA_PARENT_ACTIVITY_DB_PATH: join(runRoot, 'activity.sqlite'),
      OCENTRA_PARENT_ACTIVITY_CAPTURE_STARTUP_DISABLED: 'true',
      OCENTRA_PARENT_ACTIVITY_JOURNAL_PATH: join(runRoot, 'activity.ndjson'),
      OCENTRA_PARENT_ACTIVITY_JOURNAL_KEY_PATH: join(runRoot, 'activity.key'),
      OCENTRA_PARENT_DEV_LOG_DIR: join(runRoot, 'logs'),
    },
    stdio: ['ignore', 'pipe', 'pipe'],
    windowsHide: true,
  });
}

async function waitForHealth(agentPort, serviceOutput) {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    try {
      const response = await fetch(`http://127.0.0.1:${agentPort}/health`);
      if (response.ok) {
        return;
      }
    } catch {
      await delay(250);
    }
  }
  throw new Error(`Timed out waiting for service health. ${serviceOutput()}`);
}

function requestBlock(agentPort, ownedChild) {
  return new Promise((resolve, reject) => {
    const socket = new WebSocket(`ws://127.0.0.1:${agentPort}/api/dev/ws`);
    const timer = setTimeout(() => {
      socket.close();
      reject(new Error('Timed out waiting for screen-derived block enforcement audit event.'));
    }, timeoutMs);

    socket.addEventListener('open', () => {
      socket.send(JSON.stringify(commandEnvelope(ownedChild)));
    });
    socket.addEventListener('message', (message) => {
      const event = JSON.parse(String(message.data));
      if (event.event === 'agent.enforcement.audit.reported' || event.event === 'agent.command.rejected') {
        clearTimeout(timer);
        socket.close();
        resolve(event);
      }
    });
    socket.addEventListener('error', () => {
      clearTimeout(timer);
      reject(new Error('WebSocket error while requesting screen-derived block enforcement.'));
    });
  });
}

async function assertBlockResult(event, ownedChild) {
  const expectedProcessName = basename(process.execPath);
  if (process.platform === 'win32') {
    if (event.event !== 'agent.enforcement.audit.reported') {
      throw new Error(`Expected enforcement audit event, received ${event.event}: ${JSON.stringify(event.payload)}`);
    }
    if (event.payload.enforcementStatus !== 'actually-enforced') {
      throw new Error(`Expected actually-enforced, received ${event.payload.enforcementStatus}.`);
    }
    if (event.payload.enforcementAdapterResultCode !== 'process-terminated') {
      throw new Error(`Expected process-terminated, received ${event.payload.enforcementAdapterResultCode}.`);
    }
    await waitForExit(ownedChild);
  } else if (event.payload.enforcementStatus !== 'unavailable') {
    throw new Error(`Expected unavailable on non-Windows, received ${event.payload.enforcementStatus}.`);
  }
  if (event.payload.databaseReady !== true || Number(event.payload.eventsStored) < 1) {
    throw new Error(`Expected journal/store proof, payload=${JSON.stringify(event.payload)}`);
  }

  const enforcementAction = parseEmbeddedJson(event.payload.enforcementAction, 'enforcementAction');
  const enforcementResult = parseEmbeddedJson(event.payload.enforcementResult, 'enforcementResult');
  const adapterEvidenceRefs = enforcementAction.evidenceReferences.map((reference) => reference.evidenceReferenceId);
  const expectedEvidenceRefs = policyDecision.evidenceReferences.map((reference) => reference.evidenceReferenceId);
  if (enforcementAction.policyDecisionId !== policyDecision.decisionId) {
    throw new Error('Block adapter action did not preserve the screen policy decision id.');
  }
  if (enforcementAction.localAiResultId !== localAiSafetyResult.resultId) {
    throw new Error('Block adapter action did not preserve the local AI result id.');
  }
  if (enforcementAction.policyAction !== 'block' || enforcementAction.dryRun !== false) {
    throw new Error(`Block adapter action was not real block execution: ${JSON.stringify(enforcementAction)}`);
  }
  if (enforcementAction.target.targetType !== 'process') {
    throw new Error(`Block adapter target must be owned process, received ${enforcementAction.target.targetType}.`);
  }
  if (!expectedEvidenceRefs.every((referenceId) => adapterEvidenceRefs.includes(referenceId))) {
    throw new Error(
      `Block adapter action did not preserve screen evidence refs: expected=${expectedEvidenceRefs.join(',')} actual=${adapterEvidenceRefs.join(',')}`
    );
  }

  return {
    servicePathProven: true,
    screenPolicyDecisionId: policyDecision.decisionId,
    sourceScreenCategory: screenResult.primaryCategory,
    sourceScreenPolicyDryRun: policyDecision.dryRun,
    localAiResultId: localAiSafetyResult.resultId,
    expectedProcessName,
    status: proofToken(event.payload.enforcementStatus, 'enforcement status'),
    adapterResultCode: proofToken(event.payload.enforcementAdapterResultCode, 'enforcement adapter result code'),
    rollbackState: proofToken(event.payload.enforcementRollbackState, 'enforcement rollback state'),
    journalEventId: proofToken(event.payload.enforcementJournalEventId, 'enforcement journal event id'),
    databaseReady: event.payload.databaseReady,
    eventsStored: event.payload.eventsStored,
    adapterPolicyDecisionId: proofToken(enforcementAction.policyDecisionId, 'adapter policy decision id'),
    adapterLocalAiResultId: proofToken(enforcementAction.localAiResultId, 'adapter local AI result id'),
    adapterPolicyAction: proofToken(enforcementAction.policyAction, 'adapter policy action'),
    adapterDryRun: enforcementAction.dryRun,
    adapterTargetType: proofToken(enforcementAction.target.targetType, 'adapter target type'),
    adapterTargetValue: proofToken(enforcementAction.target.targetValue, 'adapter target value'),
    adapterEvidenceRefs: adapterEvidenceRefs.map((referenceId) =>
      proofToken(referenceId, 'adapter evidence reference id')
    ),
    expectedEvidenceRefs: expectedEvidenceRefs.map((referenceId) =>
      proofToken(referenceId, 'expected evidence reference id')
    ),
    resultStatus: proofToken(enforcementResult.status, 'enforcement result status'),
    resultAdapterCode: proofToken(enforcementResult.adapterResultCode, 'enforcement result adapter code'),
  };
}

function commandEnvelope(ownedChild) {
  const now = new Date();
  const evidenceReferenceIds = policyDecision.evidenceReferences
    .map((reference) => reference.evidenceReferenceId)
    .join(',');

  return {
    schemaVersion: 1,
    messageId: `cmd-screen-ai-block-${safeScenarioId}`,
    sentAt: now.toISOString(),
    source: { peerId: 'portal-dev', role: 'portal' },
    target: { deviceId: 'local-dev-agent', platform: platformLabel(), route: 'localhost' },
    command: 'agent.enforcement.execute',
    payload: {
      policyDecisionId: proofToken(policyDecision.decisionId, 'policy decision id'),
      policyVersion: `policy-screen-ai-block-${safeScenarioId}`,
      policyAction: 'block',
      targetType: 'process',
      targetId: `target-screen-ai-block-owned-process-${safeScenarioId}`,
      targetValue: basename(process.execPath),
      dryRun: false,
      reasonCodes: policyDecision.reasonCodes.map((reasonCode) => proofToken(reasonCode, 'reason code')).join(','),
      ruleIds: policyDecision.ruleIds.map((ruleId) => proofToken(ruleId, 'rule id')).join(','),
      evidenceReferenceIds: evidenceReferenceIds
        .split(',')
        .map((referenceId) => proofToken(referenceId, 'evidence reference id'))
        .join(','),
      localAiResultId: proofToken(localAiSafetyResult.resultId, 'local AI result id'),
      requestedAt: now.toISOString(),
      enforcementActionId: `action-screen-ai-block-${safeScenarioId}`,
      enforcementResultId: `result-screen-ai-block-${safeScenarioId}`,
      enforcementAuditEventId: `audit-screen-ai-block-${safeScenarioId}`,
      enforcementTimerEventId: `timer-screen-ai-block-${safeScenarioId}`,
      enforcementIntentId: `intent-screen-ai-block-${safeScenarioId}`,
      rollbackToken: `rollback-screen-ai-block-${safeScenarioId}`,
      processId: ownedChild.pid,
    },
  };
}

function proofSummary(assertion) {
  return {
    proof: 'screen-ai-block-action-dispatch-proof',
    proofTier: 'P3_LOCAL_DEV_MACHINE',
    scenarioId: safeScenarioId,
    platform: process.platform,
    sourceScreenCategory: screenResult.primaryCategory,
    screenPolicyDecisionId: policyDecision.decisionId,
    screenPolicyAction: policyDecision.action,
    screenPolicyWasDryRunPreview: policyDecision.dryRun,
    adapterCommandPolicyDecisionId: assertion.adapterPolicyDecisionId,
    policyDecisionLinkedToAdapter: assertion.adapterPolicyDecisionId === policyDecision.decisionId,
    localAiResultLinkedToAdapter: assertion.adapterLocalAiResultId === localAiSafetyResult.resultId,
    evidenceRefsLinkedToAdapter: assertion.expectedEvidenceRefs.every((referenceId) =>
      assertion.adapterEvidenceRefs.includes(referenceId)
    ),
    rawImageDeletedBeforeDispatch: deletionArtifact.rawImageDeletedAfterAnalysis,
    realWindowsBlockAdapterProof: process.platform === 'win32' && assertion.adapterResultCode === 'process-terminated',
    adapterResultCode: assertion.adapterResultCode,
    adapterStatus: assertion.status,
    adapterTargetType: assertion.adapterTargetType,
    adapterTargetValue: assertion.adapterTargetValue,
    sourcePolicyTarget,
    adapterTargetIsControlledOwnedProcessSubstitution: true,
    servicePathProven: assertion.servicePathProven,
    databaseReady: assertion.databaseReady,
    eventsStored: assertion.eventsStored,
    nonClaims: [
      'The source screen policy artifact is a dry-run preview; this proof validates handoff into a real adapter command path.',
      'The real adapter claim is Windows owned-process block only; it does not claim browser URL, category, network/domain, mobile, or broad block adapters.',
      'The source screen block decision may target a category, but this proof intentionally substitutes a controlled owned process because that is the only block adapter with real execution proof in this slice.',
      'Live external URL/account proof remains separate from this controlled screen fixture proof.',
    ],
  };
}

function policyRuleTarget() {
  const rules = screenPolicyArtifact.familyPolicySet?.rules ?? [];
  const matchingRule = rules.find((rule) => policyDecision.ruleIds.includes(rule.ruleId));
  return matchingRule?.target ?? null;
}

function waitForExit(child) {
  if (child.exitCode !== undefined || child.signalCode !== undefined) {
    return Promise.resolve();
  }
  return new Promise((resolve, reject) => {
    const timer = setTimeout(() => reject(new Error('Owned child process was not terminated.')), timeoutMs);
    child.once('exit', () => {
      clearTimeout(timer);
      resolve();
    });
  });
}

async function freePort() {
  const server = createServer();
  await new Promise((resolve, reject) => {
    server.once('error', reject);
    server.listen(0, '127.0.0.1', resolve);
  });
  const port = server.address().port;
  await new Promise((resolve) => server.close(resolve));
  return port;
}

function collectOutput(child) {
  const chunks = [];
  child.stdout.on('data', (chunk) => chunks.push(String(chunk)));
  child.stderr.on('data', (chunk) => chunks.push(String(chunk)));
  return () => chunks.join('');
}

function readJson(path) {
  if (!existsSync(path)) {
    throw new Error(
      `Required proof artifact is missing: ${relative(repoRoot, path)}. Run OCENTRA_SCREEN_AI_SCENARIOS=${scenarioId} node scripts/test/screen-ai-local-vlm-proof.mjs first.`
    );
  }
  return JSON.parse(readFileSync(path, 'utf8'));
}

function writeJson(path, value) {
  writeFileSync(path, `${JSON.stringify(value, null, 2)}\n`);
}

function parseEmbeddedJson(value, label) {
  if (typeof value !== 'string' || value.trim().length === 0) {
    throw new Error(`Expected ${label} serialized JSON payload.`);
  }
  return JSON.parse(value);
}

function proofToken(value, label) {
  const text = String(value ?? '');
  if (!/^[A-Za-z0-9._:-]+$/u.test(text)) {
    throw new Error(`Unexpected ${label} token shape.`);
  }
  return text;
}

function sha256(value) {
  return createHash('sha256').update(value).digest('hex');
}

function platformLabel() {
  if (process.platform === 'win32') {
    return 'windows';
  }
  if (process.platform === 'darwin') {
    return 'macos';
  }
  return 'linux';
}

function envNumber(name, fallback) {
  const parsed = Number(process.env[name]);
  return Number.isFinite(parsed) && parsed > 0 ? parsed : fallback;
}

function runCommand(command, args) {
  return new Promise((resolve, reject) => {
    const child = spawn(command, args, {
      cwd: repoRoot,
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

function printSummary(assertion) {
  console.log(
    `screen-ai-block-action-dispatch-proof-ok:${safeScenarioId}:${proofToken(policyDecision.action, 'policy action')}:${proofToken(assertion.status, 'adapter status')}:${proofToken(assertion.adapterResultCode, 'adapter result code')}`
  );
  console.log(`evidence=${relative(repoRoot, join(outputDir, 'proof-summary.json'))}`);
}
