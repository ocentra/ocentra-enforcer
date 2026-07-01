import { spawn } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';

const repoRoot = process.cwd();
const outputDir = join(repoRoot, 'test-results', 'screen-service-policy-ref-producer-proof');
const proofPath = join(outputDir, 'proof.json');
const screenOutputDir = join(repoRoot, 'output', 'screen-plan-proof', 'screen-service-policy-ref-producer');
const screenProofPath = join(screenOutputDir, 'proof-summary.json');
const pipelineOutputDir = join(repoRoot, 'output', 'screen-ai-pipeline-proof', 'screen-service-policy-ref-producer');
const pipelineProofPath = join(pipelineOutputDir, 'proof-summary.json');
const commands = [];

await main();

async function main() {
  await mkdir(outputDir, { recursive: true });
  await mkdir(screenOutputDir, { recursive: true });
  await mkdir(pipelineOutputDir, { recursive: true });

  await runCommand('cargo', ['test', '-p', 'ocentra-parent-agent-service', 'screen_ai_analysis_runtime']);
  await runCommand('cargo', ['test', '-p', 'ocentra-parent-agent-service', 'screen_ai_policy_refs']);
  await runCommand('cargo', ['test', '-p', 'ocentra-parent-agent-service', 'screen_ai_service_event_subscription']);
  await assertSourceContracts();

  const proof = {
    schemaVersion: 1,
    proofMode: 'screen-service-policy-ref-producer-proof',
    checkedAt: new Date().toISOString(),
    commit: await gitHead(),
    commands,
    evidence: {
      protocolConstants: 'crates/agent-protocol/src/constants/screen_flow.rs',
      eventRecord: 'crates/agent-service/src/screen_ai_analysis_runtime/event_record.rs',
      policyRefProducer: 'crates/agent-service/src/screen_ai_analysis_runtime/event_record/policy_refs.rs',
      policyRefProducerTests: 'crates/agent-service/tests/unit/screen_ai_policy_refs.rs',
      analysisRuntimeTests: 'crates/agent-service/tests/unit/screen_ai_analysis_runtime_tests.rs',
      serviceSubscriptionTests: 'crates/agent-service/tests/unit/screen_ai_service_event_subscription_tests.rs',
      proofHarness: 'scripts/test/screen-service-policy-ref-producer-proof.mjs',
      screenProofSummary: 'output/screen-plan-proof/screen-service-policy-ref-producer/proof-summary.json',
      pipelineProofSummary: 'output/screen-ai-pipeline-proof/screen-service-policy-ref-producer/proof-summary.json',
    },
    eventChain: [
      'service analysis event record',
      'policy-eligible row policy refs',
      'screen.service.row.ready',
      'existing service subscriber',
      'existing screen event bridge',
      'downstream screen runtime chain for safe rows',
    ],
    claimsProved: [
      'policy-eligible service analysis records carry policy decision, action, reason, parent rule, explanation, and deletion proof refs',
      'non-policy-eligible service analysis records do not fabricate policy refs',
      'the row-ready subscriber still publishes the existing downstream screen runtime chain for safe rows',
      'policy refs are produced in the Rust service event-record path before row-ready publication instead of by a duplicate downstream transform',
    ],
    claimsNotProved: [
      'broad parent-rule compiler coverage',
      'final enforcement execution',
      'new live external capture run',
      'production OCR or VLM quality',
      'authenticated-account social proof',
      'physical household LAN mesh execution',
    ],
  };

  await writeFile(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
  await writeFile(screenProofPath, `${JSON.stringify(screenProof(proof), null, 2)}\n`);
  await writeFile(pipelineProofPath, `${JSON.stringify(pipelineProof(proof), null, 2)}\n`);
  console.log('screen-service-policy-ref-producer-proof-ok');
  console.log(`evidence=${relative(repoRoot, proofPath)}`);
  console.log(`screen=${relative(repoRoot, screenProofPath)}`);
  console.log(`pipeline=${relative(repoRoot, pipelineProofPath)}`);
}

async function assertSourceContracts() {
  const screenFlow = await readText('crates/agent-protocol/src/constants/screen_flow.rs');
  const eventRecord = await readText('crates/agent-service/src/screen_ai_analysis_runtime/event_record.rs');
  const policyRefs = await readText('crates/agent-service/src/screen_ai_analysis_runtime/event_record/policy_refs.rs');
  const policyRefTests = await readText('crates/agent-service/tests/unit/screen_ai_policy_refs.rs');
  const analysisTests = await readText('crates/agent-service/tests/unit/screen_ai_analysis_runtime_tests.rs');
  const subscriptionTests = await readText('crates/agent-service/tests/unit/screen_ai_service_event_subscription_tests.rs');
  const screenChecklist = await readText('docs/plans/screen-plan/implementation-checklist.md');
  const screenFeature = await readText('docs/features/screen-evidence-analysis.md');

  assertIncludes(
    screenFlow,
    'SCREEN_SERVICE_POLICY_DECISION_ID_PREFIX',
    'protocol constants define service policy decision refs'
  );
  assertIncludes(
    eventRecord,
    'policy_refs::service_policy_refs(&image.queue_job_id, parsed.policy_eligible)',
    'event record derives policy refs before Activity Screen row creation'
  );
  assertIncludes(
    eventRecord,
    'policy_refs::screen_analysis_policy_fields(record)',
    'event record serializes policy refs through the child producer module'
  );
  assertIncludes(policyRefs, 'POLICY_DECISION_ID', 'event record writes policy decision field');
  assertIncludes(policyRefs, 'POLICY_ACTION', 'event record writes policy action field');
  assertIncludes(policyRefs, 'POLICY_RULE_IDS', 'event record writes parent rule field');
  assertIncludes(policyRefs, 'SCREEN_DELETION_REASONS', 'event record writes deletion proof field');
  assertIncludes(
    policyRefTests,
    'policy_eligible_service_record_carries_bridge_required_policy_refs',
    'unit test proves eligible rows carry refs'
  );
  assertIncludes(
    policyRefTests,
    'non_policy_eligible_service_record_does_not_fabricate_policy_refs',
    'unit test proves ineligible rows do not fabricate refs'
  );
  assertIncludes(
    analysisTests,
    'screen_analysis_cycle_publishes_row_ready_event_and_gates_missing_policy_refs',
    'analysis runtime still proves row-ready gating for incomplete rows'
  );
  assertIncludes(
    subscriptionTests,
    'screen_service_event_subscription_publishes_existing_runtime_chain',
    'subscriber test proves safe rows publish downstream chain'
  );
  assertIncludes(
    screenChecklist,
    'Screen service policy-ref producer',
    'screen checklist names policy-ref producer proof'
  );
  assertIncludes(
    screenFeature,
    'screen-service-policy-ref-producer-proof.mjs',
    'screen feature names policy-ref producer proof'
  );
}

function screenProof(proof) {
  return {
    schemaVersion: proof.schemaVersion,
    proofMode: 'screen-service-policy-ref-producer',
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
    proofMode: 'screen-service-policy-ref-producer',
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
