import { spawn } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';

const repoRoot = process.cwd();
const outputDir = join(repoRoot, 'test-results', 'screen-service-event-bridge-proof');
const proofPath = join(outputDir, 'proof.json');
const screenOutputDir = join(repoRoot, 'output', 'screen-plan-proof', 'screen-service-event-bridge');
const screenProofPath = join(screenOutputDir, 'proof-summary.json');
const pipelineOutputDir = join(repoRoot, 'output', 'screen-ai-pipeline-proof', 'screen-service-event-bridge');
const pipelineProofPath = join(pipelineOutputDir, 'proof-summary.json');
const commands = [];

await main();

async function main() {
  await mkdir(outputDir, { recursive: true });
  await mkdir(screenOutputDir, { recursive: true });
  await mkdir(pipelineOutputDir, { recursive: true });

  await runCommand('cargo', ['test', '-p', 'ocentra-parent-agent-service', 'screen_ai_service_event_bridge']);
  await runCommand('cargo', ['test', '-p', 'ocentra-parent-agent-service', 'screen_ai_service_event_subscription']);
  await assertSourceContracts();

  const proof = {
    schemaVersion: 1,
    proofMode: 'screen-service-event-bridge-proof',
    checkedAt: new Date().toISOString(),
    commit: await gitHead(),
    commands,
    evidence: {
      protocolConstants: 'crates/agent-protocol/src/constants/screen_flow.rs',
      serviceBridge: 'crates/agent-service/src/screen_ai_service_event_bridge.rs',
      serviceBridgeTests: 'crates/agent-service/tests/unit/screen_ai_service_event_bridge_tests.rs',
      serviceSubscriber: 'crates/agent-service/src/screen_ai_service_event_subscription.rs',
      serviceSubscriberTests: 'crates/agent-service/tests/unit/screen_ai_service_event_subscription_tests.rs',
      coreRuntime: 'crates/agent-core/src/screen_event_runtime.rs',
      proofHarness: 'scripts/test/screen-service-event-bridge-proof.mjs',
      screenProofSummary: 'output/screen-plan-proof/screen-service-event-bridge/proof-summary.json',
      pipelineProofSummary: 'output/screen-ai-pipeline-proof/screen-service-event-bridge/proof-summary.json',
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
    degradedEventChain: [
      'screen.capture.observed',
      'screen.queue.encrypted',
      'screen.ai.analysis.requested',
      'screen.ai.analysis.completed',
      'screen.deletion.committed',
      'screen.portal-read-model.updated',
    ],
    rejectionCases: ['raw-image-retained', 'missing-policy-decision'],
    claimsProved: [
      'service Activity Screen read-model rows map into the existing ScreenRuntimeInput contract',
      'service rows publish the ordered typed screen event chain through the reusable core screen runtime',
      'raw-image retained rows are rejected before event publication',
      'rows without policy decision refs are rejected before event publication',
      'degraded AI rows publish capture, queue, AI, deletion, and portal events without policy or action refs',
      'the service row-ready subscriber routes degraded AI rows through the degraded event chain',
      'the bridge reuses the existing core event path and does not create a duplicate eventing spine',
    ],
    claimsNotProved: [
      'always-on production service event subscriptions',
      'new live capture session',
      'production OCR or VLM quality',
      'authenticated-account social proof',
      'portal UI changes',
      'policy authority or enforcement dispatch',
      'physical household LAN mesh transport',
    ],
  };

  await writeFile(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
  await writeFile(screenProofPath, `${JSON.stringify(screenProof(proof), null, 2)}\n`);
  await writeFile(pipelineProofPath, `${JSON.stringify(pipelineProof(proof), null, 2)}\n`);
  console.log('screen-service-event-bridge-proof-ok');
  console.log(`evidence=${relative(repoRoot, proofPath)}`);
  console.log(`screen=${relative(repoRoot, screenProofPath)}`);
  console.log(`pipeline=${relative(repoRoot, pipelineProofPath)}`);
}

async function assertSourceContracts() {
  const protocolConstants = await readText('crates/agent-protocol/src/constants/screen_flow.rs');
  const serviceLib = await readText('crates/agent-service/src/lib.rs');
  const serviceBridge = await readText('crates/agent-service/src/screen_ai_service_event_bridge.rs');
  const serviceBridgeTests = await readText('crates/agent-service/tests/unit/screen_ai_service_event_bridge_tests.rs');
  const serviceSubscriber = await readText('crates/agent-service/src/screen_ai_service_event_subscription.rs');
  const serviceSubscriberTests = await readText(
    'crates/agent-service/tests/unit/screen_ai_service_event_subscription_tests.rs'
  );
  const screenChecklist = await readText('docs/plans/screen-plan/implementation-checklist.md');
  const screenFeature = await readText('docs/features/screen-evidence-analysis.md');

  assertIncludes(
    protocolConstants,
    'ERROR_SCREEN_SERVICE_EVENT_BRIDGE_PUBLISHES',
    'screen service bridge constants exist'
  );
  assertIncludes(serviceLib, 'mod screen_ai_service_event_bridge;', 'service bridge module is registered');
  assertIncludes(serviceBridge, 'screen_runtime_input_from_service_row', 'service row maps to screen runtime input');
  assertIncludes(
    serviceBridge,
    'screen_runtime_degraded_input_from_service_row',
    'service row maps to degraded runtime input'
  );
  assertIncludes(serviceBridge, 'publish_screen_runtime_chain_for_input', 'bridge reuses core screen runtime chain');
  assertIncludes(
    serviceBridge,
    'publish_screen_degraded_event_chain_for_input',
    'bridge reuses core degraded runtime chain'
  );
  assertIncludes(serviceBridge, 'RawImageRetained', 'bridge rejects raw retention');
  assertIncludes(serviceBridge, 'MissingPolicyDecision', 'bridge rejects missing policy');
  assertIncludes(
    serviceBridgeTests,
    'publishes_ordered_chain_from_service_read_model_row',
    'tests prove ordered chain publication'
  );
  assertIncludes(serviceBridgeTests, 'publishes_degraded_ai_event_path', 'tests prove degraded event publication');
  assertIncludes(
    serviceSubscriber,
    'publish_screen_degraded_event_chain',
    'subscriber can route degraded rows through degraded bridge'
  );
  assertIncludes(
    serviceSubscriberTests,
    'publishes_degraded_runtime_chain',
    'subscriber tests prove degraded event publication'
  );
  assertIncludes(screenChecklist, 'Screen service event bridge', 'screen checklist names service event bridge proof');
  assertIncludes(
    screenFeature,
    'screen-service-event-bridge-proof.mjs',
    'screen feature names service event bridge proof'
  );
  assertDoesNotInclude(serviceBridge, 'EventBus::new()', 'service bridge does not create duplicate bus');
}

function screenProof(proof) {
  return {
    schemaVersion: proof.schemaVersion,
    proofMode: 'screen-service-event-bridge',
    checkedAt: proof.checkedAt,
    commit: proof.commit,
    commands: proof.commands,
    eventChain: proof.eventChain,
    degradedEventChain: proof.degradedEventChain,
    rejectionCases: proof.rejectionCases,
    claimsProved: proof.claimsProved,
    claimsNotProved: proof.claimsNotProved,
    sourceProof: relative(repoRoot, proofPath),
  };
}

function pipelineProof(proof) {
  return {
    schemaVersion: proof.schemaVersion,
    proofMode: 'screen-service-event-bridge',
    checkedAt: proof.checkedAt,
    commit: proof.commit,
    commands: proof.commands,
    eventChain: proof.eventChain,
    degradedEventChain: proof.degradedEventChain,
    claimsProved: proof.claimsProved.filter(
      (claim) => claim.includes('service') || claim.includes('event') || claim.includes('core')
    ),
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
