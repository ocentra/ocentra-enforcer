import { mkdirSync, readdirSync, readFileSync, statSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { spawnSync } from 'node:child_process';

const testRoot = join('test-results', 'eventing-type-safety-source-gate-proof');
const proofRoot = join('output', 'eventing-plan-proof', '63-type-safety-source-gate');

mkdirSync(testRoot, { recursive: true });
mkdirSync(proofRoot, { recursive: true });

const commands = [
  {
    name: 'eventing-tests',
    command: 'cargo',
    args: ['test', '-p', 'ocentra-eventing'],
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
  const logPath = join(proofRoot, `${entry.name}.log`);
  writeFileSync(logPath, output);
  if (result.status !== 0) {
    throw new Error(`${entry.name} failed with exit ${result.status}`);
  }
  return {
    name: entry.name,
    command: [entry.command, ...entry.args].join(' '),
    status: result.status,
    log: logPath,
  };
});

const sourceEntries = sourceFiles(join('crates', 'ocentra-eventing', 'src'))
  .filter((path) => !path.includes(`${join('src', 'tests')}${separatorFor(path)}`))
  .map((path) => [path, readFileSync(path, 'utf8')]);

const sourceText = sourceEntries.map(([path, source]) => `\n// ${path}\n${source}`).join('\n');
const errorSource = readFileSync(join('crates', 'ocentra-eventing', 'src', 'error.rs'), 'utf8');
const envelopeSource = readFileSync(join('crates', 'ocentra-eventing', 'src', 'envelope.rs'), 'utf8');
const libSource = readFileSync(join('crates', 'ocentra-eventing', 'src', 'lib.rs'), 'utf8');
const reportsSource = readFileSync(join('crates', 'ocentra-eventing', 'src', 'bus', 'reports.rs'), 'utf8');

const rawPublicJson = publicLineMatches(/pub\s+[^;\n]*serde_json::Value/u);
const rawPublicStringConstants = publicLineMatches(/pub\s+const\s+\w+\s*:\s*&str/u);
const rawUuid = sourceEntries.filter(([, source]) => /\bUuid\b/u.test(source)).map(([path]) => path);
const disallowedPublicStrings = publicLineMatches(/pub\s+(?:struct|enum|fn|const|type)?[^\n]*\bString\b/u).filter(
  (entry) => !isAllowedRawPublicString(entry)
);

const typedErrorFieldPatterns = [
  /PayloadDecode\s*\{[^}]*event_type:\s*EventType/su,
  /ContractMismatch\s*\{[^}]*expected:\s*EventType[^}]*received:\s*EventType/su,
  /DuplicateEventContract\s*\{[^}]*event_type:\s*EventType/su,
  /DuplicateSubscriber\s*\{[^}]*subscriber_id:\s*SubscriberId/su,
  /HandlerPanicked\s*\{[^}]*subscriber_id:\s*SubscriberId/su,
  /HandlerTimedOut\s*\{[^}]*subscriber_id:\s*SubscriberId/su,
  /NoSubscriber\s*\{[^}]*event_type:\s*EventType/su,
  /QueueCapacityExceeded\s*\{[^}]*event_type:\s*EventType/su,
  /EventDeadlineExpired\s*\{[^}]*event_type:\s*EventType/su,
  /DuplicateInFlight\s*\{[^}]*idempotency_key:\s*IdempotencyKey/su,
  /DuplicateIdempotencyKey\s*\{[^}]*idempotency_key:\s*IdempotencyKey/su,
  /DuplicateRequest\s*\{[^}]*request_id:\s*RequestId/su,
  /RequestTimedOut\s*\{[^}]*request_id:\s*RequestId/su,
  /RequestResponseEncode\s*\{[^}]*request_id:\s*RequestId/su,
  /RequestResponseDecode\s*\{[^}]*request_id:\s*RequestId/su,
  /ReplayActionNotAllowed\s*\{[^}]*event_type:\s*EventType/su,
];

const rawErrorFieldPatterns = [
  /PayloadDecode\s*\{[^}]*event_type:\s*String/su,
  /ContractMismatch\s*\{[^}]*expected:\s*String/su,
  /DuplicateEventContract\s*\{[^}]*event_type:\s*String/su,
  /DuplicateSubscriber\s*\{[^}]*subscriber_id:\s*String/su,
  /Handler(?:Panicked|TimedOut)\s*\{[^}]*subscriber_id:\s*String/su,
  /NoSubscriber\s*\{[^}]*event_type:\s*String/su,
  /QueueCapacityExceeded\s*\{[^}]*event_type:\s*String/su,
  /EventDeadlineExpired\s*\{[^}]*event_type:\s*String/su,
  /Duplicate(?:InFlight|IdempotencyKey)\s*\{[^}]*idempotency_key:\s*String/su,
  /DuplicateRequest\s*\{[^}]*request_id:\s*String/su,
  /RequestTimedOut\s*\{[^}]*request_id:\s*String/su,
  /RequestResponse(?:Encode|Decode)\s*\{[^}]*request_id:\s*String/su,
  /ReplayActionNotAllowed\s*\{[^}]*event_type:\s*String/su,
];

const sourceAssertions = [
  ['no-public-serde-json-value', rawPublicJson.length === 0, rawPublicJson],
  ['no-public-raw-string-constants', rawPublicStringConstants.length === 0, rawPublicStringConstants],
  ['no-uuid-in-eventing-source', rawUuid.length === 0, rawUuid],
  ['no-disallowed-public-raw-strings', disallowedPublicStrings.length === 0, disallowedPublicStrings],
  [
    'stored-envelope-payload-is-wrapper',
    envelopeSource.includes('pub payload: StoredEventPayload') &&
      !envelopeSource.includes('pub payload: serde_json::Value'),
    [],
  ],
  [
    'stored-payload-wrapper-keeps-json-private',
    envelopeSource.includes('pub struct StoredEventPayload') &&
      envelopeSource.includes('value: serde_json::Value') &&
      !envelopeSource.includes('pub value: serde_json::Value'),
    [],
  ],
  ['stored-payload-wrapper-exported', libSource.includes('StoredEventEnvelope, StoredEventPayload'), []],
  [
    'dead-letter-event-type-public-api-is-typed',
    reportsSource.includes('pub fn dead_letter_recorded_event_type() -> Result<EventType, EventingError>') &&
      !reportsSource.includes('pub const DEAD_LETTER_RECORDED_EVENT_TYPE'),
    [],
  ],
  ...typedErrorFieldPatterns.map((pattern) => [`typed-error-field:${pattern.source}`, pattern.test(errorSource), []]),
  ...rawErrorFieldPatterns.map((pattern) => [`no-raw-error-field:${pattern.source}`, !pattern.test(errorSource), []]),
];

const failedAssertions = sourceAssertions.filter(([, passed]) => !passed);
if (failedAssertions.length > 0) {
  const detail = failedAssertions
    .map(([name, , matches]) => `${name}${matches?.length ? ` ${JSON.stringify(matches)}` : ''}`)
    .join('\n');
  throw new Error(`eventing type-safety source-gate assertion failed:\n${detail}`);
}

const proof = {
  proof: 'eventing-type-safety-source-gate',
  proofRoot,
  checkedAt: new Date().toISOString(),
  commands: commandResults,
  assertions: sourceAssertions.map(([name]) => name),
  scannedSources: sourceEntries.map(([path]) => path),
  provenRows: ['63 Type-safety and validation source gate'],
  notClaimed: [
    '42-50 Parent protocol/domain event contract rows remain sequenced behind protocol/service locks',
    '51-56 Parent runtime/journal-before-action rows remain sequenced behind service/protocol locks',
    '57-59 network chain rows remain partial until eventing integration reaches real adapter/runtime flows',
  ],
};

writeFileSync(join(testRoot, 'proof.json'), `${JSON.stringify(proof, null, 2)}\n`);
writeFileSync(join(proofRoot, 'proof-summary.json'), `${JSON.stringify(proof, null, 2)}\n`);

console.log(`eventing-type-safety-source-gate-proof-ok:${proof.assertions.join(',')}`);
console.log(`evidence=${join(testRoot, 'proof.json')}`);
console.log(`planEvidence=${join(proofRoot, 'proof-summary.json')}`);

function sourceFiles(root) {
  const files = [];
  for (const entry of readdirSync(root)) {
    const path = join(root, entry);
    if (statSync(path).isDirectory()) {
      files.push(...sourceFiles(path));
      continue;
    }
    if (path.endsWith('.rs')) {
      files.push(path);
    }
  }
  return files;
}

function publicLineMatches(pattern) {
  return sourceEntries.flatMap(([path, source]) =>
    source
      .split(/\r?\n/u)
      .map((line, index) => ({ path, line: index + 1, text: line.trim() }))
      .filter(({ text }) => pattern.test(text))
  );
}

function isAllowedRawPublicString(entry) {
  const normalizedPath = entry.path.replaceAll('\\', '/');
  if (normalizedPath.endsWith('ids.rs')) {
    return /pub struct \$name\(String\)|pub fn parse\(value: impl Into<String>\)|pub fn as_str\(&self\) -> &str/u.test(
      entry.text
    );
  }
  if (normalizedPath.endsWith('compatibility.rs')) {
    return /pub fn entry\(&self, semantic_id: &str\)|pub fn render_markdown\(&self\) -> String/u.test(entry.text);
  }
  if (normalizedPath.endsWith('contract_registry.rs')) {
    return /pub fn as_str\(&self\) -> &str|pub fn into_string\(self\) -> String/u.test(entry.text);
  }
  if (normalizedPath.endsWith('topology.rs')) {
    return /pub fn render_markdown\(&self\) -> String/u.test(entry.text);
  }
  return false;
}

function separatorFor(path) {
  return path.includes('\\') ? '\\' : '/';
}
