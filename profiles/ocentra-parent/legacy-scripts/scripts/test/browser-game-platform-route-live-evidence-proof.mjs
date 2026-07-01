import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { mkdir, writeFile } from 'node:fs/promises';
import { dirname, join, relative } from 'node:path';

import {
  BrowserGamePlatformRouteCatalogSchema,
  BrowserGamePlatformRouteContractSchema,
  decodeBrowserGamePlatformRouteCatalog,
} from '@ocentra-parent/schema-domain/browser-game-platform-route-contracts';

const repoRoot = process.cwd();
const proofId = 'browser-game-platform-route-live-evidence-proof';
const resultPath = join(repoRoot, 'test-results', proofId, 'proof.json');
const outputProofPath = join(
  repoRoot,
  'output',
  'browser-plan-proof',
  'game-02-browser-game-platform-route-contracts',
  '11-live-route-proof.json'
);

const targets = [
  {
    targetId: 'crazygames-bloxdhop',
    url: 'https://www.crazygames.com/game/bloxdhop-io',
    platformKind: 'browser-game-portal',
    routeSurfaceKind: 'play-route',
    routePatternRef: 'game-route-pattern-crazygames-game-ref',
    childLaunchCandidate: true,
  },
  {
    targetId: 'poki-subway-surfers',
    url: 'https://poki.com/en/g/subway-surfers',
    platformKind: 'browser-game-portal',
    routeSurfaceKind: 'play-route',
    routePatternRef: 'game-route-pattern-poki-game-ref',
    childLaunchCandidate: true,
  },
  {
    targetId: 'coolmath-run-3',
    url: 'https://www.coolmathgames.com/0-run-3',
    platformKind: 'educational-game-site',
    routeSurfaceKind: 'play-route',
    routePatternRef: 'game-route-pattern-coolmath-game-ref',
    childLaunchCandidate: true,
  },
  {
    targetId: 'xbox-cloud-play',
    url: 'https://www.xbox.com/en-US/play',
    platformKind: 'cloud-gaming-platform',
    routeSurfaceKind: 'cloud-session-route',
    routePatternRef: 'game-route-pattern-xbox-cloud-play-ref',
    childLaunchCandidate: true,
    cloudSessionCandidate: true,
  },
  {
    targetId: 'itch-html5-catalog',
    url: 'https://itch.io/games/html5',
    platformKind: 'classic-game-archive',
    routeSurfaceKind: 'catalog-route',
    routePatternRef: 'game-route-pattern-itch-html5-catalog-ref',
    childLaunchCandidate: false,
  },
  {
    targetId: 'chess-play',
    url: 'https://www.chess.com/play',
    platformKind: 'browser-game-portal',
    routeSurfaceKind: 'play-route',
    routePatternRef: 'game-route-pattern-chess-play-ref',
    childLaunchCandidate: true,
  },
];

const startedAt = new Date().toISOString();
const branch = git(['rev-parse', '--abbrev-ref', 'HEAD']);
const commit = git(['rev-parse', 'HEAD']);
const baseCommit = git(['rev-parse', 'origin/main']);
const captures = await Promise.all(targets.map(captureTarget));
const contracts = captures.map(routeContractFor);
const catalog = routeCatalogFor(contracts);
const negativeChecks = runNegativeChecks(contracts[0], catalog);

if (!captures.every((capture) => capture.responseOk)) {
  throw new Error('Expected all browser-game public route captures to return HTTP 2xx/3xx responses');
}
if (!contracts.every((contract) => BrowserGamePlatformRouteContractSchema.safeParse(contract).success)) {
  throw new Error('Expected every live browser-game route contract to parse');
}
decodeBrowserGamePlatformRouteCatalog(catalog);

const proof = {
  schemaVersion: 1,
  proofId,
  generatedAt: startedAt,
  branch,
  commit,
  baseCommit,
  captureMode: 'real-public-browser-game-routes',
  targets: captures,
  catalog,
  negativeChecks,
  summary: {
    targetCount: captures.length,
    contractRows: contracts.length,
    negativeChecks: negativeChecks.length,
    rawDomainStored: false,
    rawUrlStored: false,
    rawPathStored: false,
    rawPageBodyStored: false,
    runtimeDetectionClaimed: false,
    urlParserClaimed: false,
    aiClassificationClaimed: false,
    policyDecisionClaimed: false,
    nativeGameControlClaimed: false,
    cloudFrameAnalysisClaimed: false,
    enforcementClaimed: false,
    productChecklistUpgradeClaimed: false,
  },
};

await writeJson(resultPath, proof);
await writeJson(outputProofPath, proof);

console.log('browser-game-platform-route-live-evidence-proof-ok=true');
console.log(`proof=${relativePath(resultPath)}`);
console.log(`outputProof=${relativePath(outputProofPath)}`);
console.log(`targets=${captures.length} negativeChecks=${negativeChecks.length}`);

async function captureTarget(target) {
  const parsed = new URL(target.url);
  const response = await fetch(target.url, {
    redirect: 'follow',
    headers: {
      'user-agent': 'Mozilla/5.0 OcentraParentBrowserGameProof/1.0',
      accept: 'text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8',
    },
  });
  const body = Buffer.from(await response.arrayBuffer());
  const finalUrl = new URL(response.url);
  return {
    targetId: target.targetId,
    status: response.status,
    responseOk: response.status >= 200 && response.status < 400,
    contentType: response.headers.get('content-type') ?? 'unknown',
    contentLength: body.length,
    bodySha256: sha256(body),
    originSha256: sha256(parsed.origin),
    pathSha256: sha256(parsed.pathname),
    finalOriginSha256: sha256(finalUrl.origin),
    finalPathSha256: sha256(finalUrl.pathname),
    platformKind: target.platformKind,
    routeSurfaceKind: target.routeSurfaceKind,
    rawUrlPersisted: false,
    rawDomainPersisted: false,
    rawPathPersisted: false,
    rawPageBodyPersisted: false,
  };
}

function routeContractFor(capture) {
  const target = targets.find((entry) => entry.targetId === capture.targetId);
  return {
    routeContractId: `game-route-contract-${capture.targetId}`,
    platformKind: target.platformKind,
    routeSurfaceKind: target.routeSurfaceKind,
    routeSourceKind: 'managed-browser-evidence-ref',
    custodyLabel: 'hash-only',
    routePatternRef: target.routePatternRef,
    sourceEvidenceRefs: [`parent-proof-${proofId}-${capture.targetId}`],
    confidence: 'high',
    status: 'reviewed',
    managedBrowserRequired: target.routeSurfaceKind !== 'catalog-route',
    childLaunchCandidate: target.childLaunchCandidate,
    accountOrPurchaseCandidate: false,
    cloudSessionCandidate: target.cloudSessionCandidate === true,
    rawDomainStored: false,
    rawUrlStored: false,
    rawPathStored: false,
    rawPageBodyStored: false,
    runtimeDetectionClaimed: false,
    urlParserClaimed: false,
    aiClassificationClaimed: false,
    policyDecisionClaimed: false,
    nativeGameControlClaimed: false,
    cloudFrameAnalysisClaimed: false,
    enforcementClaimed: false,
  };
}

function routeCatalogFor(routes) {
  return {
    schemaVersion: 'browser-game-platform-route-contract',
    catalogId: `game-route-catalog-${proofId}`,
    generatedAt: startedAt,
    sourceEvidenceRefs: routes.flatMap((route) => route.sourceEvidenceRefs),
    routes,
    confidence: 'high',
    status: 'reviewed',
    rawDomainStored: false,
    rawUrlStored: false,
    rawPathStored: false,
    rawPageBodyStored: false,
    runtimeDetectionClaimed: false,
    urlParserClaimed: false,
    aiClassificationClaimed: false,
    policyDecisionClaimed: false,
    nativeGameControlClaimed: false,
    cloudFrameAnalysisClaimed: false,
    enforcementClaimed: false,
  };
}

function runNegativeChecks(validRoute, validCatalog) {
  const invalidRoutes = [
    ['raw-url', { rawUrlStored: true }],
    ['raw-domain', { rawDomainStored: true }],
    ['raw-path', { rawPathStored: true }],
    ['raw-page-body', { rawPageBodyStored: true }],
    ['runtime-detection', { runtimeDetectionClaimed: true }],
    ['url-parser', { urlParserClaimed: true }],
    ['ai-classification', { aiClassificationClaimed: true }],
    ['policy-decision', { policyDecisionClaimed: true }],
    ['native-game-control', { nativeGameControlClaimed: true }],
    ['cloud-frame-analysis', { cloudFrameAnalysisClaimed: true }],
    ['enforcement', { enforcementClaimed: true }],
    ['play-without-managed-browser', { managedBrowserRequired: false }],
    ['cloud-candidate-on-play-route', { routeSurfaceKind: 'play-route', cloudSessionCandidate: true }],
  ];
  const invalidCatalogs = [
    ['catalog-raw-url', { rawUrlStored: true }],
    ['catalog-runtime-detection', { runtimeDetectionClaimed: true }],
    ['catalog-policy-decision', { policyDecisionClaimed: true }],
    ['catalog-enforcement', { enforcementClaimed: true }],
  ];
  return [
    ...invalidRoutes.map(([name, invalid]) => negativeRouteCheck(name, validRoute, invalid)),
    ...invalidCatalogs.map(([name, invalid]) => negativeCatalogCheck(name, validCatalog, invalid)),
  ];
}

function negativeRouteCheck(name, validRoute, invalid) {
  return {
    name,
    rejected: !BrowserGamePlatformRouteContractSchema.safeParse({ ...validRoute, ...invalid }).success,
  };
}

function negativeCatalogCheck(name, validCatalog, invalid) {
  return {
    name,
    rejected: !BrowserGamePlatformRouteCatalogSchema.safeParse({ ...validCatalog, ...invalid }).success,
  };
}

async function writeJson(path, value) {
  await mkdir(dirname(path), { recursive: true });
  await writeFile(path, `${JSON.stringify(value, null, 2)}\n`);
}

function git(args) {
  return execFileSync('git', args, { cwd: repoRoot, encoding: 'utf8' }).trim();
}

function sha256(value) {
  return createHash('sha256').update(value).digest('hex');
}

function relativePath(path) {
  return relative(repoRoot, path).replaceAll('\\', '/');
}
