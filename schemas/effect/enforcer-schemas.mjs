import { Schema } from 'effect';

export const ProductName = 'ocentra-enforcer';

const StringArray = Schema.Array(Schema.String);
const OptionalStringArray = Schema.optional(StringArray);
const OptionalBoolean = Schema.optional(Schema.Boolean);
const OptionalString = Schema.optional(Schema.String);
const OptionalNumber = Schema.optional(Schema.Number);
const OptionalNullableNumber = Schema.optional(Schema.NullOr(Schema.Number));

export const LanguageSchema = Schema.Literal('rust', 'typescript', 'python');
export const RustRuleFamilySchema = Schema.Literal(
  'source',
  'domain',
  'imports-modules',
  'toolchain-cargo',
  'dependencies',
  'async-runtime'
);
export const SeveritySchema = Schema.Literal('error', 'warning', 'info');

export const RuleEntrySchema = Schema.Struct({
  id: Schema.String,
  language: LanguageSchema,
  family: RustRuleFamilySchema,
  severity: SeveritySchema,
  appliesTo: StringArray,
  triggers: StringArray,
  validator: Schema.String,
  doc: Schema.String,
});

export const RuleRegistrySchema = Schema.Struct({
  schemaVersion: Schema.Number,
  productName: Schema.String,
  languages: Schema.Array(LanguageSchema),
  rules: Schema.Array(RuleEntrySchema),
});

export const ConfigSchema = Schema.Struct({
  schemaVersion: OptionalNumber,
  profileName: OptionalString,
  failFast: OptionalBoolean,
  enforceWorkspaceFiles: OptionalBoolean,
  requireCargoDeny: OptionalBoolean,
  requireCargoAudit: OptionalBoolean,
  runCargoDoc: OptionalBoolean,
  cargoOnFileScope: OptionalBoolean,
  cargoOnDiffScope: OptionalBoolean,
  cargoTestThreads: OptionalNullableNumber,
  allowUnsafeCode: OptionalBoolean,
  allowBuildRs: OptionalBoolean,
  allowGitDependencies: OptionalBoolean,
  allowPathDependencies: OptionalBoolean,
  publicReexportPolicy: Schema.optional(Schema.Literal('forbid', 'facade-only', 'allow')),
  ignoreDirs: OptionalStringArray,
  ignoreFileGlobs: OptionalStringArray,
  rustRoots: OptionalStringArray,
  crateRootGlobs: OptionalStringArray,
  testFileGlobs: OptionalStringArray,
  rawTypeBoundaryGlobs: OptionalStringArray,
  facadeFileGlobs: OptionalStringArray,
  rawStringOwnerGlobs: OptionalStringArray,
  domainPrimitiveOwnerGlobs: OptionalStringArray,
  enforceRuntimeStringLiterals: OptionalBoolean,
  runtimeStringOwnerGlobs: OptionalStringArray,
  runtimeStringLineAllowPatterns: OptionalStringArray,
  enforceSerializedPublicDomainPrimitives: OptionalBoolean,
  serializedDomainOwnerGlobs: OptionalStringArray,
  blockedProtocolDependencies: Schema.optional(Schema.Record({ key: Schema.String, value: StringArray })),
  runtimeCrates: OptionalStringArray,
  testOnlyCrates: OptionalStringArray,
  allowedGitDependencies: OptionalStringArray,
});

export const ScopeNameSchema = Schema.Literal('workspace', 'files', 'crate', 'diff');

export const RouteRequestSchema = Schema.Struct({
  root: OptionalString,
  configPath: OptionalString,
  profile: OptionalString,
  scope: Schema.optional(ScopeNameSchema),
  files: OptionalStringArray,
  crateName: OptionalString,
  base: OptionalString,
  head: OptionalString,
  ruleId: OptionalString,
});

export const ScanToolArgumentsSchema = Schema.Struct({
  root: OptionalString,
  configPath: OptionalString,
  profile: OptionalString,
  scope: Schema.optional(ScopeNameSchema),
  files: OptionalStringArray,
  crateName: OptionalString,
  base: OptionalString,
  head: OptionalString,
  cargo: OptionalBoolean,
});

export const DoctorToolArgumentsSchema = Schema.Struct({
  root: OptionalString,
  configPath: OptionalString,
  profile: OptionalString,
  scope: Schema.optional(ScopeNameSchema),
  files: OptionalStringArray,
  crateName: OptionalString,
  base: OptionalString,
  head: OptionalString,
});

export const ExplainToolArgumentsSchema = Schema.Struct({
  ruleId: Schema.String,
});

export const AdapterNameSchema = Schema.Literal(
  'codex',
  'mcp',
  'precommit',
  'github-actions',
  'husky',
  'lefthook',
  'codeql',
  'dependency-policy',
  'secret-scan',
  'sbom'
);

export const InitRequestSchema = Schema.Struct({
  root: OptionalString,
  profile: OptionalString,
  adapters: Schema.optional(Schema.Array(AdapterNameSchema)),
  dryRun: OptionalBoolean,
  force: OptionalBoolean,
});

export const ViolationSchema = Schema.Struct({
  ruleId: Schema.String,
  title: Schema.String,
  detail: Schema.String,
  file: Schema.String,
  line: Schema.Number,
  snippet: Schema.String,
  source: Schema.optional(Schema.NullOr(Schema.String)),
});

export const ScopeReportSchema = Schema.Struct({
  mode: Schema.String,
  files: OptionalStringArray,
  crateName: OptionalString,
  crateRoot: OptionalString,
  manifest: OptionalString,
  base: OptionalString,
  head: OptionalString,
});

export const ScanReportSchema = Schema.Struct({
  ok: Schema.Boolean,
  command: Schema.String,
  violations: Schema.Array(ViolationSchema),
  root: Schema.String,
  profileName: Schema.String,
  scanOnly: Schema.Boolean,
  scope: ScopeReportSchema,
});

export const RoutedRuleSchema = Schema.Struct({
  id: Schema.String,
  family: RustRuleFamilySchema,
  severity: SeveritySchema,
  doc: Schema.String,
  validator: Schema.String,
});

export const RouteReportSchema = Schema.Struct({
  ok: Schema.Boolean,
  productName: Schema.String,
  profileName: Schema.String,
  index: Schema.String,
  scope: Schema.Unknown,
  docs: StringArray,
  rules: Schema.Array(RoutedRuleSchema),
});

export function decodeRuleRegistry(value) {
  return decodeWithSchema(RuleRegistrySchema, value, 'rule registry');
}

export function decodeEnforcerConfig(value) {
  return decodeWithSchema(ConfigSchema, value, 'enforcer config');
}

export function decodeRouteRequest(value) {
  return decodeWithSchema(RouteRequestSchema, value, 'route request');
}

export function decodeScanToolArguments(value) {
  return decodeWithSchema(ScanToolArgumentsSchema, value, 'scan tool arguments');
}

export function decodeDoctorToolArguments(value) {
  return decodeWithSchema(DoctorToolArgumentsSchema, value, 'doctor tool arguments');
}

export function decodeExplainToolArguments(value) {
  return decodeWithSchema(ExplainToolArgumentsSchema, value, 'explain tool arguments');
}

export function decodeInitRequest(value) {
  return decodeWithSchema(InitRequestSchema, value, 'init request');
}

export function decodeScanReport(value) {
  return decodeWithSchema(ScanReportSchema, value, 'scan report');
}

export function decodeRouteReport(value) {
  return decodeWithSchema(RouteReportSchema, value, 'route report');
}

export function decodeWithSchema(schema, value, label) {
  try {
    return Schema.decodeUnknownSync(schema)(value);
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    throw new Error(`${label} schema validation failed: ${message}`);
  }
}
