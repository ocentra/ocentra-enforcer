import { spawn } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';

const repoRoot = process.cwd();
const outputDir = join(repoRoot, 'test-results', 'screen-service-capture-event-producer-proof');
const proofPath = join(outputDir, 'proof.json');
const screenOutputDir = join(repoRoot, 'output', 'screen-plan-proof', 'screen-service-capture-event-producer');
const screenProofPath = join(screenOutputDir, 'proof-summary.json');
const pipelineOutputDir = join(repoRoot, 'output', 'screen-ai-pipeline-proof', 'screen-service-capture-event-producer');
const pipelineProofPath = join(pipelineOutputDir, 'proof-summary.json');
const commands = [];

await main();

async function main() {
  await mkdir(outputDir, { recursive: true });
  await mkdir(screenOutputDir, { recursive: true });
  await mkdir(pipelineOutputDir, { recursive: true });

  await runCommand('cargo', [
    'test',
    '-p',
    'ocentra-parent-agent-core',
    'screen_capture_queue_events_publish_without_ai_policy_or_action_refs',
  ]);
  await runCommand('cargo', ['test', '-p', 'ocentra-parent-agent-service', 'screen_ai_service_event_bridge']);
  await runCommand('cargo', ['test', '-p', 'ocentra-parent-agent-service', 'screen_ai_cadence_runtime']);
  await runCommand('cargo', ['test', '-p', 'ocentra-parent-agent-service', 'screen_ai_foreground_runtime']);
  await assertSourceContracts();

  const proof = {
    schemaVersion: 1,
    proofMode: 'screen-service-capture-event-producer-proof',
    checkedAt: new Date().toISOString(),
    commit: await gitHead(),
    commands,
    evidence: {
      coreRuntime: 'crates/agent-core/src/screen_event_runtime.rs',
      coreRuntimeTests: 'crates/agent-core/tests/unit/screen_event_runtime_tests.rs',
      serviceBridge: 'crates/agent-service/src/screen_ai_service_event_bridge.rs',
      serviceBridgeTests: 'crates/agent-service/tests/unit/screen_ai_service_event_bridge_tests.rs',
      cadenceRuntime: 'crates/agent-service/src/screen_ai_cadence_runtime.rs',
      foregroundRuntime: 'crates/agent-service/src/screen_ai_foreground_runtime.rs',
      proofHarness: 'scripts/test/screen-service-capture-event-producer-proof.mjs',
      screenProofSummary: 'output/screen-plan-proof/screen-service-capture-event-producer/proof-summary.json',
      pipelineProofSummary: 'output/screen-ai-pipeline-proof/screen-service-capture-event-producer/proof-summary.json',
    },
    eventChain: [
      'service cadence or foreground capture',
      'encrypted temp queue write',
      'Activity Screen metadata row',
      'screen.capture.observed event',
      'screen.queue.encrypted event',
      'AI analysis consumer remains downstream',
    ],
    claimsProved: [
      'core screen runtime can publish capture and encrypted-queue events without AI, policy, action, deletion, or portal refs',
      'service Activity Screen rows map into capture/queue event payloads with raw-image-retained rejection',
      'service cadence runtime publishes capture/queue events after a real encrypted queue write',
      'service foreground runtime publishes capture/queue events after a real encrypted queue write',
      'the capture producers reuse the existing screen event runtime path instead of adding a second event bus',
    ],
    claimsNotProved: [
      'new live external capture run',
      'production OCR or VLM quality',
      'authenticated-account social proof',
      'final enforcement execution',
      'retention sweeper deletion event producer wiring',
      'physical household LAN mesh execution',
    ],
  };

  await writeFile(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
  await writeFile(screenProofPath, `${JSON.stringify(summaryProof(proof), null, 2)}\n`);
  await writeFile(pipelineProofPath, `${JSON.stringify(summaryProof(proof), null, 2)}\n`);
  console.log('screen-service-capture-event-producer-proof-ok');
  console.log(`evidence=${relative(repoRoot, proofPath)}`);
  console.log(`screen=${relative(repoRoot, screenProofPath)}`);
  console.log(`pipeline=${relative(repoRoot, pipelineProofPath)}`);
}

async function assertSourceContracts() {
  const coreRuntime = await readText('crates/agent-core/src/screen_event_runtime.rs');
  const coreTests = await readText('crates/agent-core/tests/unit/screen_event_runtime_tests.rs');
  const serviceBridge = await readText('crates/agent-service/src/screen_ai_service_event_bridge.rs');
  const serviceTests = await readText('crates/agent-service/tests/unit/screen_ai_service_event_bridge_tests.rs');
  const cadenceRuntime = await readText('crates/agent-service/src/screen_ai_cadence_runtime.rs');
  const foregroundRuntime = await readText('crates/agent-service/src/screen_ai_foreground_runtime.rs');
  const screenChecklist = await readText('docs/plans/screen-plan/implementation-checklist.md');
  const screenFeature = await readText('docs/features/screen-evidence-analysis.md');

  assertIncludes(coreRuntime, 'ScreenRuntimeCaptureInput', 'core runtime defines capture input');
  assertIncludes(
    coreRuntime,
    'publish_screen_capture_queue_events_for_input',
    'core runtime exposes capture queue event publisher'
  );
  assertIncludes(coreRuntime, 'ScreenRuntimePhase::CaptureObserved', 'core runtime publishes capture observed phase');
  assertIncludes(coreRuntime, 'ScreenRuntimePhase::QueueEncrypted', 'core runtime publishes queue encrypted phase');
  assertIncludes(
    coreTests,
    'screen_capture_queue_events_publish_without_ai_policy_or_action_refs',
    'core test proves capture queue events avoid downstream refs'
  );
  assertIncludes(
    serviceBridge,
    'publish_screen_capture_queue_events_for_queue_job',
    'service bridge looks up capture rows by queue job'
  );
  assertIncludes(
    serviceBridge,
    'screen_runtime_capture_input_from_service_row',
    'service bridge maps rows to capture input'
  );
  assertIncludes(
    serviceTests,
    'screen_service_event_bridge_publishes_capture_queue_events_from_capture_row',
    'service test proves capture queue bridge'
  );
  assertIncludes(
    cadenceRuntime,
    'publish_screen_capture_queue_events_for_queue_job',
    'cadence runtime publishes capture queue events'
  );
  assertIncludes(
    foregroundRuntime,
    'publish_screen_capture_queue_events_for_queue_job',
    'foreground runtime publishes capture queue events'
  );
  assertIncludes(
    screenChecklist,
    'Screen service capture event producer',
    'screen checklist names capture event producer proof'
  );
  assertIncludes(
    screenFeature,
    'screen-service-capture-event-producer-proof.mjs',
    'screen feature names capture event producer proof'
  );
}

function summaryProof(proof) {
  return {
    schemaVersion: proof.schemaVersion,
    proofMode: 'screen-service-capture-event-producer',
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
    const child = spawn('git', ['rev-parse', 'HEAD'], {
      cwd: repoRoot,
      shell: process.platform === 'win32',
    });
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
