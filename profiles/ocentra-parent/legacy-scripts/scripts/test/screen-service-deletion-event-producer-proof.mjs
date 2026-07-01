import { spawn } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';

const repoRoot = process.cwd();
const outputDir = join(repoRoot, 'test-results', 'screen-service-deletion-event-producer-proof');
const proofPath = join(outputDir, 'proof.json');
const screenOutputDir = join(repoRoot, 'output', 'screen-plan-proof', 'screen-service-deletion-event-producer');
const screenProofPath = join(screenOutputDir, 'proof-summary.json');
const pipelineOutputDir = join(
  repoRoot,
  'output',
  'screen-ai-pipeline-proof',
  'screen-service-deletion-event-producer'
);
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
    'screen_deletion_event_publishes_without_policy_or_action_claims',
  ]);
  await runCommand('cargo', ['test', '-p', 'ocentra-parent-agent-service', 'screen_ai_service_event_bridge']);
  await runCommand('cargo', ['test', '-p', 'ocentra-parent-agent-service', 'screen_ai_retention_sweeper_runtime']);
  await assertSourceContracts();

  const proof = {
    schemaVersion: 1,
    proofMode: 'screen-service-deletion-event-producer-proof',
    checkedAt: new Date().toISOString(),
    commit: await gitHead(),
    commands,
    evidence: {
      coreRuntime: 'crates/agent-core/src/screen_event_runtime.rs',
      coreRuntimeTests: 'crates/agent-core/tests/unit/screen_event_runtime_tests.rs',
      serviceBridge: 'crates/agent-service/src/screen_ai_service_event_bridge.rs',
      serviceBridgeTests: 'crates/agent-service/tests/unit/screen_ai_service_event_bridge_tests.rs',
      retentionRuntime: 'crates/agent-service/src/screen_ai_retention_sweeper_runtime.rs',
      retentionProducer: 'crates/agent-service/src/screen_ai_retention_sweeper_deletion_events.rs',
      retentionTests: 'crates/agent-service/tests/unit/screen_ai_retention_sweeper_runtime_tests.rs',
      proofHarness: 'scripts/test/screen-service-deletion-event-producer-proof.mjs',
      screenProofSummary: 'output/screen-plan-proof/screen-service-deletion-event-producer/proof-summary.json',
      pipelineProofSummary: 'output/screen-ai-pipeline-proof/screen-service-deletion-event-producer/proof-summary.json',
    },
    eventChain: [
      'service retention sweeper TTL expiry',
      'encrypted queue record removal',
      'expiredDeleted Activity Screen row',
      'screen.deletion.committed event',
      'raw-image unavailable to AI, policy, and portal',
    ],
    claimsProved: [
      'core screen runtime can publish a deletion-committed event without policy or action claims',
      'service Activity Screen rows map into deletion event payloads with raw-image-retained rejection and deletion proof requirement',
      'service retention sweeper runtime publishes deletion events after expired queue removal',
      'service retention sweeper rows preserve deletion proof refs before deletion event publication',
      'the deletion producer reuses the existing screen event runtime path instead of adding a second event bus',
    ],
    claimsNotProved: [
      'new live external capture run',
      'production OCR or VLM quality',
      'final enforcement execution',
      'parent retention-duration UI persistence',
      'physical household LAN mesh execution',
    ],
  };

  await writeFile(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
  await writeFile(screenProofPath, `${JSON.stringify(summaryProof(proof), null, 2)}\n`);
  await writeFile(pipelineProofPath, `${JSON.stringify(summaryProof(proof), null, 2)}\n`);
  console.log('screen-service-deletion-event-producer-proof-ok');
  console.log(`evidence=${relative(repoRoot, proofPath)}`);
  console.log(`screen=${relative(repoRoot, screenProofPath)}`);
  console.log(`pipeline=${relative(repoRoot, pipelineProofPath)}`);
}

async function assertSourceContracts() {
  const coreRuntime = await readText('crates/agent-core/src/screen_event_runtime.rs');
  const coreTests = await readText('crates/agent-core/tests/unit/screen_event_runtime_tests.rs');
  const serviceBridge = await readText('crates/agent-service/src/screen_ai_service_event_bridge.rs');
  const serviceTests = await readText('crates/agent-service/tests/unit/screen_ai_service_event_bridge_tests.rs');
  const retentionRuntime = await readText('crates/agent-service/src/screen_ai_retention_sweeper_runtime.rs');
  const retentionProducer = await readText('crates/agent-service/src/screen_ai_retention_sweeper_deletion_events.rs');
  const retentionTests = await readText('crates/agent-service/tests/unit/screen_ai_retention_sweeper_runtime_tests.rs');
  const screenChecklist = await readText('docs/plans/screen-plan/implementation-checklist.md');
  const screenFeature = await readText('docs/features/screen-evidence-analysis.md');

  assertIncludes(coreRuntime, 'ScreenRuntimeDeletionInput', 'core runtime defines deletion input');
  assertIncludes(
    coreRuntime,
    'publish_screen_deletion_event_for_input',
    'core runtime exposes deletion event publisher'
  );
  assertIncludes(
    coreTests,
    'screen_deletion_event_publishes_without_policy_or_action_claims',
    'core test proves deletion event avoids policy and action claims'
  );
  assertIncludes(
    serviceBridge,
    'publish_screen_deletion_event_for_queue_job',
    'service bridge publishes deletion events by queue job'
  );
  assertIncludes(
    serviceBridge,
    'screen_runtime_deletion_input_from_service_row',
    'service bridge maps rows to deletion input'
  );
  assertIncludes(
    serviceTests,
    'screen_service_event_bridge_publishes_deletion_event_from_retention_row',
    'service test proves deletion event bridge'
  );
  assertIncludes(
    retentionRuntime,
    'publish_screen_retention_deletion_events',
    'retention runtime publishes deletion events'
  );
  assertIncludes(
    retentionRuntime,
    'SCREEN_DELETION_REASONS',
    'retention runtime keeps deletion proof refs on Activity Screen rows'
  );
  assertIncludes(
    retentionProducer,
    'publish_screen_deletion_event_for_queue_job',
    'retention producer reuses deletion event bridge'
  );
  assertIncludes(
    retentionTests,
    'assert_sweep_deletion_events',
    'retention tests assert deletion event producer outcome'
  );
  assertIncludes(
    screenChecklist,
    'Screen service deletion event producer',
    'screen checklist names deletion event producer proof'
  );
  assertIncludes(
    screenFeature,
    'screen-service-deletion-event-producer-proof.mjs',
    'screen feature names deletion event producer proof'
  );
}

function summaryProof(proof) {
  return {
    schemaVersion: proof.schemaVersion,
    proofMode: 'screen-service-deletion-event-producer',
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
