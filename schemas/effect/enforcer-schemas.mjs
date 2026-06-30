import { Schema } from "effect";

export const ProductName = "ocentra-enforcer";

const StringArray = Schema.Array(Schema.String);
const OptionalStringArray = Schema.optional(StringArray);
const OptionalBoolean = Schema.optional(Schema.Boolean);
const OptionalString = Schema.optional(Schema.String);
const OptionalNumber = Schema.optional(Schema.Number);
const OptionalNullableNumber = Schema.optional(Schema.NullOr(Schema.Number));

export const LanguageSchema = Schema.Literal(
  "rust",
  "typescript",
  "python",
  "common",
);
export const RustRuleFamilySchema = Schema.Literal(
  "source",
  "domain",
  "imports-modules",
  "toolchain-cargo",
  "dependencies",
  "async-runtime",
);
export const TypeScriptRuleFamilySchema = Schema.Literal(
  "source",
  "tests",
  "toolchain",
);
export const PythonRuleFamilySchema = Schema.Literal(
  "source",
  "tests",
  "toolchain",
);
export const CommonRuleFamilySchema = Schema.Literal(
  "source",
  "security",
  "generated-artifacts",
  "harness",
  "documentation",
  "tests",
  "portability",
  "source-shape",
  "contracts",
  "dependencies",
  "sbom",
  "agent-rules",
);
export const RuleFamilySchema = Schema.Union(
  RustRuleFamilySchema,
  TypeScriptRuleFamilySchema,
  PythonRuleFamilySchema,
  CommonRuleFamilySchema,
);
export const SeveritySchema = Schema.Literal("error", "warning", "info");

export const PolicyOverrideSchema = Schema.Struct({
  enabled: OptionalBoolean,
  severity: Schema.optional(SeveritySchema),
  note: OptionalString,
});

export const SourceShapePolicySchema = Schema.Struct({
  roots: OptionalStringArray,
  extensions: OptionalStringArray,
  kind: Schema.optional(Schema.Literal("typescript", "rust", "python")),
  maxClasses: OptionalNumber,
  maxExports: OptionalNumber,
  maxFunctionLines: OptionalNumber,
  maxFunctions: OptionalNumber,
  maxLines: OptionalNumber,
  maxTypes: OptionalNumber,
});

export const SourceShapeOverrideSchema = Schema.Struct({
  path: OptionalString,
  paths: OptionalStringArray,
  glob: OptionalString,
  globs: OptionalStringArray,
  note: OptionalString,
  maxClasses: OptionalNumber,
  maxExports: OptionalNumber,
  maxFunctionLines: OptionalNumber,
  maxFunctions: OptionalNumber,
  maxLines: OptionalNumber,
  maxTypes: OptionalNumber,
});

export const ImportBoundaryPolicySchema = Schema.Struct({
  roots: OptionalStringArray,
  forbiddenImports: OptionalStringArray,
  allowedImports: OptionalStringArray,
  message: OptionalString,
});

export const RuleEntrySchema = Schema.Struct({
  id: Schema.String,
  language: LanguageSchema,
  family: RuleFamilySchema,
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
  failOn: OptionalStringArray,
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
  publicReexportPolicy: Schema.optional(
    Schema.Literal("forbid", "facade-only", "allow"),
  ),
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
  blockedProtocolDependencies: Schema.optional(
    Schema.Record({ key: Schema.String, value: StringArray }),
  ),
  runtimeCrates: OptionalStringArray,
  testOnlyCrates: OptionalStringArray,
  allowedGitDependencies: OptionalStringArray,
  allowedExternalLicenses: OptionalStringArray,
  sourceShapePolicies: Schema.optional(Schema.Array(SourceShapePolicySchema)),
  sourceShapeOverrides: Schema.optional(
    Schema.Array(SourceShapeOverrideSchema),
  ),
  importBoundaryPolicies: Schema.optional(
    Schema.Array(ImportBoundaryPolicySchema),
  ),
  architecturePolicyChecks: OptionalStringArray,
  singleSourceRequiredMirrorRoots: OptionalStringArray,
  strictEmptyTestTrees: OptionalBoolean,
  generatedArtifactsMode: Schema.optional(Schema.Literal("scan", "tracked")),
  generatedArtifactsTracked: OptionalBoolean,
  agentRuleMaxLines: OptionalNumber,
  languages: OptionalStringArray,
  rules: Schema.optional(
    Schema.Record({ key: Schema.String, value: PolicyOverrideSchema }),
  ),
  tools: Schema.optional(
    Schema.Record({ key: Schema.String, value: PolicyOverrideSchema }),
  ),
  harness: Schema.optional(
    Schema.Struct({
      store: Schema.optional(Schema.Literal("ndjson-duckdb", "ndjson-only")),
      storageDir: OptionalString,
      maxArtifactBytes: OptionalNumber,
      maxRuns: OptionalNullableNumber,
      maxRunsPerTool: OptionalNullableNumber,
      maxFailedRuns: OptionalNullableNumber,
      pruneAfterDays: OptionalNullableNumber,
    }),
  ),
});

export const ScopeNameSchema = Schema.Literal(
  "workspace",
  "files",
  "crate",
  "diff",
);

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
  diagnosticLimit: OptionalNumber,
  summaryOnly: OptionalBoolean,
  groupBy: Schema.optional(Schema.Literal("file", "slice")),
  includeScope: OptionalBoolean,
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

export const CheckNameSchema = Schema.Literal(
  "no-zod-source",
  "no-naked-domain-strings",
  "no-test-doubles",
  "weak-assertions",
  "skipped-focused-tests",
  "validation-bypass",
  "placeholder-implementation",
  "reexports",
  "cross-platform-script-commands",
  "generated-artifacts",
  "secrets",
  "rust-string-boundaries",
  "source-shape",
  "required-tests",
  "single-source-contracts",
  "dependency-policy",
  "sbom",
  "ai-rule-index",
  "import-boundaries",
  "architecture-policy",
);

export const CheckToolArgumentsSchema = Schema.Struct({
  root: OptionalString,
  configPath: OptionalString,
  profile: OptionalString,
  check: CheckNameSchema,
  scope: Schema.optional(ScopeNameSchema),
  files: OptionalStringArray,
  crateName: OptionalString,
  base: OptionalString,
  head: OptionalString,
  checkConfigPath: OptionalString,
  output: OptionalString,
  dryRun: OptionalBoolean,
  staged: OptionalBoolean,
  tracked: OptionalBoolean,
  strictEmptyTestTrees: OptionalBoolean,
  diagnosticLimit: OptionalNumber,
  summaryOnly: OptionalBoolean,
  groupBy: Schema.optional(Schema.Literal("file", "slice")),
  includeScope: OptionalBoolean,
});

export const AdapterNameSchema = Schema.Literal(
  "codex",
  "mcp",
  "precommit",
  "github-actions",
  "husky",
  "lefthook",
  "codeql",
  "dependency-policy",
  "secret-scan",
  "sbom",
);

export const InitRequestSchema = Schema.Struct({
  root: OptionalString,
  profile: OptionalString,
  adapters: Schema.optional(Schema.Array(AdapterNameSchema)),
  dryRun: OptionalBoolean,
  force: OptionalBoolean,
});

export const CodexInstallRequestSchema = Schema.Struct({
  root: OptionalString,
  profile: OptionalString,
  dryRun: OptionalBoolean,
  force: OptionalBoolean,
  codexConfigPath: OptionalString,
  serverName: OptionalString,
});

export const CodexDoctorRequestSchema = Schema.Struct({
  root: OptionalString,
  codexConfigPath: OptionalString,
  serverName: OptionalString,
});

export const ViolationSchema = Schema.Struct({
  ruleId: Schema.String,
  severity: Schema.optional(SeveritySchema),
  title: Schema.String,
  detail: Schema.String,
  file: Schema.String,
  line: Schema.Number,
  snippet: Schema.String,
  doc: Schema.String,
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
  warnings: Schema.optional(Schema.Array(ViolationSchema)),
  findings: Schema.optional(Schema.Array(ViolationSchema)),
  bySeverity: Schema.optional(
    Schema.Record({ key: Schema.String, value: Schema.Number }),
  ),
  failOn: OptionalStringArray,
  root: Schema.String,
  profileName: Schema.String,
  scanOnly: Schema.Boolean,
  scope: ScopeReportSchema,
});

export const CheckReportSchema = Schema.Struct({
  ok: Schema.Boolean,
  command: Schema.Literal("check"),
  check: CheckNameSchema,
  root: Schema.String,
  profileName: Schema.String,
  violations: Schema.Array(ViolationSchema),
  warnings: Schema.optional(Schema.Array(ViolationSchema)),
  findings: Schema.optional(Schema.Array(ViolationSchema)),
  bySeverity: Schema.optional(
    Schema.Record({ key: Schema.String, value: Schema.Number }),
  ),
  scope: Schema.optional(ScopeReportSchema),
  languages: OptionalStringArray,
  checks: Schema.optional(
    Schema.Array(
      Schema.Struct({
        check: Schema.String,
        ok: Schema.Boolean,
        violations: Schema.Number,
      }),
    ),
  ),
});

export const RoutedRuleSchema = Schema.Struct({
  id: Schema.String,
  language: LanguageSchema,
  family: RuleFamilySchema,
  severity: SeveritySchema,
  enabled: OptionalBoolean,
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

export const DiagnosticSchema = Schema.Struct({
  runId: Schema.String,
  tool: Schema.String,
  language: LanguageSchema,
  severity: SeveritySchema,
  ruleId: Schema.String,
  file: Schema.String,
  line: Schema.Number,
  message: Schema.String,
  source: Schema.optional(Schema.NullOr(Schema.String)),
  fingerprint: OptionalString,
});

export const RunToolArgumentsSchema = Schema.Struct({
  root: OptionalString,
  profile: OptionalString,
  tool: OptionalString,
  language: Schema.optional(LanguageSchema),
  cwd: OptionalString,
  runId: OptionalString,
  crateName: OptionalString,
  packageName: OptionalString,
  domain: OptionalString,
  command: Schema.Array(Schema.String),
  tags: OptionalStringArray,
});

export const RunQueryArgumentsSchema = Schema.Struct({
  root: OptionalString,
  runId: OptionalString,
  limit: OptionalNumber,
  diagnosticLimit: OptionalNumber,
  severity: Schema.optional(SeveritySchema),
  status: Schema.optional(Schema.Literal("passed", "failed")),
  file: OptionalString,
  tool: OptionalString,
  crateName: OptionalString,
  packageName: OptionalString,
  domain: OptionalString,
  tag: OptionalString,
  artifact: OptionalString,
  limitBytes: OptionalNumber,
});

export const RunSummarySchema = Schema.Struct({
  runId: Schema.String,
  root: Schema.String,
  profile: Schema.String,
  tool: Schema.String,
  language: LanguageSchema,
  cwd: Schema.String,
  crateName: Schema.optional(Schema.NullOr(Schema.String)),
  packageName: Schema.optional(Schema.NullOr(Schema.String)),
  domain: Schema.optional(Schema.NullOr(Schema.String)),
  tags: OptionalStringArray,
  command: Schema.Array(Schema.String),
  status: Schema.Literal("passed", "failed"),
  exitCode: Schema.Number,
  startedAt: Schema.String,
  endedAt: Schema.String,
  diagnosticCount: Schema.Number,
  bySeverity: Schema.Record({ key: Schema.String, value: Schema.Number }),
  artifacts: Schema.Record({ key: Schema.String, value: Schema.String }),
  duckdb: Schema.Unknown,
});

export const RunReportSchema = Schema.Struct({
  ok: Schema.Boolean,
  summary: RunSummarySchema,
  diagnostics: Schema.Array(DiagnosticSchema),
});

export function decodeRuleRegistry(value) {
  return decodeWithSchema(RuleRegistrySchema, value, "rule registry");
}

export function decodeEnforcerConfig(value) {
  return decodeWithSchema(ConfigSchema, value, "enforcer config");
}

export function decodeRouteRequest(value) {
  return decodeWithSchema(RouteRequestSchema, value, "route request");
}

export function decodeScanToolArguments(value) {
  return decodeWithSchema(
    ScanToolArgumentsSchema,
    value,
    "scan tool arguments",
  );
}

export function decodeDoctorToolArguments(value) {
  return decodeWithSchema(
    DoctorToolArgumentsSchema,
    value,
    "doctor tool arguments",
  );
}

export function decodeExplainToolArguments(value) {
  return decodeWithSchema(
    ExplainToolArgumentsSchema,
    value,
    "explain tool arguments",
  );
}

export function decodeCheckToolArguments(value) {
  return decodeWithSchema(
    CheckToolArgumentsSchema,
    value,
    "check tool arguments",
  );
}

export function decodeInitRequest(value) {
  return decodeWithSchema(InitRequestSchema, value, "init request");
}

export function decodeCodexInstallRequest(value) {
  return decodeWithSchema(
    CodexInstallRequestSchema,
    value,
    "codex install request",
  );
}

export function decodeCodexDoctorRequest(value) {
  return decodeWithSchema(
    CodexDoctorRequestSchema,
    value,
    "codex doctor request",
  );
}

export function decodeScanReport(value) {
  return decodeWithSchema(ScanReportSchema, value, "scan report");
}

export function decodeCheckReport(value) {
  return decodeWithSchema(CheckReportSchema, value, "check report");
}

export function decodeRouteReport(value) {
  return decodeWithSchema(RouteReportSchema, value, "route report");
}

export function decodeRunToolArguments(value) {
  return decodeWithSchema(RunToolArgumentsSchema, value, "run tool arguments");
}

export function decodeRunQueryArguments(value) {
  return decodeWithSchema(
    RunQueryArgumentsSchema,
    value,
    "run query arguments",
  );
}

export function decodeRunReport(value) {
  return decodeWithSchema(RunReportSchema, value, "run report");
}

export function decodeWithSchema(schema, value, label) {
  try {
    return Schema.decodeUnknownSync(schema)(value);
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    throw new Error(`${label} schema validation failed: ${message}`);
  }
}
