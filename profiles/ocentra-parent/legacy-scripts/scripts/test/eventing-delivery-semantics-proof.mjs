import { spawnSync } from 'node:child_process';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const proofRoot = join('output', 'eventing-plan-proof', 'delivery-semantics');
const testRoot = join('test-results', 'eventing-delivery-semantics-proof');
mkdirSync(proofRoot, { recursive: true });
mkdirSync(testRoot, { recursive: true });

const commands = [
  {
    name: 'eventing-delivery-tests',
    command: 'cargo',
    args: ['test', '-p', 'ocentra-eventing', 'delivery_decision'],
  },
  {
    name: 'eventing-clippy',
    command: 'cargo',
    args: ['clippy', '-p', 'ocentra-eventing', '--all-targets', '--', '-D', 'warnings'],
  },
  {
    name: 'source-shape',
    command: 'node',
    args: ['scripts/check-source-shape.mjs', 'crates/ocentra-eventing'],
  },
  {
    name: 'git-diff-check',
    command: 'git',
    args: ['diff', '--check', '--', '.', ':(exclude)output', ':(exclude)test-results'],
  },
];

const commandResults = commands.map(runCommand);

const deliverySource = readFileSync('crates/ocentra-eventing/src/delivery.rs', 'utf8');
const deliveryTests = readFileSync('crates/ocentra-eventing/src/tests/delivery.rs', 'utf8');

const sourceAssertions = [
  ['delivery-proof-struct', deliverySource.includes('pub struct EventDeliveryDecisionProof')],
  ['local-route-ready-state', deliverySource.includes('LocalRouteReady')],
  ['external-transport-manual-required-state', deliverySource.includes('ExternalTransportRouteManualRequired')],
  ['external-relay-manual-required-state', deliverySource.includes('ExternalRelayRouteManualRequired')],
  ['live-external-transport-claim-rejected', deliverySource.includes('LiveExternalTransportDeliveryClaimRejected')],
  ['live-external-relay-claim-rejected', deliverySource.includes('LiveExternalRelayDeliveryClaimRejected')],
  ['decision-authority-claim-rejected', deliverySource.includes('DecisionAuthorityClaimRejected')],
  ['side-effect-authority-claim-rejected', deliverySource.includes('SideEffectAuthorityClaimRejected')],
  [
    'local-route-test',
    deliveryTests.includes('delivery_decision_allows_local_first_route_with_filter_and_backpressure'),
  ],
  [
    'external-transport-manual-required-test',
    deliveryTests.includes('delivery_decision_marks_external_transport_manual_required_without_required_artifacts'),
  ],
  [
    'external-relay-manual-required-test',
    deliveryTests.includes('delivery_decision_marks_external_relay_manual_required_for_relay_artifacts'),
  ],
  ['claim-rejection-test', deliveryTests.includes('delivery_decision_rejects_live_claims_and_invalid_route_metadata')],
];

const failedAssertion = sourceAssertions.find(([, passed]) => !passed);
if (failedAssertion) {
  throw new Error(`eventing delivery semantics assertion failed: ${failedAssertion[0]}`);
}

const proof = {
  proof: 'eventing-delivery-semantics',
  proofRoot,
  checkedAt: new Date().toISOString(),
  branch: runText('git', ['branch', '--show-current']).trim(),
  commit: runText('git', ['rev-parse', 'HEAD']).trim(),
  statusShort: runText('git', ['status', '--short']),
  commands: commandResults,
  assertions: sourceAssertions.map(([name]) => name),
  provenBehavior: [
    'local in-process and local-service routes can be marked ready with subscriber filtering and bounded backpressure metadata',
    'external transport and external relay routes enumerate required custody, auth, encryption, retention, replay, deletion, offset, dedupe, transport, and relay artifacts before they can be claimed',
    'live external transport, external relay, decision-authority, and side-effect-authority claims are rejected by the reusable eventing proof helper',
  ],
  provenRows: ['generic delivery decision support in the reusable eventing crate'],
  notClaimed: [
    'live external transport delivery',
    'external relay delivery',
    'cross-process transport implementation',
    'network decision, side-effect, or enforcement authority',
  ],
};

writeFileSync(join(proofRoot, 'proof-summary.json'), `${JSON.stringify(proof, null, 2)}\n`);
writeFileSync(join(testRoot, 'proof.json'), `${JSON.stringify(proof, null, 2)}\n`);
console.log(`eventing-delivery-semantics-proof-ok:${proof.assertions.join(',')}`);
console.log(`proof=${join(proofRoot, 'proof-summary.json')}`);

function runCommand(entry) {
  const result = spawnSync(entry.command, entry.args, { encoding: 'utf8', shell: false });
  const log = join(proofRoot, `${entry.name}.log`);
  writeFileSync(log, `${result.stdout ?? ''}${result.stderr ?? ''}`);
  if (result.status !== 0) {
    throw new Error(`${entry.name} failed with exit ${result.status}; log=${log}`);
  }
  return {
    name: entry.name,
    command: [entry.command, ...entry.args].join(' '),
    status: result.status,
    log,
  };
}

function runText(command, args) {
  const result = spawnSync(command, args, { encoding: 'utf8', shell: false });
  if (result.status !== 0) {
    throw new Error(`${command} ${args.join(' ')} failed with exit ${result.status}`);
  }
  return `${result.stdout ?? ''}${result.stderr ?? ''}`;
}
