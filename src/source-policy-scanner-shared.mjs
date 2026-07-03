import { SOURCE_POLICY_RULES } from './source-policy-rule-registry.mjs';
import { testDoublePatterns } from './source-policy-test-double-patterns.mjs';
import { windowsOnlyCommandPatterns } from './source-policy-windows-command-patterns.mjs';

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
const allowedSensitivePathPatterns = [/(^|\/)\.env\.example$/iu, /(^|\/)\.env\.sample$/iu, /(^|\/)\.env\.template$/iu];
const forbiddenSensitivePathPatterns = [
  /(^|\/)\.env(\..+)?$/iu,
  /(^|\/)google-services\.json$/iu,
  /(^|\/)GoogleService-Info\.plist$/u,
  /(^|\/)id_rsa(\.pub)?$/iu,
  /\.(pem|p12|pfx|key)$/iu,
];

export {
  SOURCE_POLICY_RULES,
  allowedSensitivePathPatterns,
  anySpreadPattern,
  anyTypePattern,
  barrelReexportPattern,
  childProcessPattern,
  consolePattern,
  dateDomainPattern,
  declareGlobalPattern,
  defaultExportPattern,
  definiteAssignmentPattern,
  doubleAssertionPattern,
  dynamicCodePattern,
  dynamicImportPattern,
  emptyCatchPattern,
  enumPattern,
  exportedArrowNoReturnPattern,
  exportedFunctionNoReturnPattern,
  exportedObjectLiteralPattern,
  forbiddenSensitivePathPatterns,
  floatingPromisePattern,
  letInitializerPattern,
  manualBrandPattern,
  mapStringDomainPattern,
  nakedDomainAliasPattern,
  namespacePattern,
  nonNullAssertionPattern,
  optionalFieldPattern,
  partialPattern,
  promiseAnyUnknownPattern,
  processEnvPattern,
  jsonParsePattern,
  rawBooleanDomainPattern,
  rawDtoSpreadPattern,
  rawNumberDomainPattern,
  recordStringDomainPattern,
  recordUnknownPayloadPattern,
  returnNullPattern,
  sharedMutationPattern,
  snapshotVolatilePattern,
  stringArrayDomainPattern,
  testDoublePatterns,
  testNetworkPattern,
  throwStringPattern,
  timerPattern,
  typeAssertionPattern,
  undefinedStatePattern,
  unknownEscapePattern,
  windowsOnlyCommandPatterns,
  zodSourcePatterns,
};
