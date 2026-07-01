import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';
import { buildReadModel } from './parent-desktop-release-support-read-model-fixture.mjs';

const repoRoot = process.cwd();
const contractPackageExport = '@ocentra-parent/schema-domain/parent-desktop-release-support';
const outputDir = join(repoRoot, 'test-results', 'parent-desktop-release-support-proof');
const proofPath = join(outputDir, 'proof.json');
const proofCommand = 'node scripts/test/parent-desktop-release-support-proof.mjs';
const commands = [];

await main();

async function main() {
  await mkdir(outputDir, { recursive: true });
  await runCommand(
    ...npmCommand([
      'run',
      'test',
      '--workspace',
      '@ocentra-parent/schema-domain',
      '--',
      '--run',
      'tests/unit/parent-release-support-contracts.test.ts',
    ]),
    {}
  );

  const packageJson = JSON.parse(await readFile(join(repoRoot, 'package.json'), 'utf8'));
  const commit = await gitHead();
  const ciArtifactProof = await buildCiArtifactProof();
  const readModel = buildReadModel(packageJson.version, commit, ciArtifactProof);
  assertReadModel(readModel);

  const proof = {
    schemaVersion: 1,
    checkedAt: new Date().toISOString(),
    commit,
    proofMode: 'parent-desktop-release-support-proof',
    commands,
    evidence: {
      contract: 'packages/schema-domain/src/parent-desktop-release-support.ts',
      contractTest: 'packages/schema-domain/tests/unit/parent-release-support-contracts.test.ts',
      packageExport: contractPackageExport,
      output: relative(repoRoot, proofPath),
      packagePreviewWorkflow: '.github/workflows/package-preview.yml',
      readinessGate: 'v8-production-release-support-readiness',
      updaterRollbackRunbookProof: 'v8-updater-rollback-runbook-status',
      featureDocs: [
        'docs/features/production-distribution-support.md',
        'docs/features/child-agent-local-service.md',
        'docs/features/remote-lan-mobile-platforms.md',
      ],
    },
    readModel,
    workpacks: {
      completed: ['04', '06', '09', '10', '11', '12', '15', '16', '17', '18', '19', '20'],
      partial: [],
      partialReason: null,
    },
    claimsProved: [
      'Parent observer read-only state rejects policy writes, approvals, and controller takeover.',
      'Parent mobile bridge state is separate from child Android and child iOS agent claims.',
      'Parent desktop package runtime uses built portal dist, the Rust service boundary, fixed loopback ownership, and package service-manager launch evidence.',
      'Update available, unavailable, and manual-required states are explicit by channel without implying production release.',
      'Rollback available, unavailable, and manual-required surfaces stay explicit, and negative teardown or revert evidence is recorded where the current proof only supports unavailable preview paths.',
      'Checksum and signature truth stay explicit by channel instead of being hidden behind a broad update label.',
      'Support diagnostics include version, commit, platform, package, service, route, capability, and degraded state without secrets, private child data, raw URLs, command lines, keystrokes, clipboard data, message contents, journals, SQLite snapshots, screenshots, or private paths.',
      'Production support incident handoff requires parent consent, support incident status metadata, explicit safe support-bundle data classes, support-safe diagnostic references, and manual-required production support states.',
      'Package preview CI artifact status is recorded as pending/manual-required unless a real Actions artifact context proves readiness.',
      'V8 production release/support readiness gate summarizes Windows, Linux, macOS, Android, and iOS package-preview artifacts while keeping signing, stores, updater rollback execution, support runbook, and production publishing manual-required or promotion-required.',
      'V8 updater rollback and release-support runbook proof covers scaffold, unsigned preview, signature-required, and production channels with rollback execution unavailable, failure status manual-required, draft runbook preview-only, and production runbook manual-required.',
    ],
    claimsNotProved: [
      'signed release publishing',
      'store distribution',
      'macOS notarization',
      'production updater rollback',
      'production package-preview promotion',
      'production support workflow',
      'support backend upload',
      'billing or public account support',
      'mobile child-agent parity',
      'cloud relay proof',
    ],
  };

  await writeFile(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
  console.log(`parent-desktop-release-support-proof-ok:${relative(repoRoot, proofPath)}`);
}

async function buildCiArtifactProof() {
  const workflow = await readFile(join(repoRoot, '.github', 'workflows', 'package-preview.yml'), 'utf8');
  assert.match(workflow, /uses: actions\/upload-artifact@v6/u);
  assert.match(workflow, /ocentra-parent-windows-x64-preview/u);
  assert.match(workflow, /ocentra-parent-linux-amd64-preview/u);
  assert.match(workflow, /ocentra-parent-macos-preview/u);
  assert.match(workflow, /ocentra-parent-android-preview/u);
  assert.match(workflow, /ocentra-parent-ios-simulator-preview/u);

  if (process.env.GITHUB_ACTIONS === 'true') {
    const runId = process.env.GITHUB_RUN_ID ?? null;
    const server = process.env.GITHUB_SERVER_URL ?? 'https://github.com';
    const repository = process.env.GITHUB_REPOSITORY ?? 'ocentra/OcentraParent';
    return {
      workflowName: 'Package Preview',
      runStatus: 'pending',
      artifactState: 'pending',
      packageReadinessClaim: 'manual-required',
      checkedBy: proofCommand,
      runUrl: runId === null ? null : `${server}/${repository}/actions/runs/${runId}`,
    };
  }

  return {
    workflowName: 'Package Preview',
    runStatus: 'not-checked-local',
    artifactState: 'not-checked-local',
    packageReadinessClaim: 'manual-required',
    checkedBy: proofCommand,
    runUrl: null,
  };
}

function assertReadModel(readModel) {
  assert.equal(readModel.schemaVersion, 'parent-desktop-release-support-proof');
  assert.equal(readModel.observerAuthority.find((entry) => entry.operation === 'write-policy').result, 'rejected');
  assert.equal(readModel.mobileBridgeBoundary.childAndroidAgentState, 'manual-required');
  assert.equal(readModel.packageRuntimeEvidence.packageFrontendSource, 'built-portal-dist');
  assert.equal(readModel.packageRuntimeEvidence.backendBoundary, 'rust-service-boundary');
  assert.equal(readModel.packageRuntimeEvidence.serviceLaunchOwner, 'package-service-manager');
  assert.equal(readModel.packageRuntimeEvidence.portConflictPolicy, 'no-foreign-process-reclaim');
  assert.equal(readModel.packageRuntimeEvidence.processOwnership, 'parent-shell-only');
  const scaffold = readModel.updateStates.find((entry) => entry.channel === 'scaffold');
  const preview = readModel.updateStates.find((entry) => entry.channel === 'unsigned-preview');
  const gated = readModel.updateStates.find((entry) => entry.channel === 'signature-required');
  const production = readModel.updateStates.find((entry) => entry.channel === 'production');
  assert.equal(scaffold.updateAvailabilityState, 'unavailable');
  assert.equal(scaffold.checksumState, 'unavailable');
  assert.equal(scaffold.signatureState, 'unavailable');
  assert.equal(scaffold.rollbackAvailabilityState, 'unavailable');
  assert.equal(scaffold.teardownEvidenceState, 'recorded');
  assert.equal(scaffold.revertEvidenceState, 'recorded');
  assert.equal(preview.updateAvailabilityState, 'available');
  assert.equal(preview.checksumState, 'verified');
  assert.equal(preview.signatureState, 'manual-required');
  assert.equal(preview.rollbackAvailabilityState, 'unavailable');
  assert.equal(preview.teardownEvidenceState, 'recorded');
  assert.equal(preview.revertEvidenceState, 'recorded');
  assert.equal(gated.updateAvailabilityState, 'manual-required');
  assert.equal(gated.rollbackAvailabilityState, 'manual-required');
  assert.equal(production.updateAvailabilityState, 'manual-required');
  assert.equal(production.rollbackAvailabilityState, 'manual-required');
  assert.equal(readModel.ciArtifactProof.packageReadinessClaim, 'manual-required');
  assert.equal(readModel.supportDiagnostics.entries.length, 8);
  assert.equal(readModel.supportIncidentHandoff.parentConsent.consentState, 'parent-approved');
  assert.equal(readModel.supportIncidentHandoff.supportBundleManifest.containsRawUrls, false);
  assert.equal(
    readModel.supportIncidentHandoff.manualProductionSupportStates.supportBackendUploadState,
    'not-implemented'
  );
  assert.equal(readModel.manualRunbook.length, 9);
  assert.equal(readModel.productionReadinessGate.gate, 'v8-production-release-support-readiness');
  assert.equal(readModel.productionReadinessGate.packagePreviewArtifacts.length, 5);
  assert.equal(readModel.productionReadinessGate.updaterRollbackExecutionState, 'rollback-unavailable');
  assert.equal(readModel.productionReadinessGate.productionPublishingState, 'production-promotion-required');
  assert.deepEqual(
    readModel.updaterRollbackRunbookProof.updaterRows.map((entry) => entry.channel),
    ['scaffold', 'unsigned-preview', 'signature-required', 'production']
  );
  assert.equal(
    readModel.updaterRollbackRunbookProof.updaterRows.find((entry) => entry.channel === 'unsigned-preview')
      .teardownEvidenceState,
    'recorded'
  );
  assert.equal(
    readModel.updaterRollbackRunbookProof.updaterRows.find((entry) => entry.channel === 'unsigned-preview')
      .revertEvidenceState,
    'recorded'
  );
  assert.equal(
    readModel.updaterRollbackRunbookProof.updaterRows.find((entry) => entry.channel === 'production')
      .failureStatusState,
    'manual-required'
  );
  assert.equal(readModel.updaterRollbackRunbookProof.runbookStatus.draftRunbookState, 'preview-only');
  assert.equal(readModel.updaterRollbackRunbookProof.runbookStatus.productionRunbookState, 'manual-required');
  assert.equal(readModel.updaterRollbackRunbookProof.runbookStatus.requiredSections.length, 6);
}

async function runCommand(commandName, args, extraEnv = {}) {
  commands.push([commandName, ...args].join(' '));
  await new Promise((resolve, reject) => {
    const child = spawn(commandName, args, {
      cwd: repoRoot,
      stdio: 'inherit',
      windowsHide: true,
      env: { ...process.env, ...extraEnv },
    });
    child.once('exit', (code) =>
      code === 0 ? resolve() : reject(new Error(`${commandName} ${args.join(' ')} exited with ${code}`))
    );
    child.once('error', reject);
  });
}

async function gitHead() {
  const chunks = [];
  await new Promise((resolve, reject) => {
    const child = spawn('git', ['rev-parse', 'HEAD'], { cwd: repoRoot, stdio: ['ignore', 'pipe', 'pipe'] });
    child.stdout.on('data', (chunk) => chunks.push(String(chunk)));
    child.once('exit', (code) => (code === 0 ? resolve() : reject(new Error('git rev-parse HEAD failed'))));
    child.once('error', reject);
  });
  return chunks.join('').trim();
}

function npmCommand(args) {
  const command = process.platform === 'win32' ? 'cmd' : 'npm';
  const commandArgs = process.platform === 'win32' ? ['/c', 'npm', ...args] : args;
  return [command, commandArgs];
}
