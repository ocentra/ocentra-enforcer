import { spawnSync } from 'node:child_process';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const proofRoot = join('output', 'eventing-plan-proof', '72-contract-registry');
mkdirSync(proofRoot, { recursive: true });

const commands = [
  {
    name: 'contract-registry-tests',
    command: 'cargo',
    args: ['test', '-p', 'ocentra-eventing', 'contract_registry'],
  },
  {
    name: 'contract-registry-docs-example',
    command: 'cargo',
    args: ['run', '-p', 'ocentra-eventing', '--example', 'contract_registry_docs'],
    captureMarkdown: true,
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
];

const commandResults = commands.map((entry) => {
  const result = spawnSync(entry.command, entry.args, {
    encoding: 'utf8',
    shell: false,
  });
  const output = `${result.stdout ?? ''}${result.stderr ?? ''}`;
  writeFileSync(join(proofRoot, `${entry.name}.log`), output);
  if (entry.captureMarkdown) {
    writeFileSync(join(proofRoot, 'event-contract-registry.generated.md'), result.stdout ?? '');
  }
  if (result.status !== 0) {
    throw new Error(`${entry.name} failed with exit ${result.status}`);
  }
  return {
    name: entry.name,
    command: [entry.command, ...entry.args].join(' '),
    status: result.status,
    log: join(proofRoot, `${entry.name}.log`),
  };
});

const registrySource = readFileSync('crates/ocentra-eventing/src/contract_registry.rs', 'utf8');
const errorSource = readFileSync('crates/ocentra-eventing/src/error.rs', 'utf8');
const registryTests = readFileSync('crates/ocentra-eventing/src/tests/contract_registry.rs', 'utf8');
const generatedDocs = readFileSync(join(proofRoot, 'event-contract-registry.generated.md'), 'utf8');

const assertions = [
  ['registry-type-exists', registrySource.includes('pub struct EventContractRegistry')],
  ['descriptor-type-exists', registrySource.includes('pub struct EventContractDescriptor')],
  ['registry-uses-sorted-map', registrySource.includes('BTreeMap<EventType, EventContractDescriptor>')],
  ['registry-rejects-duplicate-contracts', registrySource.includes('DuplicateEventContract')],
  ['duplicate-error-is-explicit', errorSource.includes('DuplicateEventContract')],
  ['markdown-renderer-exists', registrySource.includes('pub fn render_markdown')],
  [
    'markdown-docs-example-generated',
    generatedDocs.includes('# Event Contract Registry') &&
      generatedDocs.includes('| Event Type | Schema Version | Rust Type |') &&
      generatedDocs.includes('eventing.dead_letter.recorded'),
  ],
  [
    'tests-prove-order-and-duplicates',
    registryTests.includes('contract_registry_generates_markdown_in_event_type_order') &&
      registryTests.includes('contract_registry_rejects_duplicate_event_type'),
  ],
];

const failed = assertions.find(([, passed]) => !passed);
if (failed) {
  throw new Error(`eventing contract-registry assertion failed: ${failed[0]}`);
}

const proof = {
  proof: 'eventing-contract-registry',
  proofRoot,
  checkedAt: new Date().toISOString(),
  commands: commandResults,
  assertions: assertions.map(([name]) => name),
  generatedDocs: join(proofRoot, 'event-contract-registry.generated.md'),
  provenRows: ['72 Event contract registry and generated documentation'],
  policy:
    'EventContractRegistry stores typed event descriptors sorted by EventType, rejects duplicate EventType registrations, and renders deterministic Markdown through the public API.',
  notClaimed: [
    '70 event topology manifest',
    '75 event-family wrapper proof',
    'Parent-specific event contracts',
    'network broker-backed event transport',
  ],
};

writeFileSync(join(proofRoot, 'proof-summary.json'), `${JSON.stringify(proof, null, 2)}\n`);
console.log(`eventing-contract-registry-proof-ok:${proof.assertions.join(',')}`);
console.log(`proof=${join(proofRoot, 'proof-summary.json')}`);
