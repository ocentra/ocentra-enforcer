import { spawnSync } from 'node:child_process';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import assert from 'node:assert/strict';
import * as Either from 'effect/Either';
import * as Schema from 'effect/Schema';

const proofRoot = join('output', 'eventing-plan-proof', '68-fixture-parity');
const fixturePath = join('crates', 'ocentra-eventing', 'fixtures', 'branded_scalar_parity.json');
mkdirSync(proofRoot, { recursive: true });

const fixture = JSON.parse(readFileSync(fixturePath, 'utf8'));
const textSchemas = {
  eventType: brandedText('EventType'),
  eventNamespace: brandedText('EventNamespace'),
  eventId: brandedText('EventId'),
  correlationId: brandedText('CorrelationId'),
  requestId: brandedText('RequestId'),
  journalHash: brandedText('JournalHash'),
  aggregateKey: brandedText('AggregateKey'),
  idempotencyKey: brandedText('IdempotencyKey'),
  subscriberId: brandedText('SubscriberId'),
  targetHandler: brandedText('TargetHandler'),
  sourceService: brandedText('SourceService'),
  sourceComponent: brandedText('SourceComponent'),
  runtimeInstanceId: brandedText('RuntimeInstanceId'),
  recordedAt: brandedText('RecordedAt'),
};
const SchemaVersion = Schema.Number.pipe(
  Schema.filter((value) => Number.isInteger(value) && value > 0),
  Schema.brand('SchemaVersion')
);
const ValidScalars = Schema.Struct({
  ...textSchemas,
  schemaVersion: SchemaVersion,
});

assertRight(ValidScalars, fixture.valid, 'valid TypeScript branded scalar fixture');
for (const invalid of fixture.invalidText) {
  const schema = textSchemas[invalid.field];
  assert.ok(schema, `known text fixture field ${invalid.field}`);
  assertLeft(schema, invalid.value, `invalid TypeScript branded scalar ${invalid.field}`);
}
for (const value of fixture.invalidSchemaVersions) {
  assertLeft(SchemaVersion, value, `invalid TypeScript schema version ${value}`);
}

writeFileSync(
  join(proofRoot, 'typescript-effect-schema-validation.json'),
  `${JSON.stringify(
    {
      fixturePath,
      acceptedValidFields: Object.keys(fixture.valid),
      rejectedInvalidTextFields: fixture.invalidText.map((entry) => entry.field),
      rejectedInvalidSchemaVersions: fixture.invalidSchemaVersions,
      brandCount: Object.keys(textSchemas).length + 1,
    },
    null,
    2
  )}\n`
);

const commands = [
  {
    name: 'rust-fixture-parity-tests',
    command: 'cargo',
    args: ['test', '-p', 'ocentra-eventing', 'fixture_parity'],
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

const rustTests = readFileSync('crates/ocentra-eventing/tests/contract/fixture_parity.rs', 'utf8');
const proofScript = readFileSync('scripts/test/eventing-branded-fixture-parity-proof.mjs', 'utf8');

const assertions = [
  ['shared-fixture-exists', readFileSync(fixturePath, 'utf8').includes('network.domain.observed')],
  ['rust-tests-include-shared-fixture', rustTests.includes('branded_scalar_parity.json')],
  ['rust-tests-cover-valid-scalars', rustTests.includes('rust_newtypes_accept_shared_valid_parity_fixture')],
  ['rust-tests-cover-invalid-scalars', rustTests.includes('rust_newtypes_reject_shared_invalid_text_fixture_values')],
  ['typescript-uses-effect-schema-brands', proofScript.includes('Schema.brand')],
  ['typescript-validates-same-fixture', proofScript.includes('fixture.invalidText')],
];

const failed = assertions.find(([, passed]) => !passed);
if (failed) {
  throw new Error(`eventing fixture parity assertion failed: ${failed[0]}`);
}

const proof = {
  proof: 'eventing-branded-fixture-parity',
  proofRoot,
  checkedAt: new Date().toISOString(),
  fixturePath,
  commands: commandResults,
  assertions: assertions.map(([name]) => name),
  typescriptValidation: join(proofRoot, 'typescript-effect-schema-validation.json'),
  provenRows: ['68 TypeScript/Rust branded fixture parity'],
  policy:
    'The same canonical fixture is accepted and rejected by TypeScript Effect Schema brands and Rust eventing newtypes for eventing scalar boundaries.',
  notClaimed: [
    'Parent-specific event contracts',
    'agent-protocol-domain event exports',
    'cross-package TypeScript public API for eventing contracts',
    'broker-backed network delivery transport',
  ],
};

writeFileSync(join(proofRoot, 'proof-summary.json'), `${JSON.stringify(proof, null, 2)}\n`);
console.log(`eventing-branded-fixture-parity-proof-ok:${proof.assertions.join(',')}`);
console.log(`proof=${join(proofRoot, 'proof-summary.json')}`);

function brandedText(name) {
  return Schema.String.pipe(
    Schema.filter((value) => typeof value === 'string' && value.trim().length > 0),
    Schema.brand(name)
  );
}

function assertRight(schema, input, label) {
  const decoded = Schema.decodeUnknownEither(schema)(input, { errors: 'all' });
  assert.equal(Either.isRight(decoded), true, label);
}

function assertLeft(schema, input, label) {
  const decoded = Schema.decodeUnknownEither(schema)(input, { errors: 'all' });
  assert.equal(Either.isLeft(decoded), true, label);
}
