import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';

const root = process.cwd();
const outputDirectory = join(root, 'output', 'browser-plan-proof', 'social-parent-sensitivity-settings-proof');
const resultDirectory = join(root, 'test-results', 'social-parent-sensitivity-settings-proof');

const requiredFiles = [
  'packages/schema-domain/src/social-parent-sensitivity-settings.ts',
  'packages/schema-domain/src/social-parent-sensitivity-settings-values.ts',
  'packages/schema-domain/tests/unit/social-parent-sensitivity-settings.test.ts',
  'docs/features/social-video-control.md',
  'docs/plans/browser-plan/social-platform-account-feed/readme.md',
  'docs/plans/browser-plan/v0-5-social-platform-account-feed-gating-plan.md',
];

await main();

async function main() {
  await mkdir(outputDirectory, { recursive: true });
  await mkdir(resultDirectory, { recursive: true });

  const files = Object.fromEntries(await Promise.all(requiredFiles.map(async (path) => [path, await readText(path)])));
  const checks = [
    checkIncludes(files, 'packages/schema-domain/src/social-parent-sensitivity-settings.ts', [
      'SocialParentSensitivitySettingSchema',
      'PolicyCandidateInput',
      'ConnectorAuthorizationRefOnly',
      'rawMessageContentAllowed',
      'finalPolicyDecisionClaimed',
      'enforcementClaimed',
    ]),
    checkIncludes(files, 'packages/schema-domain/tests/unit/social-parent-sensitivity-settings.test.ts', [
      'accepts a redacted local sensitivity setting as policy candidate input',
      'rejects raw content connector token API UI final-policy and enforcement claims',
      'rejects manual-required and unavailable rows that pretend to be policy candidate input',
    ]),
    checkIncludes(files, 'docs/features/social-video-control.md', [
      'social-parent-sensitivity-settings-proof',
      'runtime settings UI',
      'final policy',
    ]),
    checkIncludes(files, 'docs/plans/browser-plan/social-platform-account-feed/readme.md', [
      'social-parent-sensitivity-settings-proof',
      'parent sensitivity',
    ]),
    checkIncludes(files, 'docs/plans/browser-plan/v0-5-social-platform-account-feed-gating-plan.md', [
      'social-parent-sensitivity-settings-proof',
      'Parent sensitivity settings',
    ]),
  ];
  const failures = checks.flatMap((check) => check.failures);
  const proof = {
    schemaVersion: 1,
    proofMode: 'social-parent-sensitivity-settings-proof',
    generatedAt: new Date().toISOString(),
    files: requiredFiles,
    checks,
    claims: {
      parentSensitivityContract: 'proof-present',
      policyCandidateInput: 'ref-only-contract-proof-present',
      sourcePrivacyRefs: 'required',
      aiSignalAggregateRefs: 'required',
      scheduleTimeBudgetRefs: 'required-for-policy-input',
      runtimeSettingsUi: 'not-claimed',
      rawContentCustody: 'not-claimed',
      connectorApiCalls: 'not-claimed',
      finalPolicyDecision: 'not-claimed',
      enforcement: 'not-claimed',
    },
    failures,
  };

  if (failures.length > 0) {
    throw new Error(`Social parent sensitivity settings proof failed:\n${failures.join('\n')}`);
  }

  const proofPath = join(resultDirectory, 'proof.json');
  const markdownPath = join(outputDirectory, '01-social-parent-sensitivity-settings-proof.md');
  await writeFile(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
  await writeFile(markdownPath, `${markdownFor(proof)}\n`);

  console.log('social-parent-sensitivity-settings-proof-ok=true');
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
    '# Social Parent Sensitivity Settings Proof',
    '',
    `Generated: ${proof.generatedAt}`,
    '',
    'This proof verifies the centralized schema-domain social sensitivity settings contract.',
    '',
    'Claims:',
    '',
    `- Parent sensitivity contract: ${proof.claims.parentSensitivityContract}`,
    `- Policy candidate input: ${proof.claims.policyCandidateInput}`,
    `- Runtime settings UI: ${proof.claims.runtimeSettingsUi}`,
    `- Raw content custody: ${proof.claims.rawContentCustody}`,
    `- Connector API calls: ${proof.claims.connectorApiCalls}`,
    `- Final policy decision: ${proof.claims.finalPolicyDecision}`,
    `- Enforcement: ${proof.claims.enforcement}`,
    '',
    'The contract requires source/privacy refs, AI signal aggregate refs, dashboard refs, evidence refs,',
    'and schedule/time-budget refs before a contract-only sensitivity row can feed policy candidate input.',
    'Manual-required and unavailable rows cannot feed policy input.',
  ].join('\n');
}

async function readText(path) {
  return readFile(join(root, path), 'utf8');
}

function relativePath(path) {
  return relative(root, path).replaceAll('\\', '/');
}
