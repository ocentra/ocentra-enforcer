import { spawn } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';

const repoRoot = process.cwd();
const outputDir = join(repoRoot, 'test-results', 'eventing-network-protocol-contract-proof');
const proofPath = join(outputDir, 'proof.json');
const rowOutputDir = join(repoRoot, 'output', 'eventing-plan-proof', '45-50-network-protocol-contracts');
const rowProofPath = join(rowOutputDir, 'proof-summary.json');
const commands = [];
const proofLabels = [];

await main();

async function main() {
  await mkdir(outputDir, { recursive: true });
  await mkdir(rowOutputDir, { recursive: true });

  await runCommand('cargo', ['test', '-p', 'ocentra-parent-agent-protocol', 'network']);
  await runCommand('node', ['scripts/check-source-shape.mjs', 'crates/ocentra-eventing']);
  await assertSourceContracts();

  const proof = {
    schemaVersion: 1,
    proofMode: 'eventing-network-protocol-contract-proof',
    checkedAt: new Date().toISOString(),
    commit: await gitHead(),
    commands,
    proofLabels,
    linkedArtifacts: {
      protocolContracts: 'crates/agent-protocol/src/network_flow.rs',
      protocolContractModule: 'crates/agent-protocol/src/network_flow.rs',
      protocolConstants: 'crates/agent-protocol/src/constants/network_flow.rs',
      protocolTests: 'crates/agent-protocol/tests/contract/network_flow_tests.rs',
      protocolTestFixtures: 'crates/agent-protocol/tests/contract/network_flow_event_fixtures.rs',
      proofHarness: 'scripts/test/eventing-network-protocol-contract-proof.mjs',
      rowProof: 'output/eventing-plan-proof/45-50-network-protocol-contracts/proof-summary.json',
      eventingChecklist: 'docs/plans/eventing-plan/implementation-checklist.md',
      networkChecklist: 'docs/plans/network-plan/implementation-checklist.md',
    },
    rowsCovered: [
      '45 Network event contracts',
      '46 AI event contracts',
      '47 Policy event contracts',
      '48 Enforcement event contracts',
      '49 Audit event contracts',
      '50 Portal/read-model event contracts',
      'network-plan row 04 Rust protocol parity for network contracts',
      'network-plan row 10 NetworkActivityEvent contracts and reusable Rust eventing consumption',
    ],
    claimsProved: [
      'agent-protocol owns serde contracts for network flow/domain/classification events',
      'agent-protocol owns serde contracts for network AI requested/completed events with raw-packet rejection fields',
      'agent-protocol owns serde contracts for policy evaluation/decision events with evidence and parent-rule refs',
      'agent-protocol owns serde contracts for enforcement command/result events that require policy decision and adapter capability refs',
      'agent-protocol owns serde contracts for audit committed and portal read-model update events',
      'network protocol event type constants are reused by Rust tests instead of local event-name strings',
      'manual-required enforcement result keeps adapter_action_executed false',
      'missing policyDecisionRef rejects enforcement command deserialization',
      'claim-boundary fields preserve no exact URL, no decrypted HTTPS payload, no message content, no search query, and no adapter action claims',
    ],
    claimsNotProved: [
      'parent/controller event contracts outside the network-triggered chain',
      'child-agent command transport receive/publish behavior',
      'agent-service WebSocket wiring for these event contracts',
      'journal-before-action enforcement integration',
      'real DNS/firewall/WFP/VPN/nftables/NetworkExtension adapter execution',
      'broker-backed delivery, relay-hub delivery, or production retention',
      'portal UI rendering of network event read models',
    ],
  };

  await writeFile(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
  await writeFile(rowProofPath, `${JSON.stringify(rowProof(proof), null, 2)}\n`);
  console.log(`eventing-network-protocol-contract-proof-ok:${proofLabels.join(',')}`);
  console.log(`evidence=${relative(repoRoot, proofPath)}`);
  console.log(`rowProof=${relative(repoRoot, rowProofPath)}`);
}

async function assertSourceContracts() {
  const exports = await readText('crates/agent-protocol/src/network_flow.rs');
  const contracts = await readText('crates/agent-protocol/src/network_flow.rs');
  const constants = await readText('crates/agent-protocol/src/constants/network_flow.rs');
  const tests = await readText('crates/agent-protocol/tests/contract/network_flow_tests.rs');
  const fixtures = await readText('crates/agent-protocol/tests/contract/network_flow_event_fixtures.rs');

  assertIncludes(contracts, 'pub trait NetworkRuntimeEventContract', 'event contract trait exists');
  assertIncludes(exports, 'pub struct NetworkRuntimeEventPayload', 'network runtime payload is Rust-owned');
  assertDoesNotInclude(exports, 'pub use network_flow_events::*;', 'network flow source avoids Rust re-exports');
  assertIncludes(contracts, 'pub struct NetworkFlowObservedEvent', 'network flow event contract exists');
  assertIncludes(contracts, 'pub struct NetworkDomainObservedEvent', 'network domain event contract exists');
  assertIncludes(
    contracts,
    'pub struct NetworkActivityClassifiedEvent',
    'network classification event contract exists'
  );
  assertIncludes(contracts, 'pub struct NetworkAiAnalysisRequestedEvent', 'AI request event contract exists');
  assertIncludes(contracts, 'pub struct NetworkAiAnalysisCompletedEvent', 'AI completed event contract exists');
  assertIncludes(contracts, 'pub struct NetworkPolicyEvaluationRequestedEvent', 'policy request event contract exists');
  assertIncludes(contracts, 'pub struct NetworkPolicyDecisionCompletedEvent', 'policy decision event contract exists');
  assertIncludes(
    contracts,
    'pub struct NetworkEnforcementCommandIssuedEvent',
    'enforcement command event contract exists'
  );
  assertIncludes(
    contracts,
    'pub struct NetworkEnforcementResultObservedEvent',
    'enforcement result event contract exists'
  );
  assertIncludes(contracts, 'pub struct NetworkAuditEntryCommittedEvent', 'audit event contract exists');
  assertIncludes(contracts, 'pub struct NetworkPortalReadModelUpdatedEvent', 'portal read-model event contract exists');
  assertIncludes(constants, 'EVENT_NETWORK_FLOW_OBSERVED', 'network flow event constant exists');
  assertIncludes(constants, 'EVENT_AI_ANALYSIS_REQUESTED', 'AI event constant exists');
  assertIncludes(constants, 'EVENT_POLICY_DECISION_COMPLETED', 'policy event constant exists');
  assertIncludes(constants, 'EVENT_ENFORCEMENT_COMMAND_ISSUED', 'enforcement event constant exists');
  assertIncludes(constants, 'EVENT_AUDIT_ENTRY_COMMITTED', 'audit event constant exists');
  assertIncludes(constants, 'EVENT_PORTAL_READ_MODEL_UPDATED', 'portal event constant exists');
  assertIncludes(tests, 'network_runtime_event_contracts_name_exact_event_types', 'event type constants test exists');
  assertIncludes(
    tests,
    'network_ai_and_policy_contracts_serialize_chain_refs',
    'AI and policy chain refs serde test exists'
  );
  assertIncludes(
    tests,
    'network_enforcement_audit_and_portal_contracts_serialize_refs',
    'enforcement audit and portal serde test exists'
  );
  assertIncludes(
    tests,
    'enforcement_command_contract_rejects_missing_policy_decision_ref',
    'policy decision ref negative test exists'
  );
  assertIncludes(
    tests,
    'manual_required_enforcement_result_keeps_adapter_action_false',
    'manual-required adapter negative test exists'
  );
  assertIncludes(
    fixtures,
    'fn no_claim_boundary() -> NetworkClaimBoundary',
    'contract fixture proves the no-content claim boundary'
  );

  proofLabels.push('eventing.rows-45-50.protocol-contracts');
  proofLabels.push('network.protocol-parity.rust-serde');
  proofLabels.push('network.protocol.no-content-claim-boundary');
  proofLabels.push('enforcement.protocol.requires-policy-decision-ref');
  proofLabels.push('enforcement.protocol.manual-required-no-adapter-action');
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

function assertDoesNotInclude(text, unexpected, label) {
  if (text.includes(unexpected)) {
    throw new Error(`${label}: found ${unexpected}`);
  }
}

function rowProof(proof) {
  return {
    proof: 'eventing-rows-45-50-network-protocol-contracts',
    checkedAt: proof.checkedAt,
    commit: proof.commit,
    commands: proof.commands,
    proofLabels: proof.proofLabels,
    linkedArtifacts: {
      contractProof: relative(repoRoot, proofPath),
      rowProof: relative(repoRoot, rowProofPath),
      protocolContracts: proof.linkedArtifacts.protocolContracts,
      protocolConstants: proof.linkedArtifacts.protocolConstants,
      protocolTests: proof.linkedArtifacts.protocolTests,
      eventingChecklist: proof.linkedArtifacts.eventingChecklist,
      networkChecklist: proof.linkedArtifacts.networkChecklist,
    },
    rowsCovered: proof.rowsCovered,
    claimsProved: proof.claimsProved,
    claimsNotProved: proof.claimsNotProved,
  };
}
