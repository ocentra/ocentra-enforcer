import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawnCli } from './cli-spawn.mjs';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const SCRIPT = path.join(ROOT, 'scripts', 'rust-rules.mjs');

function makeProject(files) {
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

function runGateArgs(project, args) {
  return spawnCli(process.execPath, [SCRIPT, ...args, '--root', project], {
    encoding: 'utf8',
  });
}

export { makeProject, runGateArgs };
