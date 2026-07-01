import { spawn } from 'node:child_process';
import { once } from 'node:events';
import { mkdir, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..', '..');
const portalRoot = path.join(repoRoot, 'apps', 'portal');
const proofRoot = path.join(repoRoot, 'output', 'playwright', 'lan-source-matrix-plan-completion');
const resultDir = path.join(repoRoot, 'test-results', 'v0-9-lan-source-matrix-plan-completion');
const proofPath = path.join(resultDir, 'proof.json');
const browserProofPath = path.join(proofRoot, 'browser-proof.json');
const validationLogPath = path.join(proofRoot, 'validation-commands.log');
const portalUiLogPath = path.join(proofRoot, 'portal-ui-playwright.log');
const networkProofLogPath = path.join(proofRoot, 'network-evidence-playwright.log');
const devicesScreenshotPath = path.join(proofRoot, 'devices-lan-source-matrix.png');
const activityScreenshotPath = path.join(proofRoot, 'activity-network-source-matrix.png');
const policyScreenshotPath = path.join(proofRoot, 'policy-network-target-binding.png');
const commands = [];

await main();

async function main() {
  await mkdir(proofRoot, { recursive: true });
  await mkdir(resultDir, { recursive: true });

  await runNpm(['run', 'build:contracts']);

  const portalUiProof = await runPortalPlaywright(
    'lan-source-matrix-visual-proof.spec.ts',
    {
      LAN_SOURCE_MATRIX_DEVICES_SCREENSHOT: devicesScreenshotPath,
      LAN_SOURCE_MATRIX_POLICY_TARGET_SCREENSHOT: policyScreenshotPath,
    },
    portalUiLogPath
  );
  const networkEvidenceProof = await runPortalPlaywright(
    'network-evidence-drawer-proof.spec.ts',
    {
      NETWORK_EVIDENCE_DRAWER_SCREENSHOT: activityScreenshotPath,
    },
    networkProofLogPath
  );

  await writeProof({
    portalUiProof,
    networkEvidenceProof,
  });

  console.log('lan-source-matrix-visual-proof-ok');
  console.log(`evidence=${relativePath(proofPath)}`);
}

async function runPortalPlaywright(spec, extraEnv, logPath) {
  const result = await runCommand(
    process.execPath,
    ['scripts/test/portal-playwright-runner.mjs'],
    {
      cwd: repoRoot,
      env: {
        ...process.env,
        OCENTRA_PARENT_PORTAL_PLAYWRIGHT_SPEC: spec,
        ...extraEnv,
      },
      capture: true,
    }
  );
  await writeFile(logPath, `${result.output.trimEnd()}\n`);
  return {
    spec,
    exitCode: result.exitCode,
    log: relativePath(logPath),
  };
}

async function runNpm(args, options = {}) {
  if (process.platform === 'win32') {
    await runCommand('cmd', ['/c', 'npm', ...args], options);
    return;
  }
  await runCommand('npm', args, options);
}

async function runCommand(command, args, options = {}) {
  const commandLine = [command, ...args].join(' ');
  const outputChunks = [];
  const child = spawn(command, args, {
    cwd: options.cwd ?? repoRoot,
    env: options.env ?? process.env,
    stdio: ['ignore', options.capture ? 'pipe' : 'inherit', options.capture ? 'pipe' : 'inherit'],
    windowsHide: true,
  });
  if (options.capture) {
    child.stdout?.on('data', (chunk) => {
      outputChunks.push(String(chunk));
      process.stdout.write(chunk);
    });
    child.stderr?.on('data', (chunk) => {
      outputChunks.push(String(chunk));
      process.stderr.write(chunk);
    });
  }
  const [code, signal] = await once(child, 'exit');
  const exitCode = signal === null ? (code ?? 1) : 1;
  commands.push({ command: commandLine, exitCode });
  if (exitCode !== 0) {
    throw new Error(`${commandLine} failed with exit code ${exitCode}`);
  }
  return { exitCode, output: outputChunks.join('') };
}

async function gitHead() {
  const result = await runCommand('git', ['rev-parse', 'HEAD'], {
    cwd: repoRoot,
    capture: true,
  });
  return result.output.trim();
}

async function writeProof({ portalUiProof, networkEvidenceProof }) {
  const checkedAt = new Date().toISOString();
  const commit = await gitHead();
  const proof = {
    schemaVersion: 1,
    checkedAt,
    commit,
    proofMode: 'lan-source-matrix-plan-completion-visual-proof',
    artifacts: {
      proof: relativePath(proofPath),
      browserProof: relativePath(browserProofPath),
      validationLog: relativePath(validationLogPath),
      portalUiLog: portalUiProof.log,
      networkEvidenceLog: networkEvidenceProof.log,
      devicesLanSourceMatrix: relativePath(devicesScreenshotPath),
      activityNetworkSourceMatrix: relativePath(activityScreenshotPath),
      policyNetworkTargetBinding: relativePath(policyScreenshotPath),
    },
    runner: {
      portalRoot: relativePath(portalRoot),
      portalUiSpec: portalUiProof.spec,
      networkEvidenceSpec: networkEvidenceProof.spec,
    },
    assertions: [
      'Devices route renders the live LAN surface after selecting a real LAN device from the service-backed scan and keeps the Trust, Ignore, Restore, and Revoke action surface visible.',
      'Policy Network route preserves the selected device as the policy target instead of falling back to an unscoped family view.',
      'Activity or proof-panel network evidence renders service-backed evidence refs plus LAN source-matrix diagnostics such as policy targets, relay cache, and recent LAN events without UI-owned fabrication.',
    ],
    nonClaims: [
      'This proof does not claim replayable LAN event-stream completion beyond the current route-snapshot bridge.',
      'This proof does not claim physical multi-device household readiness, router or firewall completion, or signed child hello or heartbeat completion.',
      'This proof does not claim that weak LAN discovery sources alone confirm child identity or assignment authority.',
    ],
    commands,
  };
  const content = `${JSON.stringify(proof, null, 2)}\n`;
  await writeFile(proofPath, content);
  await writeFile(browserProofPath, content);
  await writeFile(
    validationLogPath,
    commands.map((entry) => `${entry.command} -> ${entry.exitCode}`).join('\n') + '\n'
  );
}

function relativePath(targetPath) {
  return path.relative(repoRoot, targetPath).replaceAll(path.sep, '/');
}

