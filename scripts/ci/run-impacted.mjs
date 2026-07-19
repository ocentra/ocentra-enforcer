#!/usr/bin/env node
import { spawnSync } from 'node:child_process';
import process from 'node:process';

const packages = parsePackages(process.argv.slice(2));
if (packages.length === 0) {
  console.log('No graph-impacted Cargo packages were selected.');
  process.exit(0);
}

run(
  'graph-impacted format check',
  'cargo',
  ['fmt', '--check', ...packages.flatMap((packageName) => ['-p', packageName])],
);
for (const packageName of packages) {
  run(`${packageName}: Enforcer crate scan`, process.execPath, [
    'scripts/rust-rules.mjs', 'scan', '--root', '.', '--crate', packageName,
    '--languages', 'rust', '--scan-only',
  ]);
  run(`${packageName}: cargo check`, 'cargo', [
    'check', '--locked', '--package', packageName, '--all-targets', '--all-features',
  ]);
  run(`${packageName}: cargo test`, 'cargo', [
    'test', '--locked', '--package', packageName, '--all-targets', '--all-features',
  ]);
  run(`${packageName}: cargo clippy`, 'cargo', [
    'clippy', '--locked', '--package', packageName, '--all-targets', '--all-features',
    '--', '-D', 'warnings',
  ]);
}

function run(label, command, args) {
  console.log(`\n==> ${label}`);
  const result = spawnSync(command, args, { cwd: process.cwd(), stdio: 'inherit', shell: false });
  if (result.status !== 0) process.exit(result.status ?? 1);
}

function parsePackages(argv) {
  const flag = argv.indexOf('--packages');
  if (flag === -1 || !argv[flag + 1]) throw new Error('Usage: run-impacted.mjs --packages <comma-list>');
  return [...new Set(argv[flag + 1].split(',').map((value) => value.trim()).filter(Boolean))].sort();
}
