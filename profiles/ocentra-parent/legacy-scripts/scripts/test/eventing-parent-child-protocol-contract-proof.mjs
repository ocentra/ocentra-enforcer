import { spawn } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';

const repoRoot = process.cwd();
const outputDir = join(repoRoot, 'test-results', 'eventing-parent-child-protocol-contract-proof');
const proofPath = join(outputDir, 'proof.json');
const rowOutputDir = join(repoRoot, 'output', 'eventing-plan-proof', '42-44-parent-child-protocol-contracts');
const rowProofPath = join(rowOutputDir, 'proof-summary.json');
const commands = [];
const proofLabels = [];

await main();

async function main() {
  await mkdir(outputDir, { recursive: true });
  await mkdir(rowOutputDir, { recursive: true });

  await runCommand('cargo', ['test', '-p', 'ocentra-parent-agent-protocol', 'parent_controller']);
  await runCommand('cargo', ['test', '-p', 'ocentra-parent-agent-protocol', 'child_agent']);
  await runCommand('node', ['scripts/check-source-shape.mjs', 'crates/ocentra-eventing']);
  await assertSourceContracts();

  const proof = {
    schemaVersion: 1,
    proofMode: 'eventing-parent-child-protocol-contract-proof',
    checkedAt: new Date().toISOString(),
    commit: await gitHead(),
    commands,
    proofLabels,
    linkedArtifacts: {
      parentControllerContracts: 'crates/agent-protocol/src/parent_controller_events.rs',
      childAgentContracts: 'crates/agent-protocol/src/child_agent_events.rs',
      parentControllerConstants: 'crates/agent-protocol/src/constants/parent_controller.rs',
      childAgentConstants: 'crates/agent-protocol/src/constants/child_agent.rs',
      parentControllerTests: 'crates/agent-protocol/tests/contract/parent_controller_event_tests.rs',
      childAgentTests: 'crates/agent-protocol/tests/contract/child_agent_event_tests.rs',
      proofHarness: 'scripts/test/eventing-parent-child-protocol-contract-proof.mjs',
      rowProof: 'output/eventing-plan-proof/42-44-parent-child-protocol-contracts/proof-summary.json',
      eventingChecklist: 'docs/plans/eventing-plan/implementation-checklist.md',
    },
    rowsCovered: [
      '42 Parent event namespace constants',
      '43 Parent/controller event contracts',
      '44 Child-agent event contracts',
    ],
    claimsProved: [
      'agent-protocol owns parent_controller and child_agent namespace event constants',
      'namespace constants are prefixed and duplicate checked before runtime publish',
      'parent/controller Rust protocol contracts serialize validated intent, command validation, child-command forward, and read-model projection refs',
      'child-agent Rust protocol contracts serialize command receive/accept/reject, capability state, and runtime health refs',
      'serde negative tests reject missing required parent intent and child command refs',
      'contracts are exported from ocentra-parent-agent-protocol for later runtime consumption',
    ],
    claimsNotProved: [
      'parent/controller runtime publisher implementation for validated intents',
      'parent/controller to child-agent transport delivery behavior',
      'child-agent runtime receive and local publish behavior',
      'broker-backed delivery or relay-hub delivery',
      'adapter execution or host/platform enforcement',
      'portal UI ownership of business event publishing',
    ],
  };

  await writeFile(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
  await writeFile(rowProofPath, `${JSON.stringify(rowProof(proof), null, 2)}\n`);
  console.log(`eventing-parent-child-protocol-contract-proof-ok:${proofLabels.join(',')}`);
  console.log(`evidence=${relative(repoRoot, proofPath)}`);
  console.log(`rowProof=${relative(repoRoot, rowProofPath)}`);
}

async function assertSourceContracts() {
  const exports = await readText('crates/agent-protocol/src/lib.rs');
  const constants = await readText('crates/agent-protocol/src/constants.rs');
  const parentConstants = await readText('crates/agent-protocol/src/constants/parent_controller.rs');
  const childConstants = await readText('crates/agent-protocol/src/constants/child_agent.rs');
  const parentContracts = await readText('crates/agent-protocol/src/parent_controller_events.rs');
  const childContracts = await readText('crates/agent-protocol/src/child_agent_events.rs');
  const parentTests = await readText('crates/agent-protocol/tests/contract/parent_controller_event_tests.rs');
  const childTests = await readText('crates/agent-protocol/tests/contract/child_agent_event_tests.rs');
  const contractHarness = await readText('crates/agent-protocol/tests/contract.rs');

  assertIncludes(exports, 'mod parent_controller;', 'parent controller module registered');
  assertIncludes(exports, 'mod child_agent;', 'child agent module registered');
  assertIncludes(exports, 'pub use parent_controller::*;', 'parent controller contracts exported');
  assertIncludes(exports, 'pub use child_agent::*;', 'child agent contracts exported');
  assertIncludes(constants, 'pub mod parent_controller;', 'parent controller constants exported');
  assertIncludes(constants, 'pub mod child_agent;', 'child agent constants exported');
  assertIncludes(parentConstants, 'EVENT_PARENT_ACTION_RECEIVED', 'parent action event constant exists');
  assertIncludes(
    parentConstants,
    'EVENT_CHILD_COMMAND_FORWARDED',
    'parent child-command forwarded event constant exists'
  );
  assertIncludes(childConstants, 'EVENT_COMMAND_RECEIVED', 'child command received event constant exists');
  assertIncludes(childConstants, 'EVENT_RUNTIME_HEALTH_UPDATED', 'child runtime health event constant exists');
  assertIncludes(parentContracts, 'pub trait ParentControllerEventContract', 'parent contract trait exists');
  assertIncludes(parentContracts, 'pub struct ParentActionReceivedEvent', 'parent action contract exists');
  assertIncludes(
    parentContracts,
    'pub struct ParentChildCommandForwardRequestedEvent',
    'parent forward requested contract exists'
  );
  assertIncludes(parentContracts, 'pub struct ParentChildCommandForwardedEvent', 'parent forwarded contract exists');
  assertIncludes(childContracts, 'pub trait ChildAgentEventContract', 'child contract trait exists');
  assertIncludes(childContracts, 'pub struct ChildCommandReceivedEvent', 'child command received contract exists');
  assertIncludes(
    childContracts,
    'pub struct ChildCapabilityStateUpdatedEvent',
    'child capability state contract exists'
  );
  assertIncludes(
    parentTests,
    'parent_and_child_event_namespace_constants_are_unique_and_prefixed',
    'namespace duplicate test exists'
  );
  assertIncludes(
    parentTests,
    'parent_action_contract_rejects_missing_parent_intent_ref',
    'parent negative serde test exists'
  );
  assertIncludes(
    childTests,
    'child_command_contract_rejects_missing_child_command_ref',
    'child negative serde test exists'
  );
  assertIncludes(
    contractHarness,
    '#[path = "contract/parent_controller_event_tests.rs"]',
    'parent controller contract harness registration exists'
  );
  assertIncludes(
    contractHarness,
    '#[path = "contract/child_agent_event_tests.rs"]',
    'child agent contract harness registration exists'
  );

  proofLabels.push('eventing.rows-42-44.parent-child-protocol-contracts');
  proofLabels.push('parent-controller.protocol.constants-and-serde');
  proofLabels.push('child-agent.protocol.constants-and-serde');
  proofLabels.push('parent-child.protocol.required-ref-negative-tests');
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
    const child = spawn('git', ['rev-parse', 'HEAD'], { cwd: repoRoot, stdio: ['ignore', 'pipe', 'pipe'] });
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
    proof: 'eventing-rows-42-44-parent-child-protocol-contracts',
    checkedAt: proof.checkedAt,
    commit: proof.commit,
    commands: proof.commands,
    proofLabels: proof.proofLabels,
    linkedArtifacts: {
      contractProof: relative(repoRoot, proofPath),
      rowProof: relative(repoRoot, rowProofPath),
      parentControllerContracts: proof.linkedArtifacts.parentControllerContracts,
      childAgentContracts: proof.linkedArtifacts.childAgentContracts,
      parentControllerConstants: proof.linkedArtifacts.parentControllerConstants,
      childAgentConstants: proof.linkedArtifacts.childAgentConstants,
      eventingChecklist: proof.linkedArtifacts.eventingChecklist,
    },
    rowsCovered: proof.rowsCovered,
    claimsProved: proof.claimsProved,
    claimsNotProved: proof.claimsNotProved,
  };
}
