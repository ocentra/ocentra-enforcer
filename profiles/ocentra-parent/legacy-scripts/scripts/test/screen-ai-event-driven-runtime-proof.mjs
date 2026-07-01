import { spawn } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';

const repoRoot = process.cwd();
const outputDir = join(repoRoot, 'test-results', 'screen-ai-event-driven-runtime-proof');
const proofPath = join(outputDir, 'proof.json');
const screenOutputDir = join(repoRoot, 'output', 'screen-plan-proof', 'screen-eventing-consumer-boundary');
const screenProofPath = join(screenOutputDir, 'proof-summary.json');
const pipelineOutputDir = join(repoRoot, 'output', 'screen-ai-pipeline-proof', 'event-driven-runtime');
const pipelineProofPath = join(pipelineOutputDir, 'proof-summary.json');
const commands = [];

await main();

async function main() {
  await mkdir(outputDir, { recursive: true });
  await mkdir(screenOutputDir, { recursive: true });
  await mkdir(pipelineOutputDir, { recursive: true });

  await runCommand('cargo', ['test', '-p', 'ocentra-parent-agent-core', 'screen_event_runtime']);
  await runCommand('node', ['scripts/check-source-shape.mjs']);
  await assertSourceContracts();

  const proof = {
    schemaVersion: 1,
    proofMode: 'screen-ai-event-driven-runtime-proof',
    checkedAt: new Date().toISOString(),
    commit: await gitHead(),
    commands,
    evidence: {
      screenFlowConstants: 'crates/agent-protocol/src/constants/screen_flow.rs',
      screenRuntime: 'crates/agent-core/src/screen_event_runtime.rs',
      screenRuntimePhase: 'crates/agent-core/src/screen_event_runtime_phase.rs',
      screenRuntimeState: 'crates/agent-core/src/screen_event_runtime_state.rs',
      screenRuntimeRefs: 'crates/agent-core/src/screen_event_runtime_refs.rs',
      screenRuntimeTests: 'crates/agent-core/tests/unit/screen_event_runtime_tests.rs',
      proofHarness: 'scripts/test/screen-ai-event-driven-runtime-proof.mjs',
      screenPlanProofSummary: 'output/screen-plan-proof/screen-eventing-consumer-boundary/proof-summary.json',
      pipelineProofSummary: 'output/screen-ai-pipeline-proof/event-driven-runtime/proof-summary.json',
    },
    eventChain: [
      'screen.capture.observed',
      'screen.queue.encrypted',
      'screen.ai.analysis.requested',
      'screen.ai.analysis.completed',
      'screen.summary.committed',
      'screen.policy.decision.completed',
      'screen.action.dry-run.recorded',
      'screen.deletion.committed',
      'screen.portal-read-model.updated',
    ],
    claimsProved: [
      'screen lifecycle phases publish typed ocentra-eventing events instead of a direct capture-to-AI-to-policy shortcut',
      'AI request events do not carry policy or action refs before a screen AI result and screen summary event exist',
      'policy decision events depend on the screen summary and accepted AI result refs',
      'action dry-run events depend on a policy decision ref and do not execute an adapter action',
      'deletion and portal read-model events carry deletion/query-store custody after the local child-owned image lifecycle',
      'raw image availability is false for AI provider, policy, and portal boundaries in the screen runtime event payload',
    ],
    claimsNotProved: [
      'live service loop subscription to the screen event runtime',
      'real VLM/OCR model quality beyond existing service and operator proofs',
      'household LAN provider claim/lease/result validation',
      'physical cross-device provider execution',
      'browser, network, mobile, or broad enforcement adapter execution from this event chain',
    ],
  };

  await writeFile(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
  await writeFile(screenProofPath, `${JSON.stringify(screenProof(proof), null, 2)}\n`);
  await writeFile(pipelineProofPath, `${JSON.stringify(pipelineProof(proof), null, 2)}\n`);
  console.log('screen-ai-event-driven-runtime-proof-ok');
  console.log(`evidence=${relative(repoRoot, proofPath)}`);
  console.log(`screen=${relative(repoRoot, screenProofPath)}`);
  console.log(`pipeline=${relative(repoRoot, pipelineProofPath)}`);
}

async function assertSourceContracts() {
  const constantsSource = await readText('crates/agent-protocol/src/constants/screen_flow.rs');
  const coreLib = await readText('crates/agent-core/src/lib.rs');
  const runtimeSource = await readText('crates/agent-core/src/screen_event_runtime.rs');
  const phaseSource = await readText('crates/agent-core/src/screen_event_runtime_phase.rs');
  const stateSource = await readText('crates/agent-core/src/screen_event_runtime_state.rs');
  const refsSource = await readText('crates/agent-core/src/screen_event_runtime_refs.rs');
  const testsSource = await readText('crates/agent-core/tests/unit/screen_event_runtime_tests.rs');
  const screenChecklist = await readText('docs/plans/screen-plan/implementation-checklist.md');
  const pipelineChecklist = await readText('docs/plans/screen-ai-pipeline-plan/implementation-checklist.md');

  assertIncludes(constantsSource, 'EVENT_SCREEN_CAPTURE_OBSERVED', 'screen capture event constant exists');
  assertIncludes(constantsSource, 'EVENT_SCREEN_QUEUE_ENCRYPTED', 'screen queue event constant exists');
  assertIncludes(constantsSource, 'EVENT_SCREEN_AI_ANALYSIS_REQUESTED', 'screen AI request event constant exists');
  assertIncludes(constantsSource, 'EVENT_SCREEN_AI_ANALYSIS_COMPLETED', 'screen AI result event constant exists');
  assertIncludes(constantsSource, 'EVENT_SCREEN_POLICY_DECISION_COMPLETED', 'screen policy event constant exists');
  assertIncludes(runtimeSource, 'impl DomainEvent for ScreenRuntimeEventPayload', 'screen payload is a typed event');
  assertIncludes(runtimeSource, 'EventBus::new()', 'screen runtime uses reusable eventing bus');
  assertIncludes(runtimeSource, 'EventSubscriber::new', 'screen runtime registers typed subscribers');
  assertIncludes(runtimeSource, 'self.bus.publish', 'screen runtime publishes through eventing bus');
  assertIncludes(phaseSource, 'ScreenRuntimePhase::ActionDryRunRecorded', 'screen action phase exists');
  assertIncludes(stateSource, 'raw_image_available_to_policy: false', 'screen policy raw image boundary is false');
  assertIncludes(refsSource, 'SCREEN_SUMMARY_EVENT_REF', 'screen policy chain depends on summary ref');
  assertIncludes(coreLib, 'publish_screen_runtime_chain_for_input', 'agent-core exports screen runtime proof helper');
  assertIncludes(testsSource, 'without_direct_ai_to_policy_shortcut', 'tests guard no AI-to-policy shortcut');
  assertIncludes(testsSource, 'raw_image_out_of_policy_portal_and_provider', 'tests guard raw-image custody boundary');
  assertIncludes(
    screenChecklist,
    'Screen capture, queue, deletion, and summary lifecycle transitions publish',
    'screen checklist names event-driven lifecycle gate'
  );
  assertIncludes(
    pipelineChecklist,
    'Event-driven Screen-AI runtime chain proof run',
    'pipeline checklist names event-driven proof run'
  );
  assertDoesNotInclude(runtimeSource, 'struct ScreenEventBus', 'no private screen event bus');
  assertDoesNotInclude(runtimeSource, 'adapter_action_executed: true', 'no adapter action execution');
}

function screenProof(proof) {
  return {
    schemaVersion: proof.schemaVersion,
    proofMode: 'screen-eventing-consumer-boundary',
    checkedAt: proof.checkedAt,
    commit: proof.commit,
    commands: proof.commands,
    eventChain: proof.eventChain,
    claimsProved: proof.claimsProved.slice(0, 5),
    claimsNotProved: proof.claimsNotProved,
    sourceProof: relative(repoRoot, proofPath),
  };
}

function pipelineProof(proof) {
  return {
    schemaVersion: proof.schemaVersion,
    proofMode: 'screen-ai-pipeline-event-driven-runtime',
    checkedAt: proof.checkedAt,
    commit: proof.commit,
    commands: proof.commands,
    eventChain: proof.eventChain,
    claimsProved: proof.claimsProved,
    claimsNotProved: proof.claimsNotProved,
    sourceProof: relative(repoRoot, proofPath),
  };
}

async function readText(path) {
  return readFile(join(repoRoot, path), 'utf8');
}

function assertIncludes(text, value, label) {
  if (!text.includes(value)) {
    throw new Error(`${label}: missing ${value}`);
  }
}

function assertDoesNotInclude(text, value, label) {
  if (text.includes(value)) {
    throw new Error(`${label}: found ${value}`);
  }
}

async function runCommand(command, args) {
  const result = await new Promise((resolve) => {
    const child = spawn(command, args, { cwd: repoRoot, shell: process.platform === 'win32' });
    let stdout = '';
    let stderr = '';
    child.stdout.on('data', (chunk) => {
      stdout += chunk;
      process.stdout.write(chunk);
    });
    child.stderr.on('data', (chunk) => {
      stderr += chunk;
      process.stderr.write(chunk);
    });
    child.on('close', (code) => resolve({ code, stdout, stderr }));
  });
  commands.push({ command, args, exitCode: result.code });
  if (result.code !== 0) {
    throw new Error(`${command} ${args.join(' ')} failed with ${result.code}`);
  }
}

async function gitHead() {
  const result = await new Promise((resolve) => {
    const child = spawn('git', ['rev-parse', 'HEAD'], { cwd: repoRoot, shell: process.platform === 'win32' });
    let stdout = '';
    child.stdout.on('data', (chunk) => {
      stdout += chunk;
    });
    child.on('close', (code) => resolve({ code, stdout }));
  });
  if (result.code !== 0) {
    return null;
  }
  return result.stdout.trim();
}
