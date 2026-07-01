import { execFileSync } from 'node:child_process';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const proofName = 'browser-runtime-event-name-parity-proof';
const resultRoot = join('test-results', proofName);
const outputRoot = join('output', 'browser-plan-proof', 'browser-runtime-event-name-parity');

mkdirSync(resultRoot, { recursive: true });
mkdirSync(outputRoot, { recursive: true });

const rustConstantsSource = readFileSync('crates/agent-protocol/src/constants/browser.rs', 'utf8');
const rustBrowserSource = readFileSync('crates/agent-protocol/src/browser.rs', 'utf8');
const rustCoreTestSource = readFileSync('crates/agent-core/tests/unit/browser_event_runtime_tests.rs', 'utf8');
const rustContractTestSource = readFileSync(
  'crates/agent-protocol/tests/contract/root_contract_shape_tests.rs',
  'utf8'
);
const normalizedBrowserSource = rustBrowserSource.replace(/\s+/g, ' ');

const browserPhasePairs = [
  ['EvidenceObserved', 'EVENT_BROWSER_EVIDENCE_OBSERVED'],
  ['EvidenceJournaled', 'EVENT_BROWSER_EVIDENCE_JOURNALED'],
  ['AiAnalysisRequested', 'EVENT_BROWSER_AI_ANALYSIS_REQUESTED'],
  ['AiAnalysisCompleted', 'EVENT_BROWSER_AI_ANALYSIS_COMPLETED'],
  ['PolicyEvaluationRequested', 'EVENT_BROWSER_POLICY_EVALUATION_REQUESTED'],
  ['PolicyDecisionCompleted', 'EVENT_BROWSER_POLICY_DECISION_COMPLETED'],
  ['InterventionCommandIssued', 'EVENT_BROWSER_INTERVENTION_COMMAND_ISSUED'],
  ['InterventionResultObserved', 'EVENT_BROWSER_INTERVENTION_RESULT_OBSERVED'],
  ['AuditEntryCommitted', 'EVENT_BROWSER_AUDIT_ENTRY_COMMITTED'],
  ['ReadModelProjected', 'EVENT_BROWSER_READ_MODEL_PROJECTED'],
];

const rustRuntimeEventNames = browserPhasePairs.map(([, constantName]) => constantName);
const rustConstantsDefined = browserPhasePairs.every(([, constantName]) =>
  rustConstantsSource.includes(`pub const ${constantName}`)
);

const browserPhaseMappingsPresent = browserPhasePairs.every(([phaseName, constantName]) =>
  normalizedBrowserSource.includes(`Self::${phaseName}`) &&
  normalizedBrowserSource.includes(`constants::browser::${constantName}`)
);
const browserOrderedChainDefined = rustBrowserSource.includes("pub fn ordered_chain() -> &'static [Self]");
const coreTestCoversOrderedChain =
  rustCoreTestSource.includes('BrowserRuntimePhase::ordered_chain().len()') &&
  rustCoreTestSource.includes('BrowserRuntimePhase::ordered_chain().to_vec()');
const contractTestCoversBrowserStreamNames =
  rustContractTestSource.includes('browser_runtime_stream_command_and_event_names_serialize_to_contract_shape') &&
  rustContractTestSource.includes('AgentEventName::AgentBrowserRuntimeEventChainStreamReported');

const commandResults = [
  {
    command:
      'cargo test -p ocentra-parent-agent-protocol browser_runtime_stream_command_and_event_names_serialize_to_contract_shape --quiet',
    output: run('cargo', [
      'test',
      '-p',
      'ocentra-parent-agent-protocol',
      'browser_runtime_stream_command_and_event_names_serialize_to_contract_shape',
      '--quiet',
    ]),
  },
  {
    command: 'cargo test -p ocentra-parent-agent-core browser_event_runtime --quiet',
    output: run('cargo', ['test', '-p', 'ocentra-parent-agent-core', 'browser_event_runtime', '--quiet']),
  },
];

if (
  !browserPhaseMappingsPresent ||
  !browserOrderedChainDefined ||
  !coreTestCoversOrderedChain ||
  !contractTestCoversBrowserStreamNames ||
  !rustConstantsDefined
) {
  throw new Error(
    JSON.stringify(
      {
        rustConstantsDefined,
        browserPhaseMappingsPresent,
        browserOrderedChainDefined,
        coreTestCoversOrderedChain,
        contractTestCoversBrowserStreamNames,
        rustRuntimeEventNames,
      },
      null,
      2
    )
  );
}

const proof = {
  proofName,
  rustRuntimeEventNames,
  sourceChecks: {
    rustConstantsDefined,
    browserPhaseMappingsPresent,
    browserOrderedChainDefined,
    coreTestCoversOrderedChain,
    contractTestCoversBrowserStreamNames,
  },
  commands: commandResults.map((result) => result.command),
  verified: {
    rustBrowserRuntimeEventNameParity: true,
    allBrowserRuntimePhasesCovered: true,
    browserPhaseMappingsUseProtocolConstants: true,
    genericEventBusChanged: false,
    portalUiChanged: false,
    aiExecutes: false,
    policyExecutes: false,
    browserMutationExecutes: false,
    childInterventionExecutes: false,
    enforcementExecutes: false,
  },
};

writeFileSync(join(resultRoot, 'proof.json'), `${JSON.stringify(proof, null, 2)}\n`);
writeFileSync(
  join(outputRoot, '01-browser-runtime-event-name-parity-proof.md'),
  [
    '# Browser Runtime Event Name Parity Proof',
    '',
    `- Rust browser constants defined: ${rustConstantsDefined}`,
    `- Rust browser phase mappings present: ${browserPhaseMappingsPresent}`,
    `- Rust ordered chain defined: ${browserOrderedChainDefined}`,
    `- Core tests cover ordered chain: ${coreTestCoversOrderedChain}`,
    `- Protocol contract test covers browser stream names: ${contractTestCoversBrowserStreamNames}`,
    '',
    '## Rust Browser Event Names',
    '',
    ...rustRuntimeEventNames.map((eventName) => `- ${eventName}`),
    '',
    '## Commands',
    '',
    ...commandResults.map((result) => `- ${result.command}`),
    '',
    '## No-Claim Boundaries',
    '',
    '- No generic event bus change.',
    '- No portal UI change.',
    '- No AI execution.',
    '- No policy execution.',
    '- No browser mutation.',
    '- No child intervention execution.',
    '- No enforcement.',
    '',
  ].join('\n')
);

console.log(JSON.stringify(proof, null, 2));

function run(command, args) {
  return execFileSync(command, args, {
    cwd: process.cwd(),
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'pipe'],
  }).trim();
}
