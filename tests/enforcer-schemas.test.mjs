import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import test from 'node:test';

import {
  decodeEnforcerConfig,
  decodeInitRequest,
  decodeRouteReport,
  decodeRouteRequest,
  decodeRuleRegistry,
  decodeScanReport,
} from '../schemas/effect/enforcer-schemas.mjs';

const PACK_ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');

test('Effect Schema decodes valid registry, config, route, init, and reports', () => {
  const registry = decodeRuleRegistry(JSON.parse(fs.readFileSync(path.join(PACK_ROOT, 'rules', 'rules.json'), 'utf8')));
  assert.equal(registry.productName, 'ocentra-enforcer');
  assert.equal(registry.languages.includes('rust'), true);

  const config = decodeEnforcerConfig(JSON.parse(fs.readFileSync(path.join(PACK_ROOT, 'profiles', 'ocentra-parent.json'), 'utf8')));
  assert.equal(config.profileName, 'ocentra-parent');

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
    root: PACK_ROOT,
    profileName: 'strict',
    scanOnly: true,
    scope: { mode: 'files', files: ['src/lib.rs'] },
  });
  assert.equal(scanReport.ok, true);
});

test('Effect Schema rejects invalid external payloads with useful labels', () => {
  assert.throws(() => decodeRouteRequest({ files: 'src/lib.rs' }), /route request schema validation failed/u);
  assert.throws(() => decodeInitRequest({ adapters: ['husky', 'unknown'] }), /init request schema validation failed/u);
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
    ['schemas/json/route-request.schema.json', 'https://ocentra.dev/schemas/ocentra-enforcer/route-request.schema.json'],
    ['schemas/json/rule-registry.schema.json', 'https://ocentra.dev/schemas/ocentra-enforcer/rule-registry.schema.json'],
  ];

  for (const [relPath, id] of schemas) {
    const parsed = JSON.parse(fs.readFileSync(path.join(PACK_ROOT, relPath), 'utf8'));
    assert.equal(parsed.$schema, 'https://json-schema.org/draft/2020-12/schema');
    assert.equal(parsed.$id, id);
    assert.equal(parsed.title.startsWith('Ocentra Enforcer'), true);
  }
});
