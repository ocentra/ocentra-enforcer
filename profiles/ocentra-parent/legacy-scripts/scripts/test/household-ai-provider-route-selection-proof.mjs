import { spawn } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';

const repoRoot = process.cwd();
const outputDir = join(repoRoot, 'test-results', 'household-ai-provider-route-selection-proof');
const proofPath = join(outputDir, 'proof.json');
const routeOutputDir = join(repoRoot, 'output', 'ai-plan-proof', 'household-ai-provider-route-selection-proof');
const routeProofPath = join(routeOutputDir, 'proof-summary.json');
const mobileOutputDir = join(repoRoot, 'output', 'ai-plan-proof', 'mobile-dormant-ai-provider-proof');
const mobileProofPath = join(mobileOutputDir, 'proof-summary.json');
const commands = [];

await main();

async function main() {
  await mkdir(outputDir, { recursive: true });
  await mkdir(routeOutputDir, { recursive: true });
  await mkdir(mobileOutputDir, { recursive: true });

  await runCommand('cargo', ['test', '-p', 'ocentra-parent-agent-core', 'household_ai_provider_route']);
  await runCommand('node', ['scripts/check-source-shape.mjs']);
  await assertSourceContracts();

  const proof = {
    schemaVersion: 1,
    proofMode: 'household-ai-provider-route-selection-proof',
    checkedAt: new Date().toISOString(),
    commit: await gitHead(),
    commands,
    evidence: {
      constants: 'crates/agent-protocol/src/constants/household_mesh.rs',
      routeRuntime: 'crates/agent-core/src/household_ai_provider_route.rs',
      routeState: 'crates/agent-core/src/household_ai_provider_route_state.rs',
      routeTests: 'crates/agent-core/tests/unit/household_ai_provider_route_tests.rs',
      proofHarness: 'scripts/test/household-ai-provider-route-selection-proof.mjs',
      routeProofSummary: 'output/ai-plan-proof/household-ai-provider-route-selection-proof/proof-summary.json',
      mobileProofSummary: 'output/ai-plan-proof/mobile-dormant-ai-provider-proof/proof-summary.json',
    },
    routePriority: ['desktop-preferred', 'laptop-preferred', 'child-desktop-local', 'mobile-dormant'],
    rejectionCases: [
      'stale-provider',
      'offline-provider',
      'revoked-provider',
      'custody-mismatch',
      'unsupported-capability',
      'resource-degraded',
      'mobile-dormant-desktop-available',
      'mobile-fallback-denied',
      'no-provider',
    ],
    claimsProved: [
      'trusted parent desktop providers are selected before laptop, child-desktop, or mobile providers',
      'stale, offline, revoked, degraded, unsupported, and custody-mismatched providers are rejected before selection',
      'mobile providers stay dormant while trusted desktop or laptop providers are available',
      'mobile providers are eligible only for explicit light fallback jobs when battery, thermal, and fallback policy allow it',
    ],
    claimsNotProved: [
      'physical household LAN provider advertisement',
      'real cross-device provider route selection',
      'lease expiry requeue over LAN',
      'production model execution or model quality',
      'portal household mesh surface rendering',
      'policy authority or enforcement dispatch',
    ],
  };

  await writeFile(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
  await writeFile(routeProofPath, `${JSON.stringify(routeProof(proof), null, 2)}\n`);
  await writeFile(mobileProofPath, `${JSON.stringify(mobileProof(proof), null, 2)}\n`);
  console.log('household-ai-provider-route-selection-proof-ok');
  console.log(`evidence=${relative(repoRoot, proofPath)}`);
  console.log(`route=${relative(repoRoot, routeProofPath)}`);
  console.log(`mobile=${relative(repoRoot, mobileProofPath)}`);
}

async function assertSourceContracts() {
  const constantsSource = await readText('crates/agent-protocol/src/constants/household_mesh.rs');
  const coreLib = await readText('crates/agent-core/src/lib.rs');
  const routeSource = await readText('crates/agent-core/src/household_ai_provider_route.rs');
  const routeStateSource = await readText('crates/agent-core/src/household_ai_provider_route_state.rs');
  const routeTestsSource = await readText('crates/agent-core/tests/unit/household_ai_provider_route_tests.rs');
  const aiChecklist = await readText('docs/plans/ai-plan/implementation-checklist.md');
  const aiFeature = await readText('docs/features/local-ai-safety-evaluator.md');

  assertIncludes(constantsSource, 'ROUTE_REASON_MOBILE_DORMANT_DESKTOP_AVAILABLE', 'mobile dormant reason exists');
  assertIncludes(constantsSource, 'ROUTE_REASON_MOBILE_FALLBACK_ALLOWED', 'mobile fallback reason exists');
  assertIncludes(coreLib, 'select_household_ai_provider_route', 'agent-core exports route selector');
  assertIncludes(routeSource, 'select_household_ai_provider_route', 'route selector exists');
  assertIncludes(routeSource, 'desktop_or_laptop_available', 'desktop/laptop availability gate exists');
  assertIncludes(routeStateSource, 'MobileDormant', 'mobile dormant provider class exists');
  assertIncludes(routeTestsSource, 'keeps_mobile_dormant_when_desktop_is_available', 'tests keep mobile dormant');
  assertIncludes(routeTestsSource, 'allows_mobile_only_for_explicit_light_fallback', 'tests mobile explicit fallback');
  assertIncludes(
    routeTestsSource,
    'rejects_stale_offline_revoked_and_custody_mismatch',
    'tests trust and custody rejection'
  );
  assertIncludes(
    aiChecklist,
    'Household provider route selection implemented and proved',
    'AI checklist marks route proof'
  );
  assertIncludes(aiChecklist, 'Mobile dormant AI provider proof run', 'AI checklist marks mobile dormant proof run');
  assertIncludes(aiFeature, 'household-ai-provider-route-selection-proof.mjs', 'feature doc names route proof');
  assertDoesNotInclude(routeSource, 'HouseholdAiProviderClass::MobileDormant => 0', 'mobile cannot outrank desktop');
}

function routeProof(proof) {
  return {
    schemaVersion: proof.schemaVersion,
    proofMode: 'household-ai-provider-route-selection-proof',
    checkedAt: proof.checkedAt,
    commit: proof.commit,
    commands: proof.commands,
    routePriority: proof.routePriority,
    rejectionCases: proof.rejectionCases,
    claimsProved: proof.claimsProved,
    claimsNotProved: proof.claimsNotProved,
    sourceProof: relative(repoRoot, proofPath),
  };
}

function mobileProof(proof) {
  return {
    schemaVersion: proof.schemaVersion,
    proofMode: 'mobile-dormant-ai-provider-proof',
    checkedAt: proof.checkedAt,
    commit: proof.commit,
    commands: proof.commands,
    mobileClaimsProved: proof.claimsProved.filter(
      (claim) => claim.includes('mobile') || claim.includes('battery') || claim.includes('thermal')
    ),
    claimsNotProved: proof.claimsNotProved,
    sourceProof: relative(repoRoot, proofPath),
  };
}

async function readText(path) {
  return readFile(join(repoRoot, path), 'utf8');
}

function assertIncludes(text, value, label) {
  if (!text.includes(value)) {
    throw new Error(`${label}: missing ${value}`);
  }
}

function assertDoesNotInclude(text, value, label) {
  if (text.includes(value)) {
    throw new Error(`${label}: found ${value}`);
  }
}

async function runCommand(command, args) {
  const result = await new Promise((resolve) => {
    const child = spawn(command, args, { cwd: repoRoot, shell: process.platform === 'win32' });
    let stdout = '';
    let stderr = '';
    child.stdout.on('data', (chunk) => {
      stdout += chunk;
      process.stdout.write(chunk);
    });
    child.stderr.on('data', (chunk) => {
      stderr += chunk;
      process.stderr.write(chunk);
    });
    child.on('close', (code) => resolve({ code, stdout, stderr }));
  });
  commands.push({ command, args, exitCode: result.code });
  if (result.code !== 0) {
    throw new Error(`${command} ${args.join(' ')} failed with ${result.code}`);
  }
}

async function gitHead() {
  const result = await new Promise((resolve) => {
    const child = spawn('git', ['rev-parse', 'HEAD'], { cwd: repoRoot, shell: process.platform === 'win32' });
    let stdout = '';
    child.stdout.on('data', (chunk) => {
      stdout += chunk;
    });
    child.on('close', (code) => resolve({ code, stdout }));
  });
  if (result.code !== 0) {
    return null;
  }
  return result.stdout.trim();
}
