import { spawnSync } from 'node:child_process';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const proofRoot = join('output', 'eventing-plan-proof', '69-compatibility-matrix');
mkdirSync(proofRoot, { recursive: true });

const commands = [
  {
    name: 'compatibility-matrix-tests',
    command: 'cargo',
    args: ['test', '-p', 'ocentra-eventing', 'compatibility_matrix'],
  },
  {
    name: 'compatibility-matrix-example',
    command: 'cargo',
    args: ['run', '-p', 'ocentra-eventing', '--example', 'compatibility_matrix'],
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
    writeFileSync(join(proofRoot, 'eventing-compatibility-matrix.generated.md'), result.stdout ?? '');
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

const compatibilitySource = readFileSync('crates/ocentra-eventing/src/compatibility.rs', 'utf8');
const compatibilityTests = readFileSync('crates/ocentra-eventing/src/tests/compatibility_matrix.rs', 'utf8');
const generatedMatrix = readFileSync(join(proofRoot, 'eventing-compatibility-matrix.generated.md'), 'utf8');

const assertions = [
  ['compatibility-matrix-type-exists', compatibilitySource.includes('pub struct EventCompatibilityMatrix')],
  ['compatibility-status-type-exists', compatibilitySource.includes('pub enum EventCompatibilityStatus')],
  ['matrix-records-intentional-deviation', compatibilitySource.includes('IntentionalDeviation')],
  ['matrix-records-manual-required', compatibilitySource.includes('ManualRequired')],
  ['tests-cover-lineage-semantics', compatibilityTests.includes('compatibility_matrix_covers_games_lineage_semantics')],
  [
    'tests-cover-deviation-and-manual-required',
    compatibilityTests.includes('compatibility_matrix_marks_deviations_and_manual_required_scope') &&
      compatibilityTests.includes('payload-republish-override') &&
      compatibilityTests.includes('broker-backed-delivery'),
  ],
  [
    'generated-markdown-matrix-exists',
    generatedMatrix.includes('# Eventing Compatibility Matrix') &&
      generatedMatrix.includes('class-backed-contracts') &&
      generatedMatrix.includes('intentional-deviation') &&
      generatedMatrix.includes('manual-required'),
  ],
];

const failed = assertions.find(([, passed]) => !passed);
if (failed) {
  throw new Error(`eventing compatibility-matrix assertion failed: ${failed[0]}`);
}

const proof = {
  proof: 'eventing-compatibility-matrix',
  proofRoot,
  checkedAt: new Date().toISOString(),
  commands: commandResults,
  assertions: assertions.map(([name]) => name),
  generatedMatrix: join(proofRoot, 'eventing-compatibility-matrix.generated.md'),
  provenRows: ['69 Unity/TypeScript semantics conformance matrix and compatibility suite'],
  policy:
    'EventCompatibilityMatrix maps Ocentra Games/TypeScript eventing lineage semantics to Rust eventing surfaces, explicit intentional deviations, and manual-required broker delivery scope.',
  notClaimed: [
    '68 TypeScript/Rust branded fixture parity',
    'constrained republish override implementation',
    'payload-carried disposal callbacks or local handles',
    'broker-backed network delivery transport',
    'Parent-specific event contracts',
  ],
};

writeFileSync(join(proofRoot, 'proof-summary.json'), `${JSON.stringify(proof, null, 2)}\n`);
console.log(`eventing-compatibility-matrix-proof-ok:${proof.assertions.join(',')}`);
console.log(`proof=${join(proofRoot, 'proof-summary.json')}`);
