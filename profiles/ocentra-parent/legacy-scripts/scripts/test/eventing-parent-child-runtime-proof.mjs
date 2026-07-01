import { spawn } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';

const repoRoot = process.cwd();
const outputDir = join(repoRoot, 'test-results', 'eventing-parent-child-runtime-proof');
const proofPath = join(outputDir, 'proof.json');
const rowOutputDir = join(repoRoot, 'output', 'eventing-plan-proof', '51-54-parent-child-runtime');
const rowProofPath = join(rowOutputDir, 'proof-summary.json');
const commands = [];
const proofLabels = [];

await main();

async function main() {
  await mkdir(outputDir, { recursive: true });
  await mkdir(rowOutputDir, { recursive: true });

  await runCommand('cargo', ['test', '-p', 'ocentra-parent-agent-core', 'parent_child']);
  await runCommand('cargo', ['test', '-p', 'ocentra-parent-agent-protocol', 'parent_controller']);
  await runCommand('cargo', ['test', '-p', 'ocentra-parent-agent-protocol', 'child_agent']);
  await runCommand('node', ['scripts/check-source-shape.mjs', 'crates/ocentra-eventing']);
  await assertRuntimeContracts();

  const proof = {
    schemaVersion: 1,
    proofMode: 'eventing-parent-child-runtime-proof',
    checkedAt: new Date().toISOString(),
    commit: await gitHead(),
    commands,
    proofLabels,
    linkedArtifacts: {
      runtime: 'crates/agent-core/src/parent_child_event_runtime.rs',
      runtimeBuild: 'crates/agent-core/src/parent_child_event_runtime/build.rs',
      runtimePhase: 'crates/agent-core/src/parent_child_event_runtime_phase.rs',
      runtimeTests: 'crates/agent-core/tests/unit/parent_child_event_runtime_tests.rs',
      parentConstants: 'crates/agent-protocol/src/constants/parent_controller.rs',
      childConstants: 'crates/agent-protocol/src/constants/child_agent.rs',
      rowProof: 'output/eventing-plan-proof/51-54-parent-child-runtime/proof-summary.json',
      eventingChecklist: 'docs/plans/eventing-plan/implementation-checklist.md',
    },
    rowsCovered: [
      '51 Rust parent/controller validated intent publisher',
      '53 Parent/controller child-command transport handoff',
      '54 Child-agent command receive and local event publish',
    ],
    claimsProved: [
      'agent-core publishes parent/controller validated intent events through ocentra-eventing',
      'parent child-command forward requested and forwarded events share exact child command and transport message refs',
      'child-agent command received, accepted, capability, and health events are locally published after parent handoff',
      'parent read-model projection follows the child runtime health event without portal publishing business events',
      'runtime rows consume the protocol constants and structs added for rows 42-44',
    ],
    claimsNotProved: [
      'broker-backed or relay-hub transport delivery',
      'platform adapter execution or host enforcement',
      'portal UI publishing business events',
      'network packet capture or analyzer model execution',
      'physical child-device runtime installation',
    ],
  };

  await writeFile(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
  await writeFile(rowProofPath, `${JSON.stringify(rowProof(proof), null, 2)}\n`);
  console.log(`eventing-parent-child-runtime-proof-ok:${proofLabels.join(',')}`);
  console.log(`evidence=${relative(repoRoot, proofPath)}`);
  console.log(`rowProof=${relative(repoRoot, rowProofPath)}`);
}

async function assertRuntimeContracts() {
  const exports = await readText('crates/agent-core/src/lib.rs');
  const runtime = await readText('crates/agent-core/src/parent_child_event_runtime.rs');
  const runtimeBuild = await readText('crates/agent-core/src/parent_child_event_runtime/build.rs');
  const phases = await readText('crates/agent-core/src/parent_child_event_runtime_phase.rs');
  const tests = await readText('crates/agent-core/tests/unit/parent_child_event_runtime_tests.rs');
  const parentConstants = await readText('crates/agent-protocol/src/constants/parent_controller.rs');
  const childConstants = await readText('crates/agent-protocol/src/constants/child_agent.rs');

  assertIncludes(exports, 'mod parent_child_event_runtime;', 'runtime module registered');
  assertIncludes(exports, 'publish_parent_child_runtime_for_validated_intent', 'runtime publisher exported');
  assertIncludes(runtime, 'ParentChildRuntimeEventPayload', 'runtime publishes typed payload enum');
  assertIncludes(
    runtimeBuild,
    'ParentChildRuntimeEventPayload::ParentCommandValidated',
    'parent validated intent event published'
  );
  assertIncludes(
    runtimeBuild,
    'ParentChildRuntimeEventPayload::ParentChildCommandForwarded',
    'parent transport forwarded event published'
  );
  assertIncludes(runtimeBuild, 'ParentChildRuntimeEventPayload::ChildCommandReceived', 'child receive event published');
  assertIncludes(
    runtimeBuild,
    'ParentChildRuntimeEventPayload::ChildRuntimeHealthUpdated',
    'child health event published'
  );
  assertIncludes(phases, 'pub fn ordered_chain', 'runtime phase order is explicit');
  assertIncludes(parentConstants, 'CORRELATION_PARENT_CHILD_RUNTIME_PREFIX', 'parent-child correlation prefix exists');
  assertIncludes(childConstants, 'TARGET_CHILD_COMMAND_RECEIVER', 'child command receiver target exists');
  assertIncludes(tests, 'parent_child_transport_handoff_preserves_forwarded_refs', 'transport handoff ref test exists');
  assertIncludes(
    tests,
    'child_agent_receive_publishes_local_events_and_parent_read_model',
    'child local publish test exists'
  );

  proofLabels.push('eventing.row-51.parent-controller-validated-intent-publisher');
  proofLabels.push('eventing.row-53.parent-child-command-transport-handoff');
  proofLabels.push('eventing.row-54.child-agent-local-event-publish');
}

async function runCommand(command, args) {
  commands.push([command, ...args].join(' '));
  await new Promise((resolve, reject) => {
    const child = spawn(command, args, { cwd: repoRoot, stdio: 'inherit', windowsHide: true });
    child.once('exit', (code) =>
      code === 0 ? resolve() : reject(new Error(`${command} ${args.join(' ')} exited with ${code}`))
    );
    child.once('error', reject);
  });
}

async function gitHead() {
  const chunks = [];
  await new Promise((resolve, reject) => {
    const child = spawn('git', ['rev-parse', 'HEAD'], {
      cwd: repoRoot,
      stdio: ['ignore', 'pipe', 'pipe'],
    });
    child.stdout.on('data', (chunk) => chunks.push(String(chunk)));
    child.once('exit', (code) => (code === 0 ? resolve() : reject(new Error('git rev-parse HEAD failed'))));
    child.once('error', reject);
  });
  return chunks.join('').trim();
}

async function readText(path) {
  return readFile(join(repoRoot, path), 'utf8');
}

function assertIncludes(text, expected, label) {
  if (!text.includes(expected)) {
    throw new Error(`${label}: missing ${expected}`);
  }
}

function rowProof(proof) {
  return {
    proof: 'eventing-rows-51-53-54-parent-child-runtime',
    checkedAt: proof.checkedAt,
    commit: proof.commit,
    commands: proof.commands,
    proofLabels: proof.proofLabels,
    linkedArtifacts: {
      contractProof: relative(repoRoot, proofPath),
      rowProof: relative(repoRoot, rowProofPath),
      runtime: proof.linkedArtifacts.runtime,
      runtimeBuild: proof.linkedArtifacts.runtimeBuild,
      runtimePhase: proof.linkedArtifacts.runtimePhase,
      runtimeTests: proof.linkedArtifacts.runtimeTests,
      eventingChecklist: proof.linkedArtifacts.eventingChecklist,
    },
    rowsCovered: proof.rowsCovered,
    claimsProved: proof.claimsProved,
    claimsNotProved: proof.claimsNotProved,
  };
}
