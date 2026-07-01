import { readFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const fixturePath = path.join(
  path.dirname(fileURLToPath(import.meta.url)),
  'fixtures',
  'network-evidence-drawer-proof.json'
);

export const NetworkEvidenceDrawerProofFixture = Object.freeze(JSON.parse(readFileSync(fixturePath, 'utf8')));

export function networkActivityObservedAt(now = new Date()) {
  return new Date(now.getTime() + NetworkEvidenceDrawerProofFixture.observedAtFutureOffsetMs).toISOString();
}

export function networkActivityFields() {
  return { ...NetworkEvidenceDrawerProofFixture.fields };
}

export function networkActivityEvidence() {
  return [
    {
      evidenceId: NetworkEvidenceDrawerProofFixture.evidenceId,
      kind: 'local-db-row',
      digest: NetworkEvidenceDrawerProofFixture.evidenceDigest,
      uri: null,
    },
    {
      evidenceId: NetworkEvidenceDrawerProofFixture.journalEvidenceId,
      kind: 'journal-entry',
      digest: NetworkEvidenceDrawerProofFixture.journalEvidenceDigest,
      uri: null,
    },
  ];
}

export function networkEvidenceReferenceIds() {
  return [NetworkEvidenceDrawerProofFixture.evidenceId, NetworkEvidenceDrawerProofFixture.journalEvidenceId];
}
