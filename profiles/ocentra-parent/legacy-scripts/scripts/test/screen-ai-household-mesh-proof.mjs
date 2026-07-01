import { spawn } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';

const repoRoot = process.cwd();
const outputDir = join(repoRoot, 'test-results', 'screen-ai-household-mesh-proof');
const proofPath = join(outputDir, 'proof.json');
const screenOutputDir = join(repoRoot, 'output', 'screen-plan-proof', 'screen-household-mesh');
const screenProofPath = join(screenOutputDir, 'proof-summary.json');
const pipelineOutputDir = join(repoRoot, 'output', 'screen-ai-pipeline-proof', 'household-mesh-screen-ai');
const pipelineProofPath = join(pipelineOutputDir, 'proof-summary.json');
const noRawOutputDir = join(repoRoot, 'output', 'ai-plan-proof', 'no-raw-screen-transfer-mesh');
const noRawProofPath = join(noRawOutputDir, 'proof-summary.json');
const validationOutputDir = join(repoRoot, 'output', 'ai-plan-proof', 'household-ai-provider-result-validation');
const validationProofPath = join(validationOutputDir, 'proof-summary.json');
const commands = [];

await main();

async function main() {
  await mkdir(outputDir, { recursive: true });
  await mkdir(screenOutputDir, { recursive: true });
  await mkdir(pipelineOutputDir, { recursive: true });
  await mkdir(noRawOutputDir, { recursive: true });
  await mkdir(validationOutputDir, { recursive: true });

  await runCommand('cargo', ['test', '-p', 'ocentra-parent-agent-core', 'screen_household_mesh']);
  await runCommand('node', ['scripts/check-source-shape.mjs']);
  await assertSourceContracts();

  const proof = {
    schemaVersion: 1,
    proofMode: 'screen-ai-household-mesh-proof',
    checkedAt: new Date().toISOString(),
    commit: await gitHead(),
    commands,
    evidence: {
      screenFlowConstants: 'crates/agent-protocol/src/constants/screen_flow.rs',
      meshRuntime: 'crates/agent-core/src/screen_household_mesh_runtime.rs',
      meshRuntimePhase: 'crates/agent-core/src/screen_household_mesh_runtime_phase.rs',
      meshRuntimeState: 'crates/agent-core/src/screen_household_mesh_runtime_state.rs',
      meshRuntimeRefs: 'crates/agent-core/src/screen_household_mesh_runtime_refs.rs',
      meshRuntimeTests: 'crates/agent-core/tests/unit/screen_household_mesh_runtime_tests.rs',
      proofHarness: 'scripts/test/screen-ai-household-mesh-proof.mjs',
      screenProofSummary: 'output/screen-plan-proof/screen-household-mesh/proof-summary.json',
      pipelineProofSummary: 'output/screen-ai-pipeline-proof/household-mesh-screen-ai/proof-summary.json',
      noRawProofSummary: 'output/ai-plan-proof/no-raw-screen-transfer-mesh/proof-summary.json',
      resultValidationProofSummary: 'output/ai-plan-proof/household-ai-provider-result-validation/proof-summary.json',
    },
    eventChain: [
      'screen.mesh.work.queued',
      'screen.mesh.offer.published',
      'screen.mesh.claim.requested',
      'screen.mesh.claim.granted',
      'screen.mesh.lease.created',
      'screen.mesh.provider-result.returned',
      'screen.mesh.child-result.accepted',
      'screen.mesh.policy.requested',
    ],
    rejectionCases: [
      'duplicate-result',
      'expired-lease',
      'wrong-provider',
      'wrong-claim',
      'evidence-mismatch',
      'custody-mismatch',
      'raw-image-transfer',
      'provider-authority-violation',
    ],
    claimsProved: [
      'screen-derived household AI work is modeled as child-owned evented work with provider claim and lease phases',
      'provider payload mode is redacted screen summary/custody refs, not raw screenshot transfer',
      'provider result is worker output only and cannot publish policy or enforcement events',
      'child agent validates provider result before policy may run',
      'duplicate, expired, wrong-provider, wrong-claim, evidence-mismatch, custody-mismatch, raw-transfer, and provider-authority-invalid results are rejected before policy',
    ],
    claimsNotProved: [
      'physical household LAN execution on a second installed device',
      'production mesh bridge transport over authenticated LAN messages',
      'live model quality on a household provider',
      'mobile dormant/fallback provider runtime behavior',
      'browser, network, mobile, or broad adapter enforcement from the mesh result',
    ],
  };

  await writeFile(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
  await writeFile(screenProofPath, `${JSON.stringify(screenProof(proof), null, 2)}\n`);
  await writeFile(pipelineProofPath, `${JSON.stringify(pipelineProof(proof), null, 2)}\n`);
  await writeFile(noRawProofPath, `${JSON.stringify(noRawProof(proof), null, 2)}\n`);
  await writeFile(validationProofPath, `${JSON.stringify(validationProof(proof), null, 2)}\n`);
  console.log('screen-ai-household-mesh-proof-ok');
  console.log(`evidence=${relative(repoRoot, proofPath)}`);
  console.log(`screen=${relative(repoRoot, screenProofPath)}`);
  console.log(`pipeline=${relative(repoRoot, pipelineProofPath)}`);
  console.log(`noRaw=${relative(repoRoot, noRawProofPath)}`);
  console.log(`validation=${relative(repoRoot, validationProofPath)}`);
}

async function assertSourceContracts() {
  const constantsSource = await readText('crates/agent-protocol/src/constants/screen_flow.rs');
  const coreLib = await readText('crates/agent-core/src/lib.rs');
  const runtimeSource = await readText('crates/agent-core/src/screen_household_mesh_runtime.rs');
  const stateSource = await readText('crates/agent-core/src/screen_household_mesh_runtime_state.rs');
  const testsSource = await readText('crates/agent-core/tests/unit/screen_household_mesh_runtime_tests.rs');
  const screenChecklist = await readText('docs/plans/screen-plan/implementation-checklist.md');
  const pipelineChecklist = await readText('docs/plans/screen-ai-pipeline-plan/implementation-checklist.md');
  const aiChecklist = await readText('docs/plans/ai-plan/implementation-checklist.md');

  assertIncludes(constantsSource, 'EVENT_SCREEN_MESH_CLAIM_GRANTED', 'claim-granted event constant exists');
  assertIncludes(constantsSource, 'EVENT_SCREEN_MESH_LEASE_CREATED', 'lease-created event constant exists');
  assertIncludes(constantsSource, 'EVENT_SCREEN_MESH_CHILD_RESULT_ACCEPTED', 'child-accepted event constant exists');
  assertIncludes(runtimeSource, 'impl DomainEvent for ScreenHouseholdMeshEventPayload', 'mesh payload is typed event');
  assertIncludes(runtimeSource, 'EventBus::new()', 'mesh proof uses reusable eventing bus');
  assertIncludes(stateSource, 'raw_screenshot_transferred: false', 'raw transfer is disabled');
  assertIncludes(stateSource, 'provider_can_publish_policy: false', 'provider policy authority is disabled');
  assertIncludes(runtimeSource, 'ProviderAuthorityViolation', 'provider authority rejection exists');
  assertIncludes(coreLib, 'publish_screen_household_mesh_chain_for_input', 'agent-core exports mesh proof helper');
  assertIncludes(
    testsSource,
    'rejects_invalid_provider_results_before_policy',
    'tests reject invalid provider results'
  );
  assertIncludes(testsSource, 'policy_waits_for_child_accepted_result', 'tests gate policy on child acceptance');
  assertIncludes(screenChecklist, 'Screen-derived household provider jobs prove', 'screen checklist names mesh gate');
  assertIncludes(pipelineChecklist, 'Household mesh screen AI proof', 'pipeline checklist names mesh proof');
  assertIncludes(aiChecklist, 'No-raw-screen-transfer mesh proof run', 'AI checklist names no-raw mesh proof');
  assertDoesNotInclude(stateSource, 'raw_screenshot_transferred: true', 'no raw screenshot transfer');
  assertDoesNotInclude(stateSource, 'provider_can_publish_policy: true', 'no provider policy authority');
}

function screenProof(proof) {
  return {
    schemaVersion: proof.schemaVersion,
    proofMode: 'screen-household-mesh',
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
    proofMode: 'household-mesh-screen-ai',
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

function noRawProof(proof) {
  return {
    schemaVersion: proof.schemaVersion,
    proofMode: 'no-raw-screen-transfer-mesh',
    checkedAt: proof.checkedAt,
    commit: proof.commit,
    commands: proof.commands,
    claimsProved: proof.claimsProved.filter((claim) => claim.includes('raw') || claim.includes('custody')),
    claimsNotProved: proof.claimsNotProved,
    sourceProof: relative(repoRoot, proofPath),
  };
}

function validationProof(proof) {
  return {
    schemaVersion: proof.schemaVersion,
    proofMode: 'household-ai-provider-result-validation',
    checkedAt: proof.checkedAt,
    commit: proof.commit,
    commands: proof.commands,
    rejectionCases: proof.rejectionCases,
    claimsProved: proof.claimsProved.filter((claim) => claim.includes('validat') || claim.includes('rejected')),
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
