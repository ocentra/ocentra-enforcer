import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join, relative, resolve } from 'node:path';

const repoRoot = process.cwd();
const outputDir = resolve(repoRoot, 'output', 'screen-ai-pipeline-proof', 'adapter-blocker-ledger');
const proofPath = join(outputDir, 'proof-summary.json');
const ledgerPath = join(outputDir, 'adapter-blocker-ledger.json');
const snapshotPath = join(outputDir, '00-adapter-blocker-ledger.md');
const commandsPath = join(outputDir, '10-validation-commands.log');

const sourceArtifacts = {
  finalAdapterDependencyAudit: 'output/screen-ai-pipeline-proof/final-adapter-dependency-audit/proof-summary.json',
  adapterReadinessReadModel: 'output/screen-ai-pipeline-proof/adapter-readiness/read-model.json',
  finalProductPath: 'output/screen-ai-pipeline-proof/final-product-path/proof-summary.json',
  screenAiPipelineChecklist: 'docs/plans/screen-ai-pipeline-plan/implementation-checklist.md',
  aiPlanChecklist: 'docs/plans/ai-plan/implementation-checklist.md',
};

const expectedBlockers = [
  {
    rowId: 'screen-ai-broad-installed-app-manual-required',
    adapterClass: 'broad-installed-app',
    expectedSourceBoundary: 'app-game/enforcement adapter layer',
    requiredProofArtifact: 'screen-derived broad installed-app apply, rollback, and audit custody proof',
    unblocksRows: [
      'screen-ai-pipeline-plan: browser/network/mobile/broad adapter completion row',
      'ai-plan: final product-complete pipeline deferral row',
      'product-capability-checklist: Local screen evidence summaries',
      'product-capability-checklist: Child-safety AI decision',
    ],
  },
  {
    rowId: 'screen-ai-host-network-domain-manual-required',
    adapterClass: 'host-network-domain',
    expectedSourceBoundary: 'network/domain enforcement adapter layer',
    requiredProofArtifact:
      'screen-derived domain or IP decision to host DNS/filter apply, rollback, and audit custody proof',
    unblocksRows: [
      'screen-ai-pipeline-plan: browser/network/mobile/broad adapter completion row',
      'ai-plan: final product-complete pipeline deferral row',
      'product-capability-checklist: Local screen evidence summaries',
      'product-capability-checklist: Child-safety AI decision',
    ],
  },
  {
    rowId: 'screen-ai-managed-active-tab-not-claimed',
    adapterClass: 'managed-active-tab-exact-url',
    expectedSourceBoundary: 'browser managed-control adapter layer',
    requiredProofArtifact:
      'screen-derived browser decision to exact active-tab URL action, rollback, and audit custody proof',
    unblocksRows: [
      'screen-ai-pipeline-plan: browser/network/mobile/broad adapter completion row',
      'ai-plan: final product-complete pipeline deferral row',
      'product-capability-checklist: Local screen evidence summaries',
      'product-capability-checklist: Child-safety AI decision',
    ],
  },
  {
    rowId: 'screen-ai-android-mobile-control-manual-required',
    adapterClass: 'android-device-owner-or-managed-profile',
    expectedSourceBoundary: 'Android child-agent Device Owner or managed-profile adapter layer',
    requiredProofArtifact:
      'screen-derived mobile decision to Android DO/profile action with rollback, audit, and custody proof on real device or managed profile',
    unblocksRows: [
      'screen-ai-pipeline-plan: browser/network/mobile/broad adapter completion row',
      'ai-plan: final product-complete pipeline deferral row',
      'product-capability-checklist: Local screen evidence summaries',
      'product-capability-checklist: Child-safety AI decision',
    ],
  },
  {
    rowId: 'screen-ai-ios-mobile-control-manual-required',
    adapterClass: 'ios-family-controls-device-activity',
    expectedSourceBoundary: 'iOS Family Controls and DeviceActivity adapter layer',
    requiredProofArtifact:
      'screen-derived mobile decision to iOS Family Controls or DeviceActivity action with rollback, audit, and custody proof',
    unblocksRows: [
      'screen-ai-pipeline-plan: browser/network/mobile/broad adapter completion row',
      'ai-plan: final product-complete pipeline deferral row',
      'product-capability-checklist: Local screen evidence summaries',
      'product-capability-checklist: Child-safety AI decision',
    ],
  },
];

const failures = [];
const finalAdapterDependencyAudit = readJson(sourceArtifacts.finalAdapterDependencyAudit);
const adapterReadinessReadModel = readJson(sourceArtifacts.adapterReadinessReadModel);
const finalProductPath = readJson(sourceArtifacts.finalProductPath);
const screenAiPipelineChecklist = readText(sourceArtifacts.screenAiPipelineChecklist);
const aiPlanChecklist = readText(sourceArtifacts.aiPlanChecklist);
const readinessRowById = new Map(adapterReadinessReadModel.rows.map((row) => [row.rowId, row]));
const auditBlockerById = new Map(finalAdapterDependencyAudit.blockedRows.map((row) => [row.rowId, row]));

assert(
  finalAdapterDependencyAudit.status === 'blocked-by-upstream-adapter-artifacts',
  'final adapter dependency audit must remain blocked by upstream adapter artifacts'
);
assert(
  finalAdapterDependencyAudit.closure?.broadBrowserNetworkMobileProductComplete === false,
  'final adapter dependency audit unexpectedly claims broad/browser/network/mobile completion'
);
assert(
  finalAdapterDependencyAudit.closure?.blockedAdapterRows === 5,
  'final adapter dependency audit blocker count changed'
);
assert(
  finalAdapterDependencyAudit.closure?.linuxHostExecutionRows === 1,
  'final adapter dependency audit lost WSL2 Linux execution row'
);
assert(finalProductPath.status === 'ok', 'final product path artifact gate must remain ok');
assert(finalProductPath.closure?.portalReadModelProven === true, 'portal/read-model proof is not retained');
assert(finalProductPath.closure?.retentionCustodyProven === true, 'retention custody proof is not retained');
assert(
  screenAiPipelineChecklist.includes(
    '- [ ] Browser, network, mobile, and broad block adapters proven from screen-derived decisions before product-complete action claims.'
  ),
  'screen-ai pipeline product-complete adapter row must stay open'
);
assert(
  aiPlanChecklist.includes(
    '- [ ] Final product-complete pipeline proof is deferred to `docs/plans/screen-ai-pipeline-plan` after screen and AI prerequisites are merged or explicitly stacked.'
  ),
  'AI plan final product-complete deferral row must stay open'
);

const blockerRows = expectedBlockers.map((blocker) => buildBlockerLedgerRow(blocker));
assert(blockerRows.length === 5, 'expected five remaining blocked adapter ledger rows');
assert(
  blockerRows.every((row) => row.claimUpgradeAllowed === false),
  'every blocker ledger row must reject claim upgrades'
);

if (failures.length > 0) {
  throw new Error(
    `Screen AI adapter blocker ledger proof failed:\n${failures.map((failure) => `- ${failure}`).join('\n')}`
  );
}

const ledger = {
  schemaVersion: 'v0.6',
  ledgerId: 'screen-ai-adapter-blocker-ledger',
  generatedAt: new Date().toISOString(),
  sourceArtifacts,
  rows: blockerRows,
};

const proof = {
  status: 'blocked-but-actionable',
  proofKind: 'screen-ai-adapter-blocker-ledger-proof',
  generatedAt: ledger.generatedAt,
  sourceArtifacts,
  ledger: relativePath(ledgerPath),
  closure: {
    finalProductPathStillValid: true,
    adapterCompletionStillBlocked: true,
    openScreenAiPipelineRowRetained: true,
    openAiPlanDeferralRowRetained: true,
    blockerRows: blockerRows.length,
    requiredProofArtifactRows: blockerRows.length,
    linuxWsl2HostExecutionNoLongerBlocked: true,
    claimUpgradeRows: blockerRows.filter((row) => row.claimUpgradeAllowed).length,
  },
  rows: blockerRows,
  unblockedSafeWork: [
    'Keep codex/screen-ai-full-scope-b synced with main.',
    'Add proof scripts or plan rows that audit missing adapter artifacts without touching docs/product-capability-checklist.md while another lane owns it.',
    'Validate new adapter artifacts when upstream lanes provide them, then close only the matching blocker row.',
  ],
  nonClaims: [
    'This proof does not implement broad installed-app, host network/domain, managed active-tab, Android, iOS, or native Linux desktop product-complete adapters.',
    'This proof does not close the final product-complete adapter row.',
    'This proof does not edit docs/product-capability-checklist.md while another lane owns that lock.',
  ],
};

writeOutputs(ledger, proof);
console.log(`screen-ai-adapter-blocker-ledger-proof-ok:${relativePath(proofPath)}`);

function buildBlockerLedgerRow(blocker) {
  const readinessRow = readinessRowById.get(blocker.rowId);
  const auditRow = auditBlockerById.get(blocker.rowId);
  assert(Boolean(readinessRow), `missing adapter readiness row ${blocker.rowId}`);
  assert(Boolean(auditRow), `missing final adapter audit row ${blocker.rowId}`);

  const readinessState = readinessRow ? readinessRow.readinessState : 'missing';
  const actionExecutionState = readinessRow ? readinessRow.actionExecutionState : 'missing';
  const adapterExecutionProofArtifact = readinessRow ? readinessRow.adapterExecutionProofArtifact : 'missing';
  const rawImageRetained = readinessRow ? readinessRow.rawImageRetained : true;
  const rawImageDeletedBeforeAdapter = readinessRow ? readinessRow.rawImageDeletedBeforeAdapter : false;
  const claimFlagsAllFalse = readinessRow
    ? Object.values(readinessRow.claimFlags).every((flag) => flag === false)
    : false;

  assert(actionExecutionState === 'skipped', `${blocker.rowId} unexpectedly executed`);
  assert(adapterExecutionProofArtifact === null, `${blocker.rowId} unexpectedly has adapter execution proof`);
  assert(rawImageRetained === false, `${blocker.rowId} unexpectedly retained raw image`);
  assert(rawImageDeletedBeforeAdapter === true, `${blocker.rowId} lacks deleted-image custody`);
  assert(claimFlagsAllFalse, `${blocker.rowId} contains claim flag upgrade`);
  assert(
    auditRow?.missingArtifact?.length > 0,
    `${blocker.rowId} final adapter audit does not name the missing artifact`
  );

  return {
    ...blocker,
    readinessState,
    actionExecutionState,
    adapterExecutionProofArtifact,
    rawImageRetained,
    rawImageDeletedBeforeAdapter,
    finalAuditMissingArtifact: auditRow?.missingArtifact ?? null,
    claimUpgradeAllowed: false,
    claimGate:
      'Do not move this row to product-complete until requiredProofArtifact exists and cites a screen-derived policy decision, apply result, rollback or expiry result, audit ref, and custody/deletion ref.',
  };
}

function readJson(path) {
  return JSON.parse(readText(path));
}

function readText(path) {
  const absolute = resolve(repoRoot, path);
  assert(existsSync(absolute), `missing source artifact ${path}`);
  return readFileSync(absolute, 'utf8');
}

function writeOutputs(ledger, proof) {
  mkdirSync(outputDir, { recursive: true });
  writeFileSync(ledgerPath, `${JSON.stringify(ledger, null, 2)}\n`);
  writeFileSync(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
  writeFileSync(snapshotPath, markdownSnapshot(proof));
  writeFileSync(commandsPath, validationCommands());
}

function markdownSnapshot(proof) {
  const rows = proof.rows
    .map(
      (row) =>
        `- ${row.adapterClass}: ${row.requiredProofArtifact}; source boundary: ${row.expectedSourceBoundary}; readiness: ${row.readinessState}.`
    )
    .join('\n');
  return `# Screen AI Adapter Blocker Ledger Proof\n\nGenerated: ${proof.generatedAt}\n\nStatus: ${proof.status}\n\n## Blocked Adapter Evidence\n\n${rows}\n\n## Closure\n\n\`\`\`json\n${JSON.stringify(proof.closure, null, 2)}\n\`\`\`\n`;
}

function validationCommands() {
  return [
    'node --check scripts/test/screen-ai-adapter-blocker-ledger-proof.mjs',
    'node scripts/test/screen-ai-adapter-blocker-ledger-proof.mjs',
    'git diff --check',
    'npm run lanes:guard',
    'npm run hub:guard',
    '',
  ].join('\n');
}

function relativePath(path) {
  return relative(repoRoot, path).replaceAll('\\', '/');
}

function assert(condition, message) {
  if (!condition) {
    failures.push(message);
  }
}
