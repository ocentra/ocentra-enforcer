export function supportIncidentHandoff() {
  const checkedAt = new Date().toISOString();
  return {
    metadata: {
      incidentId: 'support-incident-preview-001',
      status: 'triage-ready',
      severity: 'manual-required',
      productionSupportState: 'manual-required',
      supportBackendState: 'not-implemented',
      createdAt: checkedAt,
      updatedAt: checkedAt,
    },
    parentConsent: {
      consentState: 'parent-approved',
      capturedBy: 'manual-export-action',
      disclosureState: 'shown-before-export',
      parentActor: 'parent manually exported support bundle after disclosure',
      consentRecordedAt: checkedAt,
      revocationState: 'manual-required',
    },
    supportBundleManifest: supportBundleManifest(),
    diagnosticReferences: [
      supportDiagnosticReference('proof-json', 'test-results/parent-desktop-release-support-proof/proof.json'),
      supportDiagnosticReference('package-preview-workflow', '.github/workflows/package-preview.yml'),
      supportDiagnosticReference('redaction-summary', 'docs/expectations/release-installer.md'),
      supportDiagnosticReference('manual-runbook', 'test-results/manual-platform-proof/support.json'),
      supportDiagnosticReference('support-status-row', 'docs/features/production-distribution-support.md'),
    ],
    manualProductionSupportStates: {
      supportBackendUploadState: 'not-implemented',
      supportStaffAccessState: 'manual-required',
      accountLookupState: 'not-implemented',
      billingEscalationState: 'not-implemented',
      remoteControlState: 'not-implemented',
      productionSlaState: 'manual-required',
      nonClaims: [
        'no support backend upload is implemented by this proof',
        'no Ocentra-hosted child data custody is introduced by this proof',
        'no billing or public account support path is implemented by this proof',
      ],
    },
  };
}

function supportBundleManifest() {
  return {
    manifestId: 'support-bundle-preview-001',
    custodyBoundary: 'parent-exported-local-bundle',
    destination: 'parent-controlled-support-channel',
    disclosureState: 'shown-before-export',
    retentionState: 'manual-required',
    includedDataClasses: [
      'release-version',
      'commit-id',
      'platform-family',
      'package-runtime-state',
      'service-health-state',
      'route-state',
      'capability-state',
      'degraded-state',
      'redaction-summary',
      'manual-proof-reference',
      'incident-status',
    ],
    excludedDataClasses: [
      'tokens',
      'child-activity',
      'raw-urls',
      'screenshots',
      'journals',
      'sqlite-snapshots',
      'private-paths',
      'commands',
      'keystrokes',
      'clipboard-data',
      'message-contents',
    ],
    containsChildActivity: false,
    containsRawUrls: false,
    containsScreenshots: false,
    containsJournals: false,
    containsSqliteSnapshots: false,
    containsPrivatePaths: false,
    containsCommands: false,
    containsKeystrokes: false,
    containsClipboardData: false,
    containsMessageContents: false,
  };
}

function supportDiagnosticReference(kind, reference) {
  return {
    kind,
    reference,
    sourceState: 'preview-only',
    includesSensitiveData: false,
  };
}
