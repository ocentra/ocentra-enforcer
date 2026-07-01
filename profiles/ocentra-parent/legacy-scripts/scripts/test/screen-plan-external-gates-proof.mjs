import { createHash } from 'node:crypto';
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { basename, join, relative, resolve } from 'node:path';

const repoRoot = process.cwd();
const outputDir = join(repoRoot, 'output', 'screen-plan-proof', 'external-gates');
const manifestPath = join(outputDir, 'manual-evidence-manifest.json');
const manifestTemplatePath = join(outputDir, 'manual-evidence-manifest.template.json');
const proofPath = join(outputDir, 'proof-summary.json');
const runbookPath = join(outputDir, 'manual-evidence-runbook.md');
const statusPath = join(outputDir, 'manual-evidence-status.md');

const requiredGates = [
  gate('macos-live-capture-permission', 'macos', 'platform-permission-prompt-screenshot', {
    workpack: '10 macOS capture adapter plan/proof',
    collectionMode: 'macos-runner-or-manual-macos-host',
    requirement:
      'real macOS ScreenCaptureKit session with Screen Recording permission, display/window pixels, OCR, and deletion proof',
  }),
  gate('linux-desktop-session-capture', 'linux-wayland', 'platform-session-recording', {
    workpack: '11 Linux capture adapter plan/proof',
    collectionMode: 'local-wslg-or-linux-desktop-runner',
    requirement: 'real Linux X11 or Wayland portal desktop-session capture with deletion proof',
  }),
  gate('android-physical-mediaprojection-capture', 'android-mediaprojection', 'physical-device-capture-recording', {
    workpack: '12 Android MediaProjection adapter plan/proof',
    collectionMode: 'local-adb-physical-device',
    requirement: 'real physical Android MediaProjection capture, stop callback, deletion, and local OCR proof',
  }),
  gate('ios-physical-replaykit-capture', 'ios-replaykit', 'physical-device-capture-recording', {
    workpack: '13 iOS ReplayKit adapter plan/proof',
    collectionMode: 'ios-device-or-macos-runner-with-device',
    requirement: 'real physical iOS ReplayKit or broadcast-extension capture with deletion proof',
  }),
  gate('live-view-platform-prompt', 'android-mediaprojection', 'platform-permission-prompt-screenshot', {
    workpack: '28 Live view optional mode',
    collectionMode: 'local-adb-physical-device',
    requirement: 'real live-view platform prompt artifact, not ordinary capture-only permission evidence',
  }),
  gate('live-view-physical-device-parity', 'android-mediaprojection', 'physical-device-capture-recording', {
    workpack: '28 Live view optional mode',
    collectionMode: 'local-adb-physical-device',
    requirement: 'physical-device parity for live view transport/custody/deletion behavior',
  }),
  gate('live-view-hosted-relay-infrastructure', 'hosted-relay', 'hosted-relay-proof', {
    workpack: '28 Live view optional mode',
    collectionMode: 'hosted-relay-integration-run',
    requirement: 'hosted relay infrastructure proof with end-to-end encrypted custody and no raw-frame retention',
  }),
  gate('live-view-privacy-legal-approval', 'policy-approval', 'privacy-legal-approval', {
    workpack: '28 Live view optional mode',
    collectionMode: 'manual-approval-record',
    requirement: 'privacy/legal approval record for optional live view',
  }),
  gate('authenticated-account-social-capture', 'authenticated-social', 'authenticated-account-capture-proof', {
    workpack: '30 Test suite, Playwright, rollout, PR gate',
    collectionMode: 'operator-consented-logged-in-session',
    requirement:
      'real logged-in social/feed account capture with operator consent, redacted account identifiers or identifier hash, browser session-source custody, local OCR/VLM analysis, policy dry-run, and raw image deletion proof',
  }),
];

const manifest = readManifestIfPresent();
const manifestEntries = Array.isArray(manifest?.entries) ? manifest.entries : [];
const gateResults = requiredGates.map((requiredGate) => validateGate(requiredGate, manifestEntries));
const negativeChecks = [
  rejects('fixture artifact path is rejected', () =>
    validateArtifact(requiredGates[0], {
      gateId: requiredGates[0].gateId,
      platform: requiredGates[0].platform,
      evidenceKind: requiredGates[0].evidenceKind,
      artifactPath: 'docs/fixtures/unbacked-prompt.html',
      artifactSha256: 'sha256-placeholder',
      capturedFromRealDeviceOrHost: true,
      capturesLiveSurface: true,
      rawPrivateContentIncluded: false,
    })
  ),
  rejects('raw private content is rejected', () =>
    validateArtifact(requiredGates[1], {
      gateId: requiredGates[1].gateId,
      platform: requiredGates[1].platform,
      evidenceKind: requiredGates[1].evidenceKind,
      artifactPath: 'output/screen-plan-proof/external-gates/artifacts/linux-proof.png',
      artifactSha256: 'sha256-placeholder',
      capturedFromRealDeviceOrHost: true,
      capturesLiveSurface: true,
      rawPrivateContentIncluded: true,
      localCaptureProofRef: 'linux-live-capture-proof',
      localAnalysisProofRef: 'linux-local-analysis-proof',
      rawImageDeletionProofRef: 'linux-raw-deletion-proof',
    })
  ),
  rejects('pixel evidence without analysis and deletion refs is rejected', () =>
    validateArtifact(requiredGates[2], {
      gateId: requiredGates[2].gateId,
      platform: requiredGates[2].platform,
      evidenceKind: requiredGates[2].evidenceKind,
      artifactPath: 'output/screen-plan-proof/external-gates/artifacts/android-physical-proof.mp4',
      artifactSha256: 'sha256-placeholder',
      capturedFromRealDeviceOrHost: true,
      capturesLiveSurface: true,
      rawPrivateContentIncluded: false,
    })
  ),
  rejects('authenticated account proof without consent and redaction is rejected', () =>
    validateArtifact(requiredGates[8], {
      gateId: requiredGates[8].gateId,
      platform: requiredGates[8].platform,
      evidenceKind: requiredGates[8].evidenceKind,
      artifactPath: 'output/screen-plan-proof/external-gates/artifacts/authenticated-social-proof.json',
      artifactSha256: 'sha256-placeholder',
      capturedFromRealDeviceOrHost: true,
      capturesLiveSurface: true,
      rawPrivateContentIncluded: false,
      operatorConsentRecorded: false,
      redactedAccountIdentifiers: false,
      browserSessionSourceCustodyRef: '',
    })
  ),
];

if (negativeChecks.some((check) => !check.rejected)) {
  throw new Error(`External gate negative check failed: ${JSON.stringify(negativeChecks)}`);
}

const satisfiedGateCount = gateResults.filter((result) => result.status === 'satisfied').length;
const missingGateCount = gateResults.filter((result) => result.status === 'missing').length;
const invalidGateCount = gateResults.filter((result) => result.status === 'invalid').length;

const summary = {
  proof: 'screen-plan-external-gates',
  generatedAt: new Date().toISOString(),
  manifest: {
    path: relativePath(manifestPath),
    present: manifest !== undefined,
    entryCount: manifestEntries.length,
    templatePath: relativePath(manifestTemplatePath),
    runbookPath: relativePath(runbookPath),
    statusPath: relativePath(statusPath),
  },
  gateResults,
  counts: {
    requiredGateCount: requiredGates.length,
    satisfiedGateCount,
    missingGateCount,
    invalidGateCount,
  },
  assertions: {
    allCurrentExternalGatesEnumerated: requiredGates.length === 9,
    authenticatedAccountSocialGateEnumerated: gateResults.some(
      (result) => result.gateId === 'authenticated-account-social-capture'
    ),
    strictProofRefsRequired: gateResults.every(
      (result) => result.status === 'missing' || Array.isArray(result.requiredProofRefFields)
    ),
    collectionModesEnumerated: gateResults.every((result) => typeof result.collectionMode === 'string'),
    productCompleteAllowed: satisfiedGateCount === requiredGates.length && invalidGateCount === 0,
    currentBranchMustRemainNonClaim: satisfiedGateCount !== requiredGates.length || invalidGateCount > 0,
    rejectsFixtureOrStaticEvidence: negativeChecks.every((check) => check.rejected),
  },
  negativeChecks,
  nonClaims: [
    'This proof does not fabricate or substitute platform evidence.',
    'This proof does not mark screen-plan product complete while any external gate is missing or invalid.',
    'Real external evidence must be attached through the manifest with digest-backed artifacts from live devices or live host sessions.',
  ],
};

mkdirSync(outputDir, { recursive: true });
writeFileSync(manifestTemplatePath, `${JSON.stringify(manifestTemplate(), null, 2)}\n`);
writeFileSync(runbookPath, manualEvidenceRunbook());
writeFileSync(statusPath, manualEvidenceStatus(summary));
writeFileSync(proofPath, `${JSON.stringify(summary, null, 2)}\n`);
console.log(`screen-plan-external-gates-proof-ok:${proofPath}`);

function gate(gateId, platform, evidenceKind, details) {
  return {
    gateId,
    platform,
    evidenceKind,
    ...details,
  };
}

function manifestTemplate() {
  return {
    schemaVersion: 'v0.6',
    instructions:
      'Copy this template to manual-evidence-manifest.json only after attaching digest-backed artifacts captured from real devices or live host sessions. Do not use fixtures, generated HTML pages, static JSON, raw private content, or placeholder digests as final evidence.',
    entries: requiredGates.map((requiredGate) => ({
      gateId: requiredGate.gateId,
      platform: requiredGate.platform,
      evidenceKind: requiredGate.evidenceKind,
      collectionMode: requiredGate.collectionMode,
      artifactPath: `output/screen-plan-proof/external-gates/artifacts/${requiredGate.gateId}${templateExtensionFor(
        requiredGate.evidenceKind
      )}`,
      artifactSha256: '<sha256-of-artifact-bytes>',
      capturedFromRealDeviceOrHost: requiresLiveSurfaceArtifact(requiredGate) ? true : false,
      capturesLiveSurface: requiresLiveSurfaceArtifact(requiredGate) ? true : false,
      rawPrivateContentIncluded: false,
      ...templateProofRefsFor(requiredGate),
      ...(requiredGate.evidenceKind === 'authenticated-account-capture-proof'
        ? {
            operatorConsentRecorded: true,
            redactedAccountIdentifiers: true,
            browserSessionSourceCustodyRef:
              '<proof ref for profile/storage-state/interactive browser session custody without raw credential retention>',
            policyDryRunProofRef: '<proof ref showing policy consumed the account-surface AI result>',
          }
        : {}),
    })),
  };
}

function templateExtensionFor(evidenceKind) {
  if (evidenceKind === 'platform-session-recording' || evidenceKind === 'physical-device-capture-recording') {
    return '.mp4';
  }

  if (evidenceKind === 'privacy-legal-approval') {
    return '.md';
  }

  if (evidenceKind === 'hosted-relay-proof' || evidenceKind === 'authenticated-account-capture-proof') {
    return '.json';
  }

  return '.png';
}

function manualEvidenceRunbook() {
  const rows = requiredGates
    .map((requiredGate) =>
      [
        `## ${requiredGate.gateId}`,
        '',
        `- platform: ${requiredGate.platform}`,
        `- evidence kind: ${requiredGate.evidenceKind}`,
        `- workpack: ${requiredGate.workpack}`,
        `- requirement: ${requiredGate.requirement}`,
        `- required proof refs: ${requiredProofRefFields(requiredGate).join(', ') || 'artifact digest only'}`,
        '',
      ].join('\n')
    )
    .join('\n');
  return [
    '# Screen Plan External Evidence Runbook',
    '',
    'Use real live-device or live-host artifacts only. Attach artifacts under `output/screen-plan-proof/external-gates/artifacts/`, calculate the SHA-256 digest, and copy `manual-evidence-manifest.template.json` to `manual-evidence-manifest.json` only after all fields are true for the artifact.',
    '',
    'Authenticated-account social proof must be operator-consented, must redact account identifiers or use identifier hashes, must cite browser profile/storage-state/interactive-session custody without raw credential retention, must cite local OCR/VLM/AI analysis, must cite policy dry-run consumption, and must cite raw image deletion/custody. Public social/feed proof is not enough for this gate.',
    '',
    rows,
  ].join('\n');
}

function manualEvidenceStatus(summary) {
  const rows = summary.gateResults
    .map((result) =>
      [
        `## ${result.gateId}`,
        '',
        `- status: ${result.status}`,
        `- platform: ${result.platform}`,
        `- evidence kind: ${result.evidenceKind}`,
        `- collection mode: ${result.collectionMode}`,
        `- workpack: ${result.workpack}`,
        `- artifact: ${result.artifactPath ?? 'missing'}`,
        `- required proof refs: ${result.requiredProofRefFields.join(', ') || 'artifact digest only'}`,
        `- reason: ${result.reason}`,
        '',
      ].join('\n')
    )
    .join('\n');

  return [
    '# Screen Plan External Evidence Status',
    '',
    `Generated: ${summary.generatedAt}`,
    '',
    `Manifest present: ${summary.manifest.present}`,
    `Required gates: ${summary.counts.requiredGateCount}`,
    `Satisfied gates: ${summary.counts.satisfiedGateCount}`,
    `Missing gates: ${summary.counts.missingGateCount}`,
    `Invalid gates: ${summary.counts.invalidGateCount}`,
    `Product-complete allowed: ${summary.assertions.productCompleteAllowed}`,
    '',
    'This status file is generated from the same validator as `proof-summary.json`. It is not a substitute for real artifacts; it lists what evidence must be attached before the external gates can close.',
    '',
    rows,
  ].join('\n');
}

function validateGate(requiredGate, entries) {
  const entry = entries.find((candidate) => candidate?.gateId === requiredGate.gateId);
  if (entry === undefined) {
    return {
      ...requiredGate,
      status: 'missing',
      artifactPath: null,
      requiredProofRefFields: requiredProofRefFields(requiredGate),
      reason: 'No matching manifest entry exists.',
    };
  }

  const validation = validateArtifact(requiredGate, entry);
  return {
    ...requiredGate,
    status: validation.ok ? 'satisfied' : 'invalid',
    artifactPath: typeof entry.artifactPath === 'string' ? normalizeArtifactPath(entry.artifactPath) : null,
    artifactSha256: typeof entry.artifactSha256 === 'string' ? entry.artifactSha256 : null,
    requiredProofRefFields: requiredProofRefFields(requiredGate),
    reason: validation.reason,
  };
}

function validateArtifact(requiredGate, entry) {
  if (entry.platform !== requiredGate.platform || entry.evidenceKind !== requiredGate.evidenceKind) {
    return rejected('platform or evidence kind does not match the required gate');
  }

  if (entry.collectionMode !== requiredGate.collectionMode) {
    return rejected('collection mode does not match the required gate');
  }

  if (typeof entry.artifactPath !== 'string' || !artifactPathIsAllowed(entry.artifactPath)) {
    return rejected('artifact path must live under output/screen-plan-proof/external-gates/artifacts');
  }

  if (!artifactExtensionIsAllowed(requiredGate.evidenceKind, entry.artifactPath)) {
    return rejected('artifact extension is not allowed for the required evidence kind');
  }

  if (typeof entry.artifactSha256 !== 'string' || entry.artifactSha256.length < 16) {
    return rejected('artifact digest is missing or too short');
  }

  if (
    requiresLiveSurfaceArtifact(requiredGate) &&
    (entry.capturedFromRealDeviceOrHost !== true || entry.capturesLiveSurface !== true)
  ) {
    return rejected('artifact must come from a real device or host and capture a live surface');
  }

  if (entry.rawPrivateContentIncluded !== false) {
    return rejected('artifact must not include raw private content');
  }

  const missingProofRef = requiredProofRefFields(requiredGate).find((field) => !isRealProofRef(entry[field]));
  if (missingProofRef !== undefined) {
    return rejected(`required proof ref is missing or placeholder: ${missingProofRef}`);
  }

  if (
    requiredGate.evidenceKind === 'authenticated-account-capture-proof' &&
    (entry.operatorConsentRecorded !== true || entry.redactedAccountIdentifiers !== true)
  ) {
    return rejected(
      'authenticated account proof must record operator consent, redacted account identifiers or identifier hashes, browser session-source custody, local analysis proof, policy dry-run proof, and raw image deletion proof'
    );
  }

  const absoluteArtifactPath = resolve(repoRoot, entry.artifactPath);
  let artifactBytes;
  try {
    artifactBytes = readFileSync(absoluteArtifactPath);
  } catch {
    return rejected('artifact file is not present in the current checkout');
  }

  if (artifactBytes.byteLength === 0) {
    return rejected('artifact file is empty');
  }

  const digest = createHash('sha256').update(artifactBytes).digest('hex');
  if (entry.artifactSha256 !== digest) {
    return rejected('artifact digest does not match the current file bytes');
  }

  return { ok: true, reason: 'Artifact entry satisfies the current gate contract.' };
}

function requiresLiveSurfaceArtifact(requiredGate) {
  return !['hosted-relay-proof', 'privacy-legal-approval'].includes(requiredGate.evidenceKind);
}

function requiredProofRefFields(requiredGate) {
  if (
    requiredGate.evidenceKind === 'platform-permission-prompt-screenshot' ||
    requiredGate.evidenceKind === 'platform-session-recording' ||
    requiredGate.evidenceKind === 'physical-device-capture-recording'
  ) {
    const fields = ['localCaptureProofRef', 'localAnalysisProofRef', 'rawImageDeletionProofRef'];
    if (requiredGate.gateId.startsWith('live-view-')) {
      fields.push('liveViewRuntimeProofRef', 'viewerAuditProofRef');
    }
    return fields;
  }

  if (requiredGate.evidenceKind === 'authenticated-account-capture-proof') {
    return [
      'localCaptureProofRef',
      'localAnalysisProofRef',
      'policyDryRunProofRef',
      'rawImageDeletionProofRef',
      'browserSessionSourceCustodyRef',
    ];
  }

  if (requiredGate.evidenceKind === 'hosted-relay-proof') {
    return ['relayEncryptionProofRef', 'relayNoRetentionProofRef', 'viewerAuditProofRef'];
  }

  if (requiredGate.evidenceKind === 'privacy-legal-approval') {
    return ['privacyApprovalRecordRef', 'approverRoleRef', 'approvalScopeRef'];
  }

  return [];
}

function templateProofRefsFor(requiredGate) {
  return Object.fromEntries(
    requiredProofRefFields(requiredGate).map((field) => [field, `<${field} for ${requiredGate.gateId}>`])
  );
}

function isRealProofRef(value) {
  return (
    typeof value === 'string' &&
    value.trim().length > 0 &&
    !value.includes('<') &&
    !value.includes('>') &&
    !value.toLowerCase().includes('placeholder')
  );
}

function artifactPathIsAllowed(path) {
  const normalized = normalizeArtifactPath(path);
  if (!normalized.startsWith('output/screen-plan-proof/external-gates/artifacts/')) {
    return false;
  }

  const lowerName = basename(normalized).toLowerCase();
  return !lowerName.includes('fixture') && !lowerName.includes('unbacked') && !lowerName.includes('placeholder');
}

function artifactExtensionIsAllowed(evidenceKind, path) {
  const lowerPath = path.toLowerCase();
  if (evidenceKind === 'privacy-legal-approval' || evidenceKind === 'hosted-relay-proof') {
    return lowerPath.endsWith('.json') || lowerPath.endsWith('.md');
  }

  if (evidenceKind === 'authenticated-account-capture-proof') {
    return (
      lowerPath.endsWith('.json') ||
      lowerPath.endsWith('.md') ||
      lowerPath.endsWith('.png') ||
      lowerPath.endsWith('.jpg') ||
      lowerPath.endsWith('.jpeg') ||
      lowerPath.endsWith('.webp')
    );
  }

  if (evidenceKind === 'platform-session-recording' || evidenceKind === 'physical-device-capture-recording') {
    return (
      lowerPath.endsWith('.png') ||
      lowerPath.endsWith('.jpg') ||
      lowerPath.endsWith('.jpeg') ||
      lowerPath.endsWith('.webp') ||
      lowerPath.endsWith('.mp4') ||
      lowerPath.endsWith('.mov')
    );
  }

  return (
    lowerPath.endsWith('.png') ||
    lowerPath.endsWith('.jpg') ||
    lowerPath.endsWith('.jpeg') ||
    lowerPath.endsWith('.webp')
  );
}

function readManifestIfPresent() {
  if (!existsSync(manifestPath)) {
    return null;
  }

  return JSON.parse(readFileSync(manifestPath, 'utf8'));
}

function normalizeArtifactPath(path) {
  return path.replaceAll('\\', '/');
}

function relativePath(path) {
  return relative(repoRoot, path).replaceAll('\\', '/');
}

function rejected(reason) {
  return { ok: false, reason };
}

function rejects(name, validator) {
  return { name, rejected: validator().ok === false };
}
