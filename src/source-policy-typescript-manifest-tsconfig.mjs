import { readLines } from './source-policy-helpers.mjs';
import { addViolation } from './source-policy-violation.mjs';

export function scanTsConfigStrictness(root, filePath) {
  const violations = [];
  let parsed;
  try {
    parsed = JSON.parse(readLines(filePath).join('\n'));
  } catch {
    return violations;
  }
  const options = parsed?.compilerOptions ?? {};
  const strict = options.strict === true;
  const required = [
    'noImplicitAny',
    'strictNullChecks',
    'strictFunctionTypes',
    'strictBindCallApply',
    'strictPropertyInitialization',
    'noImplicitThis',
    'alwaysStrict',
  ];
  const disabledRequired = required.filter((key) => options[key] === false);
  if (!strict || disabledRequired.length > 0) {
    addViolation(
      violations,
      root,
      filePath,
      1,
      'TS-7.1',
      `tsconfig strict mode is disabled${disabledRequired.length > 0 ? `: ${disabledRequired.join(', ')}` : ''}`,
      JSON.stringify(options),
    );
  }
  const requiredFlags = [
    ['noImplicitAny', 'TS-7.2'],
    ['strictNullChecks', 'TS-7.3'],
    ['noUncheckedIndexedAccess', 'TS-7.4'],
    ['exactOptionalPropertyTypes', 'TS-7.5'],
    ['noImplicitOverride', 'TS-7.6'],
    ['noPropertyAccessFromIndexSignature', 'TS-7.7'],
    ['useUnknownInCatchVariables', 'TS-7.8'],
  ];
  for (const [key, ruleId] of requiredFlags) {
    if (options[key] !== true) {
      addViolation(
        violations,
        root,
        filePath,
        1,
        ruleId,
        `${key} must be true`,
        JSON.stringify(options),
      );
    }
  }
  if (typeof options.skipLibCheck !== 'boolean') {
    addViolation(
      violations,
      root,
      filePath,
      1,
      'TS-7.9',
      'skipLibCheck policy must be explicit',
      JSON.stringify(options),
    );
  }
  return violations;
}
