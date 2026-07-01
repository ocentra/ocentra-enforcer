import { spawn } from 'node:child_process';
import { once } from 'node:events';
import { mkdir, mkdtemp, readdir, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { setTimeout as delay } from 'node:timers/promises';
import { fileURLToPath } from 'node:url';

import {
  ParentDevEnv,
  ParentDevHost,
  ParentDevPort,
  createAgentAddress,
  createAgentHealthUrl,
  createAgentWebSocketUrl,
  createHttpOrigin,
  createPortalCommandsUrl,
  isLikelyParentAgentOccupant,
  isLikelyParentPortalOccupant,
  resolveParentDevPort,
} from '../dev/local-dev-config.mjs';
import { ensurePortFree } from '../dev/port-utils.mjs';
import { resolveDebugAgentServicePath, spawnVitePortal, stopProcessTreeAndWait } from './agent-service-process.mjs';

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..', '..');
const portalRoot = path.join(repoRoot, 'apps', 'portal');
const proofRoot = path.join(repoRoot, 'output', 'browser-plan-proof', 'ai-20-parent-explanation-audit-ux');
const screenshotDir = path.join(proofRoot, '06-ui-snapshots');
const proofResultDir = path.join(repoRoot, 'test-results', 'browser-ai-parent-explanation-rendered-proof');
const proofPath = path.join(proofResultDir, 'proof.json');
const outputProofPath = path.join(proofRoot, '03-runtime-evidence.json');
const sourceSnapshotPath = path.join(proofRoot, '00-source-snapshot.md');
const validationLogPath = path.join(proofRoot, '10-validation-commands.log');
const securityLogPath = path.join(proofRoot, '08-security-negative-proof.log');
const playwrightLogPath = path.join(screenshotDir, 'browser-parent-explanation-ui-playwright.log');
const desktopScreenshot = path.join(screenshotDir, 'browser-parent-explanation-route.png');
const mobileScreenshot = path.join(screenshotDir, 'browser-parent-explanation-route-mobile.png');
const accessibilitySummaryPath = path.join(proofResultDir, 'accessibility-summary.json');
const childUxProofDir = path.join(repoRoot, 'test-results', 'browser-ai-child-ux-rendered-proof');
const runRoot = await mkdtemp(path.join(tmpdir(), 'ocentra-parent-browser-parent-explanation-'));
const devLogDir = path.join(runRoot, 'dev-log');
const activityDbPath = path.join(runRoot, 'activity.sqlite');
const commands = [];
const children = [];
const agentPort = resolveParentDevPort(
  process.env[ParentDevEnv.AgentPort],
  ParentDevPort.PortalSmokeAgent,
  ParentDevEnv.AgentPort
);
const portalPort = resolveParentDevPort(
  process.env[ParentDevEnv.PortalPort],
  ParentDevPort.PortalSmokePortal,
  ParentDevEnv.PortalPort
);

let stopping = false;

try {
  await mkdir(devLogDir, { recursive: true });
  await mkdir(screenshotDir, { recursive: true });
  await mkdir(proofResultDir, { recursive: true });
  await runNpm(['run', 'build:contracts']);
  const proofBundle = await buildProofBundle();
  await runNpmWorkspace('@ocentra-parent/portal-domain', ['run', 'type-check']);
  await runNpmWorkspace('@ocentra-parent/portal', ['run', 'type-check']);
  await runCommand('cargo', ['build', '-p', 'ocentra-parent-agent-service']);
  await ensurePortFree(agentPort, isLikelyParentAgentOccupant, console.log);
  await ensurePortFree(portalPort, isLikelyParentPortalOccupant, console.log);

  const agent = spawnAgent();
  trackChild(agent, 'agent');
  await waitForHttp(createAgentHealthUrl(agentPort));

  const portal = spawnVitePortal(portalPort, portalEnv(proofBundle), repoRoot);
  trackChild(portal, 'portal');
  await waitForHttp(createPortalCommandsUrl(portalPort));

  const playwright = await runPlaywright(proofBundle);
  await writeProof(proofBundle, playwright);

  console.log('browser-ai-parent-explanation-rendered-proof-ok');
  console.log(`evidence=${relativePath(proofPath)}`);
} finally {
  stopping = true;
  await Promise.all(children.map((child) => stopProcessTreeAndWait(child)));
  await rm(runRoot, { recursive: true, force: true });
}

async function buildProofBundle() {
  const [
    { BrowserAiParentExplanationBundleSchema, BrowserAiParentExplanationSchemaVersion },
    { BrowserAiAnalysisSchemaVersion },
    { BrowserAiPolicyEvaluatorSchemaVersion },
    { BrowserAiPostAnalysisActionSchemaVersion },
    { BrowserAiChildUxSchemaVersion },
  ] = await Promise.all([
    import('@ocentra-parent/schema-domain/browser-ai-parent-explanation-schemas'),
    import('@ocentra-parent/schema-domain/browser-ai-analysis-schemas'),
    import('@ocentra-parent/schema-domain/browser-ai-policy-evaluator-schemas'),
    import('@ocentra-parent/schema-domain/browser-ai-post-analysis-action-schemas'),
    import('@ocentra-parent/schema-domain/browser-ai-child-ux-schemas'),
  ]);
  const childUxProof = await readLatestChildUxProof();
  const warningCase = childUxProof.cases.find((entry) => entry.state === 'warning');
  if (warningCase === undefined) {
    throw new Error('Expected latest AI-19 child UX proof to include warning state.');
  }
  const sourceEvidenceId = 'browser-evidence-live-youtube-cdp';
  const metadataEvidenceId = 'browser-url-metadata-live-youtube-oembed';
  const policyDecision = {
    schemaVersion: BrowserAiPolicyEvaluatorSchemaVersion,
    decisionId: 'browser-policy-decision-live-youtube-warning',
    requestId: 'browser-policy-evaluator-request-live-youtube-warning',
    decidedAt: childUxProof.generatedAt,
    policyVersionRef: 'browser-policy-version-rendered-proof',
    sourceEvidenceIds: [sourceEvidenceId],
    aiAnalysisId: 'browser-ai-analysis-result-live-youtube-warning',
    memoryHitIds: ['memory-hit-live-youtube-homework-video'],
    graphRefs: ['knowledge-graph-node-live-youtube-video'],
    parentRuleRefs: ['parent-rule-school-night-video-review'],
    scheduleContextRefs: ['schedule-context-school-night'],
    outcome: 'warn',
    evaluatorMode: 'active',
    confidence: 'high',
    reasonCodes: ['explicit_parent_rule', 'schedule_match', 'ai_high_confidence', 'memory_hit', 'graph_ref'],
    auditRefs: ['browser-policy-decision-audit-live-youtube-warning'],
    adapterProofRef: warningCase.adapterProofRef,
    fallbackUsed: false,
    aiClaimedAsAuthority: false,
    portalEvaluatedClaimed: false,
    directEnforcementClaimed: false,
  };
  const postAnalysisActionPlan = {
    schemaVersion: BrowserAiPostAnalysisActionSchemaVersion,
    actionPlanId: 'browser-post-analysis-action-plan-live-youtube-warning',
    createdAt: childUxProof.generatedAt,
    sourceEvidenceIds: [sourceEvidenceId],
    aiAnalysisId: 'browser-ai-analysis-result-live-youtube-warning',
    policyDecision,
    policyDecisionAuditRefs: policyDecision.auditRefs,
    parentRuleRefs: policyDecision.parentRuleRefs,
    actionLabels: ['warning_shown_after_review'],
    trigger: 'policy_decision',
    timing: 'after_playback_started',
    childAlreadyEngaged: true,
    deliveryState: 'delivered',
    adapterProofRef: warningCase.adapterProofRef,
    rememberUntil: null,
    actionAuditRefs: ['browser-post-analysis-action-audit-live-youtube-warning'],
    realtimeBlockClaimed: false,
    browserRuntimeMutationClaimed: false,
    directEnforcementClaimed: false,
  };
  const bundle = {
    schemaVersion: BrowserAiParentExplanationSchemaVersion,
    explanationId: 'browser-parent-explanation-live-youtube-warning',
    createdAt: childUxProof.generatedAt,
    state: 'ready',
    titleTextToken: 'browser.parent.explanation.title',
    summaryTextToken: 'browser.parent.explanation.summary',
    sections: [
      'summary',
      'evidence',
      'ai-analysis',
      'policy-decision',
      'action-taken',
      'child-experience',
      'memory-cache',
      'knowledge-graph',
      'degraded-fallback',
      'audit',
    ],
    sourceEvidenceIds: [sourceEvidenceId],
    aiAnalysis: {
      schemaVersion: BrowserAiAnalysisSchemaVersion,
      analysisId: 'browser-ai-analysis-result-live-youtube-warning',
      requestId: 'browser-ai-analysis-request-live-youtube-warning',
      analyzedAt: childUxProof.generatedAt,
      expiresAt: childUxProof.generatedAt,
      sourceEvidenceIds: [sourceEvidenceId],
      metadataEvidenceIds: [metadataEvidenceId],
      memoryHitIds: ['memory-hit-live-youtube-homework-video'],
      graphRefs: ['knowledge-graph-node-live-youtube-video'],
      parentRuleRefs: policyDecision.parentRuleRefs,
      contentKind: 'video',
      videoKind: 'video',
      contentCategory: 'educational',
      contentModifiers: ['metadata-only'],
      benefitSignals: ['homework-help'],
      riskSignals: ['addictive-design'],
      recommendedPolicyInput: 'warn-candidate',
      confidence: 'high',
      uncertaintyReasons: [],
      parentSummary: 'Evidence-backed video review summary',
      childSafeSummary: 'This video was reviewed against your family rules.',
      modelRuntimeRef: 'local-model-runtime-browser-video',
      promptTemplate: {
        promptTemplateId: 'browser-video-safety-template',
        promptTemplateVersion: '2026-06-06',
        requestedTask: 'video-safety',
        allowedInputFieldRefs: ['url-shape-ref', 'metadata-ref', 'parent-rule-ref'],
        rawPromptTextIncluded: false,
        capturesRawPageBody: false,
        capturesTranscriptText: false,
      },
      degradedState: 'none',
      finalPolicyActionClaimed: false,
      enforcementActionClaimed: false,
      rawContentStored: false,
    },
    policyDecision,
    postAnalysisActionPlan,
    childUxSnapshot: {
      schemaVersion: BrowserAiChildUxSchemaVersion,
      snapshotId: warningCase.snapshotId,
      createdAt: childUxProof.generatedAt,
      sourceEvidenceIds: [sourceEvidenceId],
      state: 'warning',
      tone: 'calm',
      surface: warningCase.surface,
      primaryTextToken: 'browser.child.warning.title',
      secondaryTextToken: null,
      deliveryState: warningCase.deliveryState,
      adapterProofRef: warningCase.adapterProofRef,
      postAnalysisActionPlan,
      rawCopyClaimed: false,
      visualRenderClaimed: false,
      surveillanceCopyClaimed: false,
      shamingCopyClaimed: false,
    },
    memoryCacheEntryIds: ['browser-ai-cache-entry-live-youtube-video'],
    knowledgeGraphRefs: ['knowledge-graph-node-live-youtube-video'],
    explanationAuditRefs: ['browser-parent-explanation-audit-live-youtube-warning'],
    evidenceVisible: true,
    modelRuntimeVisible: true,
    promptVersionVisible: true,
    policyRuleVisible: true,
    actionVisible: true,
    memoryCacheVisible: true,
    childExperienceVisible: true,
    childSawPageVisible: true,
    degradedStateVisible: false,
    manualFallbackVisible: false,
    auditTrailVisible: true,
    rawPageContentIncluded: false,
    rawPromptTextIncluded: false,
    portalEvaluatedClaimed: false,
    policyAuthorityClaimed: false,
    directEnforcementClaimed: false,
  };
  const parsed = BrowserAiParentExplanationBundleSchema.parse(bundle);
  return {
    bundle: parsed,
    sourceProof: childUxProof.__sourceProof,
    sourceScreenshot: warningCase.screenshotPath,
    capturedLocationHash: hashString(childUxProof.capturedLocation),
    requestedUrlHash: hashString(childUxProof.requestedUrl),
  };
}

async function readLatestChildUxProof() {
  const entries = await readdir(childUxProofDir, { withFileTypes: true });
  const files = entries
    .filter((entry) => entry.isFile() && entry.name.endsWith('.json'))
    .map((entry) => path.join(childUxProofDir, entry.name));
  if (files.length === 0) {
    throw new Error(`Missing AI-19 child UX proof JSON in ${relativePath(childUxProofDir)}`);
  }
  const newest = files.sort().at(-1);
  const parsed = JSON.parse(await readFile(newest, 'utf8'));
  if (parsed.requestedUrl !== 'https://www.youtube.com/watch?v=XzUB8_gj6xM') {
    throw new Error(
      'Expected latest AI-19 proof to cite the live YouTube target used by this parent explanation proof.'
    );
  }
  return { ...parsed, __sourceProof: newest };
}

function spawnAgent() {
  return spawn(resolveDebugAgentServicePath(repoRoot), [], {
    cwd: repoRoot,
    detached: process.platform !== 'win32',
    env: {
      ...process.env,
      [ParentDevEnv.ActivityDbPath]: activityDbPath,
      [ParentDevEnv.AgentAddress]: createAgentAddress(agentPort),
      [ParentDevEnv.AgentAllowedOrigins]: createHttpOrigin(ParentDevHost.Loopback, portalPort),
      [ParentDevEnv.DevLogDir]: devLogDir,
    },
    stdio: ['ignore', 'pipe', 'pipe'],
  });
}

function portalEnv(proofBundle) {
  return {
    ...process.env,
    [ParentDevEnv.ActivityDbPath]: activityDbPath,
    [ParentDevEnv.DevLogDir]: devLogDir,
    [ParentDevEnv.PortalAgentWebSocketUrl]: createAgentWebSocketUrl(agentPort),
    [ParentDevEnv.PortalPort]: String(portalPort),
    BROWSER_PARENT_EXPLANATION_UI_PROOF: '1',
    VITE_BROWSER_PARENT_EXPLANATION_PROOF_BUNDLE: JSON.stringify(proofBundle.bundle),
  };
}

function trackChild(child, label) {
  children.push(child);
  child.stdout?.on('data', (chunk) => process.stdout.write(chunk));
  child.stderr?.on('data', (chunk) => process.stderr.write(chunk));
  child.once('exit', (code, signal) => {
    if (!stopping && code !== 0) {
      console.error(`${label} exited early: code=${code ?? 'null'} signal=${signal ?? 'null'}`);
    }
  });
}

async function runPlaywright(proofBundle) {
  const cliPath = path.join(repoRoot, 'node_modules', '@playwright', 'test', 'cli.js');
  const args = [
    cliPath,
    'test',
    '--config',
    path.join(portalRoot, 'playwright.config.ts'),
    'browser-ai-parent-explanation-ui-proof.spec.ts',
    '--workers=1',
  ];
  const result = await runCommand(process.execPath, args, {
    cwd: portalRoot,
    env: portalEnv(proofBundle),
    capture: true,
  });
  await writeFile(playwrightLogPath, `${result.output.trimEnd()}\n`);
  return {
    command: [process.execPath, ...args].join(' '),
    exitCode: result.exitCode,
    log: relativePath(playwrightLogPath),
  };
}

async function runNpmWorkspace(workspaceName, args) {
  await runNpm(['--workspace', workspaceName, ...args]);
}

async function runNpm(args, ...rest) {
  const command = process.platform === 'win32' ? 'cmd' : 'npm';
  const commandArgs = process.platform === 'win32' ? ['/c', 'npm', ...args] : args;
  await runCommand(command, commandArgs, ...rest);
}

async function runCommand(command, args, options = {}) {
  const commandLine = [command, ...args].join(' ');
  const chunks = [];
  const child = spawn(command, args, {
    cwd: options.cwd ?? repoRoot,
    env: options.env ?? process.env,
    stdio: ['ignore', options.capture ? 'pipe' : 'inherit', options.capture ? 'pipe' : 'inherit'],
    windowsHide: true,
  });
  if (options.capture) {
    child.stdout?.on('data', (chunk) => {
      chunks.push(String(chunk));
      process.stdout.write(chunk);
    });
    child.stderr?.on('data', (chunk) => {
      chunks.push(String(chunk));
      process.stderr.write(chunk);
    });
  }
  const [code, signal] = await once(child, 'exit');
  const exitCode = signal === null ? (code ?? 1) : 1;
  commands.push({ command: commandLine, exitCode });
  if (exitCode !== 0) {
    throw new Error(`${commandLine} exited with ${exitCode}`);
  }
  return { exitCode, output: chunks.join('') };
}

async function waitForHttp(url) {
  const deadline = Date.now() + 90_000;
  let lastError;
  while (Date.now() < deadline) {
    try {
      const response = await fetch(url);
      if (response.ok) {
        return;
      }
      lastError = new Error(`${url} returned ${response.status}`);
    } catch (error) {
      lastError = error;
    }
    await delay(500);
  }
  throw lastError instanceof Error ? lastError : new Error(`Timed out waiting for ${url}`);
}

async function writeProof(proofBundle, playwright) {
  const checkedAt = new Date().toISOString();
  const accessibilitySummary = JSON.parse(await readFile(accessibilitySummaryPath, 'utf8'));
  const proof = {
    schemaVersion: 1,
    checkedAt,
    commit: await gitHead(),
    workpackIds: ['ai-20-parent-explanation-audit-ux'],
    proofMode: 'real-portal-browser-route-parent-explanation-from-live-ai19-youtube-evidence',
    route: '#/browser',
    currentStatus: 'rendered-parent-explanation-proof-only',
    productClaimReady: false,
    artifacts: {
      proof: relativePath(proofPath),
      outputProof: relativePath(outputProofPath),
      sourceSnapshot: relativePath(sourceSnapshotPath),
      playwrightLog: playwright.log,
      securityNegativeLog: relativePath(securityLogPath),
      validationCommands: relativePath(validationLogPath),
      desktopScreenshot: relativePath(desktopScreenshot),
      mobileScreenshot: relativePath(mobileScreenshot),
      accessibilitySummary: relativePath(accessibilitySummaryPath),
      sourceChildUxProof: relativePath(proofBundle.sourceProof),
      sourceChildUxScreenshot: relativePath(proofBundle.sourceScreenshot),
    },
    sourceBoundary: {
      sourceProof: relativePath(proofBundle.sourceProof),
      sourceSurface: 'live YouTube page captured through the AI-19 child UX CDP proof before intervention render',
      requestedUrlHash: proofBundle.requestedUrlHash,
      capturedLocationHash: proofBundle.capturedLocationHash,
      rawUrlRenderedInPortal: false,
    },
    renderedBundle: {
      explanationId: proofBundle.bundle.explanationId,
      sections: proofBundle.bundle.sections,
      sourceEvidenceIds: proofBundle.bundle.sourceEvidenceIds,
      actionLabels: proofBundle.bundle.postAnalysisActionPlan.actionLabels,
      childDeliveryState: proofBundle.bundle.childUxSnapshot.deliveryState,
      auditRefs: proofBundle.bundle.explanationAuditRefs,
    },
    assertions: [
      'Portal browser route renders the Browser review region from the real app shell.',
      'The rendered parent explanation bundle is parsed by BrowserAiParentExplanationBundleSchema before reaching the portal panel intent.',
      'The source proof is the latest AI-19 live YouTube CDP child UX evidence JSON, not generated page content.',
      'The portal screenshot exposes evidence, model, policy, action, child experience, and audit sections without raw URL, page content, prompt text, or enforcement claims.',
      'Desktop and mobile screenshots were captured from the real portal route.',
    ],
    nonClaims: [
      'This proof does not claim production service delivery of parent explanation bundles.',
      'This proof does not claim final policy authority, browser mutation, enforcement, remote AI, or raw page/prompt custody.',
      'This proof does not claim product capability checklist completion.',
    ],
    remainingGapsBeforeProductReady: [
      'Service-backed parent explanation read-model/event delivery remains pending.',
      'Parent notification/report delivery remains pending.',
      'Final policy execution and enforcement remain separate proof gates.',
    ],
    accessibilitySummary,
    commands,
  };
  const proofContent = `${JSON.stringify(proof, null, 2)}\n`;
  await writeFile(proofPath, proofContent);
  await writeFile(outputProofPath, proofContent);
  await writeFile(
    sourceSnapshotPath,
    [
      '# AI-20 Parent Explanation Runtime Source Snapshot',
      '',
      `checkedAt=${checkedAt}`,
      `branch=${await gitBranch()}`,
      `commit=${await gitHead()}`,
      `sourceChildUxProof=${relativePath(proofBundle.sourceProof)}`,
      `sourceChildUxScreenshot=${relativePath(proofBundle.sourceScreenshot)}`,
      '',
      'The parent explanation proof consumes the live AI-19 YouTube CDP child UX proof as source evidence.',
      'The portal receives only a schema-decoded explanation bundle through the dedicated proof env var.',
      'Raw target URL, page body, prompt text, screenshots, cookies, tokens, and browser storage are not rendered or stored in the parent explanation bundle.',
      '',
    ].join('\n')
  );
  await writeFile(
    validationLogPath,
    commands.map((entry) => `${entry.command} # exit ${entry.exitCode}`).join('\n') + '\n'
  );
  await writeFile(
    securityLogPath,
    [
      `checkedAt=${checkedAt}`,
      'asserted=no raw YouTube URL rendered in Browser review region',
      'asserted=no raw page content claim',
      'asserted=no raw prompt text claim',
      'asserted=no direct enforcement active claim',
      'asserted=portal proof env is proof-only and productClaimReady=false',
      '',
    ].join('\n')
  );
}

function hashString(value) {
  let hash = 0;
  for (let index = 0; index < value.length; index += 1) {
    hash = (Math.imul(31, hash) + value.charCodeAt(index)) | 0;
  }
  return `hash-${(hash >>> 0).toString(16).padStart(8, '0')}`;
}

async function gitHead() {
  const result = await runCommand('git', ['rev-parse', 'HEAD'], { capture: true });
  return result.output.trim();
}

async function gitBranch() {
  const result = await runCommand('git', ['rev-parse', '--abbrev-ref', 'HEAD'], { capture: true });
  return result.output.trim();
}

function relativePath(value) {
  return path.relative(repoRoot, value).replace(/\\/gu, '/');
}
