import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { createHash } from 'node:crypto';
import { mkdir, readFile, readdir, stat, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';

const repoRoot = process.cwd();
const desktopRoot = join(repoRoot, 'apps', 'parent-desktop');
const outputDir = join(repoRoot, 'test-results', 'parent-desktop-shell-package-proof');
const proofPath = join(outputDir, 'proof.json');
const artifactRoot = join(repoRoot, 'apps', 'parent-desktop', 'src-tauri', 'target', 'release', 'bundle');
const commands = [];

await main();

async function main() {
  await mkdir(outputDir, { recursive: true });

  const defaultDryRun = await runCapturedCommand(
    'node',
    ['scripts/dev/dev-parent-desktop.mjs', '--dry-run'],
    'dev-desktop-dry-run.log'
  );
  const lanDryRun = await runCapturedCommand(
    'node',
    ['scripts/dev/dev-parent-desktop.mjs', '--dry-run', '--lan'],
    'dev-desktop-lan-dry-run.log'
  );
  const defaultGeneratedConfig = await loadGeneratedConfig(defaultDryRun.stdout);
  const lanGeneratedConfig = await loadGeneratedConfig(lanDryRun.stdout);

  assert.equal(new URL(defaultGeneratedConfig.build.devUrl).port, '4478');
  assert.equal(new URL(lanGeneratedConfig.build.devUrl).port, '4478');
  assert.ok(
    ['npm --prefix ../.. run dev:desktop:stack', 'npm --prefix ../.. run dev:desktop:stack:lan'].includes(
      defaultGeneratedConfig.build.beforeDevCommand
    )
  );
  assert.equal(lanGeneratedConfig.build.beforeDevCommand, 'npm --prefix ../.. run dev:desktop:stack:lan');
  assert.match(defaultGeneratedConfig.app.security.csp, /ws:\/\/127\.0\.0\.1:4478/u);
  assert.doesNotMatch(defaultGeneratedConfig.app.security.csp, /4477/u);

  await runCommand('cargo', [
    'test',
    '--manifest-path',
    'apps/parent-desktop/src-tauri/Cargo.toml',
    'parent_platform_proof_state',
    '--',
    '--test-threads=1',
  ]);
  await runCommand('node', ['--test', 'scripts/test/parent-desktop-runtime-package-proof.test.mjs']);

  const artifact = await findDesktopArtifact(artifactRoot);
  const artifactSha256 = await sha256File(artifact.absolutePath);
  const artifactStats = await stat(artifact.absolutePath);

  const proof = {
    schemaVersion: 1,
    checkedAt: new Date().toISOString(),
    artifact_kind: 'desktop',
    shell_kind: 'tauri',
    platform: process.platform === 'win32' ? 'windows' : process.platform,
    launch_command: [
      'node scripts/dev/dev-parent-desktop.mjs --dry-run',
      'node scripts/dev/dev-parent-desktop.mjs --dry-run --lan',
    ],
    launch_state: 'dry-run-launch-anchor-proved',
    service_bridge_state: 'ready-or-degraded-rust-proof',
    degraded_state: 'parent-desktop-runtime-degraded-when-service-unavailable',
    stale_state: 'not-claimed-as-healthy',
    artifact_path: artifact.relativePath,
    artifact_hash_state: `sha256:${artifactSha256}`,
    artifact_size_bytes: artifactStats.size,
    signing_state: 'manual-required',
    update_state: 'scaffold',
    rollback_state: 'unavailable',
    manual_required_state: [
      'signed desktop artifact',
      'production update channel',
      'production rollback execution',
      'setup completion',
      'child-agent runtime authority',
    ],
    no_claim: [
      'desktop launch and package smoke are not product readiness',
      'desktop shell proof does not claim mobile parity',
      'desktop shell proof does not claim setup completion',
      'desktop shell proof does not claim child-agent runtime authority',
    ],
    evidence: {
      defaultDryRunLog: relative(repoRoot, defaultDryRun.logPath),
      lanDryRunLog: relative(repoRoot, lanDryRun.logPath),
      tauriConfig: 'apps/parent-desktop/src-tauri/tauri.conf.json',
      rustProofSource: 'apps/parent-desktop/src-tauri/src/lib.rs',
      runtimeProofTest: 'scripts/test/parent-desktop-runtime-package-proof.test.mjs',
      generatedConfigs: [
        relative(repoRoot, join(desktopRoot, defaultDryRun.generatedConfigPath)),
        relative(repoRoot, join(desktopRoot, lanDryRun.generatedConfigPath)),
      ],
    },
    commands,
  };

  await writeFile(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
  console.log(`parent-desktop-shell-package-proof-ok:${relative(repoRoot, proofPath)}`);
}

async function runCommand(command, args) {
  commands.push([command, ...args].join(' '));
  await new Promise((resolve, reject) => {
    const child = spawn(command, args, {
      cwd: repoRoot,
      stdio: 'inherit',
      windowsHide: true,
      env: process.env,
    });
    child.once('exit', (code) =>
      code === 0 ? resolve() : reject(new Error(`${command} ${args.join(' ')} exited with ${code}`))
    );
    child.once('error', reject);
  });
}

async function runCapturedCommand(command, args, logFileName) {
  commands.push([command, ...args].join(' '));
  const stdout = [];
  const stderr = [];
  await new Promise((resolve, reject) => {
    const child = spawn(command, args, {
      cwd: repoRoot,
      stdio: ['ignore', 'pipe', 'pipe'],
      windowsHide: true,
      env: process.env,
    });
    child.stdout.on('data', (chunk) => stdout.push(String(chunk)));
    child.stderr.on('data', (chunk) => stderr.push(String(chunk)));
    child.once('exit', (code) =>
      code === 0 ? resolve() : reject(new Error(`${command} ${args.join(' ')} exited with ${code}`))
    );
    child.once('error', reject);
  });
  const stdoutText = stdout.join('');
  const stderrText = stderr.join('');
  const generatedConfigPath = parseGeneratedConfigPath(stdoutText);
  const logPath = join(outputDir, logFileName);
  await writeFile(
    logPath,
    `command: ${command} ${args.join(' ')}\n\nstdout:\n${stdoutText}\n\nstderr:\n${stderrText}`,
    'utf8'
  );
  return { generatedConfigPath, logPath, stderr: stderrText, stdout: stdoutText };
}

async function loadGeneratedConfig(stdout) {
  const generatedConfigPath = parseGeneratedConfigPath(stdout);
  const absolutePath = join(desktopRoot, generatedConfigPath);
  return JSON.parse(await readFile(absolutePath, 'utf8'));
}

function parseGeneratedConfigPath(stdout) {
  const match = stdout.match(/Generated Tauri config (.+?)\.\r?\n/u);
  assert.ok(match, 'desktop dry run should print the generated Tauri config path');
  return match[1];
}

async function findDesktopArtifact(root) {
  const entries = await collectFiles(root);
  const preferredExtensions = ['.msi', '.exe', '.app', '.dmg', '.deb', '.rpm', '.AppImage'];
  for (const extension of preferredExtensions) {
    const candidate = entries.find((entry) => entry.endsWith(extension));
    if (candidate !== undefined) {
      return {
        absolutePath: candidate,
        relativePath: relative(repoRoot, candidate),
      };
    }
  }
  throw new Error(`No desktop package artifact found under ${relative(repoRoot, root)}`);
}

async function collectFiles(root) {
  const entries = await readdir(root, { withFileTypes: true });
  const files = [];
  for (const entry of entries) {
    const absolutePath = join(root, entry.name);
    if (entry.isDirectory()) {
      files.push(...(await collectFiles(absolutePath)));
      continue;
    }
    files.push(absolutePath);
  }
  return files;
}

async function sha256File(filePath) {
  const buffer = await readFile(filePath);
  return createHash('sha256').update(buffer).digest('hex');
}
