import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { mkdtemp, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { DatabaseSync } from 'node:sqlite';
import { test } from 'node:test';

import { NetworkEvidenceDrawerProofFixture } from './network-evidence-drawer-proof-fixture.mjs';
import { PortalNetworkActivitySeed, seedPortalNetworkActivityStore } from './portal-network-activity-seed.mjs';

test('portal e2e owns agent and portal cleanup outside Playwright webServer', () => {
  const portalManifest = JSON.parse(readFileSync('apps/portal/package.json', 'utf8'));
  const configSource = readFileSync('apps/portal/playwright.config.ts', 'utf8');
  const runnerSource = readFileSync('scripts/test/portal-playwright-runner.mjs', 'utf8');
  const processSource = readFileSync('scripts/test/agent-service-process.mjs', 'utf8');

  assert.equal(portalManifest.scripts['test:e2e'], 'node ../../scripts/test/portal-playwright-runner.mjs');
  assert.equal(configSource.includes('webServer'), false);
  assert.equal(configSource.includes('OCENTRA_PARENT_PORTAL_PORT'), true);
  assert.equal(runnerSource.includes('stopProcessTree'), true);
  assert.equal(runnerSource.includes('ensureParentDevBridgeBinaryUnlocked'), true);
  assert.equal(runnerSource.includes('signal !== null'), true);
  assert.equal(runnerSource.includes('SIGKILL'), true);
  assert.equal(runnerSource.includes('resolveParentDevPort'), true);
  assert.equal(runnerSource.includes('assertAgentNetworkActivityReadModel'), true);
  assert.equal(processSource.includes('child.exitCode !== null || child.signalCode !== null'), true);
  assert.equal(processSource.includes("taskkill', ['/IM', imageName, '/T', '/F']"), true);
  assert.equal(processSource.includes('ocentra-parent-dev-bridge.exe'), true);
});

test('portal local smoke waits for process shutdown before temp cleanup', () => {
  const smokeSource = readFileSync('scripts/test/portal-local-smoke.mjs', 'utf8');
  const stopIndex = smokeSource.indexOf('await Promise.all([stopProcess(portal), stopProcess(agent)])');
  const removeIndex = smokeSource.indexOf('await removeDirectoryWithRetry(devLogDir)');

  assert.notEqual(stopIndex, -1);
  assert.notEqual(removeIndex, -1);
  assert.equal(stopIndex < removeIndex, true);
  assert.equal(smokeSource.includes('stopProcessTreeAndWait'), true);
  assert.equal(smokeSource.includes('resolveParentDevPort'), true);
});

test('portal local smoke typed activity timeout is configurable and diagnostic', () => {
  const smokeSource = readFileSync('scripts/test/portal-local-smoke.mjs', 'utf8');

  assert.equal(smokeSource.includes('OCENTRA_PARENT_PORTAL_ACTIVITY_SMOKE_TIMEOUT_MS'), true);
  assert.equal(smokeSource.includes('typedActivityAdapterSmokeTimeoutMs'), true);
  assert.equal(smokeSource.includes('describeTypedActivityTimeout(steps, stepIndex)'), true);
  assert.equal(smokeSource.includes('while waiting for ${step.event}'), true);
  assert.equal(smokeSource.includes('from ${step.command}'), true);
  assert.equal(smokeSource.includes("new Error('Typed Activity adapter smoke timed out')"), false);
  assert.equal(smokeSource.includes('), 10000);'), false);
});

test('portal network activity seed persists evidence before Rust service startup', async () => {
  const runRoot = await mkdtemp(path.join(tmpdir(), 'ocentra-parent-network-seed-'));
  const activityDbPath = path.join(runRoot, 'activity.sqlite');
  try {
    seedPortalNetworkActivityStore(activityDbPath);
    const database = new DatabaseSync(activityDbPath);
    try {
      const journalMode = database.prepare('PRAGMA journal_mode;').get();
      const row = database
        .prepare(
          `
SELECT evidence_json
FROM activity_events
WHERE event_id = ?;
`
        )
        .get(PortalNetworkActivitySeed.EventId);

      assert.equal(String(Object.values(journalMode)[0]), 'delete');
      assert.equal(typeof row.evidence_json, 'string');
      assert.equal(row.evidence_json.includes(PortalNetworkActivitySeed.EvidenceId), true);
      assert.equal(row.evidence_json.includes(PortalNetworkActivitySeed.JournalEvidenceId), true);
    } finally {
      database.close();
    }
  } finally {
    await rm(runRoot, { recursive: true, force: true });
  }
});

test('portal network activity service preflight uses shared protocol command and seed refs', () => {
  const preflightSource = readFileSync('scripts/test/portal-network-activity-service-preflight.mjs', 'utf8');

  assert.equal(preflightSource.includes('AgentCommand.NetworkFlowReadModelGet'), true);
  assert.equal(preflightSource.includes('AgentEvent.NetworkFlowReadModelReported'), true);
  assert.equal(preflightSource.includes('AgentEventEnvelopeSchema.parse'), true);
  assert.equal(preflightSource.includes('PortalNetworkActivitySeed.EvidenceId'), true);
});

test('network drawer proof ids stay single-sourced across scripts and portal tests', () => {
  const e2eSource = readFileSync('apps/portal/e2e/network-evidence-drawer-proof.spec.ts', 'utf8');
  const unitSource = readFileSync('apps/portal/tests/live-activity/live-activity-network-flow.test.ts', 'utf8');
  const seedSource = readFileSync('scripts/test/portal-network-activity-seed.mjs', 'utf8');
  const proofSource = readFileSync('scripts/test/network-parent-ui-evidence-drawer-proof.mjs', 'utf8');

  assert.equal(NetworkEvidenceDrawerProofFixture.eventId, PortalNetworkActivitySeed.EventId);
  assert.equal(NetworkEvidenceDrawerProofFixture.evidenceId, PortalNetworkActivitySeed.EvidenceId);
  assert.equal(NetworkEvidenceDrawerProofFixture.journalEvidenceId, PortalNetworkActivitySeed.JournalEvidenceId);
  assert.equal(e2eSource.includes(NetworkEvidenceDrawerProofFixture.eventId), false);
  assert.equal(e2eSource.includes(NetworkEvidenceDrawerProofFixture.evidenceId), false);
  assert.equal(unitSource.includes(NetworkEvidenceDrawerProofFixture.eventId), false);
  assert.equal(unitSource.includes(NetworkEvidenceDrawerProofFixture.evidenceId), false);
  assert.equal(seedSource.includes(NetworkEvidenceDrawerProofFixture.eventId), false);
  assert.equal(seedSource.includes(NetworkEvidenceDrawerProofFixture.evidenceId), false);
  assert.equal(proofSource.includes(NetworkEvidenceDrawerProofFixture.eventId), false);
  assert.equal(proofSource.includes(NetworkEvidenceDrawerProofFixture.evidenceId), false);
});
