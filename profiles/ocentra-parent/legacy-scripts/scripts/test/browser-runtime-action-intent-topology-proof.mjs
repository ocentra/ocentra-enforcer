import { execFileSync } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';

const root = process.cwd();
const proofDir = path.join(root, 'test-results', 'browser-runtime-action-intent-topology-proof');
const outputDir = path.join(root, 'output', 'browser-plan-proof', 'browser-runtime-action-intent-topology');

const files = {
  actionStatus: path.join(root, 'crates', 'agent-core', 'src', 'browser_event_runtime', 'action_status.rs'),
  runtime: path.join(root, 'crates', 'agent-core', 'src', 'browser_event_runtime.rs'),
  tests: path.join(root, 'crates', 'agent-core', 'src', 'browser_event_runtime_tests.rs'),
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
  const [actionStatus, runtime, tests, checklist, workpack] = await Promise.all(
    Object.values(files).map((file) => readFile(file, 'utf8'))
  );
  return {
    registersEventContract: actionStatus.includes('EventContractRegistry::new()'),
    buildsTopologyManifest: actionStatus.includes('EventTopologyManifest::from_registry'),
    declaresPublisher: actionStatus.includes('EventTopologyPublisher'),
    declaresSubscriber: actionStatus.includes('EventTopologySubscriber'),
    exportsTopologyHelper: runtime.includes('browser_runtime_action_intent_status_topology_manifest'),
    focusedTopologyTestExists: tests.includes(
      'browser_runtime_action_intent_topology_covers_named_event_and_subscriber'
    ),
    testAssertsCoveredStatus: tests.includes('EventTopologyStatus::Covered'),
    docsMentionTopologyProof: checklist.includes('browser-runtime-action-intent-topology-proof'),
    workpackMentionsTopologyProof: workpack.includes('Action-Intent Topology Addendum'),
  };
}

function assertSourceChecks(checks) {
  const missing = Object.entries(checks)
    .filter(([, ok]) => !ok)
    .map(([name]) => name);
  if (missing.length > 0) {
    throw new Error(`browser action-intent topology proof failed: ${missing.join(', ')}`);
  }
}

async function main() {
  await mkdir(proofDir, { recursive: true });
  await mkdir(outputDir, { recursive: true });

  const commands = [
    {
      command: 'cargo',
      args: [
        'test',
        '-p',
        'ocentra-parent-agent-core',
        'browser_runtime_action_intent_topology_covers_named_event_and_subscriber',
        '--quiet',
      ],
    },
  ];

  for (const item of commands) {
    run(item.command, item.args);
  }

  const checks = await sourceChecks();
  assertSourceChecks(checks);

  const manifestMarkdown = [
    '# Browser Runtime Action Intent Topology Proof',
    '',
    '| Event Type | Publisher | Subscriber | Target | Status |',
    '| --- | --- | --- | --- | --- |',
    '| browser.action-intent.status.requested | browser-event-runtime-spine | browser-action-intent-status | browser-action-intent-status | covered |',
    '',
  ].join('\n');

  const proof = {
    proofName: 'browser-runtime-action-intent-topology-proof',
    branchHead: execFileSync('git', ['log', '-1', '--oneline'], { cwd: root, encoding: 'utf8' }).trim(),
    sourceChecks: checks,
    commands: commands.map((item) => `${item.command} ${item.args.join(' ')}`),
    topology: {
      eventType: 'browser.action-intent.status.requested',
      publisher: 'browser-event-runtime-spine',
      subscriber: 'browser-action-intent-status',
      target: 'browser-action-intent-status',
      status: 'covered',
    },
    verified: {
      reusableEventingTopologyManifestUsed: true,
      namedPublisherDeclared: true,
      namedSubscriberDeclared: true,
      noUnreadyTopologyEntries: true,
      newEventBusCreated: false,
      adapterDispatchClaimed: false,
      browserMutationClaimed: false,
      childInterventionClaimed: false,
      enforcementClaimed: false,
    },
  };

  await writeFile(path.join(proofDir, 'proof.json'), `${JSON.stringify(proof, null, 2)}\n`);
  await writeFile(path.join(outputDir, '01-browser-runtime-action-intent-topology-proof.md'), manifestMarkdown);

  console.log(JSON.stringify(proof, null, 2));
}

main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
