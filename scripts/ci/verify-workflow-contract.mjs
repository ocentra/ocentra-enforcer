#!/usr/bin/env node
import { readFileSync } from 'node:fs';
import path from 'node:path';
import process from 'node:process';
import { pathToFileURL } from 'node:url';

/** Return deterministic workflow-contract failures for the selected repository root. */
export function verifyWorkflowContract(root) {
  const required = new Map([
    ['workflows/ci.yml', [
      'plan-impacted', 'run-impacted.mjs', 'check-cargo-workspace-members.mjs --fmt-check',
      'npm run ci:local', 'cargo audit --deny warnings', 'name: required',
      'Enforcer Required Gate',
      'needs: [plan-impacted, impacted, workspace, local-parity, policy, dogfood]',
      'cargo build --locked --package enforcer-cli --bin enforcer',
      './target/debug/enforcer policy secrets',
      './target/debug/enforcer policy dependency-policy',
      './target/debug/enforcer policy sbom --output target/security',
    ]],
    ['workflows/dogfood.yml', [
      'FROZEN_SAFETY_SCANNER_COMMIT: c078c5ceb7318caa295ca26a9496354c238a3b8f',
      'FROZEN_SCANNER_DIR: ${{ runner.temp }}/frozen-safety-scanner',
      'git -C "$FROZEN_SCANNER_DIR" fetch --depth=1 origin "$FROZEN_SAFETY_SCANNER_COMMIT"',
      'test "$(git -C "$FROZEN_SCANNER_DIR" rev-parse HEAD)" = "$FROZEN_SAFETY_SCANNER_COMMIT"',
      'npm ci --ignore-scripts --prefix "$FROZEN_SCANNER_DIR"',
      '- name: Frozen Enforcer Rust workspace scan\n        shell: bash',
      'node "$FROZEN_SCANNER_DIR/scripts/ocentra-enforcer.mjs" scan --root "$GITHUB_WORKSPACE" --languages rust --workspace',
      'verify-workflow-contract.mjs', 'dogfood-manifest',
    ]],
    ['workflows/release.yml', [
      '  push:\n    branches: [main]\n    tags:',
      '  pull_request:\n    branches: [main]',
      "- name: Require the release tag commit to be on main\n        if: startsWith(github.ref, 'refs/tags/v')",
      "- name: Exact local CI parity\n        if: github.event_name != 'pull_request'",
      "build:\n    if: startsWith(github.ref, 'refs/tags/v')",
      "publish:\n    if: startsWith(github.ref, 'refs/tags/v')",
      'needs: validate', 'npm run ci:local', 'Pre-publish smoke gate', 'attest-build-provenance',
      'cargo audit --deny warnings', 'release-security-material', 'macos-15-intel',
      'ubuntu-24.04-arm', 'id-token: write', 'fail_on_unmatched_files: true',
    ]],
    ['actions/setup-enforcer/action.yml', [
      'node-version: 22.22.2', 'toolchain: 1.95.0', 'npm@11.7.0',
      'cargo-audit --version 0.22.2', 'cargo-deny --version 0.20.2',
      'runs:\n  using: composite\n  steps:\n    - uses:',
      '    - name: Select the repository npm version',
      '    - name: Install locked Node dependencies',
    ]],
  ]);
  const failures = [];
  for (const [name, needles] of required) {
    const file = path.join(root, '.github', name);
    let content;
    try {
      content = readFileSync(file, 'utf8').replace(/\r\n/gu, '\n');
    } catch {
      failures.push(`${name}: missing workflow`);
      continue;
    }
    for (const needle of needles) {
      if (!content.includes(needle)) failures.push(`${name}: missing contract marker ${needle}`);
    }
    if (name === 'workflows/dogfood.yml'
      && /node\s+(?:\.\/)?scripts\/(?:rust-rules|ocentra-enforcer)\.mjs\s+scan\b/u.test(content)) {
      failures.push(`${name}: frozen gate must not execute the branch-local scanner`);
    }
    if (name === 'workflows/dogfood.yml'
      && /node\s+"\$FROZEN_SCANNER_DIR\/scripts\/ocentra-enforcer\.mjs"\s+verify\s+ci\b/u.test(content)) {
      failures.push(`${name}: frozen gate must not run the legacy verify profile; branch-native ci:local owns full verification`);
    }
    if (/uses:\s+[^\s]+@v\d+/u.test(content)) {
      failures.push(`${name}: action reference uses a mutable major-version tag`);
    }
    if (name === 'workflows/ci.yml'
      && /^\s+target\s*$/mu.test(content)) {
      failures.push(`${name}: Cargo cache must not archive generated target output`);
    }
    if (name === 'workflows/ci.yml'
      && /node\s+scripts\/ocentra-enforcer\.mjs\s+check\s+(?:secrets|dependency-policy|sbom)\b/u.test(content)) {
      failures.push(`${name}: policy job must use the native Rust policy commands, not legacy Node checks`);
    }
  }
  return failures;
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) {
  const failures = verifyWorkflowContract(process.cwd());
  if (failures.length > 0) {
    for (const failure of failures) console.error(failure);
    process.exit(1);
  }
  console.log('Workflow contract verified.');
}
