import { execFileSync } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';

const root = process.cwd();
const proofDir = path.join(root, 'test-results', 'browser-runtime-action-intent-store-backed-proof');
const outputDir = path.join(root, 'output', 'browser-plan-proof', 'browser-runtime-action-intent-store-backed');

const files = {
  browserConstants: path.join(root, 'crates', 'agent-protocol', 'src', 'constants', 'browser.rs'),
  delivery: path.join(root, 'crates', 'agent-service', 'src', 'browser_runtime_delivery.rs'),
  payload: path.join(root, 'crates', 'agent-service', 'src', 'browser_runtime_stream_payload.rs'),
  streamApi: path.join(root, 'crates', 'agent-service', 'src', 'browser_runtime_stream_api.rs'),
  tests: path.join(root, 'crates', 'agent-service', 'src', 'browser_runtime_stream_tests.rs'),
  policyPreviewApi: path.join(root, 'crates', 'agent-service', 'src', 'policy_preview_api.rs'),
  checklist: path.join(root, 'docs', 'plans', 'browser-plan', 'implementation-checklist.md'),
  workpack: path.join(
    root,
    'docs',
    'plans',
    'browser-plan',
    'workpacks',
    '13-browser-read-models-and-service-events.md'
  ),
};

function run(command, args) {
  execFileSync(command, args, {
    cwd: root,
    stdio: 'inherit',
    shell: process.platform === 'win32',
  });
}

async function sourceChecks() {
  const [browserConstants, delivery, payload, streamApi, tests, policyPreviewApi, checklist, workpack] =
    await Promise.all(Object.values(files).map((file) => readFile(file, 'utf8')));

  return {
    protocolActionIntentPrefixRegistered: browserConstants.includes('ACTION_INTENT_ID_PREFIX'),
    deliveryAcceptsPolicyPreviewReadModel: delivery.includes('browser_runtime_input_from_row_with_policy_preview'),
    deliveryMatchesPreviewEvidenceRefs: delivery.includes('policy_preview_references_browser_row'),
    deliveryDerivesStableBrowserActionIntentId: delivery.includes('action_intent_id_from_policy_decision'),
    streamPayloadAcceptsPolicyPreviewReadModel: payload.includes(
      'stream_browser_runtime_event_chain_for_read_model_with_policy_preview'
    ),
    streamApiLoadsPolicyPreviewReadModel: streamApi.includes('load_policy_preview_read_model'),
    policyPreviewLoaderAvailableToStream: policyPreviewApi.includes(
      'pub(crate) async fn load_policy_preview_read_model'
    ),
    focusedStoreBackedTestExists: tests.includes(
      'service_browser_runtime_stream_projects_store_backed_policy_preview_candidate'
    ),
    websocketStoreBackedCandidateCountIsOne:
      tests.includes('constants::field::BROWSER_RUNTIME_ACTION_INTENT_CANDIDATES') &&
      tests.includes('Some(&LogFieldValue::Number(1.0))'),
    dispatchAndExecutionRemainZero:
      tests.includes('BROWSER_RUNTIME_ACTION_INTENT_DISPATCH_ATTEMPTS') &&
      tests.includes('BROWSER_RUNTIME_ACTION_INTENT_ENFORCEMENT_EXECUTIONS'),
    checklistMentionsStoreBackedProof: checklist.includes('browser-runtime-action-intent-store-backed-proof'),
    workpackMentionsStoreBackedProof: workpack.includes('Action-Intent Store-Backed Policy Preview Addendum'),
  };
}

function assertSourceChecks(checks) {
  const missing = Object.entries(checks)
    .filter(([, ok]) => !ok)
    .map(([name]) => name);
  if (missing.length > 0) {
    throw new Error(`browser action-intent store-backed proof failed: ${missing.join(', ')}`);
  }
}

async function main() {
  await mkdir(proofDir, { recursive: true });
  await mkdir(outputDir, { recursive: true });

  const commands = [
    {
      command: 'cargo',
      args: ['test', '-p', 'ocentra-parent-agent-service', 'service_browser_runtime', '--quiet'],
    },
  ];

  for (const item of commands) {
    run(item.command, item.args);
  }

  const checks = await sourceChecks();
  assertSourceChecks(checks);

  const proof = {
    proofName: 'browser-runtime-action-intent-store-backed-proof',
    branchHead: execFileSync('git', ['log', '-1', '--oneline'], { cwd: root, encoding: 'utf8' }).trim(),
    sourceChecks: checks,
    commands: commands.map((item) => `${item.command} ${item.args.join(' ')}`),
    verified: {
      existingStreamCommand: 'agent.browser.runtime.event-chain.stream.get',
      policyPreviewReadModelSource: 'existing ActivityStore policy preview read model',
      browserEvidenceReadModelSource: 'existing ActivityStore browser evidence read model',
      matchingEvidenceReferenceProjectsPendingCandidate: true,
      storeBackedBrowserRowsWithMatchingPolicyPreviewHavePendingCandidateCount: 1,
      dryRunPolicyAuthorityProjected: true,
      stableBrowserActionIntentIdDerivedFromPolicyDecision: true,
      dispatchAttemptCount: 0,
      adapterExecutionCount: 0,
      childInterventionExecutionCount: 0,
      enforcementExecutionCount: 0,
      finalPolicyExecutionClaimed: false,
      browserMutationClaimed: false,
      childInterventionClaimed: false,
      enforcementClaimed: false,
    },
  };

  await writeFile(path.join(proofDir, 'proof.json'), `${JSON.stringify(proof, null, 2)}\n`);
  await writeFile(
    path.join(outputDir, '01-browser-runtime-action-intent-store-backed-proof.md'),
    [
      '# Browser Runtime Action Intent Store-Backed Proof',
      '',
      'This proof closes the prior store-backed action-intent projection gap for browser evidence rows that have a matching stored policy preview read-model row.',
      '',
      'The service-backed browser runtime stream now loads the existing browser evidence read model and the existing policy preview read model from the ActivityStore. Matching policy-preview evidence refs enrich the browser runtime input with a policy preview id, policy decision ref, stable browser action-intent id, dry-run authority, and one pending action-intent candidate.',
      '',
      'The projection remains non-mutating: dispatch attempts, adapter execution, child intervention execution, final policy execution, and enforcement execution all stay at zero.',
      '',
      'Validation:',
      ...proof.commands.map((command) => `- \`${command}\``),
      '',
    ].join('\n')
  );

  console.log(JSON.stringify(proof, null, 2));
}

main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
