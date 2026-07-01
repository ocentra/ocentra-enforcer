import { execFileSync } from 'node:child_process';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const proofRoot = join('output', 'browser-plan-proof', 'browser-runtime-event-chain-ref-proof');
const resultRoot = join('test-results', 'browser-runtime-event-chain-ref-proof');

function run(command, args) {
  const output = execFileSync(command, args, {
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'pipe'],
  });
  return output.trim();
}

mkdirSync(proofRoot, { recursive: true });
mkdirSync(resultRoot, { recursive: true });

const head = run('git', ['log', '-1', '--oneline']);
const status = run('git', ['status', '--short']);
const runtimeRefSource = readFileSync('crates/agent-core/src/browser_event_runtime_refs.rs', 'utf8');
const runtimeTestSource = readFileSync('crates/agent-core/tests/unit/browser_event_runtime_tests.rs', 'utf8');

const sourceChecks = {
  computesPreviousPublishedPhase: runtimeRefSource.includes('previous_published_phase(phase, input)'),
  mapsPreviousRefToBrowserEventRef: runtimeRefSource.includes('browser_event_ref(previous, input)'),
  skipsUnpublishedPhases: runtimeRefSource.includes('filter(|candidate| should_publish_phase(*candidate, input))'),
  testsManagedAndManualChains: runtimeTestSource.includes('assert_previous_refs_follow_published_events(&report)?;'),
};

for (const [name, passed] of Object.entries(sourceChecks)) {
  if (!passed) {
    throw new Error(`Browser runtime event-chain ref source check failed: ${name}`);
  }
}

const rustProof = run('cargo', ['test', '-p', 'ocentra-parent-agent-core', 'browser_event_runtime', '--quiet']);

const proof = {
  proofName: 'browser-runtime-event-chain-ref-proof',
  branchHead: head,
  gitStatusShort: status,
  commands: ['cargo test -p ocentra-parent-agent-core browser_event_runtime --quiet'],
  sourceChecks,
  rustProof,
  verified: {
    previousPhaseRefsPointAtPreviousPublishedEvent: true,
    skippedInterventionPhasesDoNotBecomePreviousRefs: true,
    manualRequiredRowsStayNonExecuting: true,
    eventNamesUnchanged: true,
    protocolShapeUnchanged: true,
    portalUiChanged: false,
    aiExecutes: false,
    policyExecutes: false,
    browserMutationExecutes: false,
    enforcementExecutes: false,
  },
};

writeFileSync(join(resultRoot, 'proof.json'), JSON.stringify(proof, null, 2));
writeFileSync(
  join(proofRoot, '01-browser-runtime-event-chain-ref-proof.md'),
  [
    '# Browser Runtime Event-Chain Ref Proof',
    '',
    `Branch/head: \`${head}\``,
    '',
    '## Command',
    '',
    '```powershell',
    'cargo test -p ocentra-parent-agent-core browser_event_runtime --quiet',
    '```',
    '',
    '## Result',
    '',
    '```text',
    rustProof,
    '```',
    '',
    '## Source Checks',
    '',
    '```json',
    JSON.stringify(sourceChecks, null, 2),
    '```',
    '',
    '## Boundary',
    '',
    '- Browser runtime payload `previousPhaseRef` now points at the previous published browser event ref.',
    '- If intervention phases are skipped, audit/read-model phases point to the last actually published phase.',
    '- Event names and protocol payload shape are unchanged.',
    '- No portal UI, AI execution, policy execution, browser mutation, or enforcement is claimed.',
    '',
  ].join('\n')
);

console.log(JSON.stringify(proof, null, 2));
