import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawnCli } from './cli-spawn.mjs';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const SCRIPT = path.join(ROOT, 'scripts', 'rust-rules.mjs');

export function makeProject(files) {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'rust-rules-'));
  fs.mkdirSync(path.join(dir, 'src'), { recursive: true });
  fs.writeFileSync(path.join(dir, 'Cargo.toml'), `
[package]
name = "fixture"
version = "0.1.0"
edition = "2021"
rust-version = "1.75"
`, 'utf8');
  fs.writeFileSync(path.join(dir, 'Cargo.lock'), '', 'utf8');
  fs.writeFileSync(path.join(dir, 'rust-toolchain.toml'), '[toolchain]\nchannel = "1.75.0"\ncomponents = ["rustfmt", "clippy"]\n', 'utf8');
  fs.writeFileSync(path.join(dir, 'clippy.toml'), '# test fixture\n', 'utf8');
  fs.writeFileSync(path.join(dir, 'deny.toml'), '[advisories]\nyanked = "deny"\nunmaintained = "deny"\n', 'utf8');
  fs.writeFileSync(path.join(dir, 'OWNERS'), '@ocentra/enforcer\n', 'utf8');
  for (const [rel, content] of Object.entries(files)) {
    const full = path.join(dir, rel);
    fs.mkdirSync(path.dirname(full), { recursive: true });
    fs.writeFileSync(full, content.trimStart(), 'utf8');
  }
  return dir;
}

export function runGate(project) {
  return spawnCli(process.execPath, [SCRIPT, 'scan', '--root', project], {
    encoding: 'utf8',
  });
}

export function runGateArgs(project, args) {
  return spawnCli(process.execPath, [SCRIPT, ...args, '--root', project], {
    encoding: 'utf8',
  });
}

export function expectFailure(project, ruleId) {
  const result = runGate(project);
  assert.notEqual(result.status, 0, `expected gate to fail for ${ruleId}`);
  const output = `${result.stdout}\n${result.stderr}`;
  assert.match(output, new RegExp(ruleId.replace('.', '\\.'), 'u'), `expected output to contain ${ruleId}. Output:\n${output}`);
  assert.match(output, /Reason:/u, 'failure output must contain a reason');
  assert.match(output, /Fix:/u, 'failure output must contain a fix snippet');
  assert.match(output, /rules\/rust\//u, 'failure output must point at indexed rules doc');
}

export function expectFailures(project, ruleIds) {
  const result = runGate(project);
  assert.notEqual(result.status, 0, `expected gate to fail for ${ruleIds.join(', ')}`);
  const output = `${result.stdout}\n${result.stderr}`;
  for (const ruleId of ruleIds) {
    assert.match(output, new RegExp(ruleId.replace('.', '\\.'), 'u'), `expected output to contain ${ruleId}. Output:\n${output}`);
  }
  assert.match(output, /Reason:/u, 'failure output must contain a reason');
  assert.match(output, /Fix:/u, 'failure output must contain a fix snippet');
  assert.match(output, /rules\/rust\//u, 'failure output must point at indexed rules doc');
}

export function rr1227Fixture(propertyTestSource) {
  const files = {
    'src/lib.rs': `
pub struct ParseInput;
pub struct ParseOutput;

/// Parses validated input; invalid and malformed cases are covered by tests.
pub fn parse_widget(input: ParseInput) -> ParseOutput {
    let _ = input;
    ParseOutput
}
`,
  };
  if (propertyTestSource !== null) files['tests/property.rs'] = propertyTestSource;
  return makeProject(files);
}

export function rr1227Output(project) {
  const result = runGate(project);
  return `${result.stdout}\n${result.stderr}`;
}
