import assert from 'node:assert/strict';
import { mkdtempSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { test } from 'node:test';

import {
  buildLaunchSmokeMatrixProof,
  loadSmokeSurfaceContracts,
  loadSmokeWorkflowSources,
  writeLaunchSmokeMatrixProof,
} from './parent-client-launch-smoke-matrix-proof.mjs';

const repoRoot = process.cwd();

test('workflow smoke wiring keeps parent Android and iOS rows separate from child-agent package defaults', async () => {
  const workflowSources = await loadSmokeWorkflowSources(repoRoot);

  assert.equal(workflowSources.desktopSmokeScripts.includes('scripts/smoke/windows-msi-smoke.ps1'), true);
  assert.equal(workflowSources.desktopSmokeScripts.includes('scripts/smoke/linux-deb-smoke.sh'), true);
  assert.equal(workflowSources.desktopSmokeScripts.includes('scripts/smoke/macos-pkg-smoke.sh'), true);
  assert.match(workflowSources.androidSmokeCommand, /ca\.ocentra\.parent\.mobile/u);
  assert.match(workflowSources.androidSmokeCommand, /ocentra-parent-mobile-android-debug-latest\.apk/u);
  assert.match(workflowSources.iosSmokeCommand, /OcentraParentMobile\.app/u);
  assert.match(workflowSources.iosSmokeCommand, /ca\.ocentra\.parent\.mobile/u);
});

test('launch smoke matrix keeps smoke rows explicit without claiming readiness, setup, or child runtime ownership', async () => {
  const proof = buildLaunchSmokeMatrixProof({
    checkedAt: '2026-06-28T17:20:00.000Z',
    workflowSources: await loadSmokeWorkflowSources(repoRoot),
    smokeSurfaceContracts: await loadSmokeSurfaceContracts(repoRoot),
    commandResults: {
      web: commandResult('node scripts/test/portal-local-smoke.mjs', '03-web-launch-smoke.log'),
      desktop: commandResult('node scripts/test/parent-desktop-shell-package-proof.mjs', '04-desktop-launch-smoke.log'),
      mobile: commandResult(
        'node scripts/test/parent-mobile-package-source-artifact-proof.mjs',
        '05-parent-mobile-launch-smoke.log'
      ),
    },
    desktopProof: {
      artifact_path: 'apps/parent-desktop/src-tauri/target/release/bundle/msi/Ocentra Parent.msi',
    },
    mobileProof: {
      packageLaunchProof: {
        android: { packageLifecycleState: 'manual-required' },
        ios: { packageLifecycleState: 'manual-required' },
      },
      runtimeProof: {
        androidObserver: {
          assistantJobState: 'degraded',
          capabilityStates: {
            'parent-mobile-observer': 'scaffold',
            'parent-mobile-controller': 'manual-required',
            notifications: 'manual-required',
          },
        },
        androidLanUnavailable: {
          assistantJobState: 'unavailable',
        },
        iosObserver: {
          assistantJobState: 'unavailable',
          capabilityStates: {
            'parent-mobile-observer': 'scaffold',
            'parent-mobile-controller': 'manual-required',
            'foreground-mobile-service': 'unavailable',
          },
        },
      },
    },
  });

  const rows = Object.fromEntries(proof.rows.map((row) => [row.artifactKind, row]));

  assert.equal(proof.smokeSurfaceCoverage, true);
  assert.equal(proof.degradedVisible, true);
  assert.equal(proof.unavailableVisible, true);
  assert.equal(proof.manualRequiredVisible, true);
  assert.equal(proof.readinessClaim, false);
  assert.equal(proof.setupCompletionClaim, false);
  assert.equal(proof.childRuntimeOwnershipClaim, false);
  assert.equal(rows.web.launchState, 'launched');
  assert.equal(rows.web.noClaimBoundary.includes('setup completion'), true);
  assert.equal(rows.desktop.launchState, 'manual-required');
  assert.equal(rows.desktop.visibleStates.includes('degraded'), true);
  assert.equal(rows.desktop.visibleStates.includes('manual-required'), true);
  assert.equal(rows.desktop.manualRequiredNote.includes('dry-run launch anchors'), true);
  assert.equal(rows['parent-android'].launchState, 'manual-required');
  assert.equal(rows['parent-android'].degradedVisible, true);
  assert.equal(rows['parent-android'].unavailableVisible, true);
  assert.equal(rows['parent-android'].noClaimBoundary.includes('child-agent runtime ownership'), true);
  assert.equal(rows['parent-ios'].launchState, 'manual-required');
  assert.equal(rows['parent-ios'].unavailableVisible, true);
  assert.equal(rows['parent-ios'].noClaimBoundary.includes('setup completion'), true);
});

test('launch smoke matrix proof writer emits the required WP09 artifact set', async () => {
  const outputDir = mkdtempSync(join(tmpdir(), 'ocentra-parent-launch-smoke-matrix-'));

  try {
    const proof = buildLaunchSmokeMatrixProof({
      checkedAt: '2026-06-28T17:20:00.000Z',
      workflowSources: await loadSmokeWorkflowSources(repoRoot),
      smokeSurfaceContracts: await loadSmokeSurfaceContracts(repoRoot),
      commandResults: {
        web: commandResult('node scripts/test/portal-local-smoke.mjs', '03-web-launch-smoke.log'),
        desktop: commandResult('node scripts/test/parent-desktop-shell-package-proof.mjs', '04-desktop-launch-smoke.log'),
        mobile: commandResult(
          'node scripts/test/parent-mobile-package-source-artifact-proof.mjs',
          '05-parent-mobile-launch-smoke.log'
        ),
      },
      desktopProof: {
        artifact_path: 'apps/parent-desktop/src-tauri/target/release/bundle/msi/Ocentra Parent.msi',
      },
      mobileProof: {
        packageLaunchProof: {
          android: { packageLifecycleState: 'manual-required' },
          ios: { packageLifecycleState: 'manual-required' },
        },
        runtimeProof: {
          androidObserver: {
            assistantJobState: 'degraded',
            capabilityStates: {
              'parent-mobile-observer': 'scaffold',
              'parent-mobile-controller': 'manual-required',
            },
          },
          androidLanUnavailable: {
            assistantJobState: 'unavailable',
          },
          iosObserver: {
            assistantJobState: 'unavailable',
            capabilityStates: {
              'foreground-mobile-service': 'unavailable',
              'parent-mobile-controller': 'manual-required',
            },
          },
        },
      },
    });

    const paths = await writeLaunchSmokeMatrixProof({
      outputDir,
      proof,
      commandResults: proof.commandResults,
    });

    const scopeSummary = readFileSync(paths.scopeSummaryPath, 'utf8');
    const negativeCase = readFileSync(paths.negativeCasePath, 'utf8');
    const manualRequired = readFileSync(paths.manualRequiredPath, 'utf8');
    const validationLog = readFileSync(paths.validationLogPath, 'utf8');

    assert.match(scopeSummary, /## Matrix/u);
    assert.match(scopeSummary, /parent-android/u);
    assert.match(negativeCase, /Desktop smoke surfaces are not setup completion or product readiness\./u);
    assert.match(manualRequired, /Desktop dry-run launch anchors and Rust service reachability proof passed/u);
    assert.match(manualRequired, /Android parent mobile launch stays manual-required/u);
    assert.match(validationLog, /node scripts\/test\/portal-local-smoke\.mjs/u);
    assert.match(validationLog, /result: pass/u);
  } finally {
    rmSync(outputDir, { recursive: true, force: true });
  }
});

test('launch smoke matrix falls back to blocked mobile rows without crashing when upstream proof output is missing', async () => {
  const proof = buildLaunchSmokeMatrixProof({
    checkedAt: '2026-06-28T17:20:00.000Z',
    workflowSources: await loadSmokeWorkflowSources(repoRoot),
    smokeSurfaceContracts: await loadSmokeSurfaceContracts(repoRoot),
    commandResults: {
      web: commandResult('node scripts/test/portal-local-smoke.mjs', '03-web-launch-smoke.log'),
      desktop: blockedCommandResult('node scripts/test/parent-desktop-shell-package-proof.mjs', '04-desktop-launch-smoke.log'),
      mobile: blockedCommandResult(
        'node scripts/test/parent-mobile-package-source-artifact-proof.mjs',
        '05-parent-mobile-launch-smoke.log'
      ),
    },
    desktopProof: null,
    mobileProof: null,
  });

  const rows = Object.fromEntries(proof.rows.map((row) => [row.artifactKind, row]));

  assert.equal(rows.desktop.launchState, 'blocked');
  assert.equal(rows['parent-android'].launchState, 'blocked');
  assert.equal(rows['parent-ios'].launchState, 'blocked');
  assert.equal(rows['parent-android'].visibleStates.includes('degraded'), true);
  assert.equal(rows['parent-ios'].visibleStates.includes('unavailable'), true);
  assert.equal(rows['parent-android'].manualRequiredNote.includes('blocked before artifact launch could be exercised'), true);
});

function commandResult(command, artifactName) {
  return {
    command,
    exit: 0,
    result: 'pass',
    artifact: `output/parent-client-runtime-distribution-plan-proof/09-parent-client-launch-smoke-matrix/${artifactName}`,
    diagnosticsSummary: `${artifactName} recorded smoke output without widening packet claims`,
  };
}

function blockedCommandResult(command, artifactName) {
  return {
    command,
    exit: 1,
    result: 'blocked',
    artifact: `output/parent-client-runtime-distribution-plan-proof/09-parent-client-launch-smoke-matrix/${artifactName}`,
    diagnosticsSummary: `${artifactName} captured an upstream blocker while preserving the smoke-state boundary`,
  };
}
