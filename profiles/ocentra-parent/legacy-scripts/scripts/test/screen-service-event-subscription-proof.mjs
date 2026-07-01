import { spawn } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';

const repoRoot = process.cwd();
const outputDir = join(repoRoot, 'test-results', 'screen-service-event-subscription-proof');
const proofPath = join(outputDir, 'proof.json');
const screenOutputDir = join(repoRoot, 'output', 'screen-plan-proof', 'screen-service-event-subscription');
const screenProofPath = join(screenOutputDir, 'proof-summary.json');
const pipelineOutputDir = join(repoRoot, 'output', 'screen-ai-pipeline-proof', 'screen-service-event-subscription');
const pipelineProofPath = join(pipelineOutputDir, 'proof-summary.json');
const commands = [];

await main();

async function main() {
  await mkdir(outputDir, { recursive: true });
  await mkdir(screenOutputDir, { recursive: true });
  await mkdir(pipelineOutputDir, { recursive: true });

  await runCommand('cargo', ['test', '-p', 'ocentra-parent-agent-service', 'screen_ai_service_event_subscription']);
  await assertSourceContracts();

  const proof = {
    schemaVersion: 1,
    proofMode: 'screen-service-event-subscription-proof',
    checkedAt: new Date().toISOString(),
    commit: await gitHead(),
    commands,
    evidence: {
      protocolConstants: 'crates/agent-protocol/src/constants/screen_flow.rs',
      serviceLib: 'crates/agent-service/src/lib.rs',
      serviceRuntime: 'crates/agent-service/src/service_runtime.rs',
      serviceSubscription: 'crates/agent-service/src/screen_ai_service_event_subscription.rs',
      serviceSubscriptionTests: 'crates/agent-service/tests/unit/screen_ai_service_event_subscription_tests.rs',
      serviceBridge: 'crates/agent-service/src/screen_ai_service_event_bridge.rs',
      coreRuntime: 'crates/agent-core/src/screen_event_runtime.rs',
      proofHarness: 'scripts/test/screen-service-event-subscription-proof.mjs',
      screenProofSummary: 'output/screen-plan-proof/screen-service-event-subscription/proof-summary.json',
      pipelineProofSummary: 'output/screen-ai-pipeline-proof/screen-service-event-subscription/proof-summary.json',
    },
    eventChain: [
      'screen.service.row.ready',
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
    rejectionCases: ['raw-image-retained-before-downstream-publish'],
    claimsProved: [
      'the Rust service startup retains the screen event subscription runtime before serving requests',
      'ScreenAiServiceEventRuntime::start registers the row-ready subscriber and dispatches through the real event bus',
      'the Rust service owns a typed screen.service.row.ready subscriber using ocentra-eventing',
      'a service row-ready event invokes the existing bridge into the reusable screen runtime chain',
      'accepted rows publish the ordered downstream capture, queue, AI, summary, policy, action, deletion, and portal-read-model event chain',
      'raw-image retained rows fail in the subscriber before downstream screen runtime events are recorded',
      'subscriber state records accepted and rejected service rows for proof and later production wiring',
    ],
    claimsNotProved: [
      'every live trigger producer is externally proved against this subscriber',
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
  console.log('screen-service-event-subscription-proof-ok');
  console.log(`evidence=${relative(repoRoot, proofPath)}`);
  console.log(`screen=${relative(repoRoot, screenProofPath)}`);
  console.log(`pipeline=${relative(repoRoot, pipelineProofPath)}`);
}

async function assertSourceContracts() {
  const protocolConstants = await readText('crates/agent-protocol/src/constants/screen_flow.rs');
  const serviceLib = await readText('crates/agent-service/src/lib.rs');
  const serviceRuntime = await readText('crates/agent-service/src/service_runtime.rs');
  const subscription = await readText('crates/agent-service/src/screen_ai_service_event_subscription.rs');
  const subscriptionTests = await readText('crates/agent-service/tests/unit/screen_ai_service_event_subscription_tests.rs');
  const serviceBridge = await readText('crates/agent-service/src/screen_ai_service_event_bridge.rs');
  const screenChecklist = await readText('docs/plans/screen-plan/implementation-checklist.md');
  const screenFeature = await readText('docs/features/screen-evidence-analysis.md');

  assertIncludes(protocolConstants, 'EVENT_SCREEN_SERVICE_ROW_READY', 'screen service row-ready event constant exists');
  assertIncludes(
    protocolConstants,
    'SUBSCRIBER_SCREEN_SERVICE_ROW_READY',
    'screen service row-ready subscriber constant exists'
  );
  assertIncludes(serviceLib, 'mod screen_ai_service_event_subscription;', 'service subscription module is registered');
  assertIncludes(
    serviceRuntime,
    'ScreenAiServiceEventRuntime::start()',
    'service startup creates the screen event subscription runtime'
  );
  assertIncludes(subscription, 'subscribe_screen_service_row_ready_events', 'service row-ready subscriber exists');
  assertIncludes(subscription, 'publish_screen_service_row_event_chain', 'subscriber invokes existing service bridge');
  assertIncludes(
    subscriptionTests,
    'runtime_start_registers_subscriber_for_production_startup',
    'tests prove startup helper registers the real subscriber'
  );
  assertIncludes(
    subscriptionTests,
    'publishes_existing_runtime_chain',
    'tests prove downstream event chain publication'
  );
  assertIncludes(
    subscriptionTests,
    'rejects_unsafe_rows_before_downstream_publish',
    'tests prove unsafe row rejection'
  );
  assertIncludes(serviceBridge, 'RawImageRetained', 'bridge still rejects raw retention');
  assertIncludes(
    screenChecklist,
    'Screen service event subscription',
    'screen checklist names service event subscription proof'
  );
  assertIncludes(
    screenFeature,
    'screen-service-event-subscription-proof.mjs',
    'screen feature names service event subscription proof'
  );
}

function screenProof(proof) {
  return {
    schemaVersion: proof.schemaVersion,
    proofMode: 'screen-service-event-subscription',
    checkedAt: proof.checkedAt,
    commit: proof.commit,
    commands: proof.commands,
    eventChain: proof.eventChain,
    rejectionCases: proof.rejectionCases,
    claimsProved: proof.claimsProved,
    claimsNotProved: proof.claimsNotProved,
    sourceProof: relative(repoRoot, proofPath),
  };
}

function pipelineProof(proof) {
  return {
    schemaVersion: proof.schemaVersion,
    proofMode: 'screen-service-event-subscription',
    checkedAt: proof.checkedAt,
    commit: proof.commit,
    commands: proof.commands,
    eventChain: proof.eventChain,
    claimsProved: proof.claimsProved.filter(
      (claim) => claim.includes('service') || claim.includes('event') || claim.includes('subscriber')
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
