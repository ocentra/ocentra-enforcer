import { supportIncidentHandoff } from './parent-desktop-release-support-incident-fixture.mjs';

export function buildReadModel(version, commit, ciArtifactProof) {
  return {
    schemaVersion: 'parent-desktop-release-support-proof',
    observerAuthority: observerAuthority(),
    mobileBridgeBoundary: {
      parentMobileState: 'scaffold',
      childAndroidAgentState: 'manual-required',
      childIosAgentState: 'manual-required',
      parentMobileClaim: 'parent mobile bridge is a parent shell route boundary only',
      childAgentNonClaim: 'child Android and child iOS agent parity is not claimed by parent desktop release support',
    },
    packageRuntimeEvidence: packageRuntimeEvidence(ciArtifactProof),
    updateStates: updateStates(),
    signingStoreStates: signingStoreStates(),
    platformCapabilityMatrix: platformRows(),
    ciArtifactProof,
    supportDiagnostics: supportDiagnostics(version, commit),
    supportIncidentHandoff: supportIncidentHandoff(),
    manualRunbook: manualRunbook(version),
    productionReadinessGate: productionReadinessGate(ciArtifactProof),
    updaterRollbackRunbookProof: updaterRollbackRunbookProof(),
    updatedAt: new Date().toISOString(),
  };
}

function observerAuthority() {
  return [
    authority('read-service-state', 'completed', null),
    authority('read-route-state', 'completed', null),
    authority('write-policy', 'rejected', 'observer-read-only'),
    authority('approve-request', 'rejected', 'observer-read-only'),
    authority('take-controller', 'disabled', 'observer-read-only'),
  ];
}

function authority(operation, result, rejectionReason) {
  return {
    operation,
    result,
    authorityRole: 'observer',
    rejectionReason,
    proofRequirement: `${operation} must preserve parent observer read-only authority`,
  };
}

function packageRuntimeEvidence(ciArtifactProof) {
  return {
    packageFrontendSource: 'built-portal-dist',
    backendBoundary: 'rust-service-boundary',
    serviceLaunchOwner: 'package-service-manager',
    serviceHealthState: 'implemented',
    connectOrDegradeState: 'degraded',
    fixedAgentAddress: '127.0.0.1:4477',
    portOwnership: 'fixed-loopback',
    portConflictPolicy: 'no-foreign-process-reclaim',
    processOwnership: 'parent-shell-only',
    blankWindowGuard: 'frontend-dist-required',
    updateRollbackPosture: 'signed-channel-required',
    artifactState: ciArtifactProof.artifactState,
    supportDiagnosticState: 'preview-only',
    nonClaim: 'CI package preview is not signing not production not store distribution proof',
  };
}

function updateStates() {
  return [
    updateState('scaffold', 'unavailable', 'scaffold', 'unavailable', 'unavailable', 'signature-required', 'rollback-unavailable', 'unavailable', 'recorded', 'recorded'),
    updateState('unsigned-preview', 'available', 'unsigned-preview', 'verified', 'manual-required', 'signature-required', 'rollback-unavailable', 'unavailable', 'recorded', 'recorded'),
    updateState('signature-required', 'manual-required', 'signature-required', 'verified', 'manual-required', 'signature-required', 'manual-required', 'manual-required', 'manual-required', 'manual-required'),
    updateState('production', 'manual-required', 'production-promotion-required', 'manual-required', 'manual-required', 'signature-required', 'manual-required', 'manual-required', 'manual-required', 'manual-required'),
  ];
}

function updateState(
  channel,
  updateAvailabilityState,
  packageState,
  checksumState,
  signatureState,
  signingState,
  rollbackState,
  rollbackAvailabilityState,
  teardownEvidenceState,
  revertEvidenceState
) {
  return {
    channel,
    updateAvailabilityState,
    packageState,
    checksumState,
    signatureState,
    signingState,
    rollbackState,
    rollbackAvailabilityState,
    teardownEvidenceState,
    revertEvidenceState,
    productionPromotionState: 'production-promotion-required',
    proofRequirement: `${channel} update and rollback states must keep checksum signature and teardown or revert evidence explicit without implying production release`,
  };
}

function signingStoreStates() {
  return ['windows-code-signing', 'macos-notarization', 'google-play', 'testflight', 'app-store'].map((surface) => ({
    surface,
    state: 'manual-required',
    credentialState: 'manual-required',
    proofRequirement: `${surface} remains manual-required until real credentials and artifacts exist`,
  }));
}

function platformRows() {
  return [
    platformRow('parent-desktop', 'unsigned-preview', 'implemented', 'preview-only', 'preview-only'),
    platformRow('parent-mobile', 'scaffold', 'manual-required', 'manual-required', 'manual-required'),
    platformRow('child-desktop', 'preview-only', 'implemented', 'preview-only', 'manual-required'),
    platformRow('child-android', 'scaffold', 'manual-required', 'manual-required', 'manual-required'),
    platformRow('child-ios', 'scaffold', 'manual-required', 'manual-required', 'manual-required'),
    platformRow('relay', 'not-implemented', 'not-implemented', 'not-implemented', 'not-ready'),
    platformRow('signing', 'signature-required', 'manual-required', 'manual-required', 'manual-required'),
    platformRow('store', 'manual-required', 'manual-required', 'manual-required', 'manual-required'),
    platformRow('support', 'preview-only', 'preview-only', 'preview-only', 'preview-only'),
  ];
}

function platformRow(target, packageState, serviceState, capabilityState, proofLevel) {
  return {
    target,
    packageState,
    serviceState,
    routeState: target === 'relay' ? 'not-implemented' : 'preview-only',
    capabilityState,
    proofLevel,
    nonClaim: `${target} state is limited to the named proof level and does not upgrade unsupported platform behavior`,
  };
}

function supportDiagnostics(version, commit) {
  return {
    outputState: 'preview-only',
    entries: [
      diagnostic('version', version),
      diagnostic('commit', commit),
      diagnostic('platform', process.platform),
      diagnostic('package', 'parent-desktop unsigned preview'),
      diagnostic('service', 'loopback service reachable or explicitly unavailable'),
      diagnostic('route', 'local route or unavailable route state'),
      diagnostic('capability', 'observer read-only release support'),
      diagnostic('degraded-state', 'signing store relay and rollback are manual-required'),
    ],
    redactedFields: [
      'tokens',
      'child activity',
      'raw urls',
      'screenshots',
      'journals',
      'SQLite snapshots',
      'private paths',
      'command lines',
      'keystrokes',
      'clipboard data',
      'message contents',
    ],
  };
}

function diagnostic(field, value) {
  return { field, value, redactionState: 'safe' };
}

function productionReadinessGate(ciArtifactProof) {
  return {
    gate: 'v8-production-release-support-readiness',
    packagePreviewArtifacts: packagePreviewArtifacts(ciArtifactProof),
    supportDiagnosticsState: 'preview-only',
    supportRunbookState: 'manual-required',
    updaterRollbackExecutionState: 'rollback-unavailable',
    signingStoreProofState: 'manual-required',
    productionPublishingState: 'production-promotion-required',
    claimBoundary:
      'V8 readiness gate is package preview support readiness not production publishing not signing not store upload proof',
    proofReferences: [
      'test-results/parent-desktop-release-support-proof/proof.json',
      '.github/workflows/package-preview.yml',
    ],
    manualRequiredGaps: [
      'windows signing',
      'macOS notarization',
      'Google Play signing',
      'TestFlight device proof',
      'App Store proof',
      'production updater rollback',
      'production support runbook',
    ],
  };
}

function packagePreviewArtifacts(ciArtifactProof) {
  return [
    'ocentra-parent-windows-x64-preview',
    'ocentra-parent-linux-amd64-preview',
    'ocentra-parent-macos-preview',
    'ocentra-parent-android-preview',
    'ocentra-parent-ios-simulator-preview',
  ].map((artifactName) => ({
    artifactName,
    runStatus: ciArtifactProof.runStatus,
    artifactState: ciArtifactProof.artifactState,
    packageReadinessClaim: 'manual-required',
    manualProofRequirement: `${artifactName} requires manual platform signing or store proof before production readiness`,
  }));
}

function updaterRollbackRunbookProof() {
  return {
    proof: 'v8-updater-rollback-runbook-status',
    updaterRows: updaterRollbackRows(),
    runbookStatus: {
      draftRunbookState: 'preview-only',
      productionRunbookState: 'manual-required',
      rollbackTriageState: 'manual-required',
      requiredSections: [
        'rollback-triage',
        'rollback-failure-status',
        'teardown-revert-evidence',
        'diagnostics-redaction',
        'manual-platform-proof',
        'support-escalation-boundary',
      ],
      proofReferences: [
        'docs/expectations/release-installer.md',
        'docs/expectations/roadmap-v8-production-hardening.md',
        'test-results/parent-desktop-release-support-proof/proof.json',
      ],
      nonClaim: 'release support runbook status is preview-only not production support execution not update execution',
    },
    claimBoundary:
      'updater rollback runbook proof is not production update execution not signing not store upload proof',
    manualRequiredGaps: [
      'signed update channel',
      'production rollback execution',
      'rollback failure smoke',
      'published support runbook',
      'support escalation execution',
    ],
  };
}

function updaterRollbackRows() {
  return ['scaffold', 'unsigned-preview', 'signature-required', 'production'].map((channel) => ({
    channel,
    updateAvailabilityState:
      channel === 'unsigned-preview' ? 'available' : channel === 'scaffold' ? 'unavailable' : 'manual-required',
    checksumState:
      channel === 'unsigned-preview' || channel === 'signature-required' ? 'verified' : channel === 'scaffold' ? 'unavailable' : 'manual-required',
    signatureState: channel === 'scaffold' ? 'unavailable' : 'manual-required',
    rollbackState:
      channel === 'scaffold' || channel === 'unsigned-preview' ? 'rollback-unavailable' : 'manual-required',
    rollbackAvailabilityState:
      channel === 'scaffold' || channel === 'unsigned-preview' ? 'unavailable' : 'manual-required',
    teardownEvidenceState:
      channel === 'scaffold' || channel === 'unsigned-preview' ? 'recorded' : 'manual-required',
    revertEvidenceState:
      channel === 'scaffold' || channel === 'unsigned-preview' ? 'recorded' : 'manual-required',
    failureStatusState: 'manual-required',
    manualRequiredState: 'manual-required',
    proofRequirement:
      channel === 'production'
        ? 'production channel requires signed production update channel and manual proof before rollback execution teardown or revert evidence'
        : `${channel} channel requires teardown or revert evidence and manual proof before rollback execution or failure status claim`,
  }));
}

function manualRunbook(version) {
  return [
    'parent-desktop',
    'parent-mobile',
    'child-desktop',
    'child-android',
    'child-ios',
    'relay',
    'signing',
    'store',
    'support',
  ].map((target) => ({
    target,
    hostOrDevice: `${target} named manual host or device`,
    commandOrUiAction: `${target} package launch or UI proof action`,
    permissions: `${target} permissions and entitlement state recorded`,
    packageVersion: version,
    logsScreenshotsProofJson: `test-results/manual-platform-proof/${target}.json`,
    knownGaps: [`${target} requires manual proof before production claim`],
  }));
}
