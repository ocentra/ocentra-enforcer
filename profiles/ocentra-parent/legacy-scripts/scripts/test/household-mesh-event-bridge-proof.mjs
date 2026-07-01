import { spawn } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';

const repoRoot = process.cwd();
const outputDir = join(repoRoot, 'test-results', 'household-mesh-event-bridge-proof');
const proofPath = join(outputDir, 'proof.json');
const bridgeOutputDir = join(repoRoot, 'output', 'ai-plan-proof', 'household-mesh-event-bridge-proof');
const bridgeProofPath = join(bridgeOutputDir, 'proof-summary.json');
const topologyOutputDir = join(repoRoot, 'output', 'ai-plan-proof', 'ai-mesh-event-topology-proof');
const topologyProofPath = join(topologyOutputDir, 'proof-summary.json');
const commands = [];

await main();

async function main() {
  await mkdir(outputDir, { recursive: true });
  await mkdir(bridgeOutputDir, { recursive: true });
  await mkdir(topologyOutputDir, { recursive: true });

  await runCommand('cargo', ['test', '-p', 'ocentra-parent-agent-core', 'household_mesh_bridge']);
  await runCommand('node', ['scripts/check-source-shape.mjs']);
  await assertSourceContracts();

  const proof = {
    schemaVersion: 1,
    proofMode: 'household-mesh-event-bridge-proof',
    checkedAt: new Date().toISOString(),
    commit: await gitHead(),
    commands,
    evidence: {
      constants: 'crates/agent-protocol/src/constants/household_mesh.rs',
      bridgeRuntime: 'crates/agent-core/src/household_mesh_bridge_runtime.rs',
      bridgePhase: 'crates/agent-core/src/household_mesh_bridge_runtime_phase.rs',
      bridgeState: 'crates/agent-core/src/household_mesh_bridge_runtime_state.rs',
      bridgeRefs: 'crates/agent-core/src/household_mesh_bridge_runtime_refs.rs',
      bridgeTests: 'crates/agent-core/tests/integration/household_mesh_bridge_runtime.rs',
      proofHarness: 'scripts/test/household-mesh-event-bridge-proof.mjs',
      bridgeProofSummary: 'output/ai-plan-proof/household-mesh-event-bridge-proof/proof-summary.json',
      topologyProofSummary: 'output/ai-plan-proof/ai-mesh-event-topology-proof/proof-summary.json',
    },
    eventChain: [
      'household.mesh.bridge.local-event.selected',
      'household.mesh.bridge.lan-message.exported',
      'household.mesh.bridge.lan-message.received',
      'household.mesh.bridge.local-event.republished',
    ],
    allowedLanMessages: ['household.mesh.ai-work-offer', 'household.mesh.ai-work-result'],
    rejectionCases: [
      'unselected-event',
      'private-local-event',
      'raw-screen-payload',
      'unauthenticated-peer',
      'unauthorized-peer',
      'direct-remote-publish',
      'unsupported-lan-message',
    ],
    claimsProved: [
      'selected local household mesh events are converted to typed LAN message envelopes',
      'validated incoming LAN messages are republished into a local runtime bus only after authentication and authorization checks',
      'remote peers cannot publish directly into another runtime local bus',
      'private local queue and capture internals plus raw screen payloads are rejected before LAN export or local republish',
      'AI mesh topology is bridge-mediated: ocentra-eventing remains local runtime infrastructure, not a LAN-wide broker',
    ],
    claimsNotProved: [
      'physical two-device LAN transport',
      'provider advertisement heartbeat gossip over real sockets',
      'provider route selection across live household devices',
      'lease expiry requeue and dead-letter over real LAN',
      'mobile dormant fallback provider behavior',
      'live service AI event producers and subscribers',
      'production portal mesh UI',
    ],
  };

  await writeFile(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
  await writeFile(bridgeProofPath, `${JSON.stringify(bridgeProof(proof), null, 2)}\n`);
  await writeFile(topologyProofPath, `${JSON.stringify(topologyProof(proof), null, 2)}\n`);
  console.log('household-mesh-event-bridge-proof-ok');
  console.log(`evidence=${relative(repoRoot, proofPath)}`);
  console.log(`bridge=${relative(repoRoot, bridgeProofPath)}`);
  console.log(`topology=${relative(repoRoot, topologyProofPath)}`);
}

async function assertSourceContracts() {
  const constantsSource = await readText('crates/agent-protocol/src/constants/household_mesh.rs');
  const coreLib = await readText('crates/agent-core/src/lib.rs');
  const runtimeSource = await readText('crates/agent-core/src/household_mesh_bridge_runtime.rs');
  const stateSource = await readText('crates/agent-core/src/household_mesh_bridge_runtime_state.rs');
  const testsSource = await readText('crates/agent-core/tests/integration/household_mesh_bridge_runtime.rs');
  const aiChecklist = await readText('docs/plans/ai-plan/implementation-checklist.md');
  const aiFeature = await readText('docs/features/local-ai-safety-evaluator.md');

  assertIncludes(constantsSource, 'EVENT_BRIDGE_LAN_EXPORTED', 'bridge LAN exported event constant exists');
  assertIncludes(constantsSource, 'MESSAGE_AI_WORK_OFFER', 'AI work offer LAN message constant exists');
  assertIncludes(
    runtimeSource,
    'impl DomainEvent for HouseholdMeshBridgeEventPayload',
    'bridge payload is typed event'
  );
  assertIncludes(runtimeSource, 'EventBus::new()', 'bridge proof uses reusable local eventing bus');
  assertIncludes(runtimeSource, 'DirectRemotePublish', 'direct remote publish rejection exists');
  assertIncludes(runtimeSource, 'PrivateLocalEvent', 'private local event rejection exists');
  assertIncludes(stateSource, 'remote_direct_publish_allowed: false', 'remote direct publish is disabled');
  assertIncludes(stateSource, 'raw_screenshot_transferred: false', 'raw screenshot transfer is disabled');
  assertIncludes(coreLib, 'publish_household_mesh_bridge_chain_for_input', 'agent-core exports bridge proof helper');
  assertIncludes(testsSource, 'rejects_export_of_unselected_private_or_raw_events', 'tests reject private/raw export');
  assertIncludes(testsSource, 'rejects_untrusted_or_direct_remote_imports', 'tests reject invalid import');
  assertIncludes(aiChecklist, 'Household mesh event bridge proof run', 'AI checklist names bridge proof run');
  assertIncludes(aiFeature, 'household-mesh-event-bridge-proof.mjs', 'feature doc names bridge proof');
  assertDoesNotInclude(stateSource, 'remote_direct_publish_allowed: true', 'no direct remote publish');
  assertDoesNotInclude(stateSource, 'raw_screenshot_transferred: true', 'no raw screenshot transfer');
}

function bridgeProof(proof) {
  return {
    schemaVersion: proof.schemaVersion,
    proofMode: 'household-mesh-event-bridge-proof',
    checkedAt: proof.checkedAt,
    commit: proof.commit,
    commands: proof.commands,
    eventChain: proof.eventChain,
    allowedLanMessages: proof.allowedLanMessages,
    rejectionCases: proof.rejectionCases,
    claimsProved: proof.claimsProved,
    claimsNotProved: proof.claimsNotProved,
    sourceProof: relative(repoRoot, proofPath),
  };
}

function topologyProof(proof) {
  return {
    schemaVersion: proof.schemaVersion,
    proofMode: 'ai-mesh-event-topology-proof',
    checkedAt: proof.checkedAt,
    commit: proof.commit,
    commands: proof.commands,
    topologyClaims: proof.claimsProved.filter(
      (claim) => claim.includes('local runtime') || claim.includes('directly') || claim.includes('topology')
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
