import { spawn } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';

const repoRoot = process.cwd();
const outputDir = join(repoRoot, 'test-results', 'screen-service-analysis-row-ready-proof');
const proofPath = join(outputDir, 'proof.json');
const screenOutputDir = join(repoRoot, 'output', 'screen-plan-proof', 'screen-service-analysis-row-ready');
const screenProofPath = join(screenOutputDir, 'proof-summary.json');
const pipelineOutputDir = join(repoRoot, 'output', 'screen-ai-pipeline-proof', 'screen-service-analysis-row-ready');
const pipelineProofPath = join(pipelineOutputDir, 'proof-summary.json');
const commands = [];

await main();

async function main() {
  await mkdir(outputDir, { recursive: true });
  await mkdir(screenOutputDir, { recursive: true });
  await mkdir(pipelineOutputDir, { recursive: true });

  await runCommand('cargo', ['test', '-p', 'ocentra-parent-agent-service', 'screen_ai_analysis_runtime']);
  await runCommand('cargo', ['test', '-p', 'ocentra-parent-agent-service', 'screen_ai_service_event_subscription']);
  await assertSourceContracts();

  const proof = {
    schemaVersion: 1,
    proofMode: 'screen-service-analysis-row-ready-proof',
    checkedAt: new Date().toISOString(),
    commit: await gitHead(),
    commands,
    evidence: {
      serviceLib: 'crates/agent-service/src/lib.rs',
      analysisRuntime: 'crates/agent-service/src/screen_ai_analysis_runtime.rs',
      analysisRuntimeTests: 'crates/agent-service/tests/unit/screen_ai_analysis_runtime_tests.rs',
      readModelMapper: 'crates/agent-service/src/activity_surface_read_models.rs',
      serviceSubscription: 'crates/agent-service/src/screen_ai_service_event_subscription.rs',
      serviceBridge: 'crates/agent-service/src/screen_ai_service_event_bridge.rs',
      proofHarness: 'scripts/test/screen-service-analysis-row-ready-proof.mjs',
      screenProofSummary: 'output/screen-plan-proof/screen-service-analysis-row-ready/proof-summary.json',
      pipelineProofSummary: 'output/screen-ai-pipeline-proof/screen-service-analysis-row-ready/proof-summary.json',
    },
    eventChain: [
      'screen.service analysis runtime records Activity Screen row',
      'screen.service.row.ready',
      'screen service subscriber invokes existing bridge',
      'bridge rejects missing policy refs before downstream publication',
    ],
    claimsProved: [
      'the service analysis runtime starts a local row-ready event runtime at service startup',
      'a recorded service analysis row is converted through the shared Activity Screen row mapper',
      'the service analysis cycle publishes screen.service.row.ready for the recorded analysis row',
      'the existing subscriber records MissingPolicyDecision for analysis rows before policy refs exist',
      'the producer uses the same service bridge and does not create a duplicate event path',
    ],
    claimsNotProved: [
      'service policy refs are produced by the analysis runtime',
      'downstream policy/action/deletion/portal chain is published for incomplete analysis rows',
      'new live external capture run',
      'production OCR or VLM quality',
      'authenticated-account social proof',
      'physical household LAN mesh execution',
    ],
  };

  await writeFile(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
  await writeFile(screenProofPath, `${JSON.stringify(screenProof(proof), null, 2)}\n`);
  await writeFile(pipelineProofPath, `${JSON.stringify(pipelineProof(proof), null, 2)}\n`);
  console.log('screen-service-analysis-row-ready-proof-ok');
  console.log(`evidence=${relative(repoRoot, proofPath)}`);
  console.log(`screen=${relative(repoRoot, screenProofPath)}`);
  console.log(`pipeline=${relative(repoRoot, pipelineProofPath)}`);
}

async function assertSourceContracts() {
  const serviceLib = await readText('crates/agent-service/src/lib.rs');
  const analysisRuntime = await readText('crates/agent-service/src/screen_ai_analysis_runtime.rs');
  const analysisTests = await readText('crates/agent-service/tests/unit/screen_ai_analysis_runtime_tests.rs');
  const readModelMapper = await readText('crates/agent-service/src/activity_surface_read_models.rs');
  const subscription = await readText('crates/agent-service/src/screen_ai_service_event_subscription.rs');
  const screenChecklist = await readText('docs/plans/screen-plan/implementation-checklist.md');
  const screenFeature = await readText('docs/features/screen-evidence-analysis.md');

  assertIncludes(
    serviceLib,
    'mod screen_ai_service_event_subscription;',
    'service subscription module is production-registered'
  );
  assertIncludes(
    analysisRuntime,
    'ScreenAiServiceEventRuntime::start',
    'analysis runtime starts service event runtime'
  );
  assertIncludes(
    analysisRuntime,
    'record_screen_ai_analysis_cycle_with_events',
    'analysis runtime exposes event-aware cycle'
  );
  assertIncludes(analysisRuntime, 'publish_row_ready', 'analysis runtime publishes row-ready events');
  assertIncludes(
    readModelMapper,
    'activity_screen_row_from_result',
    'analysis runtime reuses Activity Screen row mapper'
  );
  assertIncludes(
    subscription,
    'publish_screen_service_row_event_chain',
    'row-ready subscriber still invokes existing service bridge'
  );
  assertIncludes(
    analysisTests,
    'screen_analysis_cycle_publishes_row_ready_event_and_gates_missing_policy_refs',
    'analysis runtime test proves row-ready gating'
  );
  assertIncludes(
    screenChecklist,
    'Screen service analysis row-ready producer',
    'screen checklist names analysis row-ready producer proof'
  );
  assertIncludes(
    screenFeature,
    'screen-service-analysis-row-ready-proof.mjs',
    'screen feature names analysis row-ready proof'
  );
}

function screenProof(proof) {
  return {
    schemaVersion: proof.schemaVersion,
    proofMode: 'screen-service-analysis-row-ready',
    checkedAt: proof.checkedAt,
    commit: proof.commit,
    commands: proof.commands,
    eventChain: proof.eventChain,
    claimsProved: proof.claimsProved,
    claimsNotProved: proof.claimsNotProved,
    sourceProof: relative(repoRoot, proofPath),
  };
}

function pipelineProof(proof) {
  return {
    schemaVersion: proof.schemaVersion,
    proofMode: 'screen-service-analysis-row-ready',
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
