import { spawn } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';

const repoRoot = process.cwd();
const proofMode = 'tracking-plan-wsl-local-proof';
const outputDir = join(repoRoot, 'test-results', proofMode);
const proofPath = join(outputDir, 'proof.json');
const proofRoot = join(repoRoot, 'output', 'tracking-plan-proof', 'wsl-local-replay');
const wp32Proof = join(
  repoRoot,
  'output',
  'tracking-plan-proof',
  '32-journal-sqlite-and-read-model-proof',
  '19-wsl-local-replay-proof.json'
);
const wp33Proof = join(
  repoRoot,
  'output',
  'tracking-plan-proof',
  '33-proof-gates-fixtures-rollout-and-pr-gate',
  '17-wsl-local-proof.json'
);

const plannedCommands = [
  ['build-contracts', 'npm', ['run', 'build:contracts']],
  ['service-read-model-proof', 'node', ['scripts/test/tracking-plan-service-read-model-proof.mjs']],
  ['rust-core-read-model-test', 'cargo', ['test', '-p', 'ocentra-parent-agent-core', 'tracking_read_model']],
];

await main();

async function main() {
  await mkdir(outputDir, { recursive: true });
  await mkdir(proofRoot, { recursive: true });
  await mkdir(join(wp32Proof, '..'), { recursive: true });
  await mkdir(join(wp33Proof, '..'), { recursive: true });

  const docs = await assertDocsKeepClaimsBlocked();
  const checkedAt = new Date().toISOString();
  const wslRepoPath = windowsPathToWsl(repoRoot);
  const wslGitDir = await mappedWslGitDir();
  const probes = await runProbes(wslRepoPath, wslGitDir);
  const commandResults = probes.mappedGit.exitCode === 0 ? await runPlannedCommands(wslRepoPath, wslGitDir) : [];
  const status = classifyStatus(probes, commandResults);
  const proof = {
    schemaVersion: 1,
    checkedAt,
    commit: await gitHead(),
    proofMode,
    workpackIds: ['32-journal-sqlite-and-read-model-proof', '33-proof-gates-fixtures-rollout-and-pr-gate'],
    requiredProofTier: 'P3_LOCAL_DEV_MACHINE',
    currentProofTier: status.allPlannedCommandsPassed ? 'P3_LOCAL_DEV_MACHINE' : 'P3_LOCAL_DEV_MACHINE_PARTIAL',
    currentStatus: status.currentStatus,
    productClaimReady: false,
    wslLocalReplayReady: status.allPlannedCommandsPassed,
    wsl: {
      repoPath: wslRepoPath,
      mappedGitDir: wslGitDir,
      linkedWorktreeGitdirMappingRequired: probes.plainGit.exitCode !== 0 && probes.mappedGit.exitCode === 0,
      probes,
    },
    commands: commandResults,
    docs,
    nonClaims: [
      'WSL/local replay does not prove Android or iOS physical background behavior.',
      'WSL/local replay does not prove mobile permission grants, geofence delivery, killed-app behavior, reboot behavior, or OEM background reliability.',
      'WSL/local replay does not prove enrolled-device authority, notification provider delivery, hosted UI accessibility, or production pilot readiness.',
    ],
    nextSteps: status.nextSteps,
  };

  await writeJson(proofPath, proof);
  await writeJson(join(proofRoot, 'proof.json'), proof);
  await writeJson(wp32Proof, proof);
  await writeJson(wp33Proof, proof);
  await writeFile(join(proofRoot, '00-source-snapshot.md'), sourceSnapshot(proof));
  await writeFile(join(proofRoot, '16-validation-commands.log'), validationLog(proof));

  console.log('tracking-plan-wsl-local-proof-ok');
  console.log(`status=${proof.currentStatus}`);
  console.log(`evidence=${relativePath(proofPath)}`);
  console.log(`proofRoot=${relativePath(proofRoot)}`);
}

async function runProbes(wslRepoPath, wslGitDir) {
  const status = await runAndLog('wsl-status', 'wsl.exe', ['--status']);
  const distros = await runAndLog('wsl-distros', 'wsl.exe', ['-l', '-v']);
  const toolchain = await runWslAndLog(
    'wsl-toolchain',
    'pwd && uname -a && node --version && npm --version && cargo --version && rustc --version && git --version'
  );
  const plainGit = await runWslAndLog(
    'plain-linked-worktree-git-probe',
    `cd ${shQuote(wslRepoPath)} && git rev-parse --show-toplevel && git rev-parse HEAD`
  );
  const mappedGit = await runWslAndLog(
    'mapped-linked-worktree-git-probe',
    `cd ${shQuote(wslRepoPath)} && ${wslGitEnv(wslGitDir, wslRepoPath)} git rev-parse --show-toplevel && ${wslGitEnv(
      wslGitDir,
      wslRepoPath
    )} git rev-parse HEAD`
  );
  return { status, distros, toolchain, plainGit, mappedGit };
}

async function runPlannedCommands(wslRepoPath, wslGitDir) {
  const results = await ensureWslOptionalNodeDependencies(wslRepoPath, wslGitDir);
  for (const [id, command, args] of plannedCommands) {
    const result = await runWslAndLog(
      id,
      `cd ${shQuote(wslRepoPath)} && ${wslGitEnv(wslGitDir, wslRepoPath)} ${[command, ...args].map(shQuote).join(' ')}`
    );
    results.push({ ...result, blocker: blockerFor(result) });
  }
  return results;
}

async function ensureWslOptionalNodeDependencies(wslRepoPath, wslGitDir) {
  const results = [];
  const bindingPackage = '@rolldown/binding-linux-x64-gnu';
  const probe = await runWslAndLog(
    'rolldown-linux-binding-probe',
    `cd ${shQuote(wslRepoPath)} && if [ -d node_modules/${bindingPackage} ]; then echo present; else echo missing; fi`
  );
  results.push({ ...probe, blocker: 'none' });
  if (probe.stdout.trim() === 'missing') {
    const repair = await runWslAndLog(
      'rolldown-linux-binding-repair',
      `cd ${shQuote(wslRepoPath)} && ${wslGitEnv(
        wslGitDir,
        wslRepoPath
      )} npm install --no-save --ignore-scripts ${bindingPackage}@1.0.1`
    );
    results.push({ ...repair, blocker: blockerFor(repair) });
  }
  return results;
}

function classifyStatus(probes, commands) {
  if (probes.status.exitCode !== 0) {
    return {
      currentStatus: 'manual_required_wsl_unavailable',
      allPlannedCommandsPassed: false,
      nextSteps: ['Install or enable WSL2, then rerun npm run test:tracking-plan-wsl-local-proof.'],
    };
  }
  if (probes.mappedGit.exitCode !== 0) {
    return {
      currentStatus: 'blocked_by_wsl_gitdir_mapping',
      allPlannedCommandsPassed: false,
      nextSteps: ['Create a WSL-visible git-dir mapping for this Windows linked worktree before replay commands run.'],
    };
  }
  const allPlannedCommandsPassed = commands.every((entry) => entry.exitCode === 0);
  const optionalDependencyBlocked = commands.some(
    (entry) => entry.blocker === 'wsl_linux_optional_node_dependency_missing'
  );
  if (allPlannedCommandsPassed) {
    return {
      currentStatus: 'proved',
      allPlannedCommandsPassed,
      nextSteps: ['Use physical Android/iOS device proof for mobile background and permission claims.'],
    };
  }
  if (optionalDependencyBlocked) {
    return {
      currentStatus: 'partial_proof_blocked_by_wsl_optional_node_dependency',
      allPlannedCommandsPassed,
      nextSteps: [
        'Install Linux optional Node dependencies in a WSL-specific dependency tree or rerun npm install from WSL without disturbing Windows validation.',
        'Rerun node scripts/test/tracking-plan-service-read-model-proof.mjs from WSL after the Rolldown Linux binding is present.',
      ],
    };
  }
  return {
    currentStatus: 'partial_proof_failed_wsl_command',
    allPlannedCommandsPassed,
    nextSteps: [
      'Inspect command logs under output/tracking-plan-proof/wsl-local-replay/ and rerun the failed command.',
    ],
  };
}

function blockerFor(result) {
  const output = `${result.stdout}\n${result.stderr}`;
  if (output.includes('Cannot find native binding') || output.includes('@rolldown/binding-linux-x64-gnu')) {
    return 'wsl_linux_optional_node_dependency_missing';
  }
  return result.exitCode === 0 ? 'none' : 'command_failed';
}

async function assertDocsKeepClaimsBlocked() {
  const feature = await readRepoFile('docs/features/location-geofence-device-status.md');
  const checklist = await readRepoFile('docs/plans/tracking-plan/implementation-checklist.md');
  assertIncludes(feature, 'WSL/local', 'feature WSL proof-plan gap');
  assertIncludes(feature, 'WSL/local replay', 'feature WSL replay gap');
  assertIncludes(checklist, 'WSL/local replay', 'implementation checklist WSL gap');
  assertIncludes(
    checklist,
    'Android background claims have real device permission/background proof',
    'Android claim gate'
  );
  assertIncludes(checklist, 'iOS background/region claims have real device permission/background', 'iOS claim gate');
  return {
    featureDoc: 'docs/features/location-geofence-device-status.md',
    implementationChecklist: 'docs/plans/tracking-plan/implementation-checklist.md',
    productCapabilityChecklist: 'not edited by this worker; deltas are queued through hub DOC_DELTA',
  };
}

async function mappedWslGitDir() {
  const gitFile = await readRepoFile('.git');
  const match = gitFile.match(/^gitdir:\s*(.+)\s*$/mu);
  if (match === null) {
    return null;
  }
  return windowsPathToWsl(match[1].trim());
}

async function runWslAndLog(id, script) {
  return runAndLog(id, 'wsl.exe', ['sh', '-lc', script]);
}

async function runAndLog(id, command, args) {
  const result = await runProcess(command, args);
  const logPath = join(proofRoot, `${id}.log`);
  await writeFile(logPath, commandLog({ id, command, args, ...result }));
  return {
    id,
    command: [command, ...args].join(' '),
    exitCode: result.exitCode,
    stdout: trimForProof(result.stdout),
    stderr: trimForProof(result.stderr),
    logPath: relativePath(logPath),
  };
}

function runProcess(command, args) {
  return new Promise((resolve) => {
    const child = spawn(command, args, { cwd: repoRoot, windowsHide: true });
    const stdout = [];
    const stderr = [];
    child.stdout.on('data', (chunk) => stdout.push(chunk));
    child.stderr.on('data', (chunk) => stderr.push(chunk));
    child.once('error', (error) => {
      resolve({ exitCode: -1, stdout: '', stderr: error.message });
    });
    child.once('exit', (exitCode) => {
      resolve({
        exitCode,
        stdout: normalize(Buffer.concat(stdout).toString('utf8')),
        stderr: normalize(Buffer.concat(stderr).toString('utf8')),
      });
    });
  });
}

async function gitHead() {
  const result = await runProcess('git', ['rev-parse', 'HEAD']);
  if (result.exitCode !== 0) {
    throw new Error('git rev-parse HEAD failed');
  }
  return result.stdout.trim();
}

async function readRepoFile(path) {
  return readFile(join(repoRoot, path), 'utf8');
}

async function writeJson(path, value) {
  await mkdir(join(path, '..'), { recursive: true });
  await writeFile(path, `${JSON.stringify(value, null, 2)}\n`);
}

function commandLog(entry) {
  return (
    [
      `id=${entry.id}`,
      `command=${[entry.command, ...entry.args].join(' ')}`,
      `exit=${entry.exitCode}`,
      '',
      '[stdout]',
      normalize(entry.stdout) || '(empty)',
      '',
      '[stderr]',
      normalize(entry.stderr) || '(empty)',
    ].join('\n') + '\n'
  );
}

function sourceSnapshot(proof) {
  return [
    '# WSL Local Replay Proof',
    '',
    `Checked at: ${proof.checkedAt}`,
    `Commit: ${proof.commit}`,
    `Status: ${proof.currentStatus}`,
    `Product claim ready: ${proof.productClaimReady}`,
    '',
    '## Command Results',
    '',
    ...proof.commands.map(
      (entry) => `- ${entry.id}: exit ${entry.exitCode}; blocker=${entry.blocker}; log=${entry.logPath}`
    ),
    '',
    '## Non-Claims',
    '',
    ...proof.nonClaims.map((entry) => `- ${entry}`),
    '',
  ].join('\n');
}

function validationLog(proof) {
  return [
    ...Object.values(proof.wsl.probes).map((entry) => `${entry.id} exit=${entry.exitCode} log=${entry.logPath}`),
    ...proof.commands.map(
      (entry) => `${entry.id} exit=${entry.exitCode} blocker=${entry.blocker} log=${entry.logPath}`
    ),
    '',
  ].join('\n');
}

function windowsPathToWsl(path) {
  if (path === null) {
    return null;
  }
  const match = path.replaceAll('\\', '/').match(/^([A-Za-z]):\/(.*)$/u);
  if (match === null) {
    return path;
  }
  return `/mnt/${match[1].toLowerCase()}/${match[2]}`;
}

function wslGitEnv(gitDir, workTree) {
  return `GIT_DIR=${shQuote(gitDir)} GIT_WORK_TREE=${shQuote(workTree)}`;
}

function shQuote(value) {
  return `'${String(value).replaceAll("'", "'\\''")}'`;
}

function relativePath(path) {
  return relative(repoRoot, path).replaceAll('\\', '/');
}

function normalize(value) {
  return value
    .replace(/\u001b\[[0-?]*[ -/]*[@-~]/gu, '')
    .replace(/\u0000/gu, '')
    .replace(/[^\u0009\u000a\u000d\u0020-\u007e]/gu, '')
    .replace(/\r\n?/gu, '\n')
    .split('\n')
    .map((line) => line.trimEnd())
    .join('\n')
    .trimEnd();
}

function trimForProof(value) {
  const normalized = normalize(value);
  return normalized.length > 4000 ? `${normalized.slice(0, 4000)}\n[truncated]` : normalized;
}

function assertIncludes(value, expected, label) {
  if (!value.includes(expected)) {
    throw new Error(`${label}: missing ${expected}`);
  }
}
