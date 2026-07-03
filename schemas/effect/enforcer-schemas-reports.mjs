import {
  Schema,
  StringArray,
  OptionalStringArray,
  OptionalString,
  OptionalBoolean,
} from "./enforcer-schemas-core.mjs";
import {
  LanguageSchema,
  RuleFamilySchema,
  SeveritySchema,
} from "./enforcer-schemas-rules.mjs";
import {
  ProofCollectorSchema,
  ProofFamilySchema,
  ProofRetentionPolicySchema,
  ProofStatusSchema,
} from "./enforcer-schemas-proof.mjs";
import { CheckNameSchema } from "./enforcer-schemas-tools.mjs";

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

export const CoordinationHealthReportSchema = Schema.Struct({
  ok: Schema.Boolean,
  root: Schema.String,
  canInspect: Schema.Boolean,
  canLockPaths: Schema.Boolean,
  canWriteClaimedPaths: Schema.Boolean,
  mustWait: Schema.Boolean,
  mustRepairLedger: Schema.Boolean,
  diagnostics: Schema.Array(Schema.Unknown),
  warnings: Schema.Array(Schema.Unknown),
  conflicts: Schema.Array(Schema.Unknown),
  hardConflicts: Schema.optional(Schema.Array(Schema.Unknown)),
  branchWriteConflicts: Schema.optional(Schema.Array(Schema.Unknown)),
  mergeRisks: Schema.optional(Schema.Array(Schema.Unknown)),
  globalWriteConflicts: Schema.optional(Schema.Array(Schema.Unknown)),
  editIntents: Schema.optional(Schema.Array(Schema.Unknown)),
  staleSessions: Schema.Array(Schema.Unknown),
  guard: Schema.optional(Schema.NullOr(Schema.Unknown)),
  dashboard: Schema.Unknown,
  presence: Schema.optional(Schema.Unknown),
});

export const CoordinationPresenceReportSchema = Schema.Struct({
  ok: Schema.Boolean,
  root: Schema.String,
  generatedAt: Schema.String,
  totalRows: Schema.Number,
  rows: Schema.Array(Schema.Unknown),
  views: Schema.Unknown,
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

export const ProofArtifactSchema = Schema.Struct({
  name: Schema.String,
  kind: Schema.String,
  path: Schema.String,
  sha256: Schema.String,
  byteLength: Schema.Number,
});

export const ProofDiagnosticSchema = Schema.Struct({
  runId: Schema.String,
  proofId: Schema.String,
  severity: SeveritySchema,
  ruleId: Schema.String,
  message: Schema.String,
  file: Schema.String,
  line: Schema.Number,
});

export const ProofRunSchema = Schema.Struct({
  schemaVersion: Schema.Number,
  proofId: Schema.String,
  title: Schema.String,
  family: ProofFamilySchema,
  collector: ProofCollectorSchema,
  profile: Schema.String,
  root: Schema.String,
  runId: Schema.String,
  status: ProofStatusSchema,
  ok: Schema.Boolean,
  exitCode: Schema.Number,
  startedAt: Schema.String,
  endedAt: Schema.String,
  command: Schema.Array(Schema.String),
  diagnosticCount: Schema.Number,
  pinned: Schema.Boolean,
  git: Schema.Unknown,
  scope: Schema.Unknown,
  claimsProved: OptionalStringArray,
  claimsNotProved: OptionalStringArray,
  retention: ProofRetentionPolicySchema,
  artifacts: Schema.Array(ProofArtifactSchema),
  harness: Schema.optional(Schema.Unknown),
});

export const ProofRunReportSchema = Schema.Struct({
  ok: Schema.Boolean,
  proofRun: ProofRunSchema,
  diagnostics: Schema.Array(Schema.Unknown),
});

export const ProofClaimReportSchema = Schema.Struct({
  ok: Schema.Boolean,
  root: Schema.String,
  claim: Schema.Unknown,
});
