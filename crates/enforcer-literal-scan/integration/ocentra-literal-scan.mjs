import { spawnSync } from 'node:child_process';
import path from 'node:path';

/**
 * Run the literal scanner binary and decode its JSON report.
 */
export function runLiteralScan({ root, files = [], minScore = 40, includeLow = false, failAbove = null, binary = 'ocentra-literal-scan' }) {
  const args = ['scan', '--root', root, '--json', '--min-score', String(minScore)];
  if (includeLow) args.push('--include-low');
  if (failAbove != null) args.push('--fail-above', String(failAbove));
  if (files.length > 0) args.push('--files', ...files);
  const result = spawnSync(binary, args, {
    cwd: root,
    encoding: 'utf8',
    shell: false,
    maxBuffer: 64 * 1024 * 1024,
  });
  const report = JSON.parse(result.stdout || '{}');
  return {
    ok: result.status === 0 && report.ok === true,
    status: result.status,
    report,
    stderr: result.stderr,
  };
}

/**
 * Resolve the platform-specific debug binary shipped beneath an Enforcer pack.
 */
export function defaultLiteralScannerPath(packRoot) {
  return path.join(packRoot, 'tools', 'ocentra-literal-scan', 'target', process.platform === 'win32' ? 'debug/ocentra-literal-scan.exe' : 'debug/ocentra-literal-scan');
}
