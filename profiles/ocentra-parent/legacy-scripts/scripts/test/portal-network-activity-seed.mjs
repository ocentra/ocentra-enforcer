import { DatabaseSync } from 'node:sqlite';
import {
  NetworkEvidenceDrawerProofFixture,
  networkActivityEvidence,
  networkActivityFields,
  networkActivityObservedAt,
} from './network-evidence-drawer-proof-fixture.mjs';

export const PortalNetworkActivitySeed = Object.freeze({
  EventId: NetworkEvidenceDrawerProofFixture.eventId,
  EvidenceId: NetworkEvidenceDrawerProofFixture.evidenceId,
  JournalEvidenceId: NetworkEvidenceDrawerProofFixture.journalEvidenceId,
});

export function seedPortalNetworkActivityStore(activityDbPath) {
  const database = new DatabaseSync(activityDbPath);
  try {
    database.exec(`
PRAGMA foreign_keys = ON;
PRAGMA journal_mode = DELETE;
CREATE TABLE IF NOT EXISTS activity_events (
  event_id TEXT PRIMARY KEY,
  observed_at TEXT NOT NULL,
  device_id TEXT NOT NULL,
  platform TEXT NOT NULL,
  observer TEXT NOT NULL,
  kind TEXT NOT NULL,
  subject_kind TEXT NOT NULL,
  subject_id TEXT NOT NULL,
  subject_display_name TEXT,
  fields_json TEXT NOT NULL,
  evidence_json TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS activity_events_recent_idx
  ON activity_events (observed_at DESC, event_id DESC);
`);

    database.exec('BEGIN IMMEDIATE;');
    database
      .prepare(
        `
INSERT OR REPLACE INTO activity_events (
  event_id,
  observed_at,
  device_id,
  platform,
  observer,
  kind,
  subject_kind,
  subject_id,
  subject_display_name,
  fields_json,
  evidence_json
) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?);
`
      )
      .run(
        PortalNetworkActivitySeed.EventId,
        networkActivityObservedAt(),
        NetworkEvidenceDrawerProofFixture.deviceId,
        NetworkEvidenceDrawerProofFixture.platform,
        NetworkEvidenceDrawerProofFixture.observer,
        NetworkEvidenceDrawerProofFixture.kind,
        NetworkEvidenceDrawerProofFixture.subjectKind,
        NetworkEvidenceDrawerProofFixture.subjectId,
        NetworkEvidenceDrawerProofFixture.subjectDisplayName,
        JSON.stringify(networkActivityFields()),
        JSON.stringify(networkActivityEvidence())
      );
    database
      .prepare(
        `
INSERT OR REPLACE INTO activity_events (
  event_id,
  observed_at,
  device_id,
  platform,
  observer,
  kind,
  subject_kind,
  subject_id,
  subject_display_name,
  fields_json,
  evidence_json
) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?);
`
      )
      .run(
        'screen-summary-parent-explanation-service-event',
        screenActivityObservedAt(),
        'local-dev-agent',
        'windows',
        'local-ai',
        'activity.screen.analysis.summarized',
        'device',
        'local-dev-agent',
        null,
        JSON.stringify(screenActivityFields()),
        JSON.stringify(screenActivityEvidence())
      );
    database.exec('COMMIT;');
    assertSeededEvidence(database);
  } catch (error) {
    try {
      database.exec('ROLLBACK;');
    } catch {
      // Ignore rollback errors when SQLite already ended the transaction.
    }
    throw error;
  } finally {
    database.close();
  }
}

export function describePortalNetworkActivitySeedState(activityDbPath) {
  const database = new DatabaseSync(activityDbPath);
  try {
    const row = database
      .prepare(
        `
SELECT event_id, observed_at, device_id, kind, subject_kind, subject_id, fields_json, evidence_json
FROM activity_events
WHERE event_id = ?;
`
      )
      .get(PortalNetworkActivitySeed.EventId);
    if (row === undefined) {
      return { seeded: false, expectedEventId: PortalNetworkActivitySeed.EventId };
    }
    return {
      seeded: true,
      expectedEventId: PortalNetworkActivitySeed.EventId,
      expectedEvidenceId: PortalNetworkActivitySeed.EvidenceId,
      eventId: row.event_id,
      observedAt: row.observed_at,
      deviceId: row.device_id,
      kind: row.kind,
      subjectKind: row.subject_kind,
      subjectId: row.subject_id,
      fields: parseSeedJson(row.fields_json),
      evidenceIds: seedEvidenceIds(row.evidence_json),
    };
  } finally {
    database.close();
  }
}

function assertSeededEvidence(database) {
  const row = database
    .prepare(
      `
SELECT evidence_json
FROM activity_events
WHERE event_id = ?;
`
    )
    .get(PortalNetworkActivitySeed.EventId);
  if (row === undefined || typeof row.evidence_json !== 'string') {
    throw new Error('Network drawer E2E ActivityStore seed did not persist.');
  }
  if (!row.evidence_json.includes(PortalNetworkActivitySeed.EvidenceId)) {
    throw new Error('Network drawer E2E ActivityStore seed missed the expected evidence ref.');
  }
  const screenRow = database
    .prepare(
      `
SELECT fields_json
FROM activity_events
WHERE event_id = ?;
`
    )
    .get('screen-summary-parent-explanation-service-event');
  if (screenRow === undefined || typeof screenRow.fields_json !== 'string') {
    throw new Error('Screen summary E2E ActivityStore seed did not persist.');
  }
  if (!screenRow.fields_json.includes('screen-summary-parent-explanation-service-explanation')) {
    throw new Error('Screen summary E2E ActivityStore seed missed the parent explanation ref.');
  }
}

function parseSeedJson(value) {
  if (typeof value !== 'string') {
    return null;
  }
  try {
    return JSON.parse(value);
  } catch {
    return null;
  }
}

function screenActivityObservedAt() {
  return new Date(Date.now() + 6 * 60 * 1000).toISOString();
}

function seedEvidenceIds(value) {
  const parsed = parseSeedJson(value);
  if (!Array.isArray(parsed)) {
    return [];
  }
  return parsed.map((entry) => entry?.evidenceId).filter((evidenceId) => typeof evidenceId === 'string');
}

function screenActivityFields() {
  return {
    screenAnalysisResultId: 'screen-summary-parent-explanation-service-row',
    queueJobId: 'screen-summary-parent-explanation-service-queue',
    summary: 'Screen summary parent explanation is ready for parent audit.',
    primaryCategory: 'school',
    confidence: 0.94,
    imageDeletionState: 'deleted',
    policyEligible: true,
    modelRuntimeRef: 'screen-summary-parent-explanation-service-local-runtime',
    localModelRuntimeRefs: 'screen-summary-parent-explanation-service-local-runtime',
    modelId: 'windows-winrt-ocr-local-proof',
    providerKind: 'localOcr',
    promptOrTemplateVersion: 'screen-summary-parent-explanation-service-v1',
    captureReason: 'managedBrowserUrlChange',
    captureScope: 'selectedWindow',
    capabilityStatus: 'ready',
    imageDigest: 'sha256:screen-summary-parent-explanation-service-digest',
    custodyState: 'child-device-journal',
    policyDecisionId: 'screen-summary-parent-explanation-service-policy-decision',
    policyAction: 'allow',
    reasonCodes: 'screen-summary-linked,parent-rule-linked,deleted-image-linked',
    ruleIds: 'screen-summary-parent-explanation-service-parent-rule',
    parentExplanationRefs: 'screen-summary-parent-explanation-service-explanation',
    explanationReasons: 'screen-summary-cited,policy-decision-cited,parent-rule-cited',
    deletionReasons: 'screen-image-deleted',
  };
}

function screenActivityEvidence() {
  return [
    {
      evidenceId: 'screen-summary-parent-explanation-service-evidence',
      kind: 'journal-entry',
      digest: 'sha256:screen-summary-parent-explanation-service-digest',
      uri: null,
    },
  ];
}
