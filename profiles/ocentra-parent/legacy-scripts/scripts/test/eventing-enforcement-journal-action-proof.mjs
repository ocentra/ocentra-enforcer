import { spawn } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';

const repoRoot = process.cwd();
const outputDir = join(repoRoot, 'test-results', 'eventing-enforcement-journal-action-proof');
const proofPath = join(outputDir, 'proof.json');
const rowOutputDir = join(repoRoot, 'output', 'eventing-plan-proof', '55-56-enforcement-journal-action');
const rowProofPath = join(rowOutputDir, 'proof-summary.json');
const commands = [];
const proofLabels = [];

await main();

async function main() {
  await mkdir(outputDir, { recursive: true });
  await mkdir(rowOutputDir, { recursive: true });

  await runCommand('cargo', ['test', '-p', 'ocentra-parent-agent-service', 'enforcement_execute']);
  await runCommand('cargo', [
    'test',
    '-p',
    'ocentra-parent-agent-service',
    'enforcement_supported_adapter_runtime_proof',
  ]);
  for (const sourceShapeTarget of [
    'crates/agent-service/src/enforcement_api.rs',
    'crates/agent-service/src/enforcement_api/enforcement_pre_action_journal.rs',
    'crates/agent-service/tests/unit/enforcement_tests.rs',
    'crates/agent-protocol/src/constants/enforcement.rs',
  ]) {
    await runCommand('node', ['scripts/check-source-shape.mjs', sourceShapeTarget]);
  }
  await assertSourceContracts();

  const proof = {
    schemaVersion: 1,
    proofMode: 'eventing-enforcement-journal-action-proof',
    checkedAt: new Date().toISOString(),
    commit: await gitHead(),
    commands,
    proofLabels,
    linkedArtifacts: {
      serviceRuntime: 'crates/agent-service/src/enforcement_api.rs',
      servicePreActionJournal: 'crates/agent-service/src/enforcement_api/enforcement_pre_action_journal.rs',
      serviceTests: 'crates/agent-service/tests/unit/enforcement_tests.rs',
      enforcementConstants: 'crates/agent-protocol/src/constants/enforcement.rs',
      proofHarness: 'scripts/test/eventing-enforcement-journal-action-proof.mjs',
      rowProof: 'output/eventing-plan-proof/55-56-enforcement-journal-action/proof-summary.json',
      eventingChecklist: 'docs/plans/eventing-plan/implementation-checklist.md',
    },
    rowsCovered: ['55 Journal-before-action enforcement proof', '56 Adapter result to audit/read-model proof'],
    claimsProved: [
      'agent-service records a pre-action enforcement audit activity event immediately after typed authorization and before adapter execution',
      'encrypted activity journal line metadata preserves pre-action event id before the final enforcement audit event id',
      'final enforcement audit reports the adapter result after the pre-action journal row and keeps the final audit event as the latest store event',
      'supported-adapter runtime proof read model remains service-backed and includes adapter/audit source refs',
    ],
    claimsNotProved: [
      'parent/controller event namespace constants and contracts for rows 42-44',
      'parent/controller child-command transport handoff for row 53',
      'child-agent command receive and local event publish for row 54',
      'real network/domain blocking adapter execution',
      'agent-service WebSocket export rewiring outside existing enforcement report commands',
    ],
  };

  await writeFile(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
  await writeFile(rowProofPath, `${JSON.stringify(rowProof(proof), null, 2)}\n`);
  console.log(`eventing-enforcement-journal-action-proof-ok:${proofLabels.join(',')}`);
  console.log(`evidence=${relative(repoRoot, proofPath)}`);
  console.log(`rowProof=${relative(repoRoot, rowProofPath)}`);
}

async function assertSourceContracts() {
  const service = await readText('crates/agent-service/src/enforcement_api.rs');
  const preActionJournal = await readText('crates/agent-service/src/enforcement_api/enforcement_pre_action_journal.rs');
  const tests = await readText('crates/agent-service/tests/unit/enforcement_tests.rs');
  const constants = await readText('crates/agent-protocol/src/constants/enforcement.rs');

  assertIncludes(constants, 'JOURNAL_BEFORE_ACTION_ID_PREFIX', 'before-action id prefix constant exists');
  assertIncludes(
    service,
    'record_enforcement_audit(&request, &before_action_outcome, &paths).await?',
    'before-action audit is recorded before adapter execution'
  );
  assertIncludes(
    service,
    'let adapter_outcome = match authorization.adapter_request.as_ref()',
    'adapter execution happens after the before-action journal write'
  );
  assertIncludes(preActionJournal, 'journal_before_action_outcome', 'before-action outcome helper exists');
  assertIncludes(preActionJournal, 'EnforcementResultStatus::WouldEnforce', 'pre-action result is would-enforce');
  assertIncludes(preActionJournal, 'EnforcementAdapterResultCode::NoOp', 'pre-action result has no adapter result');
  assertIncludes(
    tests,
    'enforcement_execute_reports_final_adapter_result_after_before_action_journal',
    'final adapter result test exists'
  );
  assertIncludes(tests, 'journal_event_ids', 'journal line order assertion helper exists');

  proofLabels.push('eventing.row-55.journal-before-action');
  proofLabels.push('eventing.row-56.adapter-result-audit-read-model');
  proofLabels.push('enforcement.service.pre-action-journal-line-order');
  proofLabels.push('enforcement.service.final-adapter-result-after-journal');
}

async function runCommand(command, args) {
  commands.push([command, ...args].join(' '));
  await new Promise((resolve, reject) => {
    const child = spawn(command, args, { cwd: repoRoot, stdio: 'inherit', windowsHide: true });
    child.once('exit', (code) =>
      code === 0 ? resolve() : reject(new Error(`${command} ${args.join(' ')} exited with ${code}`))
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

async function readText(path) {
  return readFile(join(repoRoot, path), 'utf8');
}

function assertIncludes(text, expected, label) {
  if (!text.includes(expected)) {
    throw new Error(`${label}: missing ${expected}`);
  }
}

function rowProof(proof) {
  return {
    proof: 'eventing-rows-55-56-enforcement-journal-action',
    checkedAt: proof.checkedAt,
    commit: proof.commit,
    commands: proof.commands,
    proofLabels: proof.proofLabels,
    linkedArtifacts: {
      proof: relative(repoRoot, proofPath),
      rowProof: relative(repoRoot, rowProofPath),
      serviceRuntime: proof.linkedArtifacts.serviceRuntime,
      servicePreActionJournal: proof.linkedArtifacts.servicePreActionJournal,
      serviceTests: proof.linkedArtifacts.serviceTests,
      enforcementConstants: proof.linkedArtifacts.enforcementConstants,
      eventingChecklist: proof.linkedArtifacts.eventingChecklist,
    },
    rowsCovered: proof.rowsCovered,
    claimsProved: proof.claimsProved,
    claimsNotProved: proof.claimsNotProved,
  };
}
