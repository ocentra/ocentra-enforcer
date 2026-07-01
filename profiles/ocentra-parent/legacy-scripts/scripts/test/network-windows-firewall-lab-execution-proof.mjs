import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { mkdirSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const proofRoot = join('output', 'network-plan-proof', '38a-windows-firewall-lab-execution-proof');
const testRoot = join('test-results', 'network-windows-firewall-lab-execution-proof');
mkdirSync(proofRoot, { recursive: true });
mkdirSync(testRoot, { recursive: true });

const ruleName = process.env.NETWORK_WINDOWS_FIREWALL_LAB_RULE ?? 'OcentraParentNetworkLab-row38a';
const targetRemoteAddress = process.env.NETWORK_WINDOWS_FIREWALL_LAB_TARGET ?? '203.0.113.254';
const executeLab = process.env.NETWORK_WINDOWS_FIREWALL_LAB_EXECUTE === '1';

const hostLabExecution = collectHostLabExecution();
writeFileSync(join(proofRoot, 'host-lab-execution.json'), `${JSON.stringify(hostLabExecution, null, 2)}\n`);

writeFileSync(
  join(proofRoot, 'expected-windows-firewall-lab-execution-proof.json'),
  `${JSON.stringify(
    {
      row: '38a Windows Firewall lab execution proof',
      ruleName,
      targetRemoteAddress,
      safeTargetClass: 'RFC 5737 TEST-NET',
      commandSequence: ['apply lab rule', 'verify lab rule present', 'rollback lab rule', 'verify lab rule removed'],
      executionOptIn: 'NETWORK_WINDOWS_FIREWALL_LAB_EXECUTE=1',
      requiredRustState:
        'ExecutedAndRolledBack only when apply-ready adapter proof, admin permission, safe rule name, TEST-NET target, successful apply/verify/rollback/verify-removed evidence exist',
      fallbackState: 'ManualRequired or Unavailable without host/admin/command evidence',
      notClaimed: [
        'production enforcement',
        'persistent firewall rule',
        'policy engine execution',
        'enforcement command publication',
        'exact URL from network-only evidence',
        'decrypted payload or page content',
      ],
    },
    null,
    2
  )}\n`
);

const commands = [
  {
    name: 'network-windows-firewall-lab-execution-tests',
    command: 'cargo',
    args: ['test', '-p', 'ocentra-network-evidence', 'windows_firewall_lab_execution'],
    log: join(proofRoot, 'windows-firewall-lab-execution-tests.log'),
  },
  {
    name: 'network-windows-firewall-adapter-tests',
    command: 'cargo',
    args: ['test', '-p', 'ocentra-network-evidence', 'windows_firewall_adapter'],
    log: join(proofRoot, 'windows-firewall-adapter-tests.log'),
  },
  {
    name: 'network-evidence-clippy',
    command: 'cargo',
    args: ['clippy', '-p', 'ocentra-network-evidence', '--all-targets', '--', '-D', 'warnings'],
    log: join(proofRoot, 'clippy.log'),
  },
  {
    name: 'source-shape',
    command: 'node',
    args: ['scripts/check-source-shape.mjs'],
    log: join(proofRoot, 'source-shape.log'),
  },
  {
    name: 'diff-check',
    command: 'git',
    args: ['diff', '--check'],
    log: join(proofRoot, 'diff-check.log'),
  },
];
const commandResults = commands.map(runCommand);
writeFileSync(join(proofRoot, 'validation-commands.log'), validationLog(commandResults));

const proof = {
  proof: 'network-windows-firewall-lab-execution',
  checkedAt: new Date().toISOString(),
  branch: runText('git', ['branch', '--show-current']).trim(),
  commit: runText('git', ['rev-parse', 'HEAD']).trim(),
  originMain: runText('git', ['rev-parse', 'origin/main']).trim(),
  mergeBase: runText('git', ['merge-base', 'HEAD', 'origin/main']).trim(),
  statusShort: runText('git', ['status', '--short']),
  proofRoot,
  testRoot,
  commands: commandResults,
  hostLabExecution,
  artifacts: {
    expectedWindowsFirewallLabExecutionProof: join(proofRoot, 'expected-windows-firewall-lab-execution-proof.json'),
    hostLabExecution: join(proofRoot, 'host-lab-execution.json'),
    validationCommands: join(proofRoot, 'validation-commands.log'),
    proofSummary: join(proofRoot, 'proof-summary.json'),
    testProof: join(testRoot, 'proof.json'),
  },
  provenRows: ['38a Windows Firewall lab execution proof'],
  provenRootGates: [
    'Rust model requires apply-ready row38 adapter proof before lab execution can be accepted',
    'Rust model accepts only Ocentra lab rule names and RFC 5737 TEST-NET targets',
    'Rust model requires apply, verify-present, rollback, and verify-removed command evidence',
    'read-only and non-admin hosts remain manual-required or unavailable instead of claiming execution',
    'production enforcement, persistent firewall rules, policy execution, enforcement commands, and content claims are rejected',
  ],
  notClaimed: [
    'production Windows Firewall enforcement',
    'persistent host firewall mutation',
    'policy engine execution',
    'enforcement command publication',
    'exact URL, page content, or decrypted payload from network-only evidence',
    'Windows WFP, Android VpnService, Apple Network Extension, or Linux adapter execution',
  ],
};
writeFileSync(join(proofRoot, 'proof-summary.json'), `${JSON.stringify(proof, null, 2)}\n`);
writeFileSync(join(testRoot, 'proof.json'), `${JSON.stringify(proof, null, 2)}\n`);
console.log('network-windows-firewall-lab-execution-proof-ok:lab-tests,adapter-tests,clippy,source-shape,diff-check');
console.log(`hostLabState=${hostLabExecution.state}`);
console.log(`proof=${join(proofRoot, 'proof-summary.json')}`);

function collectHostLabExecution() {
  const windowsHostObserved = process.platform === 'win32';
  const readOnlyState = windowsHostObserved ? runProbe('netsh', ['advfirewall', 'show', 'allprofiles', 'state']) : null;
  const adminProbe = windowsHostObserved ? runProbe('net', ['session']) : null;
  const administratorPermissionObserved = adminProbe?.status === 0;
  const commandEvidence = [];
  const cleanupEvidence = [];

  if (!windowsHostObserved) {
    return {
      state: 'unavailable',
      executeLab,
      windowsHostObserved,
      administratorPermissionObserved: false,
      ruleName,
      targetRemoteAddress,
      readOnlyState: summarizeOptionalProbe(readOnlyState),
      commandEvidence,
      cleanupEvidence,
      notes: ['Host is not Windows; lab firewall mutation is unavailable.'],
    };
  }

  if (!executeLab) {
    return {
      state: 'manual_required',
      executeLab,
      windowsHostObserved,
      administratorPermissionObserved,
      ruleName,
      targetRemoteAddress,
      readOnlyState: summarizeOptionalProbe(readOnlyState),
      adminProbe: summarizeProbe(adminProbe),
      commandEvidence,
      cleanupEvidence,
      notes: ['Set NETWORK_WINDOWS_FIREWALL_LAB_EXECUTE=1 to run the reversible lab mutation.'],
    };
  }

  if (!administratorPermissionObserved) {
    return {
      state: 'manual_required',
      executeLab,
      windowsHostObserved,
      administratorPermissionObserved,
      ruleName,
      targetRemoteAddress,
      readOnlyState: summarizeOptionalProbe(readOnlyState),
      adminProbe: summarizeProbe(adminProbe),
      commandEvidence,
      cleanupEvidence,
      notes: ['Administrator permission is required before firewall add/delete can be proved.'],
    };
  }

  cleanupEvidence.push(commandSummary('cleanup-before', deleteRule(), false));
  const applyProbe = runProbe('netsh', [
    'advfirewall',
    'firewall',
    'add',
    'rule',
    `name=${ruleName}`,
    'dir=out',
    'action=block',
    `remoteip=${targetRemoteAddress}`,
    'enable=yes',
    'profile=any',
  ]);
  const apply = commandSummary('apply-rule', applyProbe, applyProbe.status === 0);
  commandEvidence.push(apply);
  const verifyPresentProbe = showRule();
  const verifyPresent = commandSummary(
    'verify-rule-present',
    verifyPresentProbe,
    verifyPresentProbe.status === 0 && rulePresent(verifyPresentProbe.stdout)
  );
  commandEvidence.push(verifyPresent);
  const rollbackProbe = deleteRule();
  const rollback = commandSummary('rollback-rule', rollbackProbe, false);
  commandEvidence.push(rollback);
  const verifyRemovedProbe = showRule();
  const verifyRemoved = commandSummary(
    'verify-rule-removed',
    verifyRemovedProbe,
    verifyRemovedProbe.status === 0 && rulePresent(verifyRemovedProbe.stdout)
  );
  commandEvidence.push(verifyRemoved);
  cleanupEvidence.push(commandSummary('cleanup-after', deleteRule(), false));

  const executedAndRolledBack =
    apply.exitStatus === 0 &&
    verifyPresent.exitStatus === 0 &&
    verifyPresent.rulePresentAfterCommand &&
    rollback.exitStatus === 0 &&
    !verifyRemoved.rulePresentAfterCommand;

  return {
    state: executedAndRolledBack ? 'executed_and_rolled_back' : 'manual_required',
    executeLab,
    windowsHostObserved,
    administratorPermissionObserved,
    ruleName,
    targetRemoteAddress,
    readOnlyState: summarizeOptionalProbe(readOnlyState),
    adminProbe: summarizeProbe(adminProbe),
    commandEvidence,
    cleanupEvidence,
    notes: executedAndRolledBack
      ? ['Lab rule was added, observed, deleted, and verified absent.']
      : ['Lab command sequence did not satisfy the full add/verify/delete/verify-removed proof.'],
  };
}

function showRule() {
  return runProbe('netsh', ['advfirewall', 'firewall', 'show', 'rule', `name=${ruleName}`]);
}

function deleteRule() {
  return runProbe('netsh', ['advfirewall', 'firewall', 'delete', 'rule', `name=${ruleName}`]);
}

function commandSummary(kind, probe, rulePresentAfterCommand) {
  return {
    kind,
    commandRef: `windows-firewall-lab-${kind}`,
    exitStatus: probe.status,
    stdoutSha256: hashText(probe.stdout),
    stderrSha256: hashText(probe.stderr),
    stdoutLineCount: lineCount(probe.stdout),
    stderrLineCount: lineCount(probe.stderr),
    rulePresentAfterCommand,
  };
}

function runCommand(entry) {
  const result = spawnSync(entry.command, entry.args, { encoding: 'utf8', shell: false });
  writeFileSync(entry.log, `${result.stdout ?? ''}${result.stderr ?? ''}`);
  if (result.status !== 0) {
    throw new Error(`${entry.name} failed with exit ${result.status}`);
  }
  return {
    name: entry.name,
    command: [entry.command, ...entry.args].join(' '),
    status: result.status,
    log: entry.log,
  };
}

function runProbe(command, args) {
  const result = spawnSync(command, args, { encoding: 'utf8', shell: false });
  return {
    command: [command, ...args].join(' '),
    status: result.status ?? 1,
    stdout: result.stdout ?? '',
    stderr: result.stderr ?? '',
  };
}

function summarizeProbe(probe) {
  return {
    command: probe.command,
    status: probe.status,
    stdoutSha256: hashText(probe.stdout),
    stderrSha256: hashText(probe.stderr),
    stdoutLineCount: lineCount(probe.stdout),
    stderrLineCount: lineCount(probe.stderr),
  };
}

function summarizeOptionalProbe(probe) {
  return probe ? summarizeProbe(probe) : null;
}

function runText(command, args) {
  const result = spawnSync(command, args, { encoding: 'utf8', shell: false });
  if (result.status !== 0) {
    throw new Error(`${command} ${args.join(' ')} failed with exit ${result.status}`);
  }
  return `${result.stdout ?? ''}${result.stderr ?? ''}`;
}

function validationLog(results) {
  return results.map((result) => `${result.name}: ${result.command}: exit ${result.status}`).join('\n') + '\n';
}

function hashText(value) {
  return createHash('sha256').update(value).digest('hex');
}

function lineCount(value) {
  const trimmed = value.trim();
  return trimmed.length === 0 ? 0 : trimmed.split(/\r?\n/u).length;
}

function rulePresent(stdout) {
  return stdout.toLowerCase().includes(ruleName.toLowerCase());
}
