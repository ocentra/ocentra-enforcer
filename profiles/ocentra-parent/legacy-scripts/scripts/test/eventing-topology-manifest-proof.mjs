import { spawnSync } from 'node:child_process';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const proofRoot = join('output', 'eventing-plan-proof', '70-topology-manifest');
mkdirSync(proofRoot, { recursive: true });

const commands = [
  {
    name: 'topology-manifest-tests',
    command: 'cargo',
    args: ['test', '-p', 'ocentra-eventing', 'topology_manifest'],
  },
  {
    name: 'topology-manifest-example',
    command: 'cargo',
    args: ['run', '-p', 'ocentra-eventing', '--example', 'topology_manifest'],
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
    writeFileSync(join(proofRoot, 'event-topology-manifest.generated.md'), result.stdout ?? '');
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

const topologySource = readFileSync('crates/ocentra-eventing/src/topology.rs', 'utf8');
const topologyTests = readFileSync('crates/ocentra-eventing/src/tests/topology_manifest.rs', 'utf8');
const generatedManifest = readFileSync(join(proofRoot, 'event-topology-manifest.generated.md'), 'utf8');

const assertions = [
  ['manifest-type-exists', topologySource.includes('pub struct EventTopologyManifest')],
  ['status-classification-exists', topologySource.includes('pub enum EventTopologyStatus')],
  ['publisher-descriptor-exists', topologySource.includes('pub struct EventTopologyPublisher')],
  ['subscriber-descriptor-exists', topologySource.includes('pub struct EventTopologySubscriber')],
  ['family-variant-descriptor-exists', topologySource.includes('pub struct EventTopologyFamilyVariant')],
  [
    'tests-cover-orphan-and-accepted-states',
    topologyTests.includes('topology_manifest_classifies_covered_orphan_and_accepted_states') &&
      topologyTests.includes('EventTopologyStatus::NoSubscriber') &&
      topologyTests.includes('EventTopologyStatus::AcceptedOneSided'),
  ],
  [
    'tests-cover-family-variants',
    topologyTests.includes('topology_manifest_records_family_variants_and_sorted_descriptors'),
  ],
  [
    'generated-markdown-manifest-exists',
    generatedManifest.includes('# Event Topology Manifest') &&
      generatedManifest.includes(
        '| Event Type | Schema Version | Publishers | Subscribers | Families | Status | Rust Type |'
      ) &&
      generatedManifest.includes('covered'),
  ],
];

const failed = assertions.find(([, passed]) => !passed);
if (failed) {
  throw new Error(`eventing topology-manifest assertion failed: ${failed[0]}`);
}

const proof = {
  proof: 'eventing-topology-manifest',
  proofRoot,
  checkedAt: new Date().toISOString(),
  commands: commandResults,
  assertions: assertions.map(([name]) => name),
  generatedManifest: join(proofRoot, 'event-topology-manifest.generated.md'),
  provenRows: ['70 Event topology manifest and orphan publisher/subscriber audit'],
  policy:
    'EventTopologyManifest classifies registered event contracts as covered, no-publisher, no-subscriber, or accepted-one-sided from explicit publisher, subscriber, and family variant descriptors.',
  notClaimed: [
    '69 executable Unity/TypeScript conformance matrix',
    '68 TypeScript/Rust fixture parity',
    'Parent-specific event contracts',
    'regex source scanner for all product crates',
  ],
};

writeFileSync(join(proofRoot, 'proof-summary.json'), `${JSON.stringify(proof, null, 2)}\n`);
console.log(`eventing-topology-manifest-proof-ok:${proof.assertions.join(',')}`);
console.log(`proof=${join(proofRoot, 'proof-summary.json')}`);
