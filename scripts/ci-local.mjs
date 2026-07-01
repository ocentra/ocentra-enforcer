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
  ['policy rule tests', ...npm(['run', 'test:policy'])],
  ['multi-language tests', ...npm(['run', 'test:multilang'])],
  ['fixture tests', ...npm(['run', 'test:fixtures'])],
  ['enforcer scanner', ...npm(['run', 'rust:rules:scan'])],
  ['enforcer cargo hard gate', ...npm(['run', 'rust:rules'])],
  ['enforcer self scan', ...npm(['run', 'enforcer:self'])],
  ['enforcer rule coverage', ...npm(['run', 'enforcer:coverage'])],
  ['enforcer policy integrity', ...npm(['run', 'enforcer:policy'])],
  ['enforcer schema/config verification', ...npm(['run', 'enforcer:verify'])],
  [
    'enforcer secret scan',
    process.execPath,
    [path.join(PACK_ROOT, 'scripts', 'rust-rules.mjs'), 'check', 'secrets', '--root', PACK_ROOT],
  ],
  [
    'enforcer dependency policy',
    process.execPath,
    [
      path.join(PACK_ROOT, 'scripts', 'rust-rules.mjs'),
      'check',
      'dependency-policy',
      '--root',
      PACK_ROOT,
    ],
  ],
  [
    'enforcer SBOM check',
    process.execPath,
    [path.join(PACK_ROOT, 'scripts', 'rust-rules.mjs'), 'check', 'sbom', '--root', PACK_ROOT],
  ],
  ['coordination CLI smoke', ...npm(['run', 'coordination:smoke'])],
  ['coordination presence smoke', ...npm(['run', 'coordination:presence:smoke'])],
  ['coordination stream manifest smoke', ...npm(['run', 'coordination:manifest:smoke'])],
  ['proof route smoke', ...npm(['run', 'proof:smoke'])],
  ['proof run smoke', ...npm(['run', 'proof:run:smoke'])],
  ['architecture CLI smoke', ...npm(['run', 'architecture:smoke'])],
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
