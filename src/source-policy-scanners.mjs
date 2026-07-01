import fs from 'node:fs';
import path from 'node:path';
import { normalizeRel } from './path-utils.mjs';

export const SOURCE_POLICY_RULES = {
  'TS-1.2': {
    title: 'Direct Zod source usage is forbidden',
    snippet: 'Use Effect Schema through domain-owned schemas instead of importing or exposing Zod directly.',
  },
  'TS-1.3': {
    title: 'Naked domain string aliases are forbidden',
    snippet: 'Use Effect Schema brands plus decode helpers instead of type FooId = string or manual __brand intersections.',
  },
  'TS-6.1': {
    title: 'TypeScript any is forbidden',
    snippet: 'Model unknown input with decoders and precise types; do not use any.',
  },
  'TS-6.2': {
    title: 'TypeScript unknown cannot escape boundaries',
    snippet: 'Decode unknown at the boundary and expose precise domain types inward.',
  },
  'TS-6.3': {
    title: 'TypeScript type assertions are forbidden',
    snippet: 'Replace as-casts with Effect Schema decoding, narrowing, or typed constructors.',
  },
  'TS-6.4': {
    title: 'TypeScript double assertions are forbidden',
    snippet: 'Replace as unknown as casts with real decoding or narrowing.',
  },
  'TS-6.5': {
    title: 'TypeScript non-null assertions are forbidden',
    snippet: 'Prove presence with control flow, schema decoding, or explicit error handling.',
  },
  'TS-6.6': {
    title: 'TypeScript definite assignment assertions are forbidden',
    snippet: 'Initialize fields through constructors or modeled lifecycle states instead of prop! assertions.',
  },
  'TS-6.7': {
    title: 'Raw string domain aliases are forbidden',
    snippet: 'Use Effect Schema branded domain types instead of type FooId = string.',
  },
  'TS-6.8': {
    title: 'Raw number domain values are forbidden',
    snippet: 'Use branded IDs, counts, durations, and units instead of naked number domain parameters.',
  },
  'TS-6.9': {
    title: 'Raw boolean domain parameters are forbidden',
    snippet: 'Replace boolean flags with explicit state/value objects or named option types.',
  },
  'TS-6.10': {
    title: 'Record<string, domain> APIs are forbidden',
    snippet: 'Use a branded key type or domain map instead of Record<string, T>.',
  },
  'TS-6.11': {
    title: 'Map<string, domain> APIs are forbidden',
    snippet: 'Use branded key maps or domain collections instead of Map<string, T>.',
  },
  'TS-6.12': {
    title: 'String arrays are forbidden in domain APIs',
    snippet: 'Use typed collections or branded values instead of string[] domain surfaces.',
  },
  'TS-6.13': {
    title: 'TypeScript default exports are forbidden',
    snippet: 'Use named exports from owning modules and avoid default export ambiguity.',
  },
  'TS-6.14': {
    title: 'Index barrels are forbidden',
    snippet: 'Import from owning modules directly; do not use index.ts barrel re-exports.',
  },
  'TS-6.15': {
    title: 'TypeScript namespace declarations are forbidden',
    snippet: 'Use modules and explicit imports instead of namespace declarations.',
  },
  'TS-6.16': {
    title: 'TypeScript enums are forbidden by default',
    snippet: 'Use union literals or configured enum policy instead of unchecked enum surfaces.',
  },
  'TS-6.17': {
    title: 'Ambient declare global is forbidden outside type owners',
    snippet: 'Keep global augmentation in owned type-boundary modules only.',
  },
  'TS-6.18': {
    title: 'process.env is forbidden outside config boundaries',
    snippet: 'Read environment variables in a config boundary and pass typed values inward.',
  },
  'TS-6.19': {
    title: 'JSON.parse is forbidden outside decoder boundaries',
    snippet: 'Use Effect Schema or a typed decoder for untrusted JSON input.',
  },
  'TS-6.20': {
    title: 'Date is forbidden in domain APIs',
    snippet: 'Use branded time values or decoded timestamps instead of raw Date domain surfaces.',
  },
  'TS-6.21': {
    title: 'Promise<any> and Promise<unknown> are forbidden',
    snippet: 'Return precise promise result types after boundary decoding.',
  },
  'TS-6.22': {
    title: 'Floating promises are forbidden',
    snippet: 'Await promises, return them, or intentionally wrap them in a tracked task boundary.',
  },
  'TS-6.23': {
    title: 'Swallowed promise catches are forbidden',
    snippet: 'Handle promise failures explicitly; do not use empty catch callbacks.',
  },
  'TS-6.24': {
    title: 'console logging is forbidden in source',
    snippet: 'Use project logging domains or structured diagnostics instead of console logging.',
  },
  'TS-6.25': {
    title: 'Throwing string errors is forbidden',
    snippet: 'Throw typed Error objects or return modeled domain errors.',
  },
  'TS-6.26': {
    title: 'return null is forbidden in domain APIs',
    snippet: 'Use an explicit Option/Result-like type instead of null domain returns.',
  },
  'TS-6.27': {
    title: 'undefined domain state is forbidden',
    snippet: 'Model absence explicitly instead of returning or storing undefined as state.',
  },
  'TS-6.28': {
    title: 'Optional domain fields are forbidden by default',
    snippet: 'Use explicit state unions or value objects instead of partial optional domain objects.',
  },
  'TS-6.29': {
    title: 'Partial<T> is forbidden in domain logic',
    snippet: 'Create explicit patch/input types instead of broad Partial<T> domain values.',
  },
  'TS-6.30': {
    title: 'Record<string, unknown> payloads are forbidden',
    snippet: 'Decode command and event payloads into typed schemas instead of raw records.',
  },
  'TS-6.31': {
    title: 'Timer sleeps are forbidden by default',
    snippet: 'Use fake clocks, deterministic events, or harness polling instead of setTimeout sleeps.',
  },
  'TS-6.32': {
    title: 'Dynamic imports are forbidden in domain code',
    snippet: 'Use static imports and explicit boundaries instead of import().',
  },
  'TS-6.33': {
    title: 'child_process is forbidden outside script boundaries',
    snippet: 'Move process spawning to reviewed scripts or harness adapters.',
  },
  'TS-6.34': {
    title: 'Dynamic code execution is forbidden',
    snippet: 'Remove eval, Function constructors, and dynamic code execution.',
  },
  'TS-6.35': {
    title: 'Spreading raw DTOs into domain objects is forbidden',
    snippet: 'Map decoded fields explicitly instead of spreading raw, dto, payload, or json objects.',
  },
  'TS-6.36': {
    title: 'Spreading any into domain objects is forbidden',
    snippet: 'Decode and construct domain values explicitly instead of spreading any values.',
  },
  'TS-6.37': {
    title: 'Exported functions require explicit return types',
    snippet: 'Add explicit return types to exported functions so API contracts cannot drift.',
  },
  'TS-6.38': {
    title: 'Exported object literals cannot be inferred APIs',
    snippet: 'Annotate exported object APIs or replace them with named typed values.',
  },
  'TS-6.39': {
    title: 'Use const instead of single-assignment let',
    snippet: 'Use const for local values unless reassignment is required.',
  },
  'TS-6.40': {
    title: 'Mutating imported or shared objects is forbidden',
    snippet: 'Return new values or use explicit owners instead of mutating imported/shared state.',
  },
  'TS-7.1': {
    title: 'TypeScript strict mode is required',
    snippet: 'Enable strict compiler options instead of relying on partial type checking.',
  },
  'TS-7.2': {
    title: 'noImplicitAny is required',
    snippet: 'Enable noImplicitAny in tsconfig.',
  },
  'TS-7.3': {
    title: 'strictNullChecks is required',
    snippet: 'Enable strictNullChecks in tsconfig.',
  },
  'TS-7.4': {
    title: 'noUncheckedIndexedAccess is required',
    snippet: 'Enable noUncheckedIndexedAccess in tsconfig.',
  },
  'TS-7.5': {
    title: 'exactOptionalPropertyTypes is required',
    snippet: 'Enable exactOptionalPropertyTypes in tsconfig.',
  },
  'TS-7.6': {
    title: 'noImplicitOverride is required',
    snippet: 'Enable noImplicitOverride in tsconfig.',
  },
  'TS-7.7': {
    title: 'noPropertyAccessFromIndexSignature is required',
    snippet: 'Enable noPropertyAccessFromIndexSignature in tsconfig.',
  },
  'TS-7.8': {
    title: 'useUnknownInCatchVariables is required',
    snippet: 'Enable useUnknownInCatchVariables in tsconfig.',
  },
  'TS-7.9': {
    title: 'skipLibCheck policy must be explicit',
    snippet: 'Set skipLibCheck explicitly so consumers know the project policy.',
  },
  'TS-7.10': {
    title: 'Package manager lockfile is required',
    snippet: 'Commit a package lockfile for TypeScript packages.',
  },
  'TS-7.11': {
    title: 'Loose npm dependency versions are forbidden',
    snippet: 'Pin npm dependencies exactly; do not use ^, ~, *, latest, git, or path ranges.',
  },
  'TS-7.12': {
    title: 'npm ci is required in CI',
    snippet: 'Use npm ci in workflows and scripts that represent CI gates.',
  },
  'TS-7.13': {
    title: 'ESLint must enforce unsafe TypeScript rules',
    snippet: 'Enable no-floating-promises, no-explicit-any, and no-unsafe rules in ESLint config.',
  },
  'TS-7.14': {
    title: 'Zod dependencies are forbidden by policy',
    snippet: 'Use Effect Schema through Enforcer policy instead of adding Zod dependencies.',
  },
  'TS-7.15': {
    title: 'Duplicate package managers are forbidden by default',
    snippet: 'Use one configured package manager and one lockfile family per project.',
  },
  'TS-8.1': {
    title: 'Skipped and focused TypeScript tests are forbidden',
    snippet: 'Remove .skip, .only, and .todo from checked-in tests.',
  },
  'TS-8.2': {
    title: 'TypeScript expect.any assertions are forbidden',
    snippet: 'Assert concrete values instead of expect.anything or expect.any(String/Number).',
  },
  'TS-8.3': {
    title: 'Weak TypeScript assertions are forbidden',
    snippet: 'Assert concrete behavior instead of toBeTruthy, toBeDefined, or not.toThrow.',
  },
  'TS-8.4': {
    title: 'Empty TypeScript tests are forbidden',
    snippet: 'Every test must exercise behavior and include assertions.',
  },
  'TS-8.5': {
    title: 'TypeScript tests must assert behavior',
    snippet: 'Add explicit assertions to every test body.',
  },
  'TS-8.6': {
    title: 'Network calls are forbidden in TypeScript unit tests',
    snippet: 'Move network checks to explicit integration proofs.',
  },
  'TS-8.7': {
    title: 'Real timers are forbidden in deterministic TypeScript tests',
    snippet: 'Use fake clocks, events, or controlled polling instead of real timers.',
  },
  'TS-8.8': {
    title: 'TypeScript test doubles are forbidden by default',
    snippet: 'Use real local collaborators and domain contracts instead of mocks, stubs, fakes, or spies.',
  },
  'TS-8.9': {
    title: 'Snapshots cannot contain volatile values',
    snippet: 'Redact timestamps, UUIDs, and random IDs before snapshot assertions.',
  },
  'TS-8.10': {
    title: 'Decoder and schema tests require negative cases',
    snippet: 'Add invalid-input, malformed, or throws/rejects tests for decoders and schemas.',
  },
  'TEST-1.1': {
    title: 'Test doubles are forbidden by default',
    snippet: 'Use real domain contracts, real parsers, and real local services instead of mocks, fakes, stubs, or spies.',
  },
  'PORT-1.1': {
    title: 'Platform-specific script commands must be guarded',
    snippet: 'Put Windows-only command invocations behind an explicit process.platform guard or use a cross-platform helper.',
  },
  'SEC-1.2': {
    title: 'Sensitive files are forbidden in source scope',
    snippet: 'Do not commit .env files, private keys, mobile service secrets, or credential bundles.',
  },
  'GEN-1.2': {
    title: 'Generated output folders must not be committed as source',
    snippet: 'Keep proof output, test results, reports, and generated build artifacts in ignored output folders or CI artifacts.',
  },
};

const zodSourcePatterns = [
  { label: 'direct zod import', pattern: /from\s+['"]zod['"]|require\(\s*['"]zod['"]\s*\)/u },
  { label: 'Zod resolver', pattern: /\bzodResolver\b/u },
  {
    label: 'Zod public type/API',
    pattern: /\bZod(?:Error|Issue|Type|Schema|Object|String|Number|Boolean|Array|Record|Union)\b/u,
  },
  { label: 'stale schema validator path', pattern: new RegExp(String.raw`schema[/\\]` + 'zo' + 'd', 'u') },
];

const manualBrandPattern = /\b(?:export\s+)?type\s+\w+\s*=\s*string\s*&\s*\{\s*readonly\s+__brand\b/u;
const nakedDomainAliasPattern =
  /^\s*(?:export\s+)?type\s+(\w*(?:Id|ID|Path|Key|Name|Hash|URL|Url|Type|Slug|Route|Label|Title|Description|Status|Version)\w*)\s*=\s*string\s*;/u;
const anyTypePattern =
  /(?::\s*any\b|<\s*any\s*>|\bArray\s*<\s*any\s*>|\bRecord\s*<\s*string\s*,\s*any\s*>|\bas\s+any\b)/u;
const unknownEscapePattern =
  /(?:export\s+(?:type|interface|function|const|let|class)\b.*\bunknown\b|:\s*unknown\b|Promise\s*<\s*unknown\s*>)/u;
const typeAssertionPattern =
  /\bas\s+(?!const\b|never\b|unknown\b)[A-Za-z_$][\w$]*(?:<[^>]+>)?(?:\[\])?/u;
const doubleAssertionPattern = /\bas\s+unknown\s+as\s+[A-Za-z_$][\w$]*/u;
const recordStringDomainPattern = /\bRecord\s*<\s*string\s*,\s*[A-Z][A-Za-z0-9_]*(?:\[\])?\s*>/u;
const mapStringDomainPattern = /\bMap\s*<\s*string\s*,\s*[A-Z][A-Za-z0-9_]*(?:\[\])?\s*>/u;
const stringArrayDomainPattern = /\b(?:[A-Za-z_$][\w$]*(?:Ids|Names|Keys|Paths|Urls|URLs|Tags|Labels)\s*(?::|=)\s*string\s*\[\]|Array\s*<\s*string\s*>)/u;
const nonNullAssertionPattern =
  /\b[A-Za-z_$][\w$]*(?:\.[A-Za-z_$][\w$]*|\[[^\]]+\])*!+(?!\s*[=])/u;
const definiteAssignmentPattern = /^\s*(?:public|private|protected|readonly|\s)*[A-Za-z_$][\w$]*!\s*:/u;
const defaultExportPattern = /^\s*export\s+default\b/u;
const barrelReexportPattern =
  /^\s*export\s+(?:\*\s*(?:as\s+\w+\s*)?|(?:type\s+)?\{[^}]+\})\s+from\s+['"][^'"]+['"]/u;
const namespacePattern = /^\s*(?:export\s+)?namespace\s+[A-Za-z_$][\w$]*/u;
const enumPattern = /^\s*(?:export\s+)?(?:const\s+)?enum\s+[A-Za-z_$][\w$]*/u;
const declareGlobalPattern = /^\s*declare\s+global\b/u;
const processEnvPattern = /\bprocess\.env(?:\.|\[)/u;
const jsonParsePattern = /\bJSON\.parse\s*\(/u;
const rawNumberDomainPattern =
  /\b(?:[A-Za-z_$][\w$]*(?:Id|ID|Count|Size|Length|Timeout|Delay|Duration|Ms|Millis|Seconds)|(?:id|count|timeout|delay|duration|state))\s*:\s*number\b/u;
const rawBooleanDomainPattern =
  /\b(?:is[A-Z][A-Za-z0-9_$]*|has[A-Z][A-Za-z0-9_$]*|should[A-Z][A-Za-z0-9_$]*|enabled|disabled|flag|active|ready)\s*:\s*boolean\b/u;
const dateDomainPattern = /\b(?:Date\b|:\s*Date\b|Promise\s*<\s*Date\s*>)/u;
const promiseAnyUnknownPattern = /\bPromise\s*<\s*(?:any|unknown)\s*>/u;
const emptyCatchPattern = /\.catch\s*\(\s*(?:\(\s*\)|[A-Za-z_$][\w$]*)\s*=>\s*\{\s*\}\s*\)/u;
const consolePattern = /\bconsole\.(?:log|debug|info|warn|error|trace)\s*\(/u;
const throwStringPattern = /\bthrow\s+(['"`])[^'"`]+\1/u;
const returnNullPattern = /\breturn\s+null\s*;/u;
const undefinedStatePattern = /\b(?:return\s+undefined\s*;|:\s*undefined\b|=\s*undefined\s*;)/u;
const optionalFieldPattern = /^\s*(?:readonly\s+)?[A-Za-z_$][\w$]*\??:\s*[^;]+[;,]?/u;
const partialPattern = /\bPartial\s*<\s*[A-Z][A-Za-z0-9_$]*\s*>/u;
const recordUnknownPayloadPattern = /\bRecord\s*<\s*string\s*,\s*unknown\s*>/u;
const timerPattern = /\b(?:setTimeout|setInterval)\s*\(/u;
const dynamicImportPattern = /\bimport\s*\(/u;
const childProcessPattern = /\b(?:from\s+['"]node:child_process['"]|from\s+['"]child_process['"]|require\(\s*['"](?:node:)?child_process['"]\s*\))/u;
const dynamicCodePattern = /\b(?:eval|Function)\s*\(/u;
const rawDtoSpreadPattern = /\.\.\.\s*(?:raw|dto|payload|json|input|data|[A-Za-z_$][\w$]*(?:Dto|DTO|Payload|Json|JSON|Input|Data))\b/u;
const anySpreadPattern = /\.\.\.\s*[A-Za-z_$][\w$]*Any\b|\.\.\.\s*\([^)]*\s+as\s+any\s*\)/u;
const exportedFunctionNoReturnPattern =
  /^\s*export\s+(?:async\s+)?function\s+[A-Za-z_$][\w$]*\s*\([^)]*\)\s*(?!:\s*[^={]+[={;])/u;
const exportedArrowNoReturnPattern =
  /^\s*export\s+const\s+[A-Za-z_$][\w$]*\s*=\s*(?:async\s*)?\([^)]*\)\s*=>/u;
const exportedObjectLiteralPattern = /^\s*export\s+const\s+[A-Za-z_$][\w$]*\s*=\s*\{/u;
const letInitializerPattern = /^\s*let\s+[A-Za-z_$][\w$]*\s*=/u;
const sharedMutationPattern = /\b(?:shared|imported|global|cache|state|registry|config)\w*\.(?:push|pop|splice|set|delete|clear|add)\s*\(/iu;
const floatingPromisePattern =
  /^\s*(?!await\b|return\b|void\b)(?:[A-Za-z_$][\w$]*\.)?(?:[A-Za-z_$][\w$]*Async|fetch[A-Za-z0-9_$]*)\s*\([^;]*\)\s*;/u;
const testNetworkPattern = /\b(?:fetch|axios\.|request\.|supertest\(|http\.|https\.)/u;
const snapshotVolatilePattern =
  /\b(?:toMatchSnapshot|toMatchInlineSnapshot)\s*\([^)]*(?:Date|new Date|uuid|random|timestamp|\d{4}-\d{2}-\d{2}T|[0-9a-f]{8}-[0-9a-f]{4})/iu;
const windowsOnlyCommandPatterns = [
  {
    label: 'Windows cmd npm invocation',
    pattern: /['"]cmd(?:\.exe)?['"]\s*,\s*\[\s*['"]\/c['"]\s*,\s*['"]npm['"]/u,
  },
];
const word = (...parts) => parts.join('');
const doubleTerms = {
  m: word('mo', 'ck'),
  f: word('fa', 'ke'),
  s: word('st', 'ub'),
  p: word('sp', 'y'),
  po: word('sp', 'y', 'On'),
};
const testDoublePatterns = [
  { label: 'module double API', pattern: new RegExp(String.raw`\b(?:vi|jest)\.${doubleTerms.m}\b`, 'iu') },
  { label: 'double function API', pattern: /\b(?:vi|jest)\.fn\b/iu },
  {
    label: 'observer double API',
    pattern: new RegExp(String.raw`\b(?:vi|jest)\.${doubleTerms.po}\b|\b${doubleTerms.po}\b`, 'iu'),
  },
  {
    label: 'test-double package',
    pattern: new RegExp(String.raw`\b(?:${word('si', 'non')}|${word('no', 'ck')}|${word('m', 'sw')})\b`, 'iu'),
  },
  {
    label: 'test-double vocabulary',
    pattern: new RegExp(String.raw`\b(?:${doubleTerms.m}|${doubleTerms.f}|${doubleTerms.s}|${doubleTerms.p})\b`, 'iu'),
  },
];
const allowedSensitivePathPatterns = [/(^|\/)\.env\.example$/iu, /(^|\/)\.env\.sample$/iu, /(^|\/)\.env\.template$/iu];
const forbiddenSensitivePathPatterns = [
  /(^|\/)\.env(\..+)?$/iu,
  /(^|\/)google-services\.json$/iu,
  /(^|\/)GoogleService-Info\.plist$/u,
  /(^|\/)id_rsa(\.pub)?$/iu,
  /\.(pem|p12|pfx|key)$/iu,
];

export function scanAdditionalTypeScriptFile(root, filePath) {
  const rel = normalizeRel(root, filePath);
  const lines = readLines(filePath);
  const violations = [];
  const generatedPath = isGeneratedSourcePath(rel);
  const toolingBoundary = isToolingBoundaryPath(rel);

  lines.forEach((line, index) => {
    const lineNo = index + 1;
    const maskedLine = maskJavaScriptLine(line);
    for (const rule of zodSourcePatterns) {
      if (rule.pattern.test(line)) {
        addViolation(violations, root, filePath, lineNo, 'TS-1.2', rule.label, line);
      }
    }

    if (!generatedPath && manualBrandPattern.test(line)) {
      addViolation(violations, root, filePath, lineNo, 'TS-1.3', 'manual string brand', line);
    }

    const nakedDomainAlias = line.match(nakedDomainAliasPattern);
    if (!generatedPath && nakedDomainAlias) {
      addViolation(violations, root, filePath, lineNo, 'TS-1.3', `naked domain string alias ${nakedDomainAlias[1]}`, line);
      addViolation(violations, root, filePath, lineNo, 'TS-6.7', `raw string domain alias ${nakedDomainAlias[1]}`, line);
    }

    if (!generatedPath && recordStringDomainPattern.test(maskedLine)) {
      addViolation(violations, root, filePath, lineNo, 'TS-6.10', 'Record<string, domain> API found', line);
    }

    if (!generatedPath && anyTypePattern.test(maskedLine)) {
      addViolation(violations, root, filePath, lineNo, 'TS-6.1', 'any type usage found', line);
    }

    if (!generatedPath && !isDecoderBoundaryPath(rel) && unknownEscapePattern.test(maskedLine)) {
      addViolation(violations, root, filePath, lineNo, 'TS-6.2', 'unknown type escapes decoder boundary', line);
    }

    if (!generatedPath && !toolingBoundary && doubleAssertionPattern.test(maskedLine)) {
      addViolation(violations, root, filePath, lineNo, 'TS-6.4', 'double type assertion found', line);
    }

    if (!generatedPath && !toolingBoundary && (typeAssertionPattern.test(maskedLine) || doubleAssertionPattern.test(maskedLine))) {
      addViolation(violations, root, filePath, lineNo, 'TS-6.3', 'type assertion found', line);
    }

    if (!generatedPath && !toolingBoundary && nonNullAssertionPattern.test(maskedLine)) {
      addViolation(violations, root, filePath, lineNo, 'TS-6.5', 'non-null assertion found', line);
    }

    if (!generatedPath && !toolingBoundary && definiteAssignmentPattern.test(maskedLine)) {
      addViolation(violations, root, filePath, lineNo, 'TS-6.6', 'definite assignment assertion found', line);
    }

    if (!generatedPath && rawNumberDomainPattern.test(maskedLine)) {
      addViolation(violations, root, filePath, lineNo, 'TS-6.8', 'raw number domain value found', line);
    }

    if (!generatedPath && rawBooleanDomainPattern.test(maskedLine)) {
      addViolation(violations, root, filePath, lineNo, 'TS-6.9', 'raw boolean domain parameter found', line);
    }

    if (!generatedPath && mapStringDomainPattern.test(maskedLine)) {
      addViolation(violations, root, filePath, lineNo, 'TS-6.11', 'Map<string, domain> API found', line);
    }

    if (!generatedPath && stringArrayDomainPattern.test(maskedLine)) {
      addViolation(violations, root, filePath, lineNo, 'TS-6.12', 'string[] domain API found', line);
    }

    if (!generatedPath && !toolingBoundary && defaultExportPattern.test(maskedLine)) {
      addViolation(violations, root, filePath, lineNo, 'TS-6.13', 'default export found', line);
    }

    if (!generatedPath && isIndexModule(rel) && barrelReexportPattern.test(line)) {
      addViolation(violations, root, filePath, lineNo, 'TS-6.14', 'index barrel re-export found', line);
    }

    if (!generatedPath && namespacePattern.test(maskedLine)) {
      addViolation(violations, root, filePath, lineNo, 'TS-6.15', 'namespace declaration found', line);
    }

    if (!generatedPath && enumPattern.test(maskedLine)) {
      addViolation(violations, root, filePath, lineNo, 'TS-6.16', 'enum declaration found', line);
    }

    if (!generatedPath && !isTypeOwnerPath(rel) && declareGlobalPattern.test(maskedLine)) {
      addViolation(violations, root, filePath, lineNo, 'TS-6.17', 'declare global outside type owner', line);
    }

    if (!generatedPath && processEnvPattern.test(maskedLine) && !isConfigBoundaryPath(rel)) {
      addViolation(violations, root, filePath, lineNo, 'TS-6.18', 'process.env read outside config boundary', line);
    }

    if (!generatedPath && jsonParsePattern.test(maskedLine) && !isDecoderBoundaryPath(rel)) {
      addViolation(violations, root, filePath, lineNo, 'TS-6.19', 'JSON.parse outside decoder boundary', line);
    }

    if (!generatedPath && !isDecoderBoundaryPath(rel) && dateDomainPattern.test(maskedLine)) {
      addViolation(violations, root, filePath, lineNo, 'TS-6.20', 'Date domain API found', line);
    }

    if (!generatedPath && promiseAnyUnknownPattern.test(maskedLine)) {
      addViolation(violations, root, filePath, lineNo, 'TS-6.21', 'Promise<any|unknown> found', line);
    }

    if (!generatedPath && !toolingBoundary && !isTestPath(rel) && floatingPromisePattern.test(maskedLine)) {
      addViolation(violations, root, filePath, lineNo, 'TS-6.22', 'possible floating promise call found', line);
    }

    if (!generatedPath && !toolingBoundary && emptyCatchPattern.test(maskedLine)) {
      addViolation(violations, root, filePath, lineNo, 'TS-6.23', 'empty promise catch handler found', line);
    }

    if (!generatedPath && !toolingBoundary && consolePattern.test(maskedLine)) {
      addViolation(violations, root, filePath, lineNo, 'TS-6.24', 'console logging found', line);
    }

    if (!generatedPath && /\bthrow\b/u.test(maskedLine) && throwStringPattern.test(line)) {
      addViolation(violations, root, filePath, lineNo, 'TS-6.25', 'throw string found', line);
    }

    if (!generatedPath && !toolingBoundary && returnNullPattern.test(maskedLine)) {
      addViolation(violations, root, filePath, lineNo, 'TS-6.26', 'return null found', line);
    }

    if (!generatedPath && !toolingBoundary && undefinedStatePattern.test(maskedLine)) {
      addViolation(violations, root, filePath, lineNo, 'TS-6.27', 'undefined domain state found', line);
    }

    if (!generatedPath && !toolingBoundary && optionalFieldPattern.test(maskedLine)) {
      addViolation(violations, root, filePath, lineNo, 'TS-6.28', 'optional domain field found', line);
    }

    if (!generatedPath && partialPattern.test(maskedLine)) {
      addViolation(violations, root, filePath, lineNo, 'TS-6.29', 'Partial<T> domain value found', line);
    }

    if (!generatedPath && recordUnknownPayloadPattern.test(maskedLine)) {
      addViolation(violations, root, filePath, lineNo, 'TS-6.30', 'Record<string, unknown> payload found', line);
    }

    if (!generatedPath && timerPattern.test(maskedLine) && !hasNearbyTimerJustification(lines, index)) {
      addViolation(violations, root, filePath, lineNo, 'TS-6.31', 'timer sleep found', line);
      if (isTestPath(rel)) {
        addViolation(violations, root, filePath, lineNo, 'TS-8.7', 'real timer in test found', line);
      }
    }

    if (!generatedPath && !toolingBoundary && dynamicImportPattern.test(maskedLine)) {
      addViolation(violations, root, filePath, lineNo, 'TS-6.32', 'dynamic import found', line);
    }

    if (!generatedPath && !toolingBoundary && childProcessPattern.test(line)) {
      addViolation(violations, root, filePath, lineNo, 'TS-6.33', 'child_process import outside script boundary', line);
    }

    if (!generatedPath && !toolingBoundary && dynamicCodePattern.test(maskedLine)) {
      addViolation(violations, root, filePath, lineNo, 'TS-6.34', 'dynamic code execution found', line);
    }

    if (!generatedPath && !toolingBoundary && rawDtoSpreadPattern.test(maskedLine)) {
      addViolation(violations, root, filePath, lineNo, 'TS-6.35', 'raw DTO spread found', line);
    }

    if (!generatedPath && !toolingBoundary && anySpreadPattern.test(maskedLine)) {
      addViolation(violations, root, filePath, lineNo, 'TS-6.36', 'any spread found', line);
    }

    if (!generatedPath && !toolingBoundary && (exportedFunctionNoReturnPattern.test(maskedLine) || exportedArrowNoReturnPattern.test(maskedLine))) {
      addViolation(violations, root, filePath, lineNo, 'TS-6.37', 'exported function lacks explicit return type', line);
    }

    if (!generatedPath && !toolingBoundary && exportedObjectLiteralPattern.test(maskedLine)) {
      addViolation(violations, root, filePath, lineNo, 'TS-6.38', 'exported object literal has inferred API type', line);
    }

    if (!generatedPath && !toolingBoundary && !isTestPath(rel) && letInitializerPattern.test(maskedLine)) {
      addViolation(violations, root, filePath, lineNo, 'TS-6.39', 'let initializer found where const is expected', line);
    }

    if (!generatedPath && !toolingBoundary && sharedMutationPattern.test(maskedLine)) {
      addViolation(violations, root, filePath, lineNo, 'TS-6.40', 'shared/imported object mutation found', line);
    }

    if (isTestPath(rel)) {
      if (/\b(?:describe|it|test)\s*\.\s*(?:skip|only|todo)\s*\(/u.test(line)) {
        addViolation(violations, root, filePath, lineNo, 'TS-8.1', 'skipped/focused TypeScript test found', line);
      }
      if (/expect\.(?:anything|any)\s*\(/u.test(line)) {
        addViolation(violations, root, filePath, lineNo, 'TS-8.2', 'expect.any assertion found', line);
      }
      if (/\.(?:toBeTruthy|toBeDefined|not\.toThrow)\s*\(/u.test(line)) {
        addViolation(violations, root, filePath, lineNo, 'TS-8.3', 'weak TypeScript assertion found', line);
      }
      if (testNetworkPattern.test(maskedLine)) {
        addViolation(violations, root, filePath, lineNo, 'TS-8.6', 'network call in unit test found', line);
      }
      if (/\b(?:vi|jest)\.(?:mock|fn|spyOn)\b|\b(?:mock|stub|fake|spy)\b/iu.test(line)) {
        addViolation(violations, root, filePath, lineNo, 'TS-8.8', 'test double found', line);
      }
      if (snapshotVolatilePattern.test(line)) {
        addViolation(violations, root, filePath, lineNo, 'TS-8.9', 'volatile snapshot assertion found', line);
      }
    }
  });

  if (isTestPath(rel)) {
    violations.push(...scanTypeScriptTestBlocks(root, filePath, lines));
  }

  if (path.basename(rel) === 'package.json') {
    violations.push(...scanPackageManifestForZod(root, filePath));
    violations.push(...scanPackageManifestForTypeScriptPolicy(root, filePath));
  }

  if (/^tsconfig(?:\.[^.]+)?\.json$/iu.test(path.basename(rel))) {
    violations.push(...scanTsConfigStrictness(root, filePath));
  }

  return violations;
}

function scanTsConfigStrictness(root, filePath) {
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

function scanPackageManifestForTypeScriptPolicy(root, filePath) {
  const violations = [];
  let parsed;
  try {
    parsed = JSON.parse(readLines(filePath).join('\n'));
  } catch {
    return violations;
  }
  const dir = path.dirname(filePath);
  const lockfiles = ['package-lock.json', 'npm-shrinkwrap.json', 'pnpm-lock.yaml', 'yarn.lock'];
  const presentLocks = lockfiles.filter((name) => fs.existsSync(path.join(dir, name)));
  if (presentLocks.length === 0) {
    addViolation(violations, root, filePath, 1, 'TS-7.10', 'package manager lockfile missing', path.basename(filePath));
  }
  if (presentLocks.length > 1) {
    addViolation(violations, root, filePath, 1, 'TS-7.15', `multiple lockfiles found: ${presentLocks.join(', ')}`, path.basename(filePath));
  }
  for (const section of ['dependencies', 'devDependencies', 'peerDependencies', 'optionalDependencies']) {
    const dependencies = parsed[section];
    if (dependencies == null || typeof dependencies !== 'object') continue;
    for (const [name, version] of Object.entries(dependencies)) {
      if (typeof version !== 'string') continue;
      if (/^(?:\^|~|\*|latest\b|git\+|github:|file:|link:|workspace:\*)/iu.test(version)) {
        addViolation(violations, root, filePath, 1, 'TS-7.11', `loose dependency version ${name}@${version}`, `${name}: ${version}`);
      }
      if (['zod', 'zod-to-json-schema', 'zod-validation-error'].includes(name)) {
        addViolation(violations, root, filePath, 1, 'TS-7.14', `direct ${name} dependency in ${section}`, name);
      }
    }
  }
  return violations;
}

function scanTypeScriptTestBlocks(root, filePath, lines) {
  const violations = [];
  let current = null;
  let braceDepth = 0;
  const flush = () => {
    if (!current) return;
    const body = current.lines.join('\n').trim();
    if (body === '' || body === '{}' || /(?:=>|function\s*\([^)]*\))\s*\{\s*\}\s*\)\s*;?$/u.test(body)) {
      addViolation(violations, root, filePath, current.line, 'TS-8.4', 'empty TypeScript test body', current.header);
    } else if (!/\b(?:expect|assert|should|toEqual|toBe|toStrictEqual|expectFailure|expectPass|expectViolation|assertFixtureRules|assertViolation|assertViolations)\b/u.test(body)) {
      addViolation(violations, root, filePath, current.line, 'TS-8.5', 'TypeScript test has no assertion', current.header);
    }
    current = null;
    braceDepth = 0;
  };
  lines.forEach((line, index) => {
    if (!current && /^\s*(?:it|test)\s*\(/u.test(line)) {
      flush();
      current = { line: index + 1, header: line, lines: [line] };
      braceDepth = braceDelta(maskJavaScriptLine(line));
      if (braceDepth <= 0 && /\)\s*;?\s*$/u.test(line)) flush();
      return;
    }
    if (current) {
      current.lines.push(line);
      braceDepth += braceDelta(maskJavaScriptLine(line));
      if (braceDepth <= 0 && /^\s*\}\s*\)\s*;?\s*$/u.test(line)) flush();
    }
  });
  flush();
  return violations;
}

function braceDelta(line) {
  let delta = 0;
  for (const char of line) {
    if (char === '{') delta += 1;
    if (char === '}') delta -= 1;
  }
  return delta;
}

function isIndexModule(rel) {
  return /(?:^|\/)index\.[cm]?[jt]sx?$/iu.test(rel);
}

function isTestPath(rel) {
  return /(?:^|\/)(?:tests?|__tests__|spec)(?:\/|$)|\.(?:test|spec)\.[cm]?[jt]sx?$/iu.test(rel);
}

function isTypeOwnerPath(rel) {
  return /(?:^|\/)(?:types?|globals?|ambient|declarations)(?:\/|\.|-)|\.d\.ts$/iu.test(rel);
}

export function scanAdditionalCommonFile(root, filePath, lines) {
  const rel = normalizeRel(root, filePath);
  const text = lines.join('\n');
  const violations = [];

  if (isForbiddenSensitivePath(rel)) {
    addViolation(violations, root, filePath, 1, 'SEC-1.2', 'forbidden sensitive file path', rel);
  }

  if (/^(?:output|test-results|playwright-report)\//iu.test(rel)) {
    addViolation(violations, root, filePath, 1, 'GEN-1.2', 'generated output path is in source scope', rel);
  }

  if (path.basename(rel) === 'package.json') {
    violations.push(...scanPackageManifestForZod(root, filePath));
    violations.push(...scanPackageManifestForTypeScriptPolicy(root, filePath));
  }

  if (/\.github\/workflows\/.*\.ya?ml$/iu.test(rel) || rel.startsWith('scripts/')) {
    lines.forEach((line, index) => {
      if (/\bnpm\s+install\b/iu.test(line) && !/\bnpm\s+ci\b/iu.test(line)) {
        addViolation(violations, root, filePath, index + 1, 'TS-7.12', 'CI/script uses npm install instead of npm ci', line);
      }
    });
  }

  if (/(?:^|\/)(?:eslint\.config\.[cm]?js|\.eslintrc(?:\.[cm]?js|\.json)?)$/iu.test(rel)) {
    const requiredRules = [
      '@typescript-eslint/no-floating-promises',
      '@typescript-eslint/no-explicit-any',
      '@typescript-eslint/no-unsafe-assignment',
    ];
    const missing = requiredRules.filter((ruleName) => !text.includes(ruleName));
    if (missing.length > 0) {
      addViolation(violations, root, filePath, 1, 'TS-7.13', `ESLint config misses strict TypeScript rules: ${missing.join(', ')}`, rel);
    }
  }

  if (isTestPath(rel) && /(?:schema|decoder|codec)/iu.test(rel) && !/\b(?:invalid|malformed|bad input|reject|throws?|toThrow|rejects)\b/iu.test(text)) {
    addViolation(violations, root, filePath, 1, 'TS-8.10', 'decoder/schema test lacks negative invalid-input coverage', rel);
  }

  if (isSourceLikeForTestDoubles(rel)) {
    lines.forEach((line, index) => {
      for (const rule of testDoublePatterns) {
        if (rule.pattern.test(line)) {
          addViolation(violations, root, filePath, index + 1, 'TEST-1.1', rule.label, line);
        }
      }
    });
  }

  if (rel.startsWith('scripts/') && rel.endsWith('.mjs')) {
    lines.forEach((line, index) => {
      for (const rule of windowsOnlyCommandPatterns) {
        if (rule.pattern.test(line) && !hasNearbyWindowsGuard(lines, index)) {
          addViolation(violations, root, filePath, index + 1, 'PORT-1.1', rule.label, line);
        }
      }
    });
  }

  return violations;
}

function scanPackageManifestForZod(root, filePath) {
  const violations = [];
  let parsed;
  try {
    parsed = JSON.parse(readLines(filePath).join('\n'));
  } catch {
    return violations;
  }

  for (const section of ['dependencies', 'devDependencies', 'peerDependencies', 'optionalDependencies']) {
    const dependencies = parsed[section];
    if (dependencies == null || typeof dependencies !== 'object') continue;
    for (const name of Object.keys(dependencies)) {
      if (['zod', 'zod-to-json-schema', 'zod-validation-error'].includes(name)) {
        addViolation(violations, root, filePath, 1, 'TS-1.2', `direct ${name} dependency in ${section}`, name);
      }
    }
  }

  return violations;
}

function readLines(filePath) {
  return fs.readFileSync(filePath, 'utf8').split(/\r?\n/u);
}

function hasNearbyWindowsGuard(lines, lineIndex) {
  const start = Math.max(0, lineIndex - 8);
  const nearby = lines.slice(start, lineIndex + 1).join('\n');
  return /process\.platform\s*={2,3}\s*['"]win32['"]|process\.platform\s*!={1,2}\s*['"]win32['"]/u.test(nearby);
}

function hasNearbyTimerJustification(lines, lineIndex) {
  const start = Math.max(0, lineIndex - 4);
  const end = Math.min(lines.length, lineIndex + 2);
  const nearby = lines.slice(start, end).join('\n');
  return /\b(?:TIMER|HARNESS-TIMER)-JUSTIFICATION:/u.test(nearby);
}

function isForbiddenSensitivePath(rel) {
  if (allowedSensitivePathPatterns.some((pattern) => pattern.test(rel))) return false;
  return forbiddenSensitivePathPatterns.some((pattern) => pattern.test(rel));
}

function isSourceLikeForTestDoubles(rel) {
  return /\.(?:ts|tsx|js|jsx|mjs|cjs|mts|cts|py|rs)$/iu.test(rel);
}

function isGeneratedSourcePath(rel) {
  return /(?:^|\/)generated(?:\/|$)/iu.test(rel);
}

function isToolingBoundaryPath(rel) {
  return /^(?:scripts|mcp|eslint-rules|adapters|tests|schemas)\//u.test(rel) ||
    /^src\/(?:checks|codex-install|harness|path-utils|policy|proof|routing|rule-registry|source-policy-scanners)\.mjs$/u.test(rel) ||
    /^src\/coordination\//u.test(rel);
}

function isConfigBoundaryPath(rel) {
  return isToolingBoundaryPath(rel) || /(?:^|\/)(?:config|configs|configuration|env|environment)(?:\/|\.|-)|(?:^|\/)[^/]*(?:config|env)[^/]*\.(?:ts|tsx|js|mjs|cjs)$/iu.test(rel);
}

function isDecoderBoundaryPath(rel) {
  return isToolingBoundaryPath(rel) || /(?:^|\/)(?:schema|schemas|decoder|decoders|codec|codecs|boundary|boundaries|adapter|adapters|transport|serde)(?:\/|\.|-)/iu.test(rel);
}

function maskJavaScriptLine(line) {
  return line
    .replace(/\/\/.*$/u, '')
    .replace(/'(?:[^'\\]|\\.)*'/gu, "''")
    .replace(/"(?:[^"\\]|\\.)*"/gu, '""')
    .replace(/`(?:[^`\\]|\\.)*`/gu, '``');
}

function addViolation(violations, root, filePath, line, ruleId, detail, sourceLine = null) {
  const rule = SOURCE_POLICY_RULES[ruleId] ?? { title: 'Unknown rule', snippet: '' };
  violations.push({
    ruleId,
    title: rule.title,
    detail,
    file: normalizeRel(root, filePath),
    line,
    snippet: rule.snippet,
    source: sourceLine?.trim() ?? null,
  });
}
