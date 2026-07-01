import { spawnSync } from 'node:child_process';
import { mkdirSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const proofRoot = join('output', 'eventing-plan-proof', 'reusable-eventing-runtime');
const testRoot = join('test-results', 'eventing-runtime-proof');
const logRoot = join(proofRoot, 'command-logs');
mkdirSync(proofRoot, { recursive: true });
mkdirSync(testRoot, { recursive: true });
mkdirSync(logRoot, { recursive: true });

const sourceBranch = runText('git', ['branch', '--show-current']).trim();
const sourceCommit = runText('git', ['rev-parse', 'HEAD']).trim();
const sourceOriginMain = runText('git', ['rev-parse', 'origin/main']).trim();
const sourceMergeBase = runText('git', ['merge-base', 'HEAD', 'origin/main']).trim();
const sourceStatusShort = runText('git', ['status', '--short']);

const proofScripts = [
  'eventing-branded-fixture-parity-proof.mjs',
  'eventing-compatibility-matrix-proof.mjs',
  'eventing-contract-registry-proof.mjs',
  'eventing-delivery-semantics-proof.mjs',
  'eventing-duplicate-subscriber-proof.mjs',
  'eventing-family-variant-proof.mjs',
  'eventing-handler-policy-proof.mjs',
  'eventing-journal-replay-proof.mjs',
  'eventing-lifecycle-clear-proof.mjs',
  'eventing-lock-await-proof.mjs',
  'eventing-manual-clock-proof.mjs',
  'eventing-metrics-testkit-proof.mjs',
  'eventing-production-shutdown-proof.mjs',
  'eventing-queue-policy-proof.mjs',
  'eventing-request-response-proof.mjs',
  'eventing-runtime-lifecycle-proof.mjs',
  'eventing-source-safety-proof.mjs',
  'eventing-topology-manifest-proof.mjs',
  'eventing-type-safety-source-gate-proof.mjs',
];

const directCommands = [
  {
    name: 'ocentra-eventing-tests',
    command: 'cargo',
    args: ['test', '-p', 'ocentra-eventing'],
  },
  {
    name: 'ocentra-eventing-clippy',
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

const scriptResults = proofScripts.map((scriptName) =>
  runCommand({
    name: scriptName.replace(/\.mjs$/, ''),
    command: 'node',
    args: [join('scripts', 'test', scriptName)],
  })
);
const directResults = directCommands.map(runCommand);
const commands = [...scriptResults, ...directResults];

writeFileSync(join(proofRoot, '00-source-snapshot.md'), sourceSnapshot());
writeGroupedLog('01-contract-proof.log', commands, [
  'eventing-type-safety-source-gate-proof',
  'eventing-contract-registry-proof',
  'eventing-family-variant-proof',
  'eventing-topology-manifest-proof',
  'eventing-compatibility-matrix-proof',
  'eventing-branded-fixture-parity-proof',
  'ocentra-eventing-tests',
]);
writeGroupedLog('02-dispatch-proof.log', commands, [
  'eventing-runtime-lifecycle-proof',
  'eventing-duplicate-subscriber-proof',
  'eventing-family-variant-proof',
  'ocentra-eventing-tests',
]);
writeGroupedLog('03-queue-retry-timeout-proof.log', commands, [
  'eventing-queue-policy-proof',
  'eventing-handler-policy-proof',
  'eventing-manual-clock-proof',
  'eventing-production-shutdown-proof',
  'ocentra-eventing-tests',
]);
writeGroupedLog('04-request-response-proof.log', commands, [
  'eventing-request-response-proof',
  'eventing-manual-clock-proof',
  'ocentra-eventing-tests',
]);
writeGroupedLog('05-journal-replay-proof.log', commands, ['eventing-journal-replay-proof', 'ocentra-eventing-tests']);
writeGroupedLog('06-delivery-semantics-proof.log', commands, [
  'eventing-delivery-semantics-proof',
  'ocentra-eventing-tests',
]);
writeGroupedLog('07-metrics-testkit-proof.log', commands, [
  'eventing-handler-policy-proof',
  'eventing-metrics-testkit-proof',
  'ocentra-eventing-tests',
]);
writeGroupedLog('08-security-negative-proof.log', commands, [
  'eventing-source-safety-proof',
  'eventing-lock-await-proof',
  'eventing-queue-policy-proof',
  'eventing-production-shutdown-proof',
  'eventing-type-safety-source-gate-proof',
]);
writeFileSync(
  join(proofRoot, '09-manual-platform-proof.md'),
  [
    '# Manual Platform Proof',
    '',
    'N/A for the reusable eventing runtime phase.',
    '',
    'This phase proves the generic Rust event bus and local runtime primitives only.',
    'It does not claim network runtime adoption, broker delivery, relay-hub delivery, parent/child transport, portal publishing, platform adapter execution, host filtering, or production OS/device support.',
    '',
  ].join('\n')
);
writeGroupedLog(
  '10-validation-commands.log',
  commands,
  commands.map((entry) => entry.name)
);

const proof = {
  proof: 'eventing-runtime-phase-1',
  checkedAt: new Date().toISOString(),
  branch: sourceBranch,
  commit: sourceCommit,
  originMain: sourceOriginMain,
  mergeBase: sourceMergeBase,
  statusShort: sourceStatusShort,
  proofRoot,
  testRoot,
  commands,
  requiredProofPack: [
    '00-source-snapshot.md',
    '01-contract-proof.log',
    '02-dispatch-proof.log',
    '03-queue-retry-timeout-proof.log',
    '04-request-response-proof.log',
    '05-journal-replay-proof.log',
    '06-delivery-semantics-proof.log',
    '07-metrics-testkit-proof.log',
    '08-security-negative-proof.log',
    '09-manual-platform-proof.md',
    '10-validation-commands.log',
  ].map((name) => join(proofRoot, name)),
  provenRows: [
    '05-41 reusable eventing crate runtime rows',
    '63-78 reusable eventing type-safety, compatibility, lifecycle, topology, and source-safety rows',
  ],
  notClaimed: [
    'parent/controller runtime publisher integration',
    'child-agent runtime integration',
    'network runtime adoption',
    'network to AI to policy to enforcement event-chain execution',
    'broker-backed delivery',
    'relay-hub delivery',
    'platform adapter execution',
    'host DNS/filter enforcement',
    'portal-owned business event publishing',
  ],
};

writeFileSync(join(proofRoot, 'proof-summary.json'), `${JSON.stringify(proof, null, 2)}\n`);
writeFileSync(join(testRoot, 'proof.json'), `${JSON.stringify(proof, null, 2)}\n`);
console.log('eventing-runtime-proof-ok:phase-1-reusable-event-bus');
console.log(`proof=${join(proofRoot, 'proof-summary.json')}`);

function runCommand(entry) {
  const result = spawnSync(entry.command, entry.args, { encoding: 'utf8', shell: false });
  const safeName = entry.name.replace(/[^a-zA-Z0-9_.-]/g, '-');
  const log = join(logRoot, `${safeName}.log`);
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

function writeGroupedLog(filename, commands, names) {
  const selected = commands.filter((entry) => names.includes(entry.name));
  const body = selected
    .map((entry) => [`command=${entry.command}`, `status=${entry.status}`, `log=${entry.log}`].join('\n'))
    .join('\n\n');
  writeFileSync(join(proofRoot, filename), `${body}\n`);
}

function sourceSnapshot() {
  return [
    '# Reusable Eventing Runtime Source Snapshot',
    '',
    `branch: ${sourceBranch}`,
    `head: ${sourceCommit}`,
    `origin/main: ${sourceOriginMain}`,
    `merge-base: ${sourceMergeBase}`,
    '',
    '## Status',
    '',
    '```text',
    sourceStatusShort.trimEnd(),
    '```',
    '',
    '## Inspected Paths',
    '',
    '- crates/ocentra-eventing',
    '- docs/plans/eventing-plan',
    '- output/eventing-plan-proof',
    '',
    '## Phase Boundary',
    '',
    'This proof pack validates the reusable Rust event bus/runtime only.',
    'Network, parent/child runtime, portal, service, broker, relay-hub, policy, AI, enforcement, and platform-adapter proofs are consumer phases layered on top of this crate.',
    '',
  ].join('\n');
}

function runText(command, args) {
  const result = spawnSync(command, args, { encoding: 'utf8', shell: false });
  if (result.status !== 0) {
    throw new Error(`${command} ${args.join(' ')} failed with exit ${result.status}`);
  }
  return `${result.stdout ?? ''}${result.stderr ?? ''}`;
}
