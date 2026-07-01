import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const proofRoot = join('output', 'network-plan-proof', '34-evidence-grade-policy-mapping');
const testRoot = join('test-results', 'network-evidence-policy-mapping-proof');
mkdirSync(proofRoot, { recursive: true });
mkdirSync(testRoot, { recursive: true });

const sourceFiles = [
  'scripts/test/network-evidence-policy-mapping-proof.mjs',
  'crates/ocentra-network-evidence/src/lib.rs',
  'crates/ocentra-network-evidence/src/policy.rs',
  'crates/ocentra-network-evidence/src/tests/policy.rs',
  'docs/features/network-domain-control.md',
  'docs/plans/network-plan/implementation-checklist.md',
];

assertSourceContracts();

writeJson(join(proofRoot, 'expected-evidence-grade-policy-mapping.json'), {
  requiredRefs: ['policy-decision-ref', 'parent-rule-ref', 'evidence-refs'],
  gradeMapping: {
    A: 'dry-run requested action, no adapter or enforcement command',
    B: 'dry-run monitor/warn, parent-review for limit/block',
    C: 'parent-review only',
    D: 'observe-only none',
  },
  adapterState: 'never-authorized in this row; adapter proof remains a later row',
  enforcementState: 'never-authorized in this row',
  localAiRefs: 'optional local-AI result refs are refs only and must be non-empty when present',
  noClaims: [
    'adapter execution or adapter authorization',
    'enforcement command publication',
    'policy engine completeness',
    'portal UI rendering',
    'host DNS/firewall filtering',
    'AI model execution',
  ],
});

const commands = [
  {
    name: 'network-evidence-policy-mapping-tests',
    command: 'cargo',
    args: ['test', '-p', 'ocentra-network-evidence', 'policy_mapping'],
    log: join(proofRoot, 'policy-mapping-tests.log'),
  },
  {
    name: 'network-evidence-clippy',
    command: 'cargo',
    args: ['clippy', '-p', 'ocentra-network-evidence', '--all-targets', '--', '-D', 'warnings'],
    log: join(proofRoot, 'clippy.log'),
  },
  {
    name: 'rust-format',
    command: 'cargo',
    args: ['fmt', '--all', '--check'],
    log: join(proofRoot, 'rust-format.log'),
  },
  {
    name: 'source-shape',
    command: 'node',
    args: ['scripts/check-source-shape.mjs'],
    log: join(proofRoot, 'source-shape.log'),
  },
  {
    name: 'git-diff-check',
    command: 'git',
    args: ['diff', '--check'],
    log: join(proofRoot, 'git-diff-check.log'),
  },
];
const commandResults = commands.map(runCommand);

const proof = {
  proof: 'network-evidence-policy-mapping',
  proofRevision: 'network-evidence-policy-mapping-proof/v2',
  checkedAt: 'deterministic:network-evidence-policy-mapping-proof/v2',
  sourceFingerprint: `source-tree:${sourceFingerprint()}`,
  sourceRefs: sourceFiles,
  sourceBase: 'deterministic:network-evidence-policy-mapping-source-set/v2',
  proofRoot: artifactPath(proofRoot),
  testRoot: artifactPath(testRoot),
  commands: commandResults,
  artifacts: {
    expectedEvidenceGradePolicyMapping: artifactPath(join(proofRoot, 'expected-evidence-grade-policy-mapping.json')),
    proofSummary: artifactPath(join(proofRoot, 'proof-summary.json')),
    testProof: artifactPath(join(testRoot, 'proof.json')),
  },
  provenRows: ['34 Evidence-grade policy mapping'],
  provenBoundaries: [
    'grade A requested actions stay dry-run without adapter or enforcement authorization',
    'grade B block and limit requests route to parent review ask-parent',
    'grade C routes to parent review ask-parent',
    'grade D routes to observe-only none',
    'required policy, parent-rule, and evidence refs must be non-empty',
    'optional local-AI and adapter proof refs are refs only and must be non-empty when present',
    'adapter_action_authorized and enforcement_command_authorized remain false for every mapping',
  ],
  notClaimed: [
    'adapter execution or adapter authorization',
    'enforcement command publication',
    'policy engine completeness',
    'portal UI rendering',
    'host DNS/firewall filtering',
    'AI model execution',
  ],
};
writeJson(join(proofRoot, 'proof-summary.json'), proof);
writeJson(join(testRoot, 'proof.json'), proof);
console.log('network-evidence-policy-mapping-proof-ok:policy-tests,clippy,fmt,source-shape,diff-check');
console.log(`proof=${artifactPath(join(proofRoot, 'proof-summary.json'))}`);

function runCommand(entry) {
  const result = spawnSync(entry.command, entry.args, { encoding: 'utf8', shell: false });
  writeFileSync(entry.log, normalizeCommandLog(entry.name, `${result.stdout ?? ''}${result.stderr ?? ''}`));
  if (result.status !== 0) {
    throw new Error(`${entry.name} failed with exit ${result.status}`);
  }
  return {
    name: entry.name,
    command: [entry.command, ...entry.args].join(' '),
    status: result.status,
    log: artifactPath(entry.log),
  };
}

function assertSourceContracts() {
  const networkPolicy = readText('crates/ocentra-network-evidence/src/policy.rs');
  const networkPolicyTests = readText('crates/ocentra-network-evidence/src/tests/policy.rs');
  const networkLib = readText('crates/ocentra-network-evidence/src/lib.rs');
  const featureDoc = readText('docs/features/network-domain-control.md');
  const checklist = readText('docs/plans/network-plan/implementation-checklist.md');
  const requiredSnippets = [
    [networkLib, 'map_network_evidence_grade_to_policy'],
    [networkPolicy, 'NetworkEvidencePolicyMappingInput'],
    [networkPolicy, 'adapter_action_authorized: false'],
    [networkPolicy, 'enforcement_command_authorized: false'],
    [networkPolicy, 'mapped_mode_and_action'],
    [networkPolicyTests, 'policy_mapping_allows_grade_a_dry_run_without_adapter_authority'],
    [networkPolicyTests, 'policy_mapping_routes_grade_b_block_requests_to_parent_review'],
    [networkPolicyTests, 'policy_mapping_keeps_grade_c_and_d_non_enforcing'],
    [networkPolicyTests, 'policy_mapping_rejects_missing_policy_rule_or_evidence_refs'],
    [featureDoc, 'evidence-grade policy mapper'],
    [checklist, '34-evidence-grade-policy-mapping/proof-summary.json'],
  ];
  for (const [haystack, needle] of requiredSnippets) {
    assertIncludes(haystack, needle, `source contract snippet ${needle}`);
  }
}

function sourceFingerprint() {
  const hash = createHash('sha256');
  for (const filePath of sourceFiles) {
    hash.update(filePath);
    hash.update('\0');
    hash.update(readText(filePath));
    hash.update('\0');
  }
  return hash.digest('hex');
}

function readText(path) {
  return readFileSync(path, 'utf8');
}

function writeJson(path, value) {
  writeFileSync(path, `${JSON.stringify(value, null, 2)}\n`);
}

function assertIncludes(text, expected, label) {
  if (!text.includes(expected)) {
    throw new Error(`${label}: missing ${expected}`);
  }
}

function artifactPath(path) {
  return path.replace(/\\/g, '/');
}

function normalizeCommandLog(name, text) {
  if (name === 'source-shape') {
    return normalizeSourceShapeLog(text);
  }
  return normalizeLogText(text);
}

function normalizeSourceShapeLog(text) {
  const normalized = normalizeLogText(text);
  const scopedWarnings = normalized
    .split('\n')
    .filter((line) => sourceFiles.some((filePath) => line.startsWith(filePath)))
    .sort();
  const passedLine = normalized.includes('Source shape guard passed.') ? 'Source shape guard passed.' : '';
  return (
    ['Source shape warnings scoped to row34 source refs:', ...scopedWarnings, passedLine]
      .filter((line) => line.length > 0)
      .join('\n') + '\n'
  );
}

function normalizeLogText(text) {
  const normalizedLines = sortSourceShapeWarningLines(
    sortConsecutiveTestLines(
      normalizeWorkspacePaths(text)
        .replace(/\r\n/g, '\n')
        .split('\n')
        .filter((line) => !line.includes('Blocking waiting for'))
        .filter((line) => !line.trimStart().startsWith('Compiling '))
        .filter((line) => !line.trimStart().startsWith('Checking '))
        .map((line) =>
          line
            .replace(/finished in [0-9.]+s/g, 'finished in <duration>')
            .replace(/target\(s\) in [0-9.]+s/g, 'target(s) in <duration>')
            .replace(/target\(s\) in [0-9]+m [0-9]+s/g, 'target(s) in <duration>')
            .replace(/Duration\s+[0-9.]+(?:ms|s)/g, 'Duration <duration>')
            .replace(/Start at\s+[0-9:]+/g, 'Start at <time>')
            .replace(/duration_ms: [0-9.]+/g, 'duration_ms: <duration>')
            .replace(/\b[0-9.]+(?:ms|s)\b/g, '<duration>')
        )
    )
  );
  const trimmed = normalizedLines
    .join('\n')
    .replace(/[ \t]+$/gm, '')
    .replace(/\s+$/u, '');
  return trimmed.length === 0 ? '' : `${trimmed}\n`;
}

function normalizeWorkspacePaths(text) {
  const workspacePath = process.cwd();
  const workspacePathForward = workspacePath.replace(/\\/g, '/');
  return text
    .replace(new RegExp(escapeRegExp(workspacePath), 'g'), '<workspace>')
    .replace(new RegExp(escapeRegExp(workspacePathForward), 'g'), '<workspace>');
}

function escapeRegExp(value) {
  return value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}

function sortSourceShapeWarningLines(lines) {
  const warningHeaderIndex = lines.findIndex((line) => line.startsWith('Source shape warnings:'));
  if (warningHeaderIndex === -1) {
    return lines;
  }
  return [
    ...lines.slice(0, warningHeaderIndex + 1),
    ...lines
      .slice(warningHeaderIndex + 1)
      .filter((line) => line.trim().length > 0)
      .sort(),
  ];
}

function sortConsecutiveTestLines(lines) {
  const sortedLines = [];
  let testLineBuffer = [];
  for (const line of lines) {
    if (line.startsWith('test ') && line.endsWith(' ... ok')) {
      testLineBuffer.push(line);
      continue;
    }
    if (testLineBuffer.length > 0) {
      sortedLines.push(...testLineBuffer.sort());
      testLineBuffer = [];
    }
    sortedLines.push(line);
  }
  if (testLineBuffer.length > 0) {
    sortedLines.push(...testLineBuffer.sort());
  }
  return sortedLines;
}
