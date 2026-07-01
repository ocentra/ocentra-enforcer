import assert from 'node:assert/strict';
import { dirname, resolve } from 'node:path';
import { test } from 'node:test';
import { fileURLToPath } from 'node:url';
import { ESLint } from 'eslint';
import tseslint from 'typescript-eslint';
import ocentraParentRules from '../../eslint-rules/index.js';

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), '../..');

async function lintRule(ruleName, code) {
  const eslint = new ESLint({
    overrideConfigFile: true,
    overrideConfig: [
      {
        files: ['**/*.ts'],
        languageOptions: {
          ecmaVersion: 2022,
          parser: tseslint.parser,
          sourceType: 'module',
        },
        plugins: {
          'ocentra-parent': ocentraParentRules,
        },
        rules: {
          [`ocentra-parent/${ruleName}`]: 'error',
        },
      },
    ],
  });

  const [result] = await eslint.lintText(code, { filePath: 'sample.ts' });
  assert.ok(result);
  return result.messages;
}

test('no-app-string-literals allows module specifiers and rejects runtime literals', async () => {
  const messages = await lintRule(
    'no-app-string-literals',
    "import { value } from './domain';\nconst runtime = 'bad';\n"
  );

  assert.equal(messages.length, 1);
  assert.equal(messages[0].ruleId, 'ocentra-parent/no-app-string-literals');
});

test('no-runtime-string-types rejects raw app string annotations', async () => {
  const messages = await lintRule('no-runtime-string-types', 'function render(value: string): void {}\n');

  assert.equal(messages.length, 1);
  assert.equal(messages[0].ruleId, 'ocentra-parent/no-runtime-string-types');
});

test('no-naked-domain-string-types rejects manual and alias brands', async () => {
  const aliasMessages = await lintRule('no-naked-domain-string-types', 'export type DeviceId = string;\n');
  const manualMessages = await lintRule(
    'no-naked-domain-string-types',
    "export type DeviceId = string & { readonly __brand: 'DeviceId' };\n"
  );

  assert.equal(aliasMessages.length, 1);
  assert.equal(aliasMessages[0].ruleId, 'ocentra-parent/no-naked-domain-string-types');
  assert.equal(manualMessages.length, 1);
  assert.equal(manualMessages[0].ruleId, 'ocentra-parent/no-naked-domain-string-types');
});

test('workspace eslint config wires app source string rules', async () => {
  const eslint = new ESLint({ cwd: repoRoot });
  const [result] = await eslint.lintText("const value: string = 'bad';\n", {
    filePath: resolve(repoRoot, 'apps/portal/src/sample.ts'),
  });

  assert.ok(result);
  const ruleIds = new Set(result.messages.map((message) => message.ruleId));
  assert.equal(ruleIds.has('ocentra-parent/no-app-string-literals'), true);
  assert.equal(ruleIds.has('ocentra-parent/no-runtime-string-types'), true);
});
