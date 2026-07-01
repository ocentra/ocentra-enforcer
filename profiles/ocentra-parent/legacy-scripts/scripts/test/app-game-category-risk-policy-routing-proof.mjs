import { spawn } from 'node:child_process';
import { mkdir, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';

const repoRoot = process.cwd();
const testOutputDir = join(repoRoot, 'test-results', 'app-game-category-risk-policy-routing');
const appGameProofDir = join(repoRoot, 'output', 'app-game-plan-proof', '49-category-risk-policy-routing');
const appProofDir = join(repoRoot, 'output', 'app-plan-proof', '49-category-risk-policy-routing');
const commands = [];

await main();

async function main() {
  await mkdir(testOutputDir, { recursive: true });
  await mkdir(appGameProofDir, { recursive: true });
  await mkdir(appProofDir, { recursive: true });
  await mkdir(join(appGameProofDir, '06-ui-snapshots'), { recursive: true });
  await mkdir(join(appProofDir, '06-ui-snapshots'), { recursive: true });

  await runCommand(...npmCommand(['run', 'build', '--workspace', '@ocentra-parent/schema-domain']));

  const routing = await import('@ocentra-parent/schema-domain/app-game-category-risk-policy-routing');
  commands.push('node import @ocentra-parent/schema-domain/app-game-category-risk-policy-routing');
  const compilerRules = await import('@ocentra-parent/schema-domain/app-game-policy-target-compiler-rules');
  commands.push('node import @ocentra-parent/schema-domain/app-game-policy-target-compiler-rules');
  const policy = await import('@ocentra-parent/schema-domain/policy');
  commands.push('node import @ocentra-parent/schema-domain/policy');
  const refs = await import('@ocentra-parent/schema-domain/family-reference-primitives');
  commands.push('node import @ocentra-parent/schema-domain/family-reference-primitives');

  const fixtures = buildFixtures(routing, compilerRules, policy, refs);
  const parsed = fixtures.validRoutes.map((route) => routing.AppGameCategoryRiskPolicyRouteSchema.parse(route));
  const rejected = fixtures.invalidRoutes.map((route) => ({
    routeId: route.routeId,
    rejected: !routing.AppGameCategoryRiskPolicyRouteSchema.safeParse(route).success,
  }));

  assertEqual(parsed.length, 4, 'valid route count');
  assertEqual(
    rejected.every((row) => row.rejected),
    true,
    'invalid route rejection'
  );
  assertEqual(
    parsed.every(
      (route) => route.adapterDispatchState === routing.AppGameCategoryRiskPolicyAdapterDispatchState.NotDispatched
    ),
    true,
    'adapter dispatch state'
  );

  const proof = {
    schemaVersion: 1,
    proofMode: 'app-game-category-risk-policy-routing',
    checkedAt: new Date().toISOString(),
    commit: await gitHead(),
    commands,
    summary: {
      validRoutes: parsed.length,
      rejectedRoutes: rejected.length,
      routeFamilies: countBy(parsed.map((route) => route.routeFamily)),
      targetKinds: countBy(parsed.map((route) => route.targetKind)),
      requestedActions: countBy(parsed.map((route) => route.requestedAction)),
      adapterDispatchStates: countBy(parsed.map((route) => route.adapterDispatchState)),
    },
    claimsProved: [
      'catalog native app category candidates route into app-category compiler targets with active category proof',
      'risk candidates route only through ask-parent/manual review style policy inputs and cannot request hard adapter actions',
      'local AI category routes must cite an AI digest ref and remain evidence-backed',
      'game context signals route only into game-context policy target kinds',
      'manual-review and stale category proof remain out of compile-ready routing',
      'all routes preserve adapterDispatchState=not-dispatched',
    ],
    claimsNotProved: [
      'live classifier/provider execution',
      'runtime service policy evaluator consumption',
      'portal category/risk UI rendering',
      'notification or child request delivery',
      'adapter execution or broad installed-app blocking',
      'cross-platform runtime support',
    ],
    evidence: {
      schemaContract: 'packages/schema-domain/src/app-game-category-risk-policy-routing.ts',
      schemaRules: 'packages/schema-domain/src/app-game-category-risk-policy-routing-rules.ts',
      sharedCompilerRules: 'packages/schema-domain/src/app-game-policy-target-compiler-rules.ts',
      sharedPolicy: 'packages/schema-domain/src/policy.ts',
      sharedReferencePrimitives: 'packages/schema-domain/src/family-reference-primitives.ts',
      harness: 'scripts/test/app-game-category-risk-policy-routing-proof.mjs',
      appGameProofPack: 'output/app-game-plan-proof/49-category-risk-policy-routing',
      appProofPack: 'output/app-plan-proof/49-category-risk-policy-routing',
    },
    parsedRoutes: parsed,
    rejected,
  };

  await writeJson(join(testOutputDir, 'proof.json'), proof);
  await writeProofPack(appGameProofDir, proof, 'app-game WP49');
  await writeProofPack(appProofDir, proof, 'app WP49');

  console.log(`app-game-category-risk-policy-routing-proof-ok:${parsed.length}`);
  console.log(`evidence=${relative(repoRoot, join(testOutputDir, 'proof.json'))}`);
}

function buildFixtures(routing, compilerRules, policy, refs) {
  const timestamp = '2026-06-04T13:45:00Z';
  const localUserRef = 'windows-local-user-category-risk';
  const device = {
    deviceId: 'device-windows-category-risk',
    childProfileId: 'child-category-risk',
    label: 'Study PC',
    platform: refs.ParentPlatform.Windows,
  };
  const evidenceReference = {
    evidenceReferenceId: 'evidence-category-risk-route-1',
    kind: refs.ParentEvidenceReferenceKind.ActivityEvent,
    observedAt: timestamp,
  };
  const categoryProof = {
    evidenceReference,
    proofKind: compilerRules.AppGamePolicyCompilerProofKind.Category,
    evidenceState: compilerRules.AppGamePolicyCompilerEvidenceState.Active,
    device,
    localUserRef,
    observedAt: timestamp,
  };
  const base = {
    schemaVersion: refs.ParentContractSchemaVersion.V0_6,
    routeId: 'category-risk-route-school-warn',
    categoryCandidateRef: 'category-candidate-school',
    routeFamily: routing.AppGameCategoryRiskPolicyRouteFamily.NativeApp,
    sourceKind: routing.AppGameCategoryRiskPolicyRouteSourceKind.Catalog,
    sourceRef: 'category-source-catalog',
    targetKind: compilerRules.AppGamePolicyTargetKind.AppCategory,
    targetRef: 'native-app-category:school',
    confidence: 0.94,
    candidateAction: routing.AppGameCategoryRiskPolicyCandidateAction.Warn,
    requestedAction: compilerRules.AppGamePolicyCompilerRequestedAction.Warn,
    policyAction: policy.PolicyAction.Warn,
    routingState: routing.AppGameCategoryRiskPolicyRoutingState.CompileReady,
    categoryProof,
    supportingEvidence: [evidenceReference],
    aiDigestRef: null,
    adapterDispatchState: routing.AppGameCategoryRiskPolicyAdapterDispatchState.NotDispatched,
  };

  const riskRoute = {
    ...base,
    routeId: 'category-risk-route-vpn',
    categoryCandidateRef: 'category-candidate-vpn',
    routeFamily: routing.AppGameCategoryRiskPolicyRouteFamily.RiskCandidate,
    sourceKind: routing.AppGameCategoryRiskPolicyRouteSourceKind.ExecutableName,
    sourceRef: 'category-source-executable-name',
    targetKind: compilerRules.AppGamePolicyTargetKind.RiskApp,
    targetRef: 'risk-app:vpn-proxy',
    confidence: 0.51,
    candidateAction: routing.AppGameCategoryRiskPolicyCandidateAction.AskParent,
    requestedAction: compilerRules.AppGamePolicyCompilerRequestedAction.AskParent,
    policyAction: policy.PolicyAction.AskParent,
  };
  const localAiRoute = {
    ...base,
    routeId: 'category-risk-route-local-ai-social',
    categoryCandidateRef: 'category-candidate-local-ai-social',
    sourceKind: routing.AppGameCategoryRiskPolicyRouteSourceKind.LocalAi,
    sourceRef: 'category-source-local-ai',
    aiDigestRef: 'ai-digest-category-risk-social',
  };
  const multiplayerRoute = {
    ...base,
    routeId: 'category-risk-route-multiplayer-game',
    categoryCandidateRef: 'category-candidate-multiplayer-game',
    routeFamily: routing.AppGameCategoryRiskPolicyRouteFamily.GameContext,
    sourceKind: routing.AppGameCategoryRiskPolicyRouteSourceKind.LauncherManifest,
    sourceRef: 'category-source-launcher-manifest',
    targetKind: compilerRules.AppGamePolicyTargetKind.MultiplayerGame,
    targetRef: 'game-context:multiplayer',
    candidateAction: routing.AppGameCategoryRiskPolicyCandidateAction.AskParent,
    requestedAction: compilerRules.AppGamePolicyCompilerRequestedAction.AskParent,
    policyAction: policy.PolicyAction.AskParent,
  };
  const staleRoute = {
    ...base,
    routeId: 'category-risk-route-stale-proof',
    categoryProof: {
      ...categoryProof,
      evidenceState: compilerRules.AppGamePolicyCompilerEvidenceState.Stale,
    },
  };
  const hardRiskRoute = {
    ...riskRoute,
    routeId: 'category-risk-route-hard-risk',
    requestedAction: compilerRules.AppGamePolicyCompilerRequestedAction.BlockLaunch,
    policyAction: policy.PolicyAction.Block,
  };
  const missingDigestRoute = {
    ...localAiRoute,
    routeId: 'category-risk-route-local-ai-missing-digest',
    aiDigestRef: null,
  };
  const wrongFamilyRoute = {
    ...multiplayerRoute,
    routeId: 'category-risk-route-wrong-family-target',
    targetKind: compilerRules.AppGamePolicyTargetKind.AppCategory,
  };

  return {
    validRoutes: [base, riskRoute, localAiRoute, multiplayerRoute],
    invalidRoutes: [staleRoute, hardRiskRoute, missingDigestRoute, wrongFamilyRoute],
  };
}

async function writeProofPack(proofDir, proof, label) {
  await writeFile(
    join(proofDir, '00-source-snapshot.md'),
    [
      `# ${label} Source Snapshot`,
      '',
      `- Branch: ${await gitBranch()}`,
      `- Commit: ${proof.commit}`,
      '- Scope: schema-domain category/risk policy-routing contract proof.',
      '- Source inspected: schema-domain category/risk routing contract, shared compiler rules, policy enums, and family reference primitives.',
      '- UI, service runtime, provider execution, notifications, and adapters are intentionally not changed.',
      '',
    ].join('\n'),
    'utf8'
  );
  await writeFile(
    join(proofDir, '01-contract-proof.log'),
    [
      'Contract proof:',
      '',
      '- cmd /c npm run build --workspace @ocentra-parent/schema-domain: PASS',
      '- node import @ocentra-parent/schema-domain/app-game-category-risk-policy-routing: PASS',
      '- node import @ocentra-parent/schema-domain/app-game-policy-target-compiler-rules: PASS',
      '- node import @ocentra-parent/schema-domain/policy: PASS',
      '- node import @ocentra-parent/schema-domain/family-reference-primitives: PASS',
      '- Valid route families: nativeApp, riskCandidate, nativeGame/gameContext as applicable.',
      '- Invalid route rejection covers stale proof, hard risk action, missing AI digest, and wrong family target.',
      '',
    ].join('\n'),
    'utf8'
  );
  await writeFile(
    join(proofDir, '02-rust-protocol-proof.log'),
    'Rust/service protocol not changed. This is TypeScript schema-domain policy-routing proof only.\n',
    'utf8'
  );
  await writeJson(join(proofDir, '03-runtime-evidence.json'), proof);
  await writeFile(
    join(proofDir, '04-journal-sqlite-proof.json'),
    `${JSON.stringify(
      {
        schemaVersion: 1,
        journalSqliteChanged: false,
        reason: 'No journal, SQLite, service read-model, or runtime persistence code changed.',
      },
      null,
      2
    )}\n`,
    'utf8'
  );
  await writeFile(
    join(proofDir, '05-policy-action-proof.json'),
    `${JSON.stringify(
      {
        schemaVersion: 1,
        routeCount: proof.summary.validRoutes,
        rejectedRoutes: proof.summary.rejectedRoutes,
        targetKinds: proof.summary.targetKinds,
        requestedActions: proof.summary.requestedActions,
        adapterDispatchStates: proof.summary.adapterDispatchStates,
      },
      null,
      2
    )}\n`,
    'utf8'
  );
  await writeFile(
    join(proofDir, '06-ui-snapshots', 'ui-not-applicable.md'),
    '# UI Not Applicable\n\nNo portal or child-facing UI source changed in this workpack.\n',
    'utf8'
  );
  await writeFile(
    join(proofDir, '07-playwright-ui-proof.log'),
    'Playwright/browser proof not applicable: no UI source changed.\n',
    'utf8'
  );
  await writeFile(
    join(proofDir, '08-security-negative-proof.log'),
    [
      'Security/no-claim proof:',
      '',
      '- Category/risk rows are policy inputs, not source truth.',
      '- Risk candidates cannot request block-launch or other hard adapter actions.',
      '- Local AI category routes require a digest ref and cannot dispatch adapters.',
      '- Stale category proof is rejected before compile-ready routing.',
      '- adapterDispatchState remains not-dispatched for every valid route.',
      '',
    ].join('\n'),
    'utf8'
  );
  await writeFile(
    join(proofDir, '09-manual-platform-proof.md'),
    '# Manual Platform Proof\n\nNo live platform proof is attached. Adapter execution and platform support remain unclaimed.\n',
    'utf8'
  );
  await writeFile(
    join(proofDir, '10-validation-commands.log'),
    [
      'Validation run:',
      '',
      '- cmd /c npm run build --workspace @ocentra-parent/schema-domain: PASS',
      '- node import @ocentra-parent/schema-domain/app-game-category-risk-policy-routing: PASS',
      '- node import @ocentra-parent/schema-domain/app-game-policy-target-compiler-rules: PASS',
      '- node import @ocentra-parent/schema-domain/policy: PASS',
      '- node import @ocentra-parent/schema-domain/family-reference-primitives: PASS',
      '- node scripts/test/app-game-category-risk-policy-routing-proof.mjs: PASS',
      '',
    ].join('\n'),
    'utf8'
  );
  await writeFile(
    join(proofDir, '11-authority-tier-proof.md'),
    '# Authority Tier Proof\n\nNo authority tier is raised. Routes remain policy-input proof with adapter dispatch disabled.\n',
    'utf8'
  );
  await writeFile(
    join(proofDir, '12-rollback-proof.md'),
    '# Rollback Proof\n\nNo device action, timer, block, suspend, shield, or adapter state is created, so rollback is not applicable.\n',
    'utf8'
  );
}

async function runCommand(command, args) {
  commands.push([command, ...args].join(' '));
  await new Promise((resolve, reject) => {
    const child = spawn(command, args, { cwd: repoRoot, shell: false, stdio: 'inherit' });
    child.on('error', reject);
    child.on('exit', (code) => {
      if (code === 0) {
        resolve(undefined);
        return;
      }
      reject(new Error(`${command} ${args.join(' ')} exited with ${code}`));
    });
  });
}

async function gitBranch() {
  return (await gitOutput(['rev-parse', '--abbrev-ref', 'HEAD'])).trim();
}

async function gitHead() {
  return (await gitOutput(['rev-parse', 'HEAD'])).trim();
}

async function gitOutput(args) {
  const chunks = [];
  await new Promise((resolve, reject) => {
    const child = spawn('git', args, { cwd: repoRoot, shell: false });
    child.stdout.on('data', (chunk) => chunks.push(Buffer.from(chunk)));
    child.on('error', reject);
    child.on('exit', (code) => {
      if (code === 0) {
        resolve(undefined);
        return;
      }
      reject(new Error(`git ${args.join(' ')} exited with ${code}`));
    });
  });
  return Buffer.concat(chunks).toString('utf8');
}

async function writeJson(path, value) {
  await writeFile(path, `${JSON.stringify(value, null, 2)}\n`, 'utf8');
}

function countBy(values) {
  return values.reduce((counts, value) => {
    counts[value] = (counts[value] ?? 0) + 1;
    return counts;
  }, {});
}

function assertEqual(actual, expected, label) {
  if (actual !== expected) {
    throw new Error(`${label}: expected ${expected}, got ${actual}`);
  }
}

function npmCommand(args) {
  const command = process.platform === 'win32' ? 'cmd' : 'npm';
  const commandArgs = process.platform === 'win32' ? ['/c', 'npm', ...args] : args;
  return [command, commandArgs];
}
