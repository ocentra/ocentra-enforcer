import { spawnSync } from 'node:child_process';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const proofRoot = join('output', 'eventing-plan-proof', 'full-eventing-plan');
const testRoot = join('test-results', 'eventing-full-plan-proof');
const logRoot = join(proofRoot, 'command-logs');
mkdirSync(proofRoot, { recursive: true });
mkdirSync(testRoot, { recursive: true });
mkdirSync(logRoot, { recursive: true });

const proofScripts = [
  'eventing-branded-fixture-parity-proof.mjs',
  'eventing-command-boundary-proof.mjs',
  'eventing-compatibility-matrix-proof.mjs',
  'eventing-contract-registry-proof.mjs',
  'eventing-delivery-semantics-proof.mjs',
  'eventing-duplicate-subscriber-proof.mjs',
  'eventing-enforcement-journal-action-proof.mjs',
  'eventing-family-variant-proof.mjs',
  'eventing-handler-policy-proof.mjs',
  'eventing-household-mesh-consumer-proof.mjs',
  'eventing-journal-replay-proof.mjs',
  'eventing-lifecycle-clear-proof.mjs',
  'eventing-lock-await-proof.mjs',
  'eventing-manual-clock-proof.mjs',
  'eventing-network-protocol-contract-proof.mjs',
  'eventing-network-backpressure-proof.mjs',
  'eventing-network-delivery-decision-proof.mjs',
  'eventing-network-runtime-proof.mjs',
  'eventing-network-service-event-chain-stream-proof.mjs',
  'eventing-network-service-runtime-delivery-proof.mjs',
  'eventing-network-ts-event-parity-proof.mjs',
  'eventing-parent-child-protocol-contract-proof.mjs',
  'eventing-parent-child-runtime-proof.mjs',
  'eventing-production-shutdown-proof.mjs',
  'eventing-queue-policy-proof.mjs',
  'eventing-request-response-proof.mjs',
  'eventing-runtime-proof.mjs',
  'eventing-runtime-lifecycle-proof.mjs',
  'eventing-metrics-testkit-proof.mjs',
  'eventing-source-safety-proof.mjs',
  'eventing-topology-manifest-proof.mjs',
  'eventing-type-safety-source-gate-proof.mjs',
  'eventing-ui-typed-intent-boundary-proof.mjs',
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

const sideEffectSnapshot = snapshotTrackedProofSideEffects();
let scriptResults = [];
try {
  scriptResults = proofScripts.map((scriptName) =>
    runCommand({
      name: scriptName.replace(/\.mjs$/, ''),
      command: 'node',
      args: [join('scripts', 'test', scriptName)],
    })
  );
} finally {
  restoreTrackedProofSideEffects(sideEffectSnapshot);
}
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
  'eventing-parent-child-protocol-contract-proof',
  'eventing-network-protocol-contract-proof',
  'eventing-network-ts-event-parity-proof',
  'ocentra-eventing-tests',
]);
writeGroupedLog('02-dispatch-proof.log', commands, [
  'eventing-runtime-lifecycle-proof',
  'eventing-delivery-semantics-proof',
  'eventing-handler-policy-proof',
  'eventing-duplicate-subscriber-proof',
  'eventing-family-variant-proof',
  'ocentra-eventing-tests',
]);
writeGroupedLog('03-queue-retry-timeout-proof.log', commands, [
  'eventing-queue-policy-proof',
  'eventing-network-backpressure-proof',
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
writeGroupedLog('05-journal-replay-proof.log', commands, [
  'eventing-journal-replay-proof',
  'eventing-enforcement-journal-action-proof',
  'ocentra-eventing-tests',
]);
writeGroupedLog('06-parent-runtime-boundary-proof.log', commands, [
  'eventing-parent-child-runtime-proof',
  'eventing-network-runtime-proof',
  'eventing-network-service-runtime-delivery-proof',
  'eventing-network-service-event-chain-stream-proof',
  'eventing-enforcement-journal-action-proof',
]);
writeGroupedLog('07-ui-boundary-proof.log', commands, [
  'eventing-ui-typed-intent-boundary-proof',
  'eventing-command-boundary-proof',
]);
writeGroupedLog('08-security-negative-proof.log', commands, [
  'eventing-command-boundary-proof',
  'eventing-network-delivery-decision-proof',
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
    'N/A for the reusable eventing crate proof pack.',
    '',
    'The eventing plan establishes local typed runtime/event bus behavior and protocol/runtime boundaries.',
    'It does not claim broker delivery, relay-hub delivery, platform adapter execution, host filtering, or device OS support.',
    '',
  ].join('\n')
);
writeGroupedLog(
  '10-validation-commands.log',
  commands,
  commands.map((entry) => entry.name)
);
writeGroupedLog('11-network-consumer-proof.log', commands, [
  'eventing-network-protocol-contract-proof',
  'eventing-network-ts-event-parity-proof',
  'eventing-network-runtime-proof',
  'eventing-network-backpressure-proof',
  'eventing-network-service-runtime-delivery-proof',
  'eventing-network-service-event-chain-stream-proof',
  'eventing-network-delivery-decision-proof',
]);
writeGroupedLog('12-household-mesh-consumer-proof.log', commands, ['eventing-household-mesh-consumer-proof']);

const proof = {
  schemaVersion: 1,
  proof: 'eventing-full-plan',
  proofRoot,
  testRoot,
  runContext:
    'This committed proof artifact is deterministic; branch, commit, pushed state, and validation command output are reported in the worker handoff.',
  commands,
  requiredProofPack: [
    '00-source-snapshot.md',
    '01-contract-proof.log',
    '02-dispatch-proof.log',
    '03-queue-retry-timeout-proof.log',
    '04-request-response-proof.log',
    '05-journal-replay-proof.log',
    '06-parent-runtime-boundary-proof.log',
    '07-ui-boundary-proof.log',
    '08-security-negative-proof.log',
    '09-manual-platform-proof.md',
    '10-validation-commands.log',
    '11-network-consumer-proof.log',
    '12-household-mesh-consumer-proof.log',
  ].map((name) => join(proofRoot, name)),
  provenRows: [
    '05-41 reusable eventing crate runtime rows',
    '42-62 parent/controller, child-agent, network, UI, enforcement, and command-boundary consumer rows',
    '63-78 reusable eventing type-safety, compatibility, lifecycle, topology, delivery, and source-safety rows',
    '12-household-mesh-consumer proof-pack row for Household Mesh consumer bridge boundary',
  ],
  networkConsumerProof: {
    proofLog: join(proofRoot, '11-network-consumer-proof.log'),
    proves:
      'network consumes ocentra-eventing for typed publish/routing, queue/drain, request-response, service read-model delivery, service event-chain streaming, TypeScript parity, and broker/relay-hub manual-required delivery decisions without adding network business logic to crates/ocentra-eventing',
  },
  householdMeshConsumerProof: {
    proofLog: join(proofRoot, '12-household-mesh-consumer-proof.log'),
    proves:
      'Household Mesh consumer bridge exports only selected local events into typed authenticated LAN messages, validates incoming messages before local republish, rejects direct remote publish into another runtime bus, rejects raw payload transfer and provider/parent policy-authority escalation, and preserves child-agent-only AI policy authority without adding LAN, AI, policy, or enforcement behavior to crates/ocentra-eventing',
  },
  notClaimed: [
    'broker-backed delivery',
    'relay-hub delivery',
    'shared LAN-wide event bus',
    'remote direct publish into another runtime local bus',
    'provider-owned policy authority',
    'raw screenshot or capture payload transfer by default',
    'physical household provider execution',
    'platform adapter execution',
    'host DNS/filter enforcement',
    'portal-owned business event publishing',
  ],
};

writeFileSync(join(proofRoot, 'proof-summary.json'), `${JSON.stringify(proof, null, 2)}\n`);
writeFileSync(join(testRoot, 'proof.json'), `${JSON.stringify(proof, null, 2)}\n`);
console.log('eventing-full-plan-proof-ok:all-eventing-harnesses');
console.log(`proof=${join(proofRoot, 'proof-summary.json')}`);

function runCommand(entry) {
  const result = spawnSync(entry.command, entry.args, { encoding: 'utf8', shell: false });
  const safeName = entry.name.replace(/[^a-zA-Z0-9_.-]/g, '-');
  const log = join(logRoot, `${safeName}.log`);
  writeFileSync(log, normalizeCommandOutput(`${result.stdout ?? ''}${result.stderr ?? ''}`));
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

function snapshotTrackedProofSideEffects() {
  const result = spawnSync(
    'git',
    ['ls-files', 'output/eventing-plan-proof', 'output/network-plan-proof', 'test-results'],
    {
      encoding: 'utf8',
      shell: false,
    }
  );
  if (result.status !== 0) {
    throw new Error(`git ls-files failed with exit ${result.status}`);
  }
  return result.stdout
    .split(/\r?\n/u)
    .filter(Boolean)
    .filter((path) => !isAggregateProofPath(path))
    .map((path) => [path, readFileSync(path)]);
}

function restoreTrackedProofSideEffects(snapshot) {
  for (const [path, contents] of snapshot) {
    writeFileWithRetry(path, contents);
  }
}

function writeFileWithRetry(path, contents) {
  let lastError = null;
  for (let attempt = 0; attempt < 8; attempt += 1) {
    try {
      writeFileSync(path, contents);
      return;
    } catch (error) {
      lastError = error;
      if (!isTransientWriteError(error)) {
        throw error;
      }
      sleepSync(75 * (attempt + 1));
    }
  }
  throw lastError;
}

function isTransientWriteError(error) {
  return ['UNKNOWN', 'EBUSY', 'EPERM', 'EACCES'].includes(error?.code);
}

function sleepSync(milliseconds) {
  Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, milliseconds);
}

function isAggregateProofPath(path) {
  const normalized = path.replace(/\\/gu, '/');
  const normalizedProofRoot = proofRoot.replace(/\\/gu, '/');
  const normalizedTestRoot = testRoot.replace(/\\/gu, '/');
  return normalized.startsWith(`${normalizedProofRoot}/`) || normalized.startsWith(`${normalizedTestRoot}/`);
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
    '# Eventing Full Plan Source Snapshot',
    '',
    'Deterministic full-eventing-plan proof for reusable eventing and approved consumer-boundary evidence.',
    '',
    'Run-specific branch, commit, pushed state, and validation command output are reported in the worker handoff; this committed artifact is kept deterministic so rerunning the proof does not dirty the checkout.',
    '',
    '## Inspected Paths',
    '',
    '- crates/ocentra-eventing',
    '- crates/agent-protocol',
    '- crates/agent-core',
    '- crates/agent-service',
    '- apps/portal/src/transport.ts',
    '- docs/plans/eventing-plan',
    '- output/eventing-plan-proof',
    '',
    '## Before-State Gap',
    '',
    'The row-level eventing proof artifacts existed, but the eventing checklist did not have one consolidated proof pack tying the source snapshot, grouped logs, manual platform non-claims, and validation commands together for PR-ready handoff.',
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

function normalizeCommandOutput(value) {
  const lines = value
    .replace(/\r\n/gu, '\n')
    .replace(/\\/gu, '/')
    .replace(/target\/debug\/deps\/[^\s)]+/gu, 'target/debug/deps/<test-binary>')
    .replace(/\b\d+\.\d+s\b/gu, '<duration>s')
    .replace(/\b\d+\.\d{2}ms\b/gu, '<duration>ms')
    .replace(/target\(s\) in [^\n]+/gu, 'target(s) in <duration>')
    .replace(/finished in [^\n]+/giu, 'finished in <duration>')
    .replace(/Duration [^\n]+/gu, 'Duration <duration>')
    .replace(/Start at\s+[0-9:]+/gu, 'Start at <time>')
    .replace(
      /file has \d+ lines; crossed \d+-line advisory band; maximum is \d+/gu,
      'file has <lines> lines; crossed <band>-line advisory band; maximum is <max>'
    )
    .replace(
      /function has \d+ lines; warning starts at \d+ of \d+/gu,
      'function has <lines> lines; warning starts at <warn> of <max>'
    )
    .replace(
      /file has \d+ functions; warning starts at \d+ of \d+/gu,
      'file has <functions> functions; warning starts at <warn> of <max>'
    )
    .replace(
      /file has \d+ structs\/enums; warning starts at \d+ of \d+/gu,
      'file has <structs-enums> structs/enums; warning starts at <warn> of <max>'
    )
    .split('\n')
    .filter((line) => !/^\s+Compiling /u.test(line))
    .filter((line) => !/^\s+Blocking waiting for file lock on build directory$/u.test(line));
  return `${stableRustTestLines(lines).join('\n').trim()}\n`;
}

function stableRustTestLines(lines) {
  const sortedTestLines = lines.filter(isRustTestLine).sort();
  let nextTestLine = 0;
  return lines.map((line) => {
    if (!isRustTestLine(line)) {
      return line;
    }
    const sortedLine = sortedTestLines[nextTestLine];
    nextTestLine += 1;
    return sortedLine;
  });
}

function isRustTestLine(line) {
  return /^test .+ \.\.\. ok$/u.test(line);
}
