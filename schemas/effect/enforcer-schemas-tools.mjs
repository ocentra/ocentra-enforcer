import {
  Schema,
  OptionalStringArray,
  OptionalNumber,
  OptionalBoolean,
  OptionalString,
} from "./enforcer-schemas-core.mjs";
import {
  LanguageSchema,
  SeveritySchema,
} from "./enforcer-schemas-rules.mjs";
import {
  ProofCapabilitySchema,
  ProofStatusSchema,
} from "./enforcer-schemas-proof.mjs";

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
  "literal-risk",
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
  "rule-coverage",
  "policy-integrity",
  "config-lockdown",
  "waiver-policy",
  "docs-completeness",
  "ci-integrity",
  "repo-governance",
  "scanner-fixtures",
  "package-determinism",
  "mutation-risk",
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
  minScore: OptionalNumber,
  includeLow: OptionalBoolean,
  includeIgnored: OptionalBoolean,
  includeUnknownCode: OptionalBoolean,
  respectGitignore: OptionalBoolean,
  maxFileBytes: OptionalNumber,
  failAbove: OptionalNumber,
  hardCategories: OptionalStringArray,
  hardRuleIds: OptionalStringArray,
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
  ledgerRoot: OptionalString,
  serverName: OptionalString,
  installSkill: OptionalBoolean,
  installGlobalAgents: OptionalBoolean,
});

export const CodexUninstallRequestSchema = Schema.Struct({
  codexConfigPath: OptionalString,
  serverName: OptionalString,
  removeSkill: OptionalBoolean,
  removeGlobalAgents: OptionalBoolean,
  dryRun: OptionalBoolean,
});

export const CodexDoctorRequestSchema = Schema.Struct({
  root: OptionalString,
  codexConfigPath: OptionalString,
  serverName: OptionalString,
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

export const ProofRouteRequestSchema = Schema.Struct({
  root: OptionalString,
  profile: OptionalString,
  scope: Schema.optional(ScopeNameSchema),
  files: OptionalStringArray,
  plan: OptionalString,
  capability: Schema.optional(ProofCapabilitySchema),
  proofId: OptionalString,
});

export const ProofRunArgumentsSchema = Schema.Struct({
  root: OptionalString,
  profile: OptionalString,
  proofId: OptionalString,
  files: OptionalStringArray,
  plan: OptionalString,
  capability: Schema.optional(ProofCapabilitySchema),
  runId: OptionalString,
  command: Schema.optional(Schema.Array(Schema.String)),
  tags: OptionalStringArray,
  pin: OptionalBoolean,
});

export const ProofQueryArgumentsSchema = Schema.Struct({
  root: OptionalString,
  profile: OptionalString,
  proofId: OptionalString,
  runId: OptionalString,
  status: Schema.optional(ProofStatusSchema),
  artifact: OptionalString,
  legacyPaths: OptionalStringArray,
  dryRun: OptionalBoolean,
  limit: OptionalNumber,
  diagnosticLimit: OptionalNumber,
  limitBytes: OptionalNumber,
  includeScripts: OptionalBoolean,
  includeAllScripts: OptionalBoolean,
  scriptRoot: OptionalString,
  write: OptionalBoolean,
});

export const ProofClaimArgumentsSchema = Schema.Struct({
  root: OptionalString,
  profile: OptionalString,
  proofId: OptionalString,
  proofIds: OptionalStringArray,
  claimId: OptionalString,
  prReady: OptionalBoolean,
  allowDirty: OptionalBoolean,
});

export const CoordinationToolArgumentsSchema = Schema.Struct({
  root: OptionalString,
  stateRoot: OptionalString,
  hub: OptionalString,
  lane: OptionalString,
  from: OptionalString,
  to: OptionalString,
  subject: OptionalString,
  body: OptionalString,
  message: OptionalString,
  messageId: OptionalString,
  paths: OptionalStringArray,
  changedPaths: OptionalStringArray,
  reason: OptionalString,
  summary: OptionalString,
  owner: OptionalString,
  operation: Schema.optional(
    Schema.Literal(
      "inspect",
      "edit",
      "commit",
      "push",
      "rebase",
      "merge",
      "pr_ready",
    ),
  ),
  lockKind: Schema.optional(
    Schema.Literal(
      "writeLock",
      "globalWriteLock",
      "branchLease",
      "workReservation",
    ),
  ),
  onConflict: Schema.optional(Schema.Literal("fail", "intent")),
  claimGroup: OptionalString,
  waitMs: OptionalNumber,
  taskId: OptionalString,
  state: OptionalString,
  sessionId: OptionalString,
  action: OptionalString,
  peer: OptionalString,
  peerUrl: OptionalString,
  url: OptionalString,
  name: OptionalString,
  token: OptionalString,
  tokenEnv: OptionalString,
  mode: Schema.optional(Schema.Literal("pull", "push", "both")),
  host: OptionalString,
  port: OptionalNumber,
  keepLatest: OptionalNumber,
  projectId: OptionalString,
  repoRoot: OptionalString,
  worktreeRoot: OptionalString,
  cwd: OptionalString,
  gitRemote: OptionalString,
  branch: OptionalString,
  commit: OptionalString,
  codexThreadId: OptionalString,
  codexSessionId: OptionalString,
  stateFile: OptionalString,
  peek: OptionalBoolean,
  dryRun: OptionalBoolean,
  write: OptionalBoolean,
  focused: OptionalBoolean,
  allowPrimaryWithoutClaims: OptionalBoolean,
  allowMergeRisks: OptionalBoolean,
  all: OptionalBoolean,
  allOwned: OptionalBoolean,
  allLanes: OptionalBoolean,
  allowOtherNode: OptionalBoolean,
  releaseOwned: OptionalBoolean,
  repairStale: OptionalBoolean,
  limit: OptionalNumber,
});
