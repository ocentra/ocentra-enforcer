#!/usr/bin/env node
import { spawnSync } from 'node:child_process';
import path from 'node:path';
import process from 'node:process';
import { fileURLToPath } from 'node:url';

const SCRIPT_PATH = fileURLToPath(import.meta.url);
const PACK_ROOT = path.resolve(path.join(path.dirname(SCRIPT_PATH), '..'));
const args = parseArgs(process.argv.slice(2));
const npm = npmStep;

const steps = [
  ['git diff whitespace check', 'git', ['diff', '--check']],
  ['test suite', ...npm(['test'])],
  ['enforcer scanner', ...npm(['run', 'rust:rules:scan'])],
  ['enforcer cargo hard gate', ...npm(['run', 'rust:rules'])],
  ['Codex skill/plugin asset validation', ...npm(['run', 'validate:codex-assets'])],
  ['MCP test suite', ...npm(['run', 'test:mcp'])],
  ['MCP smoke content-length', ...npm(['run', 'mcp:smoke'])],
  ['MCP smoke NDJSON', ...npm(['run', 'mcp:smoke:ndjson'])],
];

if (args.parentRoot) {
  steps.push([
    'Ocentra Parent read-only file-scope smoke',
    process.execPath,
    [
      path.join(PACK_ROOT, 'scripts', 'rust-rules.mjs'),
      'scan',
      '--root',
      args.parentRoot,
      '--profile',
      'ocentra-parent',
      '--files',
      'crates/agent-protocol/src/lib.rs',
      '--json',
    ],
  ]);
}

for (const [label, command, commandArgs] of steps) {
  console.log(`\n==> ${label}`);
  const result = spawnSync(command, commandArgs, {
    cwd: PACK_ROOT,
    stdio: 'inherit',
    shell: false,
  });
  if (result.status !== 0) {
    process.exit(result.status ?? 1);
  }
}

console.log('\nOcentra Enforcer local CI gate passed.');

function parseArgs(argv) {
  const parsed = {
    parentRoot: process.env.OCENTRA_PARENT_ROOT || null,
  };
  for (let i = 0; i < argv.length; i += 1) {
    const arg = argv[i];
    if (arg === '--parent-root') {
      parsed.parentRoot = argv[++i] ?? null;
    } else if (arg === '--help' || arg === '-h') {
      console.log(
        'Usage: node scripts/ci-local.mjs [--parent-root <OcentraParent checkout>]',
      );
      process.exit(0);
    } else {
      throw new Error(`Unknown argument: ${arg}`);
    }
  }
  return parsed;
}

function npmStep(args) {
  if (process.platform !== 'win32') return ['npm', args];
  return [process.env.ComSpec || 'cmd.exe', ['/d', '/s', '/c', 'npm', ...args]];
}
