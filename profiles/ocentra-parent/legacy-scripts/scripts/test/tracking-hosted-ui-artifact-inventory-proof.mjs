import { mkdir, readFile, rm, stat, writeFile } from 'node:fs/promises';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';

const repoRoot = dirname(dirname(dirname(fileURLToPath(import.meta.url))));
const outputDir = join(repoRoot, 'test-results', 'tracking-hosted-ui-artifact-inventory-proof');
const proofDir = join(repoRoot, 'output', 'tracking-plan-proof', 'tracking-hosted-ui-artifact-inventory-proof');
const wp30ProofDir = join(repoRoot, 'output', 'tracking-plan-proof', '30-parent-and-child-ui-ux-surfaces');
const wp31ProofDir = join(
  repoRoot,
  'output',
  'tracking-plan-proof',
  '31-platform-extension-checklists-and-proof-routing'
);
const wp33ProofDir = join(repoRoot, 'output', 'tracking-plan-proof', '33-proof-gates-fixtures-rollout-and-pr-gate');
const generatedAt = '2026-06-06T23:45:00.000Z';

const requiredScreenshots = [
  {
    root: wp30ProofDir,
    path: '11-ui-snapshots/hosted-policy-tracking-live-summary.png',
  },
  {
    root: wp30ProofDir,
    path: '11-ui-snapshots/hosted-policy-tracking-live-summary-mobile.png',
  },
  {
    root: wp30ProofDir,
    path: '11-ui-snapshots/hosted-policy-tracking-citation-detail.png',
  },
  {
    root: wp30ProofDir,
    path: '11-ui-snapshots/hosted-policy-tracking-evidence-drawer.png',
  },
  {
    root: wp30ProofDir,
    path: '11-ui-snapshots/hosted-policy-tracking-child-check-in.png',
  },
  {
    root: wp30ProofDir,
    path: '11-ui-snapshots/hosted-policy-tracking-child-runtime-ui.png',
  },
  {
    root: wp30ProofDir,
    path: '11-ui-snapshots/hosted-parent-overview-shell.png',
  },
  {
    root: wp30ProofDir,
    path: '11-ui-snapshots/hosted-parent-devices-shell.png',
  },
  {
    root: wp30ProofDir,
    path: '11-ui-snapshots/hosted-policy-tracking-family-dashboard-rollup.png',
  },
  {
    root: wp30ProofDir,
    path: '11-ui-snapshots/hosted-policy-tracking-report-policy-consumer.png',
  },
  {
    root: wp30ProofDir,
    path: '11-ui-snapshots/hosted-policy-tracking-report-export.png',
  },
  {
    root: wp30ProofDir,
    path: '11-ui-snapshots/hosted-policy-tracking-notification-parent-surface.png',
  },
  {
    root: wp30ProofDir,
    path: '11-ui-snapshots/hosted-policy-tracking-parent-action-readiness.png',
  },
  {
    root: wp30ProofDir,
    path: '11-ui-snapshots/hosted-policy-tracking-missing-device.png',
  },
  {
    root: wp30ProofDir,
    path: '11-ui-snapshots/hosted-policy-tracking-retention-settings.png',
  },
  {
    root: wp31ProofDir,
    path: '19-unsupported-manual-hosted-ui.png',
  },
];

const requiredAssertions = [
  'named-region',
  'visible-heading',
  'enabled-refresh-button',
  'service-backed-row-citation-visible',
  'service-data-coverage-visible',
  'family-dashboard-rollup-visible',
  'family-dashboard-rollup-screenshot',
  'report-policy-consumer-visible',
  'report-policy-consumer-screenshot',
  'report-export-read-model-visible',
  'report-export-read-model-screenshot',
  'notification-parent-surface-history-visible',
  'notification-parent-surface-history-screenshot',
  'parent-action-readiness-visible',
  'parent-action-readiness-screenshot',
  'missing-device-visible',
  'missing-device-screenshot',
  'service-backed-evidence-drawer-visible',
  'service-backed-evidence-drawer-screenshot',
  'service-backed-citation-detail-visible',
  'service-backed-citation-detail-screenshot',
  'retention-settings-read-model-visible',
  'retention-settings-local-write-clicked',
  'retention-settings-local-write-result-visible',
  'retention-settings-screenshot',
  'manual-required-visible',
  'physical-device-required-visible',
  'no-product-claim-visible',
  'child-check-in-copy-visible',
  'child-check-in-actions-visible',
  'child-device-delivery-not-claimed',
  'child-runtime-disclosure-visible',
  'child-runtime-safe-help-response-visible',
  'child-runtime-location-share-consent-visible',
  'child-runtime-hosted-only-boundary-visible',
  'unsupported-manual-platform-render-state-visible',
  'unsupported-manual-platform-screenshot',
  'parent-overview-shell-screenshot',
  'parent-devices-shell-screenshot',
  'no-unlabeled-buttons',
  'no-proof-card-overlap',
  'desktop-screenshot',
  'child-check-in-screenshot',
  'child-runtime-ui-screenshot',
  'mobile-screenshot',
];

await rm(outputDir, { recursive: true, force: true });
await rm(proofDir, { recursive: true, force: true });
await mkdir(outputDir, { recursive: true });
await mkdir(proofDir, { recursive: true });
await mkdir(wp33ProofDir, { recursive: true });

const proof = await buildProof();

assertProof(proof);
await writeJson(join(outputDir, 'proof.json'), proof);
await writeJson(join(proofDir, 'proof.json'), proof);
await writeFile(join(proofDir, '00-source-snapshot.md'), sourceSnapshot(proof), 'utf8');
await writeFile(join(proofDir, '16-validation-commands.log'), validationLog(proof), 'utf8');
await writeJson(join(wp30ProofDir, '21-hosted-ui-artifact-inventory-proof.json'), proof);
await writeJson(join(wp33ProofDir, '28-hosted-ui-artifact-inventory-proof.json'), proof);

console.log('tracking-hosted-ui-artifact-inventory-proof-ok');
console.log(`evidence=${join('test-results', 'tracking-hosted-ui-artifact-inventory-proof', 'proof.json')}`);

async function buildProof() {
  const hostedProof = await readJson(
    'output/tracking-plan-proof/30-parent-and-child-ui-ux-surfaces/17-hosted-ui-proof.json'
  );
  const evidenceDrawerProof = await readJson(
    'output/tracking-plan-proof/30-parent-and-child-ui-ux-surfaces/20-evidence-drawer-hosted-ui-proof.json'
  );
  const unsupportedManualProof = await readJson(
    'output/tracking-plan-proof/31-platform-extension-checklists-and-proof-routing/19-unsupported-manual-hosted-ui-proof.json'
  );
  const childRuntimeBoundaryProof = await readJson(
    'output/tracking-plan-proof/30-parent-and-child-ui-ux-surfaces/26-child-runtime-delivery-boundary-proof.json'
  );
  const childRuntimeExecutionReadinessProof = await readJson(
    'output/tracking-plan-proof/30-parent-and-child-ui-ux-surfaces/27-child-runtime-execution-readiness-proof.json'
  );
  const childRuntimeSnapshotRequirementsProof = await readJson(
    'output/tracking-plan-proof/30-parent-and-child-ui-ux-surfaces/28-child-runtime-snapshot-requirements-proof.json'
  );
  const accessibilitySummary = await readJson('test-results/tracking-plan-hosted-ui-proof/accessibility-summary.json');
  const screenshots = await Promise.all(requiredScreenshots.map(readScreenshot));

  return {
    schemaVersion: 1,
    proofMode: 'tracking-hosted-ui-artifact-inventory-proof',
    generatedAt,
    branch: gitOutput(['rev-parse', '--abbrev-ref', 'HEAD']),
    baseCommitAtGeneration: gitOutput(['rev-parse', 'HEAD']),
    requiredProofTier: 'P2_HOSTED_CI',
    currentProofTier: 'P2_HOSTED_CI',
    currentStatus: 'proved',
    productClaimReady: false,
    sourceProofs: {
      hostedProof: proofSummary(hostedProof),
      evidenceDrawerProof: proofSummary(evidenceDrawerProof),
      unsupportedManualProof: proofSummary(unsupportedManualProof),
      childRuntimeBoundaryProof: childRuntimeBoundaryProofSummary(childRuntimeBoundaryProof),
      childRuntimeExecutionReadinessProof: childRuntimeExecutionReadinessProofSummary(
        childRuntimeExecutionReadinessProof
      ),
      childRuntimeSnapshotRequirementsProof: childRuntimeSnapshotRequirementsProofSummary(
        childRuntimeSnapshotRequirementsProof
      ),
      accessibilitySummary: accessibilitySummarySummary(accessibilitySummary),
    },
    screenshots,
    requiredAssertions,
    layoutProof: layoutProof(accessibilitySummary),
    provedClaims: {
      parentPortalShellScreenshotsClaimed: true,
    },
    proofPaths: {
      evidence: 'test-results/tracking-hosted-ui-artifact-inventory-proof/proof.json',
      workpack30Proof:
        'output/tracking-plan-proof/30-parent-and-child-ui-ux-surfaces/21-hosted-ui-artifact-inventory-proof.json',
      wp33Proof:
        'output/tracking-plan-proof/33-proof-gates-fixtures-rollout-and-pr-gate/28-hosted-ui-artifact-inventory-proof.json',
      accessibilitySummary: 'test-results/tracking-plan-hosted-ui-proof/accessibility-summary.json',
      childRuntimeBoundaryProof:
        'output/tracking-plan-proof/30-parent-and-child-ui-ux-surfaces/26-child-runtime-delivery-boundary-proof.json',
      childRuntimeExecutionReadinessProof:
        'output/tracking-plan-proof/30-parent-and-child-ui-ux-surfaces/27-child-runtime-execution-readiness-proof.json',
      childRuntimeSnapshotRequirementsProof:
        'output/tracking-plan-proof/30-parent-and-child-ui-ux-surfaces/28-child-runtime-snapshot-requirements-proof.json',
    },
    nonClaims: {
      fullParentChildUiClaimed: false,
      childDeviceRuntimeClaimed: false,
      physicalDeviceProofClaimed: false,
      authorityProofClaimed: false,
      providerDeliveryClaimed: false,
      productionProofClaimed: false,
      productReadyTrackingClaimed: false,
    },
  };
}

async function readScreenshot(screenshot) {
  const absolutePath = join(screenshot.root, screenshot.path);
  const buffer = await readFile(absolutePath);
  const stats = await stat(absolutePath);
  const dimensions = pngDimensions(buffer, screenshot.path);

  return {
    path: relativeProofPath(screenshot),
    bytes: stats.size,
    width: dimensions.width,
    height: dimensions.height,
  };
}

function assertProof(proof) {
  if (proof.screenshots.length !== requiredScreenshots.length) {
    throw new Error(`Unexpected screenshot count: ${proof.screenshots.length}`);
  }
  const badScreenshot = proof.screenshots.find((screenshot) => screenshot.bytes <= 1024 || screenshot.width <= 0);
  if (badScreenshot) {
    throw new Error(`Invalid screenshot artifact: ${JSON.stringify(badScreenshot)}`);
  }
  if (proof.sourceProofs.hostedProof.productClaimReady !== false) {
    throw new Error('Hosted proof must keep productClaimReady=false.');
  }
  if (proof.sourceProofs.evidenceDrawerProof.productClaimReady !== false) {
    throw new Error('Evidence drawer proof must keep productClaimReady=false.');
  }
  if (proof.sourceProofs.unsupportedManualProof.productClaimReady !== false) {
    throw new Error('Unsupported/manual platform proof must keep productClaimReady=false.');
  }
  if (
    proof.sourceProofs.childRuntimeBoundaryProof.productReadyClaimed !== false ||
    proof.sourceProofs.childRuntimeBoundaryProof.childDeviceExecutionRuntimeClaimed !== false ||
    proof.sourceProofs.childRuntimeBoundaryProof.physicalDeviceProofClaimed !== false ||
    proof.sourceProofs.childRuntimeBoundaryProof.authorityProofClaimed !== false
  ) {
    throw new Error(
      `Child runtime boundary proof overclaimed runtime/device behavior: ${JSON.stringify(
        proof.sourceProofs.childRuntimeBoundaryProof
      )}`
    );
  }
  if (
    proof.sourceProofs.childRuntimeExecutionReadinessProof.productReadyClaimed !== false ||
    proof.sourceProofs.childRuntimeExecutionReadinessProof.childDeviceExecutionRuntimeClaimed !== false ||
    proof.sourceProofs.childRuntimeExecutionReadinessProof.physicalDeviceProofClaimed !== false ||
    proof.sourceProofs.childRuntimeExecutionReadinessProof.authorityProofClaimed !== false ||
    proof.sourceProofs.childRuntimeExecutionReadinessProof.executionRequirementRefCount <= 0 ||
    proof.sourceProofs.childRuntimeExecutionReadinessProof.runtimeObservationRequirementRefCount <= 0
  ) {
    throw new Error(
      `Child runtime execution readiness proof is missing refs or overclaimed runtime/device behavior: ${JSON.stringify(
        proof.sourceProofs.childRuntimeExecutionReadinessProof
      )}`
    );
  }
  if (
    proof.sourceProofs.childRuntimeSnapshotRequirementsProof.productReadyClaimed !== false ||
    proof.sourceProofs.childRuntimeSnapshotRequirementsProof.childDeviceExecutionRuntimeClaimed !== false ||
    proof.sourceProofs.childRuntimeSnapshotRequirementsProof.physicalDeviceProofClaimed !== false ||
    proof.sourceProofs.childRuntimeSnapshotRequirementsProof.authorityProofClaimed !== false ||
    proof.sourceProofs.childRuntimeSnapshotRequirementsProof.requiredSnapshotKindCount <= 0 ||
    proof.sourceProofs.childRuntimeSnapshotRequirementsProof.visibleSnapshotRequirementCount <= 0 ||
    proof.sourceProofs.childRuntimeSnapshotRequirementsProof.runtimeObservationRequirementCount <= 0
  ) {
    throw new Error(
      `Child runtime snapshot requirements proof is missing refs or overclaimed runtime/device behavior: ${JSON.stringify(
        proof.sourceProofs.childRuntimeSnapshotRequirementsProof
      )}`
    );
  }
  const missingAssertions = requiredAssertions.filter(
    (assertion) => !proof.sourceProofs.accessibilitySummary.assertions.includes(assertion)
  );
  if (missingAssertions.length > 0) {
    throw new Error(`Missing hosted accessibility assertions: ${missingAssertions.join(', ')}`);
  }
  if (!proof.layoutProof.noOverlap || proof.layoutProof.boxes.length !== 11) {
    throw new Error(`Hosted UI layout proof did not prove non-overlap: ${JSON.stringify(proof.layoutProof)}`);
  }
  if (
    proof.provedClaims.parentPortalShellScreenshotsClaimed !== true ||
    proof.nonClaims.fullParentChildUiClaimed !== false ||
    proof.nonClaims.childDeviceRuntimeClaimed !== false ||
    proof.nonClaims.physicalDeviceProofClaimed !== false ||
    proof.nonClaims.authorityProofClaimed !== false ||
    proof.nonClaims.providerDeliveryClaimed !== false ||
    proof.nonClaims.productionProofClaimed !== false ||
    proof.nonClaims.productReadyTrackingClaimed !== false
  ) {
    throw new Error(`Hosted UI inventory proof overclaimed behavior: ${JSON.stringify(proof.nonClaims)}`);
  }
}

function relativeProofPath(screenshot) {
  if (screenshot.root === wp30ProofDir) {
    return `output/tracking-plan-proof/30-parent-and-child-ui-ux-surfaces/${screenshot.path}`;
  }
  if (screenshot.root === wp31ProofDir) {
    return `output/tracking-plan-proof/31-platform-extension-checklists-and-proof-routing/${screenshot.path}`;
  }
  throw new Error(`Unknown screenshot root: ${screenshot.root}`);
}

function proofSummary(proof) {
  return {
    proofMode: proof.proofMode,
    currentProofTier: proof.currentProofTier,
    currentStatus: proof.currentStatus,
    productClaimReady: proof.productClaimReady,
  };
}

function childRuntimeBoundaryProofSummary(proof) {
  return {
    proofMode: proof.proofLabels?.[0] ?? proof.workpackId,
    currentProofTier: proof.currentProofTier,
    currentStatus: proof.status,
    rowCount: proof.readModel?.rows?.length ?? 0,
    childDeviceExecutionRuntimeClaimed: proof.productClaims?.childDeviceExecutionRuntimeClaimed,
    physicalDeviceProofClaimed: proof.productClaims?.physicalDeviceProofClaimed,
    authorityProofClaimed: proof.productClaims?.authorityProofClaimed,
    productReadyClaimed: proof.productClaims?.productReadyClaimed,
  };
}

function childRuntimeExecutionReadinessProofSummary(proof) {
  return {
    proofMode: proof.proofLabels?.[0] ?? proof.workpackId,
    currentProofTier: proof.currentProofTier,
    currentStatus: proof.status,
    rowCount: proof.readModel?.rows?.length ?? 0,
    executionRequirementRefCount: proof.readModel?.executionRequirementRefCount ?? 0,
    runtimeObservationRequirementRefCount: proof.readModel?.runtimeObservationRequirementRefCount ?? 0,
    childDeviceExecutionRuntimeClaimed: proof.productClaims?.childDeviceExecutionRuntimeClaimed,
    physicalDeviceProofClaimed: proof.productClaims?.physicalDeviceProofClaimed,
    authorityProofClaimed: proof.productClaims?.authorityProofClaimed,
    productReadyClaimed: proof.productClaims?.productReadyClaimed,
  };
}

function childRuntimeSnapshotRequirementsProofSummary(proof) {
  return {
    proofMode: proof.proofLabels?.[0] ?? proof.workpackId,
    currentProofTier: proof.currentProofTier,
    currentStatus: proof.status,
    rowCount: proof.readModel?.rows?.length ?? 0,
    requiredSnapshotKindCount: proof.readModel?.requiredSnapshotKindCount ?? 0,
    visibleSnapshotRequirementCount: proof.readModel?.visibleSnapshotRequirementCount ?? 0,
    runtimeObservationRequirementCount: proof.readModel?.runtimeObservationRequirementCount ?? 0,
    childDeviceExecutionRuntimeClaimed: proof.productClaims?.childDeviceExecutionRuntimeClaimed,
    physicalDeviceProofClaimed: proof.productClaims?.physicalDeviceProofClaimed,
    authorityProofClaimed: proof.productClaims?.authorityProofClaimed,
    productReadyClaimed: proof.productClaims?.productReadyClaimed,
  };
}

function accessibilitySummarySummary(summary) {
  return {
    route: summary.route,
    assertions: summary.assertions,
    hasNamedRegion: summary.summary?.hasNamedRegion === true,
    headingCount: summary.summary?.headings?.length ?? 0,
    labelCount: summary.summary?.labels?.length ?? 0,
    layoutBoxCount: summary.summary?.layoutBoxes?.length ?? 0,
  };
}

function layoutProof(summary) {
  const boxes = summary.summary?.layoutBoxes ?? [];
  return {
    proofMode: 'hosted-tracking-no-overlap-layout-proof',
    noOverlap: boxes.length === 11 && !boxesOverlap(boxes),
    boxes,
  };
}

function boxesOverlap(boxes) {
  return boxes.some((box, index) => boxes.slice(index + 1).some((otherBox) => rectanglesOverlap(box, otherBox)));
}

function rectanglesOverlap(first, second) {
  return (
    first.left < second.right && first.right > second.left && first.top < second.bottom && first.bottom > second.top
  );
}

function pngDimensions(buffer, relativePath) {
  const signature = '89504e470d0a1a0a';
  if (buffer.subarray(0, 8).toString('hex') !== signature) {
    throw new Error(`Screenshot is not a PNG: ${relativePath}`);
  }
  return {
    width: buffer.readUInt32BE(16),
    height: buffer.readUInt32BE(20),
  };
}

function sourceSnapshot(proof) {
  return [
    '# Tracking Hosted UI Artifact Inventory Proof Source Snapshot',
    '',
    `- Branch: ${proof.branch}`,
    `- Base commit at generation: ${proof.baseCommitAtGeneration}`,
    '- Source proof: existing hosted UI Playwright proof artifacts, accessibility summary, and layout geometry.',
    '- Scope: verify stored hosted screenshots, evidence drawer proof, unsupported/manual platform proof, child-runtime delivery boundary proof, child-runtime execution readiness proof, accessibility assertions, and no-overlap layout boxes for WP30/WP33 handoff.',
    '- Boundary: inventory proof only; parent portal shell screenshots are proved, while child-device runtime, physical-device proof, authority, provider delivery, full product parent/child UI, production proof, and product-ready tracking remain unclaimed.',
    '',
  ].join('\n');
}

function validationLog(proof) {
  return [
    '$ node scripts/test/tracking-hosted-ui-artifact-inventory-proof.mjs',
    'tracking-hosted-ui-artifact-inventory-proof-ok',
    `evidence=${proof.proofPaths.evidence}`,
  ].join('\n');
}

async function readJson(relativePath) {
  return JSON.parse(await readFile(join(repoRoot, relativePath), 'utf8'));
}

async function writeJson(path, value) {
  await mkdir(dirname(path), { recursive: true });
  await writeFile(path, `${JSON.stringify(value, null, 2)}\n`, 'utf8');
}

function gitOutput(args) {
  const result = spawnSync('git', args, { cwd: repoRoot, encoding: 'utf8', shell: false });
  if (result.status !== 0) {
    throw new Error(`git ${args.join(' ')} failed: ${result.stderr}`);
  }
  return result.stdout.trim();
}
