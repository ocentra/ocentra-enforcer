import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { test } from 'node:test';

import { buildReadModel } from './parent-desktop-release-support-read-model-fixture.mjs';

const repoRoot = process.cwd();

function ciArtifactProof() {
  return {
    workflowName: 'Package Preview',
    runStatus: 'not-checked-local',
    artifactState: 'not-checked-local',
    packageReadinessClaim: 'manual-required',
    checkedBy: 'node scripts/test/parent-desktop-release-support-proof.mjs',
    runUrl: null,
  };
}

test('WP08 read-model fixture keeps update, rollback, checksum, and signature states explicit by channel', () => {
  const readModel = buildReadModel('0.1.1', 'test-commit', ciArtifactProof());
  const byChannel = Object.fromEntries(readModel.updateStates.map((entry) => [entry.channel, entry]));

  assert.equal(byChannel.scaffold.updateAvailabilityState, 'unavailable');
  assert.equal(byChannel.scaffold.checksumState, 'unavailable');
  assert.equal(byChannel.scaffold.signatureState, 'unavailable');
  assert.equal(byChannel.scaffold.rollbackAvailabilityState, 'unavailable');
  assert.equal(byChannel.scaffold.teardownEvidenceState, 'recorded');
  assert.equal(byChannel.scaffold.revertEvidenceState, 'recorded');

  assert.equal(byChannel['unsigned-preview'].updateAvailabilityState, 'available');
  assert.equal(byChannel['unsigned-preview'].checksumState, 'verified');
  assert.equal(byChannel['unsigned-preview'].signatureState, 'manual-required');
  assert.equal(byChannel['unsigned-preview'].rollbackAvailabilityState, 'unavailable');
  assert.equal(byChannel['unsigned-preview'].teardownEvidenceState, 'recorded');
  assert.equal(byChannel['unsigned-preview'].revertEvidenceState, 'recorded');

  assert.equal(byChannel['signature-required'].updateAvailabilityState, 'manual-required');
  assert.equal(byChannel['signature-required'].rollbackAvailabilityState, 'manual-required');
  assert.equal(byChannel.production.updateAvailabilityState, 'manual-required');
  assert.equal(byChannel.production.rollbackAvailabilityState, 'manual-required');
});

test('WP08 updater rollback runbook keeps negative teardown or revert evidence explicit', () => {
  const readModel = buildReadModel('0.1.1', 'test-commit', ciArtifactProof());
  const byChannel = Object.fromEntries(
    readModel.updaterRollbackRunbookProof.updaterRows.map((entry) => [entry.channel, entry])
  );

  assert.equal(byChannel.scaffold.teardownEvidenceState, 'recorded');
  assert.equal(byChannel.scaffold.revertEvidenceState, 'recorded');
  assert.equal(byChannel['unsigned-preview'].teardownEvidenceState, 'recorded');
  assert.equal(byChannel['unsigned-preview'].revertEvidenceState, 'recorded');
  assert.equal(byChannel.production.teardownEvidenceState, 'manual-required');
  assert.equal(byChannel.production.revertEvidenceState, 'manual-required');
  assert.ok(
    readModel.updaterRollbackRunbookProof.runbookStatus.requiredSections.includes('teardown-revert-evidence')
  );
});

test('WP08 proof script uses schema-domain as the live contract and test owner', () => {
  const script = readFileSync(join(repoRoot, 'scripts', 'test', 'parent-desktop-release-support-proof.mjs'), 'utf8');

  assert.ok(script.includes("'test'"));
  assert.ok(script.includes("'--workspace'"));
  assert.ok(script.includes("'@ocentra-parent/schema-domain'"));
  assert.ok(script.includes("'tests/unit/parent-release-support-contracts.test.ts'"));
  assert.ok(script.includes("packages/schema-domain/tests/unit/parent-release-support-contracts.test.ts"));
  assert.doesNotMatch(script, new RegExp('@ocentra-parent\\/parent-' + 'domain', 'u'));
  assert.doesNotMatch(script, /parent-desktop-release-support\.test\.ts/u);
});
