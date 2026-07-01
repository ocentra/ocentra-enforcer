import { spawnSync } from 'node:child_process';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const proofRoot = join('output', 'eventing-plan-proof', '75-family-variants');
mkdirSync(proofRoot, { recursive: true });

const commands = [
  {
    name: 'family-variant-tests',
    command: 'cargo',
    args: ['test', '-p', 'ocentra-eventing', 'family_variants'],
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

const familyTests = readFileSync('crates/ocentra-eventing/src/tests/family_variants.rs', 'utf8');

const assertions = [
  ['typed-family-enum-exists', familyTests.includes('enum DecisionFamilyEvent')],
  ['tagged-serde-wrapper-exists', familyTests.includes('tag = "family_variant"')],
  ['handler-matches-typed-variants', familyTests.includes('match context.payload()')],
  [
    'stored-decode-contract-mismatch-test',
    familyTests.includes('family_variant_stored_decode_rejects_contract_variant_mismatch'),
  ],
  [
    'variants-register-distinct-contracts',
    familyTests.includes('family_variants_register_as_distinct_contract_descriptors'),
  ],
  ['no-json-shape-inspection', !familyTests.includes('serde_json::Value')],
  ['no-downcast-routing', !familyTests.includes('downcast_ref') && !familyTests.includes('std::any')],
];

const failed = assertions.find(([, passed]) => !passed);
if (failed) {
  throw new Error(`eventing family-variant assertion failed: ${failed[0]}`);
}

const proof = {
  proof: 'eventing-family-variants',
  proofRoot,
  checkedAt: new Date().toISOString(),
  commands: commandResults,
  assertions: assertions.map(([name]) => name),
  provenRows: ['75 Event-family enum/wrapper variant proof for inherited/generic lineage patterns'],
  policy:
    'Family subscribers receive a typed Rust enum wrapper and match variants directly; stored decode validates that the variant payload contract matches the stored event contract.',
  notClaimed: [
    '70 event topology manifest',
    '69 executable Unity/TypeScript conformance matrix',
    'Parent-specific event contracts',
    'cross-language TypeScript/Rust fixture parity',
  ],
};

writeFileSync(join(proofRoot, 'proof-summary.json'), `${JSON.stringify(proof, null, 2)}\n`);
console.log(`eventing-family-variant-proof-ok:${proof.assertions.join(',')}`);
console.log(`proof=${join(proofRoot, 'proof-summary.json')}`);
