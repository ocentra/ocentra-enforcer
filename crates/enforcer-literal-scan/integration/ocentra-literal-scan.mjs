import { spawnSync } from 'node:child_process';
import path from 'node:path';

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
  let report;
  try {
    report = JSON.parse(result.stdout || '{}');
  } catch (error) {
    throw new Error(`literal scanner emitted invalid JSON: ${error.message}\n${result.stdout}\n${result.stderr}`);
  }
  return {
    ok: result.status === 0 && report.ok === true,
    status: result.status,
    report,
    stderr: result.stderr,
  };
}

export function defaultLiteralScannerPath(packRoot) {
  return path.join(packRoot, 'tools', 'ocentra-literal-scan', 'target', process.platform === 'win32' ? 'debug/ocentra-literal-scan.exe' : 'debug/ocentra-literal-scan');
}
