import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { existsSync, mkdirSync, readFileSync, statSync, writeFileSync } from 'node:fs';
import { dirname, join, relative } from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright';
import { SocialPlatformConnectorAuthorizationBoundarySchema } from '../../packages/schema-domain/dist/social-platform-connector-authorization.js';

const scriptDir = dirname(fileURLToPath(import.meta.url));
const repoRoot = join(scriptDir, '..', '..');
const proofRoot = join(repoRoot, 'output/browser-plan-proof/social-18-platform-connector-authorization-boundary');
const screenshotRoot = join(proofRoot, '06-live-screenshots');
const testResultPath = join(repoRoot, 'test-results/social-platform-connector-authorization-proof/proof.json');
const outputProofPath = join(proofRoot, '11-live-public-connector-boundary-proof.json');
const observedAt = new Date().toISOString();

const builtFiles = [
  'packages/schema-domain/dist/social-platform-connector-authorization-values.js',
  'packages/schema-domain/dist/social-platform-connector-authorization.js',
];

const liveTargets = [
  {
    targetId: 'google-youtube-supervision-public-doc',
    provider: 'google-youtube-supervision',
    url: 'https://support.google.com/youtube/answer/10314940?hl=en',
    allowedHosts: ['support.google.com'],
    proofRef: 'parent-proof-google-youtube-supervision-public-doc',
  },
  {
    targetId: 'meta-family-center-public-page',
    provider: 'meta-family-center',
    url: 'https://familycenter.meta.com/',
    allowedHosts: ['familycenter.meta.com'],
    proofRef: 'parent-proof-meta-family-center-public-page',
  },
  {
    targetId: 'tiktok-family-pairing-public-doc',
    provider: 'tiktok-family-pairing',
    url: 'https://support.tiktok.com/en/account-and-privacy/account-privacy-settings/family-pairing',
    allowedHosts: ['support.tiktok.com', 'www.tiktok.com'],
    proofRef: 'parent-proof-tiktok-family-pairing-public-doc',
  },
];

assertBuiltContractsExist();
mkdirSync(screenshotRoot, { recursive: true });

const browser = await chromium.launch({ headless: true });
const captures = [];
try {
  for (const target of liveTargets) {
    captures.push(await capturePublicConnectorPage(browser, target));
  }
} finally {
  await browser.close();
}

const successfulCaptures = captures.filter((capture) => capture.screenshotCaptured);
if (successfulCaptures.length !== liveTargets.length) {
  throw new Error(
    `Expected screenshots for all SOCIAL-18 public connector targets, captured ${successfulCaptures.length} of ${liveTargets.length}`
  );
}
if (!successfulCaptures.every((capture) => capture.allowedHostObserved)) {
  throw new Error('Expected all SOCIAL-18 public connector targets to resolve to an allowed provider host');
}

const boundary = SocialPlatformConnectorAuthorizationBoundarySchema.parse(buildBoundary(successfulCaptures));
const negativeChecks = buildNegativeChecks(boundary);
if (!negativeChecks.every((check) => check.rejected)) {
  throw new Error('Expected SOCIAL-18 connector boundary negative checks to reject dishonest runtime/provider claims');
}

const proof = {
  schemaVersion: 1,
  proofId: 'social-platform-connector-authorization-proof',
  generatedAt: observedAt,
  branch: git(['branch', '--show-current']),
  commit: git(['rev-parse', 'HEAD']),
  baseCommit: git(['rev-parse', 'origin/main']),
  liveCaptureSummary: {
    realPublicProviderSurfacesUsed: true,
    generatedOrFixturePageUsed: false,
    passiveNavigationOnly: true,
    formsSubmitted: false,
    credentialsCaptured: false,
    rawPageBodyPersisted: false,
    rawDomPersisted: false,
    rawTitlePersisted: false,
    screenshotsPersisted: true,
    screenshotCount: successfulCaptures.length,
    providerApiCalled: false,
    oauthClientImplemented: false,
    tokenStored: false,
    rawAccountDataCaptured: false,
    messageContentCaptured: false,
    feedContentCaptured: false,
    accountIdentityVerified: false,
    coreGatingDependencyClaimed: false,
    policyDecisionClaimed: false,
    aiRuntimeClaimed: false,
    uiDeliveredClaimed: false,
    nativeAppControlClaimed: false,
    enforcementClaimed: false,
  },
  captures,
  authorizationBoundary: {
    authorizationBoundaryId: boundary.authorizationBoundaryId,
    generatedAt: boundary.generatedAt,
    rows: boundary.rows.map((row) => ({
      provider: row.provider,
      authorizationState: row.authorizationState,
      proofState: row.proofState,
      custodyState: row.custodyState,
      scopes: row.scopes,
      reasons: row.reasons,
      proofRefs: row.proofRefs,
      coreGatingDependency: row.coreGatingDependency,
      providerApiCallClaimed: row.providerApiCallClaimed,
      rawTokenStoredClaimed: row.rawTokenStoredClaimed,
      oauthClientImplementedClaimed: row.oauthClientImplementedClaimed,
      rawAccountDataCaptured: row.rawAccountDataCaptured,
      messageContentCaptured: row.messageContentCaptured,
      feedContentCaptured: row.feedContentCaptured,
      accountIdentityVerifiedClaimed: row.accountIdentityVerifiedClaimed,
      nativeAppControlClaimed: row.nativeAppControlClaimed,
      policyDecisionClaimed: row.policyDecisionClaimed,
      aiRuntimeClaimed: row.aiRuntimeClaimed,
      uiDeliveredClaimed: row.uiDeliveredClaimed,
      enforcementClaimed: row.enforcementClaimed,
    })),
    claimBoundaries: boundary.claimBoundaries,
  },
  parseChecks: {
    boundaryAccepted: true,
    requiredProvidersPresent: ['google-youtube-supervision', 'meta-family-center', 'tiktok-family-pairing'].every(
      (provider) => boundary.rows.some((row) => row.provider === provider)
    ),
    publicProofRefsAttached: successfulCaptures.every((capture) =>
      boundary.rows.some((row) => row.proofRefs.includes(capture.proofRef))
    ),
  },
  negativeChecks,
  noClaimChecks: {
    providerApiCalls: false,
    oauthClient: false,
    tokenStorage: false,
    rawAccountData: false,
    messageContent: false,
    feedContent: false,
    accountIdentityVerification: false,
    coreGatingDependency: false,
    policyDecision: false,
    aiRuntime: false,
    uiDelivery: false,
    nativeAppControl: false,
    enforcement: false,
  },
};

writeJson(testResultPath, proof);
writeJson(outputProofPath, proof);

console.log('social-platform-connector-authorization-proof-ok=true');
console.log(`proof=${relativePath(testResultPath)}`);
console.log(`outputProof=${relativePath(outputProofPath)}`);
console.log(`screenshotCount=${successfulCaptures.length}`);
console.log(`providers=${successfulCaptures.map((capture) => capture.provider).join(',')}`);

async function capturePublicConnectorPage(browserInstance, target) {
  const page = await browserInstance.newPage({
    viewport: { width: 1280, height: 900 },
    userAgent:
      'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/125 Safari/537.36',
  });
  try {
    const response = await page.goto(target.url, { waitUntil: 'domcontentloaded', timeout: 45000 });
    await page.waitForTimeout(2500);
    const finalUrl = page.url();
    const finalHost = new URL(finalUrl).host;
    const title = await page.title();
    const screenshotPath = join(screenshotRoot, `${target.targetId}.png`);
    await page.screenshot({ path: screenshotPath, fullPage: false });
    return {
      targetId: target.targetId,
      provider: target.provider,
      requestedUrl: target.url,
      finalUrl,
      allowedHosts: target.allowedHosts,
      finalHost,
      allowedHostObserved: target.allowedHosts.some((host) => finalHost === host || finalHost.endsWith(`.${host}`)),
      httpStatus: response ? response.status() : null,
      proofRef: target.proofRef,
      titleSha256: sha256(title),
      rawTitlePersisted: false,
      screenshotPath: relativePath(screenshotPath),
      screenshotSha256: sha256File(screenshotPath),
      screenshotBytes: statSync(screenshotPath).size,
      screenshotCaptured: true,
      rawPageBodyPersisted: false,
      rawDomPersisted: false,
      providerApiCalled: false,
      oauthClientImplemented: false,
      tokenStored: false,
      accountDataCaptured: false,
      enforcementClaimed: false,
    };
  } finally {
    await page.close();
  }
}

function buildBoundary(capturedTargets) {
  return {
    schemaVersion: 'social-platform-connector-authorization-boundary',
    authorizationBoundaryId: 'social-connector-boundary-live-public-proof',
    familyId: 'family-social-connector',
    childProfileId: 'child-social-connector',
    generatedAt: observedAt,
    rows: [...providerRows(capturedTargets), manualExportRow(), parentProvidedRow()],
    claimBoundaries: {
      tokenStorage: 'not-claimed',
      oauthClient: 'not-claimed',
      providerApiCalls: 'not-claimed',
      rawAccountData: 'not-claimed',
      messageContent: 'not-claimed',
      feedContent: 'not-claimed',
      accountIdentityVerification: 'not-claimed',
      coreGatingDependency: 'not-claimed',
      policyDecision: 'not-claimed',
      aiRuntime: 'not-claimed',
      uiDelivery: 'not-claimed',
      nativeAppControl: 'not-claimed',
      enforcement: 'not-claimed',
      reviewerSummary:
        'Live public connector pages prove only adjacent provider boundary visibility; OAuth, tokens, APIs, UI delivery, and enforcement remain unclaimed.',
    },
  };
}

function providerRows(capturedTargets) {
  return [
    providerRow('google-youtube-supervision', ['account-supervision-state', 'video-channel-metadata'], capturedTargets),
    providerRow('meta-family-center', ['family-center-state'], capturedTargets),
    providerRow('tiktok-family-pairing', ['family-pairing-state'], capturedTargets),
  ];
}

function providerRow(provider, scopes, capturedTargets) {
  const capture = capturedTargets.find((target) => target.provider === provider);
  return connectorRow(provider, {
    authorizationState: 'not-implemented',
    proofState: 'provider-artifact-required',
    custodyState: 'parent-owned-token-required',
    scopes,
    reasons: [
      'optional-adjacent-source',
      'parent-authorization-required',
      'provider-api-not-implemented',
      'token-storage-not-implemented',
      'core-gating-independent',
      'message-content-unavailable',
      'feed-content-unavailable',
    ],
    proofRefs: [capture.proofRef],
  });
}

function manualExportRow() {
  return connectorRow('platform-export-import', {
    authorizationState: 'manual-required',
    proofState: 'manual-export-required',
    custodyState: 'manual-export-required',
    scopes: ['manual-export-file'],
    reasons: ['manual-export-required', 'core-gating-independent'],
    proofRefs: ['parent-proof-platform-export-import-manual-required'],
  });
}

function parentProvidedRow() {
  return connectorRow('parent-provided-account-ref', {
    authorizationState: 'parent-authorized',
    proofState: 'parent-consent-record-only',
    custodyState: 'redacted-parent-input-only',
    scopes: ['parent-declared-account-ref'],
    reasons: [
      'optional-adjacent-source',
      'visible-setting-required',
      'redacted-input-required',
      'core-gating-independent',
      'message-content-unavailable',
      'feed-content-unavailable',
    ],
    proofRefs: ['parent-proof-redacted-parent-provided-account-ref'],
    authorizedByActorId: 'parent-actor-social-connector',
    authorizedAt: observedAt,
    expiresAt: new Date(Date.parse(observedAt) + 30 * 24 * 60 * 60 * 1000).toISOString(),
    visibleParentSettingRef: 'parent-visible-setting-social-connector',
  });
}

function connectorRow(provider, overrides) {
  return {
    provider,
    authorizationState: 'manual-required',
    proofState: 'provider-artifact-required',
    custodyState: 'not-applicable',
    scopes: ['account-supervision-state'],
    reasons: ['optional-adjacent-source'],
    proofRefs: [`parent-proof-${provider}`],
    authorizedByActorId: null,
    authorizedAt: null,
    expiresAt: null,
    revokedAt: null,
    visibleParentSettingRef: null,
    coreGatingDependency: 'not-required',
    rawTokenStoredClaimed: false,
    oauthClientImplementedClaimed: false,
    providerApiCallClaimed: false,
    rawAccountDataCaptured: false,
    messageContentCaptured: false,
    feedContentCaptured: false,
    accountIdentityVerifiedClaimed: false,
    nativeAppControlClaimed: false,
    policyDecisionClaimed: false,
    aiRuntimeClaimed: false,
    uiDeliveredClaimed: false,
    enforcementClaimed: false,
    ...overrides,
  };
}

function buildNegativeChecks(boundary) {
  const runtimeMutations = [
    ['raw-token-storage', { rawTokenStoredClaimed: true }],
    ['oauth-client', { oauthClientImplementedClaimed: true }],
    ['provider-api-call', { providerApiCallClaimed: true }],
    ['raw-account-data', { rawAccountDataCaptured: true }],
    ['message-content', { messageContentCaptured: true }],
    ['feed-content', { feedContentCaptured: true }],
    ['account-identity', { accountIdentityVerifiedClaimed: true }],
    ['native-app-control', { nativeAppControlClaimed: true }],
    ['policy-decision', { policyDecisionClaimed: true }],
    ['ai-runtime', { aiRuntimeClaimed: true }],
    ['ui-delivery', { uiDeliveredClaimed: true }],
    ['enforcement', { enforcementClaimed: true }],
  ];

  const checks = runtimeMutations.map(([mutation, invalid]) => ({
    mutation,
    rejected: !SocialPlatformConnectorAuthorizationBoundarySchema.safeParse({
      ...boundary,
      rows: boundary.rows.map((row) => (row.provider === 'google-youtube-supervision' ? { ...row, ...invalid } : row)),
    }).success,
  }));

  checks.push({
    mutation: 'provider-authorized-with-public-page-only',
    rejected: !SocialPlatformConnectorAuthorizationBoundarySchema.safeParse({
      ...boundary,
      rows: boundary.rows.map((row) =>
        row.provider === 'meta-family-center'
          ? {
              ...row,
              authorizationState: 'parent-authorized',
              proofState: 'parent-consent-record-only',
              authorizedByActorId: 'parent-actor-social-connector',
              authorizedAt: observedAt,
              visibleParentSettingRef: 'parent-visible-setting-social-connector',
            }
          : row
      ),
    }).success,
  });

  return checks;
}

function assertBuiltContractsExist() {
  const missing = builtFiles.filter((path) => !existsSync(join(repoRoot, path)));
  if (missing.length > 0) {
    throw new Error(`Run cmd /c npm run build:contracts before this proof. Missing: ${missing.join(', ')}`);
  }
}

function git(args) {
  return execFileSync('git', args, { cwd: repoRoot, encoding: 'utf8' }).trim();
}

function sha256(value) {
  return createHash('sha256').update(value).digest('hex');
}

function sha256File(path) {
  return createHash('sha256').update(readFileSync(path)).digest('hex');
}

function writeJson(path, value) {
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, `${JSON.stringify(value, null, 2)}\n`);
}

function relativePath(path) {
  return relative(repoRoot, path).replaceAll('\\', '/');
}
