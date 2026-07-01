import { spawnSync } from 'node:child_process';
import { mkdirSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const proofRoot = join('output', 'network-plan-proof', '39-windows-wfp-proof-gate');
const testRoot = join('test-results', 'network-windows-wfp-proof-gate');
mkdirSync(proofRoot, { recursive: true });
mkdirSync(testRoot, { recursive: true });

writeFileSync(
  join(proofRoot, 'expected-windows-wfp-proof-gate.json'),
  `${JSON.stringify(
    {
      requiredRefs: [
        'policy-decision-ref',
        'parent-rule-ref',
        'evidence-refs',
        'target-ref',
        'wfp-provider-ref',
        'wfp-layer-ref',
        'administrator-permission-proof-ref',
        'driver-signing-proof-ref',
        'driver-package-proof-ref',
        'provider-registration-plan-ref',
        'layer-capability-matrix-ref',
        'rollback-plan-ref',
        'lab-result-artifact-ref',
        'audit-event-ref',
      ],
      labProofReadyState:
        'grade A block policy plus lab-ready WFP capability and signed permissioned authority artifacts',
      researchOnlyState: 'non-executable and allowed without authority artifacts',
      manualRequiredState: 'weak evidence, non-block policy, manual capability, or missing authority artifacts',
      unavailableState: 'non-executable WFP capability-unavailable state',
      unsupportedClaimsRejected: [
        'exact URL',
        'decrypted payload',
        'page content',
        'live driver install',
        'callout registration',
        'packet block',
        'kernel payload inspection',
        'command invocation',
      ],
      adapterApplyAuthorized: false,
      enforcementCommandPublished: false,
    },
    null,
    2
  )}\n`
);

const commands = [
  {
    name: 'network-windows-wfp-gate-tests',
    command: 'cargo',
    args: ['test', '-p', 'ocentra-network-evidence', 'windows_wfp_gate'],
    log: join(proofRoot, 'windows-wfp-gate-tests.log'),
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
];
const commandResults = commands.map(runCommand);

const proof = {
  proof: 'network-windows-wfp-proof-gate',
  checkedAt: new Date().toISOString(),
  branch: runText('git', ['branch', '--show-current']).trim(),
  commit: runText('git', ['rev-parse', 'HEAD']).trim(),
  statusShort: runText('git', ['status', '--short']),
  proofRoot,
  testRoot,
  commands: commandResults,
  artifacts: {
    expectedWindowsWfpProofGate: join(proofRoot, 'expected-windows-wfp-proof-gate.json'),
    proofSummary: join(proofRoot, 'proof-summary.json'),
    testProof: join(testRoot, 'proof.json'),
  },
  provenRows: ['39 Windows WFP research/proof gate'],
  notClaimed: [
    'live WFP driver installation',
    'WFP callout registration',
    'packet blocking',
    'kernel payload inspection',
    'OS permission elevation',
    'adapter action authorization',
    'enforcement command publication',
    'decrypted payload or page content inspection',
    'exact URL claim from network-only evidence',
  ],
};
writeFileSync(join(proofRoot, 'proof-summary.json'), `${JSON.stringify(proof, null, 2)}\n`);
writeFileSync(join(testRoot, 'proof.json'), `${JSON.stringify(proof, null, 2)}\n`);
console.log('network-windows-wfp-proof-gate-ok:wfp-gate-tests,clippy,source-shape');
console.log(`proof=${join(proofRoot, 'proof-summary.json')}`);

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

function runText(command, args) {
  const result = spawnSync(command, args, { encoding: 'utf8', shell: false });
  if (result.status !== 0) {
    throw new Error(`${command} ${args.join(' ')} failed with exit ${result.status}`);
  }
  return `${result.stdout ?? ''}${result.stderr ?? ''}`;
}
