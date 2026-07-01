import { execFileSync } from 'node:child_process';
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join, relative, resolve } from 'node:path';

const repoRoot = process.cwd();
const outputDir = resolve(repoRoot, 'output', 'screen-ai-pipeline-proof', 'linux-host-adapter-execution');
const proofPath = join(outputDir, 'proof-summary.json');
const executionPath = join(outputDir, 'linux-host-adapter-execution.json');
const snapshotPath = join(outputDir, '00-source-snapshot.md');
const commandsPath = join(outputDir, '10-validation-commands.log');
const chainName = 'OCSAIPROOF';
const targetAddress = '203.0.113.42';

const sourceArtifacts = {
  blockPolicySource: 'output/screen-ai-pipeline-proof/block-action-dispatch/00-screen-block-source.json',
  linuxHostCustody: 'output/screen-ai-pipeline-proof/linux-host-adapter-custody/proof-summary.json',
  finalAdapterAudit: 'output/screen-ai-pipeline-proof/final-adapter-dependency-audit/proof-summary.json',
};

const failures = [];
const blockPolicySource = readJson(sourceArtifacts.blockPolicySource);
const linuxHostCustody = readJson(sourceArtifacts.linuxHostCustody);
const finalAdapterAudit = readJson(sourceArtifacts.finalAdapterAudit);

assert(process.platform === 'win32', 'Linux host adapter execution proof expects Windows-hosted WSL2 target');
assert(blockPolicySource.scenarioId === 'bypass-tool', 'Linux execution proof must use bypass-tool source');
assert(blockPolicySource.sourcePolicyAction === 'block', 'Linux execution proof must start from block action');
assert(blockPolicySource.sourcePolicyDryRun === true, 'source policy must remain dry-run');
assert(
  blockPolicySource.rawImageDeletedBeforeDispatch === true,
  'raw image must be deleted before Linux adapter handoff'
);
assert(
  linuxHostCustody.status === 'linux-host-custody-artifact-written-final-execution-blocked',
  'Linux custody artifact status changed'
);
assert(linuxHostCustody.closure?.linuxHostApplyCustodyRecorded === true, 'Linux apply custody missing');
assert(linuxHostCustody.closure?.linuxHostApplyExecuted === false, 'custody artifact should not claim execution');
assert(
  finalAdapterAudit.nextRequiredArtifacts?.some(
    (row) => row.expectedProofFile === 'output/screen-ai-pipeline-proof/linux-host-adapter-execution/proof-summary.json'
  ),
  'final adapter audit no longer expects Linux execution artifact'
);

if (failures.length > 0) {
  throw new Error(
    `Screen AI Linux host adapter execution proof failed before execution:\n${failures
      .map((failure) => `- ${failure}`)
      .join('\n')}`
  );
}

const commands = [];
let execution;

try {
  const beforeRules = runWsl('iptables -S OUTPUT; iptables -S OCSAIPROOF 2>/dev/null || true');
  const targetInfo = runWsl('uname -a; id; command -v iptables; iptables --version');

  cleanupProofChain();
  runWsl(`iptables -N ${chainName}`);
  runWsl(`iptables -I OUTPUT 1 -j ${chainName}`);
  runWsl(`iptables -A ${chainName} -d ${targetAddress} -j REJECT`);
  const appliedRule = runWsl(`iptables -C ${chainName} -d ${targetAddress} -j REJECT; iptables -S ${chainName}`);

  runWsl(`iptables -D ${chainName} -d ${targetAddress} -j REJECT`);
  runWsl(`iptables -D OUTPUT -j ${chainName}`);
  runWsl(`iptables -F ${chainName}`);
  runWsl(`iptables -X ${chainName}`);

  const afterRules = runWsl('iptables -S OUTPUT; iptables -S OCSAIPROOF 2>/dev/null || true');
  const rollbackClean = !afterRules.includes(chainName);

  execution = {
    schemaVersion: 'v0.6',
    executionId: 'screen-ai-linux-host-adapter-execution-wsl2',
    generatedAt: new Date().toISOString(),
    target: {
      kind: 'wsl2-linux-host',
      targetInfo,
      chainName,
      targetAddress,
    },
    sourcePolicyDecisionId: blockPolicySource.policyDecisionId,
    sourcePolicyAction: blockPolicySource.sourcePolicyAction,
    sourcePolicyDryRun: blockPolicySource.sourcePolicyDryRun,
    sourceEvidenceReferences: blockPolicySource.evidenceReferences,
    rawImageRetained: false,
    rawImageDeletedBeforeAdapter: blockPolicySource.rawImageDeletedBeforeDispatch,
    apply: {
      state: 'executed',
      commandFamily: 'iptables',
      appliedRuleContainsTarget: appliedRule.includes(targetAddress),
      appliedRuleContainsReject: appliedRule.toLowerCase().includes('reject'),
      beforeRules,
      appliedRule,
    },
    rollback: {
      state: rollbackClean ? 'executed-clean' : 'failed',
      afterRules,
      proofChainRemoved: rollbackClean,
    },
    audit: {
      auditRef: 'screen-ai-linux-host-adapter-execution-audit',
      sourceArtifacts,
      commands,
      custodyRef: sourceArtifacts.linuxHostCustody,
      rawImageRetained: false,
      rawImageDeletedBeforeAdapter: true,
    },
    claimBoundary:
      'This proves a reversible WSL2 Linux iptables host mutation for a screen-derived block decision; it does not claim native Linux desktop capture parity, Wayland/PipeWire proof, or broad Linux rollout readiness.',
  };

  const proof = {
    status: 'linux-host-adapter-execution-proved-wsl2',
    proofKind: 'screen-ai-linux-host-adapter-execution-proof',
    generatedAt: execution.generatedAt,
    sourceArtifacts,
    execution: relativePath(executionPath),
    closure: {
      screenDerivedBlockDecisionPreserved: true,
      rawImageDeletedBeforeAdapter: true,
      rawImageRetained: false,
      linuxWsl2HostMutationExecuted: execution.apply.state === 'executed',
      linuxWsl2RollbackExecuted: execution.rollback.state === 'executed-clean',
      linuxExecutionAuditRecorded: true,
      finalAdapterCompletionClaimed: false,
      nativeLinuxDesktopProductReady: false,
    },
    nonClaims: [
      'This proof does not claim native Linux desktop Wayland/PipeWire capture or control parity.',
      'This proof does not claim production Linux rollout readiness beyond the local WSL2 iptables target.',
      'This proof does not edit product checklist rows or mark the final adapter row product-complete.',
    ],
  };

  assert(proof.closure.linuxWsl2HostMutationExecuted === true, 'Linux WSL2 host mutation was not executed');
  assert(proof.closure.linuxWsl2RollbackExecuted === true, 'Linux WSL2 rollback was not clean');
  if (failures.length > 0) {
    throw new Error(
      `Screen AI Linux host adapter execution proof failed:\n${failures.map((failure) => `- ${failure}`).join('\n')}`
    );
  }

  mkdirSync(outputDir, { recursive: true });
  writeJson(executionPath, execution);
  writeJson(proofPath, proof);
  writeFileSync(snapshotPath, snapshot(proof, execution));
  writeFileSync(commandsPath, validationCommands(commands));
  console.log(`screen-ai-linux-host-adapter-execution-proof-ok:${relativePath(proofPath)}`);
} finally {
  cleanupProofChain();
}

function cleanupProofChain() {
  runWsl(`iptables -D ${chainName} -d ${targetAddress} -j REJECT 2>/dev/null || true`);
  runWsl(`iptables -D OUTPUT -j ${chainName} 2>/dev/null || true`);
  runWsl(`iptables -F ${chainName} 2>/dev/null || true`);
  runWsl(`iptables -X ${chainName} 2>/dev/null || true`);
}

function runWsl(script) {
  commands.push(`wsl.exe -e sh -lc ${JSON.stringify(script)}`);
  return execFileSync('wsl.exe', ['-e', 'sh', '-lc', script], {
    cwd: repoRoot,
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'pipe'],
  });
}

function readJson(path) {
  return JSON.parse(readText(path));
}

function readText(path) {
  const absolute = resolve(repoRoot, path);
  assert(existsSync(absolute), `missing source artifact ${path}`);
  return readFileSync(absolute, 'utf8');
}

function writeJson(path, value) {
  writeFileSync(path, `${JSON.stringify(value, null, 2)}\n`);
}

function snapshot(proof, execution) {
  return [
    '# Screen AI Linux Host Adapter Execution Proof',
    '',
    `Generated: ${proof.generatedAt}`,
    '',
    '## Closure',
    '',
    '```json',
    JSON.stringify(proof.closure, null, 2),
    '```',
    '',
    '## Target',
    '',
    '```json',
    JSON.stringify(execution.target, null, 2),
    '```',
    '',
  ].join('\n');
}

function validationCommands(executedCommands) {
  return [
    'node --check scripts/test/screen-ai-linux-host-adapter-execution-proof.mjs',
    'node scripts/test/screen-ai-linux-host-adapter-execution-proof.mjs',
    'git diff --check',
    'npm run lanes:guard',
    'npm run hub:guard',
    '',
    'Executed WSL commands:',
    ...executedCommands,
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
