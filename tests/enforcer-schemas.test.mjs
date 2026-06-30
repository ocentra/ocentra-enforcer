import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import test from 'node:test';

import {
  decodeCheckReport,
  decodeCheckToolArguments,
  decodeEnforcerConfig,
  decodeInitRequest,
  decodeRouteReport,
  decodeRouteRequest,
  decodeRuleRegistry,
  decodeRunReport,
  decodeRunToolArguments,
  decodeScanReport,
} from '../schemas/effect/enforcer-schemas.mjs';

const PACK_ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');

test('Effect Schema decodes valid registry, config, route, init, and reports', () => {
  const registry = decodeRuleRegistry(JSON.parse(fs.readFileSync(path.join(PACK_ROOT, 'rules', 'rules.json'), 'utf8')));
  assert.equal(registry.productName, 'ocentra-enforcer');
  assert.equal(registry.languages.includes('rust'), true);
  assert.equal(registry.languages.includes('typescript'), true);
  assert.equal(registry.languages.includes('python'), true);

  const config = decodeEnforcerConfig(JSON.parse(fs.readFileSync(path.join(PACK_ROOT, 'profiles', 'ocentra-parent.json'), 'utf8')));
  assert.equal(config.profileName, 'ocentra-parent');

  const policyConfig = decodeEnforcerConfig({
    profileName: 'docs-advisory',
    failOn: ['error'],
    rules: {
      'DOC-1.1': { enabled: true, severity: 'warning', note: 'advisory only' },
      'TS-2.1': { severity: 'error' },
    },
    tools: {
      cargoDoc: { enabled: false, severity: 'warning' },
    },
  });
  assert.equal(policyConfig.rules['DOC-1.1'].severity, 'warning');

  const route = decodeRouteRequest({ root: PACK_ROOT, profile: 'strict', scope: 'files', files: ['src/lib.rs'] });
  assert.deepEqual(route.files, ['src/lib.rs']);

  const init = decodeInitRequest({ root: PACK_ROOT, profile: 'strict', adapters: ['codex', 'mcp', 'precommit'], dryRun: true });
  assert.equal(init.dryRun, true);

  const routeReport = decodeRouteReport({
    ok: true,
    productName: 'ocentra-enforcer',
    profileName: 'strict',
    index: 'rules/INDEX.md',
    scope: { mode: 'files', files: ['src/lib.rs'] },
    docs: ['rules/rust/source.md#covered-rules'],
    rules: [
      {
        id: 'RR-4.1',
        language: 'rust',
        family: 'source',
        severity: 'error',
        doc: 'rules/rust/source.md#covered-rules',
        validator: 'rust/source-scan',
      },
    ],
  });
  assert.equal(routeReport.rules[0].id, 'RR-4.1');

  const scanReport = decodeScanReport({
    ok: true,
    command: 'scan',
    violations: [],
    warnings: [
      {
        ruleId: 'DOC-1.1',
        severity: 'warning',
        title: 'Public API documentation is recommended',
        detail: 'Exported API has no docs.',
        file: 'src/lib.rs',
        line: 1,
        snippet: 'Add a short doc comment.',
        source: 'pub fn thing() {}',
      },
    ],
    findings: [],
    bySeverity: { warning: 1 },
    failOn: ['error'],
    root: PACK_ROOT,
    profileName: 'strict',
    scanOnly: true,
    scope: { mode: 'files', files: ['src/lib.rs'] },
  });
  assert.equal(scanReport.ok, true);

  const runArgs = decodeRunToolArguments({ root: PACK_ROOT, tool: 'node', command: [process.execPath, '--version'] });
  assert.equal(runArgs.command[0], process.execPath);

  const checkArgs = decodeCheckToolArguments({
    root: PACK_ROOT,
    check: 'source-shape',
    scope: 'files',
    files: ['src/checks.mjs'],
    dryRun: true,
  });
  assert.equal(checkArgs.check, 'source-shape');

  const checkReport = decodeCheckReport({
    ok: true,
    command: 'check',
    check: 'source-shape',
    root: PACK_ROOT,
    profileName: 'strict',
    violations: [],
    warnings: [],
    findings: [],
    bySeverity: {},
    scope: { mode: 'files', files: ['src/checks.mjs'] },
  });
  assert.equal(checkReport.ok, true);

  const runReport = decodeRunReport({
    ok: true,
    summary: {
      runId: 'run-1',
      root: PACK_ROOT,
      profile: 'strict',
      tool: 'node',
      language: 'common',
      cwd: '.',
      command: [process.execPath, '--version'],
      status: 'passed',
      exitCode: 0,
      startedAt: '2026-01-01T00:00:00.000Z',
      endedAt: '2026-01-01T00:00:01.000Z',
      diagnosticCount: 0,
      bySeverity: {},
      artifacts: {
        stdout: '.enforce/runs/run-1/raw/stdout.log',
        stderr: '.enforce/runs/run-1/raw/stderr.log',
      },
      duckdb: { available: false },
    },
    diagnostics: [],
  });
  assert.equal(runReport.summary.status, 'passed');
});

test('Effect Schema rejects invalid external payloads with useful labels', () => {
  assert.throws(() => decodeRouteRequest({ files: 'src/lib.rs' }), /route request schema validation failed/u);
  assert.throws(() => decodeInitRequest({ adapters: ['husky', 'unknown'] }), /init request schema validation failed/u);
  assert.throws(() => decodeCheckToolArguments({ check: 'not-real' }), /check tool arguments schema validation failed/u);
  assert.throws(() => decodeRunToolArguments({ command: 'node --version' }), /run tool arguments schema validation failed/u);
  assert.throws(
    () =>
      decodeRuleRegistry({
        schemaVersion: 1,
        productName: 'ocentra-enforcer',
        languages: ['rust'],
        rules: [{ id: 'RR-1.1', language: 'rust', family: 'not-real' }],
      }),
    /rule registry schema validation failed/u
  );
});

test('JSON-schema-compatible artifacts are present for non-Effect consumers', () => {
  const schemas = [
    ['schemas/json/enforcer-config.schema.json', 'https://ocentra.dev/schemas/ocentra-enforcer/enforcer-config.schema.json'],
    ['schemas/json/check-tool-arguments.schema.json', 'https://ocentra.dev/schemas/ocentra-enforcer/check-tool-arguments.schema.json'],
    ['schemas/json/check-report.schema.json', 'https://ocentra.dev/schemas/ocentra-enforcer/check-report.schema.json'],
    ['schemas/json/route-request.schema.json', 'https://ocentra.dev/schemas/ocentra-enforcer/route-request.schema.json'],
    ['schemas/json/rule-registry.schema.json', 'https://ocentra.dev/schemas/ocentra-enforcer/rule-registry.schema.json'],
    ['schemas/json/diagnostic.schema.json', 'https://ocentra.dev/schemas/ocentra-enforcer/diagnostic.schema.json'],
    ['schemas/json/run-report.schema.json', 'https://ocentra.dev/schemas/ocentra-enforcer/run-report.schema.json'],
    ['schemas/json/run-tool-arguments.schema.json', 'https://ocentra.dev/schemas/ocentra-enforcer/run-tool-arguments.schema.json'],
  ];

  for (const [relPath, id] of schemas) {
    const parsed = JSON.parse(fs.readFileSync(path.join(PACK_ROOT, relPath), 'utf8'));
    assert.equal(parsed.$schema, 'https://json-schema.org/draft/2020-12/schema');
    assert.equal(parsed.$id, id);
    assert.equal(parsed.title.startsWith('Ocentra Enforcer'), true);
  }
});
