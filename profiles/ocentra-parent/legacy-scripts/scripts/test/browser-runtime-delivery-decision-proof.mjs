import { execFileSync } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';

const root = process.cwd();
const proofDir = path.join(root, 'test-results', 'browser-runtime-delivery-decision-proof');
const outputDir = path.join(root, 'output', 'browser-plan-proof', 'browser-runtime-delivery-decision');

const files = {
  delivery: path.join(root, 'crates', 'agent-core', 'src', 'browser_event_runtime', 'delivery.rs'),
  runtime: path.join(root, 'crates', 'agent-core', 'src', 'browser_event_runtime.rs'),
  tests: path.join(root, 'crates', 'agent-core', 'src', 'browser_event_runtime_tests.rs'),
  checklist: path.join(root, 'docs', 'plans', 'browser-plan', 'implementation-checklist.md'),
  feature: path.join(root, 'docs', 'features', 'browser-web-control.md'),
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
  const [delivery, runtime, tests, checklist, feature, workpack] = await Promise.all([
    readFile(files.delivery, 'utf8'),
    readFile(files.runtime, 'utf8'),
    readFile(files.tests, 'utf8'),
    readFile(files.checklist, 'utf8'),
    readFile(files.feature, 'utf8'),
    readFile(files.workpack, 'utf8'),
  ]);
  return {
    usesSharedEventingDecisionApi: delivery.includes('decide_event_delivery_route'),
    provesLocalServiceRoute: delivery.includes('EventDeliveryRouteKind::LocalService'),
    provesLocalInProcessRoute: delivery.includes('EventDeliveryRouteKind::LocalInProcess'),
    provesActionIntentHandoffRoute:
      delivery.includes('action_intent_handoff_delivery') &&
      delivery.includes('constants::browser::EVENT_BROWSER_ACTION_INTENT_HANDOFF_REQUESTED') &&
      delivery.includes('constants::browser::SUBSCRIBER_BROWSER_ACTION_INTENT_HANDOFF') &&
      delivery.includes('constants::browser::TARGET_BROWSER_ACTION_INTENT_HANDOFF'),
    provesRuntimeStreamReportRoute:
      delivery.includes('runtime_stream_report_delivery') &&
      delivery.includes('constants::browser::EVENT_BROWSER_RUNTIME_STREAM_REPORT_REQUESTED') &&
      delivery.includes('constants::browser::SUBSCRIBER_BROWSER_RUNTIME_STREAM_REPORT') &&
      delivery.includes('constants::browser::TARGET_BROWSER_RUNTIME_STREAM_REPORT'),
    provesSocialProviderReceiptStatusRoute:
      delivery.includes('social_provider_receipt_status_delivery') &&
      delivery.includes('constants::browser::EVENT_BROWSER_SOCIAL_PROVIDER_RECEIPT_STATUS_REQUESTED') &&
      delivery.includes('constants::browser::SUBSCRIBER_BROWSER_SOCIAL_PROVIDER_RECEIPT_STATUS') &&
      delivery.includes('constants::browser::TARGET_BROWSER_SOCIAL_PROVIDER_RECEIPT_STATUS'),
    provesSocialReportWriterDeliveryStatusRoute:
      delivery.includes('social_report_writer_delivery_status_delivery') &&
      delivery.includes('constants::browser::EVENT_BROWSER_SOCIAL_REPORT_WRITER_DELIVERY_STATUS_REQUESTED') &&
      delivery.includes('constants::browser::SUBSCRIBER_BROWSER_SOCIAL_REPORT_WRITER_DELIVERY_STATUS') &&
      delivery.includes('constants::browser::TARGET_BROWSER_SOCIAL_REPORT_WRITER_DELIVERY_STATUS'),
    provesSocialParentNotificationDeliveryStatusRoute:
      delivery.includes('social_parent_notification_delivery_status_delivery') &&
      delivery.includes('constants::browser::EVENT_BROWSER_SOCIAL_PARENT_NOTIFICATION_DELIVERY_STATUS_REQUESTED') &&
      delivery.includes('constants::browser::SUBSCRIBER_BROWSER_SOCIAL_PARENT_NOTIFICATION_DELIVERY_STATUS') &&
      delivery.includes('constants::browser::TARGET_BROWSER_SOCIAL_PARENT_NOTIFICATION_DELIVERY_STATUS'),
    provesSocialParentSurfaceStatusRoute:
      delivery.includes('social_parent_surface_status_delivery') &&
      delivery.includes('constants::browser::EVENT_BROWSER_SOCIAL_ALERT_REPORT_PARENT_SURFACE_STATUS_REQUESTED') &&
      delivery.includes('constants::browser::SUBSCRIBER_BROWSER_SOCIAL_ALERT_REPORT_PARENT_SURFACE_STATUS') &&
      delivery.includes('constants::browser::TARGET_BROWSER_SOCIAL_ALERT_REPORT_PARENT_SURFACE_STATUS'),
    provesExternalTransportManualRequired: delivery.includes('ExternalTransportRouteManualRequired'),
    rejectsExecutionClaims:
      delivery.includes('adapter_dispatch_claimed: false') &&
      delivery.includes('browser_mutation_claimed: false') &&
      delivery.includes('child_intervention_execution_claimed: false') &&
      delivery.includes('final_policy_execution_claimed: false') &&
      delivery.includes('enforcement_claimed: false'),
    runtimeExportsProof:
      runtime.includes('prove_browser_runtime_delivery_decision') &&
      runtime.includes('BrowserRuntimeDeliveryDecisionReport'),
    focusedTestExists: tests.includes('browser_runtime_delivery_decision_keeps_current_routes_local_only'),
    focusedTestAssertsEightLocalRoutes: tests.includes('assert_eq!(report.local_ready_route_count, 8)'),
    focusedTestAssertsHandoffRoute:
      tests.includes('report.action_intent_handoff_delivery') &&
      tests.includes('EventDeliveryDecisionState::LocalRouteReady'),
    focusedTestAssertsReceiptStatusRoute:
      tests.includes('report.social_provider_receipt_status_delivery') &&
      tests.includes('EventDeliveryDecisionState::LocalRouteReady'),
    focusedTestAssertsReportWriterDeliveryStatusRoute:
      tests.includes('report.social_report_writer_delivery_status_delivery') &&
      tests.includes('EventDeliveryDecisionState::LocalRouteReady'),
    focusedTestAssertsParentNotificationDeliveryStatusRoute:
      tests.includes('report.social_parent_notification_delivery_status_delivery') &&
      tests.includes('EventDeliveryDecisionState::LocalRouteReady'),
    focusedTestAssertsParentSurfaceStatusRoute:
      tests.includes('report.social_parent_surface_status_delivery') &&
      tests.includes('EventDeliveryDecisionState::LocalRouteReady'),
    focusedTestAssertsRuntimeStreamReportRoute:
      tests.includes('report.runtime_stream_report_delivery') &&
      tests.includes('EventDeliveryDecisionState::LocalRouteReady'),
    testAssertsMissingArtifacts: tests.includes('EventDeliveryRequiredArtifact::TransportConfig'),
    checklistMentionsProof: checklist.includes('browser-runtime-delivery-decision-proof'),
    featureMentionsProof: feature.includes('browser runtime delivery-decision proof'),
    workpackMentionsProof: workpack.includes('Delivery Decision Addendum'),
  };
}

function assertSourceChecks(checks) {
  const missing = Object.entries(checks)
    .filter(([, ok]) => !ok)
    .map(([name]) => name);
  if (missing.length > 0) {
    throw new Error(`browser runtime delivery-decision proof failed: ${missing.join(', ')}`);
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
        'browser_runtime_delivery_decision_keeps_current_routes_local_only',
        '--quiet',
      ],
    },
  ];

  for (const item of commands) {
    run(item.command, item.args);
  }

  const checks = await sourceChecks();
  assertSourceChecks(checks);

  const proof = {
    proofName: 'browser-runtime-delivery-decision-proof',
    branchHead: execFileSync('git', ['log', '-1', '--oneline'], {
      cwd: root,
      encoding: 'utf8',
    }).trim(),
    sourceChecks: checks,
    commands: commands.map((item) => `${item.command} ${item.args.join(' ')}`),
    deliveryDecisions: {
      browserRuntimeChain: {
        routeKind: 'local-service',
        decisionState: 'local-route-ready',
        publisher: 'browser-event-runtime-spine',
        subscriber: 'browser-read-model',
      },
      browserActionIntentStatus: {
        routeKind: 'local-in-process',
        decisionState: 'local-route-ready',
        publisher: 'browser-event-runtime-spine',
        subscriber: 'browser-action-intent-status',
      },
      browserActionIntentHandoff: {
        routeKind: 'local-in-process',
        decisionState: 'local-route-ready',
        publisher: 'browser-event-runtime-spine',
        subscriber: 'browser-action-intent-handoff',
      },
      browserRuntimeStreamReport: {
        routeKind: 'local-in-process',
        decisionState: 'local-route-ready',
        publisher: 'browser-event-runtime-spine',
        subscriber: 'browser-runtime-stream-report',
      },
      browserSocialProviderReceiptStatus: {
        routeKind: 'local-in-process',
        decisionState: 'local-route-ready',
        publisher: 'browser-event-runtime-spine',
        subscriber: 'browser-social-provider-receipt-status',
      },
      browserSocialReportWriterDeliveryStatus: {
        routeKind: 'local-in-process',
        decisionState: 'local-route-ready',
        publisher: 'browser-event-runtime-spine',
        subscriber: 'browser-social-report-writer-delivery-status',
      },
      browserSocialParentNotificationDeliveryStatus: {
        routeKind: 'local-in-process',
        decisionState: 'local-route-ready',
        publisher: 'browser-event-runtime-spine',
        subscriber: 'browser-social-parent-notification-delivery-status',
      },
      browserSocialParentSurfaceStatus: {
        routeKind: 'local-in-process',
        decisionState: 'local-route-ready',
        publisher: 'browser-event-runtime-spine',
        subscriber: 'browser-social-alert-report-parent-surface-status',
      },
      browserExternalTransport: {
        routeKind: 'external-transport',
        decisionState: 'external-transport-route-manual-required',
        missingArtifacts: [
          'custody-proof',
          'publisher-auth-proof',
          'subscriber-auth-proof',
          'encryption-proof',
          'retention-policy',
          'replay-plan',
          'deletion-plan',
          'offset-policy',
          'dedupe-policy',
          'transport-config',
        ],
      },
    },
    verified: {
      reusableEventingDeliveryDecisionUsed: true,
      localServiceRouteReady: true,
      localInProcessRouteReady: true,
      localReadyRouteCount: 8,
      actionIntentStatusRouteReady: true,
      actionIntentHandoffRouteReady: true,
      runtimeStreamReportRouteReady: true,
      socialProviderReceiptStatusRouteReady: true,
      socialReportWriterDeliveryStatusRouteReady: true,
      socialParentNotificationDeliveryStatusRouteReady: true,
      socialParentSurfaceStatusRouteReady: true,
      externalTransportManualRequired: true,
      externalTransportDeliveryImplemented: false,
      externalRelayDeliveryImplemented: false,
      adapterDispatchClaimed: false,
      browserMutationClaimed: false,
      childInterventionExecutionClaimed: false,
      finalPolicyExecutionClaimed: false,
      enforcementClaimed: false,
    },
  };

  const markdown = [
    '# Browser Runtime Delivery Decision Proof',
    '',
    '| Boundary | Route | Decision | Subscriber | Status |',
    '| --- | --- | --- | --- | --- |',
    '| browser runtime event chain | local-service | local-route-ready | browser-read-model | covered |',
    '| browser action-intent status | local-in-process | local-route-ready | browser-action-intent-status | covered |',
    '| browser action-intent handoff | local-in-process | local-route-ready | browser-action-intent-handoff | covered |',
    '| browser runtime stream report | local-in-process | local-route-ready | browser-runtime-stream-report | covered |',
    '| browser social-provider receipt status | local-in-process | local-route-ready | browser-social-provider-receipt-status | covered |',
    '| browser social report-writer delivery status | local-in-process | local-route-ready | browser-social-report-writer-delivery-status | covered |',
    '| browser social parent-notification delivery status | local-in-process | local-route-ready | browser-social-parent-notification-delivery-status | covered |',
    '| browser social alert/report parent-surface status | local-in-process | local-route-ready | browser-social-alert-report-parent-surface-status | covered |',
    '| browser external transport | external-transport | external-transport-route-manual-required | browser-intervention-command | manual-required |',
    '',
    'The proof uses the reusable `ocentra-eventing` delivery decision API. External transport and relay delivery remain unimplemented, and the proof does not claim adapter dispatch, browser mutation, child intervention execution, final policy execution, or enforcement.',
    '',
  ].join('\n');

  await writeFile(path.join(proofDir, 'proof.json'), `${JSON.stringify(proof, null, 2)}\n`);
  await writeFile(path.join(outputDir, '01-browser-runtime-delivery-decision-proof.md'), markdown);

  console.log(JSON.stringify(proof, null, 2));
}

main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
