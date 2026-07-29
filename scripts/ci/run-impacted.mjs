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

  for (const command of impactedValidationCommands(root, packages)) {
    run(command.label, command.command, command.args, root);
  }
}

/** Builds the complete, batched validation plan for graph-impacted packages. */
export function impactedValidationCommands(root, packages) {
  const packageArgs = packages.flatMap((packageName) => ['--package', packageName]);
  const boundedTests = impactedCargoTestCommand(root, packages);
  return [
    {
      label: 'graph-impacted format check',
      command: 'cargo',
      args: ['fmt', '--check', ...packageArgs],
    },
    {
      label: 'graph-impacted Enforcer Rust workspace scan',
      command: process.execPath,
      // One workspace scan covers every selected package in a single indexed pass.
      args: ['scripts/rust-rules.mjs', 'scan', '--root', '.', '--languages', 'rust', '--workspace', '--scan-only'],
    },
    {
      label: 'graph-impacted cargo check',
      command: 'cargo',
      args: ['check', '--locked', ...packageArgs, '--all-targets', '--all-features'],
    },
    {
      label: 'graph-impacted bounded Cargo tests',
      command: boundedTests.command,
      args: boundedTests.args,
    },
    {
      label: 'graph-impacted cargo clippy',
      command: 'cargo',
      args: ['clippy', '--locked', ...packageArgs, '--all-targets', '--all-features', '--', '-D', 'warnings'],
    },
  ];
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
