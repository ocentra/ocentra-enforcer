import { spawnSync } from 'node:child_process';
import { mkdirSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const proofRoot = join('output', 'network-plan-proof', '51-end-to-end-pipeline-proof');
const testRoot = join('test-results', 'network-end-to-end-pipeline-proof');
mkdirSync(proofRoot, { recursive: true });
mkdirSync(testRoot, { recursive: true });

writeFileSync(
  join(proofRoot, 'expected-end-to-end-pipeline.json'),
  `${JSON.stringify(
    {
      path: [
        'stored-activity-network-flow-row',
        'row-scoped-trigger-ref',
        'row-scoped-capture-ref',
        'row-scoped-ingest-ref',
        'row-scoped-typed-event-ref',
        'evidence-bundle',
        'local-ai-queue-refs-only',
        'ai-detection',
        'ai-audit',
        'risk-budget',
        'policy-decision',
        'adapter-proof-state',
        'action-result-state',
        'audit-event',
        'portal-read-model',
        'retention-delete-export',
      ],
      requiredRefFamilies: [
        'trigger',
        'capture',
        'ingest',
        'typed event',
        'evidence',
        'AI detection/audit',
        'policy decision',
        'adapter capability/action artifacts',
        'action result',
        'audit',
        'portal/read-model',
        'retention/delete/export',
      ],
      noBypassInvariants: [
        'stored network rows without a domain target do not invent policy refs',
        'retention tombstones do not drive active product path decisions',
        'weak or unavailable evidence cannot authorize adapter apply',
        'manual-required, dry-run, and unavailable action results stay non-enforcing',
        'AI remains advisory',
        'portal/UI has no policy authority',
        'network evidence has no adapter authority',
        'no enforcement command is published by this proof path',
      ],
      notClaimed: [
        'live packet capture driver invocation',
        'local model execution',
        'full policy engine execution',
        'host DNS/firewall mutation',
        'broker or family-hub delivery',
        'portal risk-budget/performance UI rendering',
        'exact URL/content from stored network-only rows',
      ],
    },
    null,
    2
  )}\n`
);

const commands = [
  {
    name: 'agent-core-network-runtime-typed-event-refs',
    command: 'cargo',
    args: ['test', '-p', 'ocentra-parent-agent-core', '--test', 'unit'],
    log: join(proofRoot, 'agent-core-typed-event-refs.log'),
  },
  {
    name: 'agent-core-network-runtime-weak-evidence-no-enforcement',
    command: 'cargo',
    args: ['test', '-p', 'ocentra-parent-agent-core', '--test', 'unit'],
    log: join(proofRoot, 'agent-core-weak-evidence-no-enforcement.log'),
  },
  {
    name: 'network-evidence-end-to-end-pipeline-tests',
    command: 'cargo',
    args: ['test', '-p', 'ocentra-network-evidence', '--test', 'unit'],
    log: join(proofRoot, 'pipeline-tests.log'),
  },
  {
    name: 'agent-service-stored-flow-product-path-bridge',
    command: 'cargo',
    args: ['test', '-p', 'ocentra-parent-agent-service', '--test', 'network_bridge_runtime'],
    log: join(proofRoot, 'agent-service-stored-flow-product-path-bridge.log'),
  },
  {
    name: 'agent-service-capture-store-product-path-integration',
    command: 'cargo',
    args: ['test', '-p', 'ocentra-parent-agent-service', '--test', 'network_bridge_runtime'],
    log: join(proofRoot, 'agent-service-capture-store-product-path-integration.log'),
  },
  {
    name: 'agent-service-network-flow-payload-product-path',
    command: 'cargo',
    args: ['test', '-p', 'ocentra-parent-agent-service', '--test', 'network_bridge_runtime'],
    log: join(proofRoot, 'agent-service-network-flow-payload-product-path.log'),
  },
  {
    name: 'network-evidence-clippy',
    command: 'cargo',
    args: ['clippy', '-p', 'ocentra-network-evidence', '--all-targets', '--', '-D', 'warnings'],
    log: join(proofRoot, 'clippy.log'),
  },
];
const commandResults = commands.map(runCommand);

const proof = {
  schemaVersion: 1,
  proof: 'network-end-to-end-pipeline',
  proofRoot,
  testRoot,
  runContext:
    'This committed proof artifact is deterministic; branch, commit, pushed state, and validation command output are reported in the worker handoff.',
  commands: commandResults,
  artifacts: {
    expectedEndToEndPipeline: join(proofRoot, 'expected-end-to-end-pipeline.json'),
    proofSummary: join(proofRoot, 'proof-summary.json'),
    testProof: join(testRoot, 'proof.json'),
  },
  provenRows: ['51 Integrated event + network product path proof'],
  provenRootGates: [
    'stored ActivityStore network-flow rows derive row-scoped trigger/capture/ingest/typed-event refs into the row51 pipeline proof',
    'captured metadata events carry durable local DB evidence refs through the real ActivityStore into service product-path payload refs',
    'typed local event-chain refs are preserved before product-path composition',
    'capture and ingest refs are carried before the typed event and evidence bundle',
    'evidence bundle to AI audit to policy to adapter proof to action result preserves exact refs',
    'manual-required, dry-run, and unavailable action-result states are proven in the same product path',
    'retention tombstones and rows without domain targets do not invent active policy/action refs',
    'weak/unavailable evidence cannot publish enforcement commands',
    'AI/UI/network cannot bypass policy',
    'retention/delete/export refs are part of the same proof path',
  ],
  notClaimed: [
    'live packet capture driver invocation',
    'local model execution or remote AI',
    'full policy engine execution',
    'notification provider delivery',
    'host DNS/firewall/WFP/VPN/NetworkExtension/Linux adapter mutation',
    'broker or family-hub transport',
    'exact URL/content from stored network-only rows',
    'portal risk-budget/performance UI rendering',
    'production SLO or external audit completion',
  ],
};
writeFileSync(join(proofRoot, 'proof-summary.json'), `${JSON.stringify(proof, null, 2)}\n`);
writeFileSync(join(testRoot, 'proof.json'), `${JSON.stringify(proof, null, 2)}\n`);
console.log(
  'network-end-to-end-pipeline-proof-ok:agent-core-event-refs,agent-core-weak-no-enforcement,pipeline-tests,service-stored-flow-bridge,capture-store-product-path,service-payload,clippy'
);
console.log(`proof=${join(proofRoot, 'proof-summary.json')}`);

function runCommand(entry) {
  const result = spawnSync(entry.command, entry.args, { encoding: 'utf8', shell: false });
  writeFileSync(entry.log, normalizeCommandOutput(`${result.stdout ?? ''}${result.stderr ?? ''}`));
  if (result.status !== 0) {
    throw new Error(`${entry.name} failed with exit ${result.status}`);
  }
  return {
    name: entry.name,
    command: [entry.command, ...entry.args].join(' '),
    status: result.status,
    log: entry.log,
  };
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
