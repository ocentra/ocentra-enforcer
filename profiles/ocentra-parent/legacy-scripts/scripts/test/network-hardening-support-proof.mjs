import { spawnSync } from 'node:child_process';
import { mkdirSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const proofRoot = join('output', 'network-plan-proof', '11a-hardening-support-proof');
const testRoot = join('test-results', 'network-hardening-support-proof');
const sourceBranch = runText('git', ['branch', '--show-current']).trim();
const sourceCommit = runText('git', ['rev-parse', 'HEAD']).trim();
const sourceStatusShort = runText('git', ['status', '--short']);

mkdirSync(proofRoot, { recursive: true });
mkdirSync(testRoot, { recursive: true });

const readinessMatrix = [
  readinessRecord({
    gate: 'Key rotation and secret handling',
    refs: ['key rotation ref', 'secret handling ref'],
    proofState:
      'required before production readiness; internal proof can name refs without enabling production rollout',
    ownerBoundary: 'network readiness proof only, not credential storage or secret rotation implementation',
  }),
  readinessRecord({
    gate: 'Rule and model provenance with rollback',
    refs: ['rule-set provenance ref', 'rule-set rollback ref', 'AI model promotion ref', 'AI model rollback ref'],
    proofState: 'required for production readiness and rollback auditability',
    ownerBoundary: 'network readiness proof only, not model execution, model hosting, or rules-engine deployment',
  }),
  readinessRecord({
    gate: 'External audit or penetration-test signoff',
    refs: ['external audit or penetration-test ref when production rollout is claimed'],
    proofState: 'production rollout remains blocked without external signoff',
    ownerBoundary: 'records signoff requirement only; does not claim external audit completion',
  }),
  readinessRecord({
    gate: 'Parent/user guide, FAQ, support playbook, and staff training',
    refs: ['parent guide ref', 'user guide ref', 'FAQ ref', 'support playbook ref', 'staff training ref'],
    proofState: 'support refs are required before release/support claims',
    ownerBoundary: 'network readiness proof only, not the E-C production-support feature surface',
  }),
  readinessRecord({
    gate: 'Deployment rollback, staged rollout, monitoring, incident response, and known gap signoff',
    refs: [
      'deployment runbook ref',
      'rollback runbook ref',
      'staged rollout plan ref',
      'monitoring ref',
      'incident response ref',
      'known gap signoff ref',
    ],
    proofState: 'required before production readiness can be claimed',
    ownerBoundary: 'network readiness proof only, not production deployment execution',
  }),
];

writeFileSync(
  join(proofRoot, 'hardening-support-readiness-matrix.json'),
  `${JSON.stringify(readinessMatrix, null, 2)}\n`
);

const commands = [
  {
    name: 'network-readiness-hardening-support-tests',
    command: 'cargo',
    args: ['test', '-p', 'ocentra-network-evidence', 'readiness_proof'],
    log: join(proofRoot, 'readiness-proof-tests.log'),
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
  proof: 'network-hardening-support-proof',
  checkedAt: new Date().toISOString(),
  branch: sourceBranch,
  commit: sourceCommit,
  statusShort: sourceStatusShort,
  proofRoot,
  testRoot,
  commands: commandResults,
  artifacts: {
    readinessMatrix: join(proofRoot, 'hardening-support-readiness-matrix.json'),
    hardeningSupportProof: join(proofRoot, '11a-hardening-support-proof.md'),
    proofSummary: join(proofRoot, 'proof-summary.json'),
    testProof: join(testRoot, 'proof.json'),
  },
  coveredRows: [
    '11a hardening/support proof pack',
    '50 security, privacy, compliance, deployment, support, and staged rollout proof',
  ],
  readinessMatrix,
  provenBoundaries: [
    'key rotation and secret handling refs are required readiness evidence',
    'rule/model provenance and rollback refs are required readiness evidence',
    'parent/user guide, FAQ, support playbook, and staff-training refs are required before release/support claims',
    'production rollout remains blocked without external audit or penetration-test signoff',
    'unsupported upload/content/authority/enforcement claims are rejected by the readiness tests',
  ],
  notClaimed: [
    'production deployment or rollout execution',
    'external audit or penetration-test completion unless a signoff ref is supplied',
    'full support-material authoring outside referenced proof refs',
    'default remote upload of child network evidence',
    'raw PCAP without custody',
    'exact URL, page content, private message, search query, or decrypted payload availability',
    'policy authority, adapter authority, or enforcement command publication',
  ],
};

writeFileSync(join(proofRoot, 'proof-summary.json'), `${JSON.stringify(proof, null, 2)}\n`);
writeFileSync(join(testRoot, 'proof.json'), `${JSON.stringify(proof, null, 2)}\n`);
writeFileSync(join(proofRoot, '11a-hardening-support-proof.md'), `${renderMarkdownProof(proof)}\n`);

console.log('network-hardening-support-proof-ok:readiness-tests,clippy,source-shape');
console.log(`proof=${join(proofRoot, 'proof-summary.json')}`);

function readinessRecord({ gate, refs, proofState, ownerBoundary }) {
  return { gate, requiredRefs: refs, proofState, ownerBoundary };
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

function renderMarkdownProof(proof) {
  const sections = proof.readinessMatrix.flatMap((record) => [
    `## ${record.gate}`,
    `Required refs: ${record.requiredRefs.join(', ')}`,
    `Proof state: ${record.proofState}`,
    `Ownership boundary: ${record.ownerBoundary}`,
    '',
  ]);
  return [
    '# Network Hardening Support Proof',
    '',
    `Branch: ${proof.branch}`,
    `Source commit: ${proof.commit}`,
    `Source status: ${proof.statusShort.length === 0 ? 'clean' : proof.statusShort}`,
    '',
    'This proof aggregates the existing network readiness contract into the required network-plan 11a hardening/support proof pack.',
    'It records the hardening, support, rollout, and external-signoff evidence that must exist before production or release/support claims are upgraded.',
    '',
    ...sections,
    '## Not Claimed',
    ...proof.notClaimed.map((claim) => `- ${claim}`),
  ].join('\n');
}

function runText(command, args) {
  const result = spawnSync(command, args, { encoding: 'utf8', shell: false });
  if (result.status !== 0) {
    throw new Error(`${command} ${args.join(' ')} failed with exit ${result.status}`);
  }
  return `${result.stdout ?? ''}${result.stderr ?? ''}`;
}
