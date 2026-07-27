#!/usr/bin/env node
import { spawnSync } from 'node:child_process';
import path from 'node:path';
import process from 'node:process';
import { fileURLToPath } from 'node:url';

/** Runs all graph-impacted package gates with bounded generated test artifacts. */
export function runImpacted(packages, root = process.cwd()) {
  if (packages.length === 0) {
    console.log('No graph-impacted Cargo packages were selected.');
    return;
  }

  run(
    'graph-impacted format check',
    'cargo',
    ['fmt', '--check', ...packages.flatMap((packageName) => ['-p', packageName])],
    root,
  );
  for (const packageName of packages) {
    run(`${packageName}: Enforcer crate scan`, process.execPath, [
      'scripts/rust-rules.mjs', 'scan', '--root', '.', '--crate', packageName,
      '--languages', 'rust', '--scan-only',
    ], root);
    run(`${packageName}: cargo check`, 'cargo', [
      'check', '--locked', '--package', packageName, '--all-targets', '--all-features',
    ], root);
  }

  const boundedTests = impactedCargoTestCommand(root, packages);
  run('graph-impacted bounded Cargo tests', boundedTests.command, boundedTests.args, root);

  for (const packageName of packages) {
    run(`${packageName}: cargo clippy`, 'cargo', [
      'clippy', '--locked', '--package', packageName, '--all-targets', '--all-features',
      '--', '-D', 'warnings',
    ], root);
  }
}

/** Builds the bounded Cargo test command for the selected package set. */
export function impactedCargoTestCommand(root, packages) {
  return {
    command: process.execPath,
    args: [
      path.join(root, 'scripts', 'check-cargo-workspace-tests.mjs'),
      ...packages.flatMap((packageName) => ['--package', packageName]),
    ],
  };
}

function run(label, command, args, root) {
  console.log(`\n==> ${label}`);
  const result = spawnSync(command, args, { cwd: root, stdio: 'inherit', shell: false });
  if (result.status !== 0) process.exit(result.status ?? 1);
}

/** Parses and normalizes the graph-impacted package selection. */
export function parsePackages(argv) {
  const flag = argv.indexOf('--packages');
  if (flag === -1 || !argv[flag + 1]) throw new Error('Usage: run-impacted.mjs --packages <comma-list>');
  return [...new Set(argv[flag + 1].split(',').map((value) => value.trim()).filter(Boolean))].sort();
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  runImpacted(parsePackages(process.argv.slice(2)));
}
