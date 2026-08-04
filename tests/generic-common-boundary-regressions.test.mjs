import assert from 'node:assert/strict';
import test from 'node:test';

import { analyzeBoundarySignatures } from '../src/generic-common-boundary-signatures.mjs';
import { decisionCodeText } from '../src/generic-common-domain-ownership.mjs';

test('boundary signature analysis does not invent undeclared raw DTO ownership', () => {
  const externalTypedRequest = `
pub fn execute(request: &NativeScanRequest) -> Result<NativeScanResult, NativeScanError> {
    todo!()
}`;
  const analysis = analyzeBoundarySignatures(externalTypedRequest, new Set());
  assert.equal(analysis.hasRawInput, false);
  assert.equal(analysis.publicRawReturn, false);
  assert.equal(analysis.untypedConversion, false);
});

test('boundary decision analysis ignores policy words inside transport literals', () => {
  const literalOnly = decisionCodeText([
    'match name {',
    '  "dependency-policy" => execute(),',
    '  _ => reject(),',
    '}',
  ]);
  assert.doesNotMatch(literalOnly, /\bpolicy\b/iu);

  const realDecision = decisionCodeText(['match policy { Allow => execute() }']);
  assert.match(realDecision, /\bpolicy\b/iu);
});
