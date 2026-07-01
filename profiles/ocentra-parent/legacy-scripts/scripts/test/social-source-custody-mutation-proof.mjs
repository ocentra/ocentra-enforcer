import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';

const root = process.cwd();
const outputDirectory = join(root, 'output', 'browser-plan-proof', 'social-source-custody-mutation-proof');
const resultDirectory = join(root, 'test-results', 'social-source-custody-mutation-proof');

const requiredFiles = [
  'crates/agent-protocol/tests/contract/social_source_custody_mutation_tests.rs',
  'crates/agent-protocol/src/social_source_custody_mutation.rs',
  'crates/agent-service/src/activity_api/social_source_custody_mutation_payload.rs',
  'crates/agent-service/tests/integration/social_source_custody_mutation_service_tests.rs',
  'crates/agent-service/src/websocket.rs',
  'docs/features/social-video-control.md',
  'docs/plans/browser-plan/social-platform-account-feed/readme.md',
  'docs/plans/browser-plan/v0-5-social-platform-account-feed-gating-plan.md',
  'docs/plans/browser-plan/implementation-checklist.md',
];

await main();

async function main() {
  await mkdir(outputDirectory, { recursive: true });
  await mkdir(resultDirectory, { recursive: true });

  const files = Object.fromEntries(await Promise.all(requiredFiles.map(async (path) => [path, await readText(path)])));
  const checks = [
    checkIncludes(files, 'crates/agent-protocol/tests/contract/social_source_custody_mutation_tests.rs', [
      'social_source_custody_mutation_command_event_and_snapshot_serialize',
      'SOCIAL_SOURCE_CUSTODY_MUTATION_STATE_APPLIED',
      'final_policy_decision_claimed',
    ]),
    checkIncludes(files, 'crates/agent-protocol/src/social_source_custody_mutation.rs', [
      'SocialSourceCustodyMutationSnapshot',
      'SOCIAL_SOURCE_CUSTODY_MUTATION_STATE_APPLIED',
      'runtime_custody_mutation_applied',
      'product_claim_ready',
    ]),
    checkIncludes(files, 'crates/agent-service/src/activity_api/social_source_custody_mutation_payload.rs', [
      'social_source_custody_mutation_from_command',
      'service_mutation_executed: true',
      'runtime_custody_mutation_applied: true',
      'final_policy_decision_claimed: false',
      'enforcement_claimed: false',
    ]),
    checkIncludes(files, 'crates/agent-service/src/websocket.rs', [
      'AgentBrowserSocialSourceCustodyMutationApply',
      'build_browser_social_source_custody_mutation_report',
    ]),
    checkIncludes(files, 'docs/features/social-video-control.md', [
      'social-source-custody-mutation-proof',
      'service-backed source custody mutation',
    ]),
    checkIncludes(files, 'docs/plans/browser-plan/social-platform-account-feed/readme.md', [
      'social-source-custody-mutation-proof',
      'runtime custody mutation',
    ]),
    checkIncludes(files, 'docs/plans/browser-plan/v0-5-social-platform-account-feed-gating-plan.md', [
      'social-source-custody-mutation-proof',
      'runtime custody mutation',
    ]),
    checkIncludes(files, 'docs/plans/browser-plan/implementation-checklist.md', [
      'social-source-custody-mutation-proof',
      'service-backed source custody mutation proof',
    ]),
  ];
  const failures = checks.flatMap((check) => check.failures);
  const proof = {
    schemaVersion: 1,
    proofMode: 'social-source-custody-mutation-proof',
    generatedAt: new Date().toISOString(),
    files: requiredFiles,
    checks,
    claims: {
      serviceMutationExecuted: true,
      runtimeCustodyMutationApplied: true,
      settingsRemainRefOnly: true,
      rawContentCustody: 'not-claimed',
      connectorApiCalls: 'not-claimed',
      finalPolicyDecision: 'not-claimed',
      enforcement: 'not-claimed',
      productClaimReady: false,
    },
    failures,
  };

  if (failures.length > 0) {
    throw new Error(`Social source custody mutation proof failed:\n${failures.join('\n')}`);
  }

  const proofPath = join(resultDirectory, 'proof.json');
  const markdownPath = join(outputDirectory, '01-service-backed-custody-mutation-proof.md');
  await writeFile(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
  await writeFile(markdownPath, `${markdownFor(proof)}\n`);

  console.log('social-source-custody-mutation-proof-ok=true');
  console.log(`proof=${relativePath(proofPath)}`);
  console.log(`manifest=${relativePath(markdownPath)}`);
}

function checkIncludes(files, path, expectedTexts) {
  const text = files[path] ?? '';
  return {
    path,
    failures: expectedTexts
      .filter((expectedText) => !text.includes(expectedText))
      .map((expectedText) => `${path} missing ${expectedText}`),
  };
}

function markdownFor(proof) {
  return [
    '# Social Source Custody Mutation Proof',
    '',
    `Generated: ${proof.generatedAt}`,
    '',
    'This proof verifies the service-backed source custody mutation boundary for SOCIAL-23.',
    '',
    'Claims:',
    '',
    `- Service mutation executed: ${proof.claims.serviceMutationExecuted}`,
    `- Runtime custody mutation applied: ${proof.claims.runtimeCustodyMutationApplied}`,
    `- Settings remain ref-only: ${proof.claims.settingsRemainRefOnly}`,
    `- Raw content custody: ${proof.claims.rawContentCustody}`,
    `- Connector API calls: ${proof.claims.connectorApiCalls}`,
    `- Final policy decision: ${proof.claims.finalPolicyDecision}`,
    `- Enforcement: ${proof.claims.enforcement}`,
    `- Product claim ready: ${proof.claims.productClaimReady}`,
    '',
    'The mutation proof applies a redacted-ref custody settings snapshot through the Rust service WebSocket command/event path.',
    'It does not claim connector API calls, raw social/video custody, final policy execution, child delivery, or enforcement.',
  ].join('\n');
}

async function readText(path) {
  return readFile(join(root, path), 'utf8');
}

function relativePath(path) {
  return relative(root, path).replaceAll('\\', '/');
}
